pub mod api;
pub mod auth;
pub mod cache;
pub mod compute;
pub mod display;

use anyhow::Result;
use chrono::{TimeZone, Utc};
use tokio::time::{sleep, Duration};

use crate::cli::TimeArgs;
use crate::config;
use crate::error::AppError;
use compute::{
    format_week_range, get_target_hours, get_week_end_sunday, is_past_week, weeks_to_fetch, WeekRow,
};

const BATCH_SIZE: usize = 10;
const BATCH_DELAY_MS: u64 = 500;

pub async fn run(args: TimeArgs) -> Result<()> {
    let mut cfg = config::load()?;
    config::ensure_time_credentials(&mut cfg)?;

    let time_cfg = &cfg.time;
    let email = time_cfg.email.as_deref().unwrap();
    let company_id = crate::config::TIMEDOCTOR_COMPANY_ID;
    let timezone = time_cfg.timezone.as_deref().unwrap();
    let start_date = time_cfg.start_date.unwrap();
    let skip_current = args.skip_current_week || time_cfg.skip_current_week;
    let reset_from = time_cfg.reset_cumulative_from_date;
    let contract_periods = time_cfg.contract_periods.as_deref().unwrap_or(&[]);

    // Get auth token (with retry on 401)
    let cookie = auth::get_or_refresh_token(email).await?;

    let client = reqwest::Client::new();

    let mondays = weeks_to_fetch(start_date, skip_current);
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

        let mut batch_futures = Vec::new();
        for &monday in chunk {
            batch_futures.push(fetch_week(
                &client,
                &auth_cookie,
                monday,
                company_id,
                timezone,
                contract_periods,
                args.no_cache,
            ));
        }

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
                        auth::delete_token_from_keychain()?;
                        auth_cookie = auth::get_or_refresh_token(email).await?;
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
    display::print_results(&rows);

    Ok(())
}

async fn fetch_week(
    client: &reqwest::Client,
    cookie: &str,
    monday: chrono::NaiveDate,
    company_id: &str,
    timezone: &str,
    contract_periods: &[crate::config::ContractPeriod],
    no_cache: bool,
) -> Result<WeekRow> {
    let week_label = format_week_range(monday);
    let past = is_past_week(monday);

    // Try cache for past weeks
    if past && !no_cache {
        if let Some(stats) = cache::read_week_cache(company_id, monday) {
            let worked_secs = stats.data.first().map(|d| d.total).unwrap_or(0);
            let target_hours = get_target_hours(monday, contract_periods);
            let balance_hours = (worked_secs as f64 / 3600.0) - target_hours;
            return Ok(WeekRow {
                monday,
                week_label,
                worked_secs,
                target_hours,
                balance_hours,
                cumulative_hours: 0.0,
                from_cache: true,
            });
        }
    }

    let sunday = get_week_end_sunday(monday);
    let from_dt = Utc.from_utc_datetime(&monday.and_hms_opt(0, 0, 0).unwrap());
    let to_dt = Utc.from_utc_datetime(&sunday.and_hms_opt(23, 59, 59).unwrap());

    let stats = api::get_week_stats(client, cookie, from_dt, to_dt, company_id, timezone).await?;

    if past {
        cache::write_week_cache(company_id, monday, &stats);
    }

    let worked_secs = stats.data.first().map(|d| d.total).unwrap_or(0);
    let target_hours = get_target_hours(monday, contract_periods);
    let balance_hours = (worked_secs as f64 / 3600.0) - target_hours;

    Ok(WeekRow {
        monday,
        week_label,
        worked_secs,
        target_hours,
        balance_hours,
        cumulative_hours: 0.0,
        from_cache: false,
    })
}
