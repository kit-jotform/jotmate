pub mod api;
pub mod auth;
pub mod cache;
pub mod compute;
pub mod display;
pub mod fetch;
pub mod keychain;

use anyhow::Result;
use chrono::NaiveDate;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal;

use crate::cli::TimeArgs;
use crate::config::{self, TIMEDOCTOR_COMPANY_ID};
use crate::error::AppError;
use compute::{build_week_row_from_cache, get_week_start_monday, WeekRow};
pub use fetch::{fetch_week, FetchOpts};

pub async fn run(args: TimeArgs) -> Result<()> {
    let mut cfg = config::load()?;
    config::ensure_time_credentials(&mut cfg)?;

    let time_cfg = &cfg.time;
    let email = time_cfg.email.as_deref().unwrap();
    let timezone = time_cfg.timezone.as_deref().unwrap();
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
    };

    let mondays = compute::weeks_to_fetch(start_date, skip_current);
    if mondays.is_empty() {
        println!("Total Weekly: no weeks");
        return Ok(());
    }

    // Split mondays into cached (compute cumulative now) and uncached (fetch in background).
    let today = chrono::Local::now().date_naive();
    let current_monday = get_week_start_monday(today);
    let mut cached_rows: Vec<WeekRow> = Vec::new();
    let mut uncached_mondays: Vec<NaiveDate> = Vec::new();

    if !args.no_cache {
        for &monday in &mondays {
            if monday < current_monday {
                if let Some(stats) = cache::read_week_cache(TIMEDOCTOR_COMPANY_ID, monday) {
                    cached_rows.push(build_week_row_from_cache(
                        monday,
                        &stats,
                        &opts.contract_periods,
                    ));
                    continue;
                }
            }
            uncached_mondays.push(monday);
        }
    } else {
        uncached_mondays = mondays.clone();
    }

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<Vec<WeekRow>>>();
    if uncached_mondays.is_empty() {
        let _ = tx.send(Ok(vec![]));
    } else {
        let email_owned = email.to_string();
        let opts_clone = opts.clone();
        tokio::spawn(async move {
            let _ = tx.send(fetch_weeks_parallel(&email_owned, uncached_mondays, opts_clone).await);
        });
    }

    let new_rows = run_spinner(rx).await?;
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

/// Animate spinner until fetch completes or user presses 'q'.
/// Returns `None` if the user quit, otherwise `Some((rows, elapsed_secs))`.
async fn run_spinner(
    rx: tokio::sync::oneshot::Receiver<Result<Vec<WeekRow>>>,
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

/// Fetch a specific set of weeks in parallel for the TUI.
/// Authenticates once, retries on token expiry, and returns all rows.
pub async fn fetch_weeks_parallel(
    email: &str,
    mondays: Vec<chrono::NaiveDate>,
    opts: FetchOpts,
) -> Result<Vec<WeekRow>> {
    if mondays.is_empty() {
        return Ok(vec![]);
    }

    let mut auth_cookie = auth::get_or_refresh_token(email).await?;
    let client = reqwest::Client::new();

    let results = futures::future::join_all(
        mondays
            .iter()
            .map(|&m| fetch_week(&client, &auth_cookie, m, &opts)),
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
            Err(e) => return Err(e),
        }
    }

    if !retry_mondays.is_empty() {
        auth_cookie = auth::reauth(email).await?;
        let retry_results = futures::future::join_all(
            retry_mondays
                .iter()
                .map(|&m| fetch_week(&client, &auth_cookie, m, &opts)),
        )
        .await;
        for result in retry_results {
            rows.push(result?);
        }
    }

    Ok(rows)
}
