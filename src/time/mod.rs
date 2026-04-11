pub mod api;
pub mod auth;
pub mod cache;
pub mod compute;
pub mod display;
pub mod fetch;

use anyhow::Result;
use tokio::time::{sleep, Duration};

use crate::cli::TimeArgs;
use crate::config;
use crate::error::AppError;
use compute::WeekRow;
pub use fetch::{fetch_week, FetchOpts};

const BATCH_SIZE: usize = 10;
const BATCH_DELAY_MS: u64 = 500;

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
    let show_cumulative = time_cfg.show_cumulative;
    let reset_from = time_cfg.reset_cumulative_from_date;

    let opts = FetchOpts {
        timezone: timezone.to_string(),
        contract_periods: contract_periods.to_vec(),
        start_date,
        skip_current,
        no_cache: args.no_cache,
    };

    // Get auth token (with retry on 401)
    let cookie = auth::get_or_refresh_token(email).await?;

    let client = reqwest::Client::new();

    let mondays = compute::weeks_to_fetch(opts.start_date, opts.skip_current);
    if mondays.is_empty() {
        println!("No weeks to fetch.");
        return Ok(());
    }

    eprintln!(
        "Fetching {} weeks in batches of {}...",
        mondays.len(),
        BATCH_SIZE
    );

    let mut rows: Vec<WeekRow> = Vec::new();
    let mut auth_cookie = cookie;

    for (batch_idx, chunk) in mondays.chunks(BATCH_SIZE).enumerate() {
        if batch_idx > 0 {
            sleep(Duration::from_millis(BATCH_DELAY_MS)).await;
        }

        let batch_futures = chunk
            .iter()
            .map(|&monday| fetch_week(&client, &auth_cookie, monday, &opts));
        let results = futures::future::join_all(batch_futures).await;

        for result in results {
            match result {
                Ok(row) => rows.push(row),
                Err(e) => {
                    // Check if token expired, re-auth once
                    if e.downcast_ref::<AppError>()
                        .map(|ae| matches!(ae, AppError::TokenExpired))
                        .unwrap_or(false)
                    {
                        eprintln!("Session expired, re-authenticating...");
                        auth_cookie = auth::reauth(email).await?;
                        // Note: the failed weeks in this batch are skipped; user can re-run
                        eprintln!(
                            "Re-authenticated. Some weeks may be missing — re-run to fetch them."
                        );
                    } else {
                        eprintln!("Warning: {e}");
                    }
                }
            }
        }
    }

    compute::compute_cumulative(&mut rows, reset_from);
    display::print_results(&rows, show_cumulative);

    Ok(())
}

/// Fetch all weeks sequentially for the TUI. Bails on token expiry.
pub async fn fetch_report(email: &str, opts: FetchOpts) -> Result<Vec<WeekRow>> {
    let auth_cookie = auth::get_or_refresh_token(email).await?;
    let client = reqwest::Client::new();
    let mondays = compute::weeks_to_fetch(opts.start_date, opts.skip_current);

    let mut rows: Vec<WeekRow> = Vec::new();
    for monday in mondays {
        let row = fetch_week(&client, &auth_cookie, monday, &opts).await?;
        rows.push(row);
        // Small delay between API calls to avoid hammering
        sleep(Duration::from_millis(50)).await;
    }
    Ok(rows)
}
