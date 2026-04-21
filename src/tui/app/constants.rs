use chrono::{Local, NaiveDate};

use crate::config::DEFAULT_TIMEZONE;
use crate::time::compute::get_week_start_monday;

pub const TIMEZONES: &[&str] = &[
    "America/New_York",
    "America/Chicago",
    "America/Denver",
    "America/Los_Angeles",
    "America/Sao_Paulo",
    "Europe/London",
    "Europe/Berlin",
    DEFAULT_TIMEZONE,
    "Europe/Moscow",
    "Asia/Dubai",
    "Asia/Kolkata",
    "Asia/Shanghai",
    "Asia/Tokyo",
    "Australia/Sydney",
    "Pacific/Auckland",
];

pub const WEEKLY_HOURS_OPTIONS: &[f64] = &[16.0, 20.0, 24.0, 28.0];

pub(super) fn timezone_index(tz: &str) -> usize {
    TIMEZONES
        .iter()
        .position(|&t| t == tz)
        .or_else(|| TIMEZONES.iter().position(|&t| t == DEFAULT_TIMEZONE))
        .unwrap_or(0)
}

pub(super) fn this_monday() -> NaiveDate {
    get_week_start_monday(Local::now().date_naive())
}
