pub mod api;
pub mod auth;
pub mod cache;
pub mod compute;
pub mod display;
pub mod fetch;
pub mod keychain;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal;
use std::sync::Arc;

use crate::cli::TimeArgs;
use crate::config;
use crate::ctx::Ctx;
use crate::error::AppError;
use crate::time::keychain::KeychainStore;
use compute::WeekRow;
pub use fetch::{fetch_week, split_cached_weeks, FetchOpts};

pub async fn run(ctx: &Ctx, args: TimeArgs) -> Result<()> {
    let mut cfg = config::load(&ctx.paths)?;
    config::ensure_time_credentials(&ctx.paths, &mut cfg)?;

    let time_cfg = &cfg.time;
    let email = time_cfg
        .email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("No TimeDoctor email configured"))?;
    let timezone = time_cfg
        .timezone
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("No timezone configured"))?;
    let contract_periods = time_cfg.contract_periods.as_deref().unwrap_or(&[]);
    let start_date = time_cfg
        .start_date
        .or_else(|| contract_periods.first().map(|p| p.from))
        .ok_or_else(|| anyhow::anyhow!("No start date or contract periods configured"))?;
    let skip_current = args.skip_current_week || time_cfg.skip_current_week;
    let reset_from = time_cfg.reset_cumulative_from_date;

    let opts = FetchOpts {
        timezone: timezone.to_string(),
        contract_periods: contract_periods.to_vec(),
        no_cache: args.no_cache,
        stats_url: ctx.http.stats.clone(),
    };

    let mondays = compute::weeks_to_fetch(start_date, skip_current);
    if mondays.is_empty() {
        println!("Total Weekly: no weeks");
        return Ok(());
    }

    let (cached_rows, uncached_mondays) = split_cached_weeks(
        &ctx.paths,
        &mondays,
        &opts.timezone,
        &opts.contract_periods,
        args.no_cache,
    );

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<Vec<WeekRow>>>();
    let fetch_handle: Option<tokio::task::JoinHandle<()>> = if uncached_mondays.is_empty() {
        let _ = tx.send(Ok(vec![]));
        None
    } else {
        let email_owned = email.to_string();
        let opts_clone = opts.clone();
        let paths_clone = ctx.paths.clone();
        let keychain_clone = ctx.keychain.clone();
        let http_clone = ctx.http.clone();
        Some(tokio::spawn(async move {
            let _ = tx.send(
                fetch_weeks_parallel(
                    &paths_clone,
                    keychain_clone,
                    &http_clone,
                    &email_owned,
                    uncached_mondays,
                    opts_clone,
                )
                .await,
            );
        }))
    };

    let new_rows = run_spinner(rx, fetch_handle).await?;
    if new_rows.is_none() {
        return Ok(());
    }
    let (new_rows, elapsed) = new_rows.unwrap();

    let mut rows = cached_rows;
    rows.extend(new_rows);
    compute::compute_cumulative(&mut rows, reset_from);

    let newest = rows.iter().max_by_key(|r| r.monday);
    let weekly = newest.map(|r| r.balance_hours).unwrap_or(0.0);
    let cumulative = newest.map(|r| r.cumulative_hours).unwrap_or(0.0);
    display::print_final(weekly, cumulative, elapsed);

    Ok(())
}

async fn run_spinner(
    rx: tokio::sync::oneshot::Receiver<Result<Vec<WeekRow>>>,
    fetch_handle: Option<tokio::task::JoinHandle<()>>,
) -> Result<Option<(Vec<WeekRow>, f64)>> {
    let start = std::time::Instant::now();
    let _ = terminal::enable_raw_mode();
    display::hide_cursor();

    let mut tick: usize = 0;
    let mut rx = rx;
    let result = loop {
        display::print_progress(tick, start.elapsed().as_secs_f64());

        while event::poll(std::time::Duration::ZERO).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    if let Some(h) = fetch_handle {
                        h.abort();
                    }
                    let _ = terminal::disable_raw_mode();
                    display::show_cursor();
                    println!();
                    return Ok(None);
                }
            }
        }

        tokio::select! {
            res = &mut rx => {
                break res.map_err(|_| anyhow::anyhow!("fetch task dropped"))?;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                tick += 1;
            }
        }
    };
    let _ = terminal::disable_raw_mode();
    display::show_cursor();

    let rows = result?;
    Ok(Some((rows, start.elapsed().as_secs_f64())))
}

pub async fn fetch_weeks_parallel(
    paths: &crate::ctx::Paths,
    keychain: Arc<dyn KeychainStore>,
    http: &crate::ctx::HttpBase,
    email: &str,
    mondays: Vec<chrono::NaiveDate>,
    opts: FetchOpts,
) -> Result<Vec<WeekRow>> {
    if mondays.is_empty() {
        return Ok(vec![]);
    }

    let mut auth_cookie = auth::get_or_refresh_token(keychain.clone(), http, email).await?;
    let client = api::shared_client();

    let results = futures::future::join_all(
        mondays
            .iter()
            .map(|&m| fetch_week(paths, client, &auth_cookie, m, &opts)),
    )
    .await;

    let mut rows = Vec::with_capacity(mondays.len());
    let mut retry_mondays = Vec::new();

    for (monday, result) in mondays.iter().zip(results) {
        match result {
            Ok(row) => rows.push(row),
            Err(e)
                if e.downcast_ref::<AppError>()
                    .map(|ae| matches!(ae, AppError::TokenExpired))
                    .unwrap_or(false) =>
            {
                retry_mondays.push(*monday);
            }
            Err(e) => return Err(e.context(format!("week of {monday}"))),
        }
    }

    if !retry_mondays.is_empty() {
        auth_cookie = auth::reauth(keychain.clone(), http, email).await?;
        let retry_results = futures::future::join_all(
            retry_mondays
                .iter()
                .map(|&m| fetch_week(paths, client, &auth_cookie, m, &opts)),
        )
        .await;
        for (monday, result) in retry_mondays.iter().zip(retry_results) {
            rows.push(result.map_err(|e| e.context(format!("week of {monday}")))?);
        }
    }

    Ok(rows)
}
