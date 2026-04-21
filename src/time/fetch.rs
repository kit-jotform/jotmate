use anyhow::Result;
use chrono::{NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;

use crate::config::{ContractPeriod, TIMEDOCTOR_COMPANY_ID};

use super::api;
use super::cache;
use super::compute::{
    build_week_row_from_cache, format_week_range, get_target_hours, get_week_end_sunday,
    get_week_start_monday, is_past_week, WeekRow,
};

#[derive(Clone)]
pub struct FetchOpts {
    pub timezone: String,
    pub contract_periods: Vec<ContractPeriod>,
    pub no_cache: bool,
}

pub async fn fetch_week(
    client: &reqwest::Client,
    cookie: &str,
    monday: NaiveDate,
    opts: &FetchOpts,
) -> Result<WeekRow> {
    let company_id = TIMEDOCTOR_COMPANY_ID;
    let week_label = format_week_range(monday);
    let past = is_past_week(monday);

    if past && !opts.no_cache {
        if let Some(stats) = cache::read_week_cache(company_id, monday) {
            return Ok(build_row(
                monday,
                week_label,
                &stats,
                &opts.contract_periods,
                true,
            ));
        }
    }

    let sunday = get_week_end_sunday(monday);
    let tz: Tz = opts.timezone.parse().unwrap_or(chrono_tz::UTC);
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

    let stats =
        api::get_week_stats(client, cookie, from_dt, to_dt, company_id, &opts.timezone).await?;

    if past {
        cache::write_week_cache(company_id, monday, &stats);
    }

    Ok(build_row(
        monday,
        week_label,
        &stats,
        &opts.contract_periods,
        false,
    ))
}

/// Partition `mondays` into (cached WeekRows, still-uncached mondays) by
/// reading the per-week cache for every past week. When `no_cache` is true,
/// nothing is read from cache and every monday ends up in the uncached list.
///
/// This is the cache-split step shared by `time::run` (CLI) and
/// `App::launch_td_report` (TUI); keeping it in one place prevents the two
/// code paths from drifting.
pub fn split_cached_weeks(
    mondays: &[NaiveDate],
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
            if let Some(stats) = cache::read_week_cache(TIMEDOCTOR_COMPANY_ID, monday) {
                cached.push(build_week_row_from_cache(monday, &stats, contract_periods));
                continue;
            }
        }
        uncached.push(monday);
    }
    (cached, uncached)
}

fn build_row(
    monday: NaiveDate,
    week_label: String,
    stats: &api::StatsResponse,
    contract_periods: &[ContractPeriod],
    from_cache: bool,
) -> WeekRow {
    let worked_secs = stats.data.first().map(|d| d.total).unwrap_or(0);
    let target_hours = get_target_hours(monday, contract_periods);
    let balance_hours = (worked_secs as f64 / 3600.0) - target_hours;
    WeekRow {
        monday,
        week_label,
        worked_secs,
        target_hours,
        balance_hours,
        cumulative_hours: 0.0,
        from_cache,
    }
}
