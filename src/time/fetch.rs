use anyhow::Result;
use chrono::{NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;

use crate::config::{ContractPeriod, TIMEDOCTOR_COMPANY_ID};
use crate::ctx::Paths;

use super::api;
use super::cache;
use super::compute::{
    build_week_row, get_week_end_sunday, get_week_start_monday, is_past_week, WeekRow,
};

#[derive(Clone)]
pub struct FetchOpts {
    pub timezone: String,
    pub contract_periods: Vec<ContractPeriod>,
    pub no_cache: bool,
    pub stats_url: String,
}

pub async fn fetch_week(
    paths: &Paths,
    client: &reqwest::Client,
    cookie: &str,
    monday: NaiveDate,
    opts: &FetchOpts,
) -> Result<WeekRow> {
    let company_id = TIMEDOCTOR_COMPANY_ID;
    let past = is_past_week(monday);

    if past && !opts.no_cache {
        if let Some(stats) = cache::read_week_cache(paths, company_id, &opts.timezone, monday) {
            return Ok(build_week_row(monday, &stats, &opts.contract_periods, true));
        }
    }

    let sunday = get_week_end_sunday(monday);
    let tz: Tz = opts.timezone.parse().map_err(|_| {
        anyhow::anyhow!(
            "Invalid timezone '{}' — set a valid IANA name (e.g. Europe/Istanbul) in Settings → Time Doctor",
            opts.timezone
        )
    })?;
    let monday_midnight = monday
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid midnight for {monday}"))?;
    let sunday_eod = sunday
        .and_hms_opt(23, 59, 59)
        .ok_or_else(|| anyhow::anyhow!("Invalid end-of-day for {sunday}"))?;
    let from_dt = tz
        .from_local_datetime(&monday_midnight)
        .earliest()
        .map(|dt| dt.with_timezone(&Utc))
        .ok_or_else(|| {
            anyhow::anyhow!("Could not convert {} to timezone {}", monday, opts.timezone)
        })?;
    let to_dt = tz
        .from_local_datetime(&sunday_eod)
        .latest()
        .map(|dt| dt.with_timezone(&Utc))
        .ok_or_else(|| {
            anyhow::anyhow!("Could not convert {} to timezone {}", sunday, opts.timezone)
        })?;

    let stats = api::get_week_stats(
        client,
        &opts.stats_url,
        cookie,
        from_dt,
        to_dt,
        company_id,
        &opts.timezone,
    )
    .await?;

    if past {
        cache::write_week_cache(paths, company_id, &opts.timezone, monday, &stats);
    }

    Ok(build_week_row(
        monday,
        &stats,
        &opts.contract_periods,
        false,
    ))
}

/// Single source of truth for CLI (`time::run`) and TUI (`App::launch_td_report`).
pub fn split_cached_weeks(
    paths: &Paths,
    mondays: &[NaiveDate],
    timezone: &str,
    contract_periods: &[ContractPeriod],
    no_cache: bool,
) -> (Vec<WeekRow>, Vec<NaiveDate>) {
    if no_cache {
        return (Vec::new(), mondays.to_vec());
    }
    let today = chrono::Local::now().date_naive();
    let current_monday = get_week_start_monday(today);
    let mut cached = Vec::new();
    let mut uncached = Vec::new();
    for &monday in mondays {
        if monday < current_monday {
            if let Some(stats) =
                cache::read_week_cache(paths, TIMEDOCTOR_COMPANY_ID, timezone, monday)
            {
                cached.push(build_week_row(monday, &stats, contract_periods, true));
                continue;
            }
        }
        uncached.push(monday);
    }
    (cached, uncached)
}
