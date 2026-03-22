use chrono::{Datelike, Local, NaiveDate};

use crate::config::ContractPeriod;

#[derive(Debug, Clone)]
pub struct WeekRow {
    pub monday: NaiveDate,
    pub week_label: String,
    pub worked_secs: u64,
    pub target_hours: f64,
    pub balance_hours: f64,
    pub cumulative_hours: f64,
    pub from_cache: bool,
}

pub fn get_week_start_monday(date: NaiveDate) -> NaiveDate {
    let days_from_monday = date.weekday().num_days_from_monday();
    date - chrono::Duration::days(days_from_monday as i64)
}

pub fn format_week_range(monday: NaiveDate) -> String {
    let sunday = monday + chrono::Duration::days(6);
    let month_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let m_start = month_names[(monday.month() - 1) as usize];
    let m_end = month_names[(sunday.month() - 1) as usize];
    format!(
        "{} {} - {} {}, {}",
        m_start,
        monday.day(),
        m_end,
        sunday.day(),
        sunday.year()
    )
}

pub fn get_target_hours(monday: NaiveDate, periods: &[ContractPeriod]) -> f64 {
    let mut applicable = periods.first().map(|p| p.weekly_hours).unwrap_or(0.0);
    for period in periods {
        if monday >= period.from {
            applicable = period.weekly_hours;
        } else {
            break; // sorted oldest-first
        }
    }
    applicable
}

pub fn format_hours(decimal_hours: f64) -> String {
    let sign = if decimal_hours < 0.0 { "-" } else { "" };
    let abs = decimal_hours.abs();
    let h = abs.floor() as u64;
    let m = ((abs - abs.floor()) * 60.0).round() as u64;
    if m == 0 {
        format!("{sign}{h}h")
    } else {
        format!("{sign}{h}h {m}m")
    }
}

/// Returns Mondays from start_date to today, newest first.
pub fn weeks_to_fetch(start_date: NaiveDate, skip_current_week: bool) -> Vec<NaiveDate> {
    let today = Local::now().date_naive();
    let this_week_monday = get_week_start_monday(today);
    let mut weeks = Vec::new();
    let mut current = this_week_monday;

    loop {
        if skip_current_week && current >= this_week_monday {
            // skip current/future weeks
        } else if current >= start_date {
            weeks.push(current);
        }

        if current <= start_date {
            break;
        }
        current -= chrono::Duration::days(7);
    }

    weeks // already newest-first
}

pub fn compute_cumulative(rows: &mut [WeekRow], reset_from: Option<NaiveDate>) {
    // Compute oldest→newest, then assign back
    let mut ordered: Vec<usize> = (0..rows.len()).collect();
    ordered.sort_by_key(|&i| rows[i].monday);

    let mut running = 0.0f64;
    let mut cumulative_map = std::collections::HashMap::new();

    for &i in &ordered {
        let monday = rows[i].monday;
        if let Some(reset) = reset_from {
            if monday < reset {
                cumulative_map.insert(i, 0.0f64);
                continue;
            }
        }
        running += rows[i].balance_hours;
        cumulative_map.insert(i, running);
    }

    for (i, row) in rows.iter_mut().enumerate() {
        row.cumulative_hours = cumulative_map.get(&i).copied().unwrap_or(0.0);
    }
}

pub fn get_week_end_sunday(monday: NaiveDate) -> NaiveDate {
    monday + chrono::Duration::days(6)
}

pub fn is_past_week(monday: NaiveDate) -> bool {
    let today = Local::now().date_naive();
    let this_week_monday = get_week_start_monday(today);
    monday < this_week_monday
}
