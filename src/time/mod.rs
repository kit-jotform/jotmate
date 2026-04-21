pub mod api;
pub mod auth;
pub mod cache;
pub mod compute;
pub mod fetch;

use anyhow::Result;

use crate::cli::TimeArgs;
use crate::config;
use crate::error::AppError;
use compute::WeekRow;
pub use fetch::{fetch_week, FetchOpts};

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
const GREEN: &str = "\x1b[92m";
const RED: &str = "\x1b[91m";
const CYAN: &str = "\x1b[96m";
const MUTED: &str = "\x1b[90m";
const RESET: &str = "\x1b[0m";

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

    // Spawn the parallel fetch and animate a spinner on the same line.
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<Vec<WeekRow>>>();
    let email_owned = email.to_string();
    let mondays_clone = mondays.clone();
    let opts_clone = opts.clone();
    tokio::spawn(async move {
        let _ = tx.send(fetch_weeks_parallel(&email_owned, mondays_clone, opts_clone).await);
    });

    // Animate spinner until result arrives.
    let mut tick: usize = 0;
    let mut rx = rx;
    let result = loop {
        let spinner_ch = SPINNER[tick % SPINNER.len()];
        print!("\r{MUTED}Total Weekly:{RESET} {CYAN}{spinner_ch}{RESET} ");
        use std::io::Write;
        let _ = std::io::stdout().flush();

        tokio::select! {
            res = &mut rx => {
                break res.map_err(|_| anyhow::anyhow!("fetch task dropped"))?;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                tick += 1;
            }
        }
    };

    let mut rows = result?;
    compute::compute_cumulative(&mut rows, reset_from);

    let total: f64 = rows.iter().map(|r| r.balance_hours).sum();
    let color = if total >= 0.0 { GREEN } else { RED };
    let value = compute::format_hours_signed(total);
    println!("\r{MUTED}Total Weekly:{RESET} {color}{value}{RESET}  ");

    Ok(())
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

    // Fetch all in parallel, collect results
    let results = futures::future::join_all(
        mondays
            .iter()
            .map(|&m| fetch_week(&client, &auth_cookie, m, &opts)),
    )
    .await;

    // Check if any failed with TokenExpired; if so reauth and retry the failed ones
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
