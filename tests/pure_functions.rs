//! Pure-function unit tests. No filesystem, no HTTP, no keychain — these
//! are deterministic and fast.
//!
//! Covered edge cases (cross-reference to the manual audit list):
//!   B11–B20   contract-period / start-date / reset logic
//!   B12/B13   future / current-week start dates
//!   B14       skip_current_week on a Monday boundary
//!   B16–B20   cumulative reset dates, multi-period transitions
//!   A22–A24   repo URL normalization / dedup rules (via parse_contract_periods
//!             sibling — URL helper tests live in repo_url_validation.rs)
//!   plus general arithmetic / formatting coverage

use chrono::NaiveDate;
use jotmate::config::parse::parse_contract_periods;
use jotmate::config::ContractPeriod;
use jotmate::time::compute::{
    compute_cumulative, format_hours, format_hours_signed, format_week_range, get_target_hours,
    get_week_end_sunday, get_week_start_monday, is_past_week, weeks_to_fetch, WeekRow,
};

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn row(monday: &str, balance: f64) -> WeekRow {
    WeekRow {
        monday: d(monday),
        week_label: String::new(),
        worked_secs: 0,
        target_hours: 0.0,
        balance_hours: balance,
        cumulative_hours: 0.0,
        from_cache: false,
    }
}

// ─── get_week_start_monday / get_week_end_sunday ────────────────────────────

#[test]
fn week_start_monday_is_identity_on_monday() {
    // 2025-01-06 is a Monday.
    assert_eq!(get_week_start_monday(d("2025-01-06")), d("2025-01-06"));
}

#[test]
fn week_start_monday_walks_back_from_sunday() {
    // 2025-01-12 is a Sunday → week starts 2025-01-06.
    assert_eq!(get_week_start_monday(d("2025-01-12")), d("2025-01-06"));
}

#[test]
fn week_start_monday_walks_back_from_any_weekday() {
    // Wednesday 2025-01-08 → Monday 2025-01-06.
    assert_eq!(get_week_start_monday(d("2025-01-08")), d("2025-01-06"));
}

#[test]
fn week_end_sunday_is_monday_plus_six() {
    assert_eq!(get_week_end_sunday(d("2025-01-06")), d("2025-01-12"));
}

// ─── weeks_to_fetch ─────────────────────────────────────────────────────────

#[test]
fn weeks_to_fetch_future_start_returns_empty() {
    // B12 — start in the far future.
    let future = d("2099-01-05"); // Monday
    assert!(weeks_to_fetch(future, false).is_empty());
    assert!(weeks_to_fetch(future, true).is_empty());
}

#[test]
fn weeks_to_fetch_includes_start_monday() {
    // Any start a few Mondays back must include that exact Monday when
    // skip_current_week is false.
    let start =
        get_week_start_monday(chrono::Local::now().date_naive()) - chrono::Duration::days(7 * 3);
    let weeks = weeks_to_fetch(start, false);
    assert!(weeks.contains(&start));
}

#[test]
fn weeks_to_fetch_skip_current_excludes_this_monday() {
    // B14 — with skip_current_week, this week's Monday must not appear.
    let this_monday = get_week_start_monday(chrono::Local::now().date_naive());
    let start = this_monday - chrono::Duration::days(7 * 3);
    let weeks = weeks_to_fetch(start, true);
    assert!(!weeks.contains(&this_monday));
    assert_eq!(weeks.len(), 3); // 3 past Mondays, no current
}

#[test]
fn weeks_to_fetch_stops_at_start_date() {
    let this_monday = get_week_start_monday(chrono::Local::now().date_naive());
    let start = this_monday - chrono::Duration::days(7 * 5);
    let weeks = weeks_to_fetch(start, false);
    // 5 past Mondays + current = 6. `weeks_to_fetch` walks backwards; ordering
    // not asserted here, only membership and count.
    assert_eq!(weeks.len(), 6);
    assert!(weeks.contains(&start));
    assert!(weeks.contains(&this_monday));
}

// ─── get_target_hours ───────────────────────────────────────────────────────

#[test]
fn target_hours_uses_matching_period() {
    // B20 — multi-period transition.
    let periods = vec![
        ContractPeriod {
            from: d("2025-01-06"),
            weekly_hours: 20.0,
        },
        ContractPeriod {
            from: d("2025-06-02"),
            weekly_hours: 40.0,
        },
    ];
    assert_eq!(get_target_hours(d("2025-01-06"), &periods), 20.0);
    assert_eq!(get_target_hours(d("2025-05-26"), &periods), 20.0);
    assert_eq!(get_target_hours(d("2025-06-02"), &periods), 40.0);
    assert_eq!(get_target_hours(d("2025-12-29"), &periods), 40.0);
}

#[test]
fn target_hours_before_first_period_uses_first() {
    // Current behavior: `get_target_hours` seeds with `periods.first()`, so a
    // monday earlier than every period still returns the earliest period's
    // weekly hours. Documented here so future refactors don't drift.
    let periods = vec![ContractPeriod {
        from: d("2025-06-02"),
        weekly_hours: 40.0,
    }];
    assert_eq!(get_target_hours(d("2024-01-01"), &periods), 40.0);
}

#[test]
fn target_hours_empty_periods_is_zero() {
    // B11 — with no periods, target is 0 (balance ≡ worked hours).
    assert_eq!(get_target_hours(d("2025-01-06"), &[]), 0.0);
}

// ─── compute_cumulative ─────────────────────────────────────────────────────

#[test]
fn cumulative_without_reset_sums_balances_in_date_order() {
    let mut rows = vec![
        row("2025-01-13", 2.0),
        row("2025-01-06", 5.0),
        row("2025-01-20", -1.0),
    ];
    compute_cumulative(&mut rows, None);
    // Rows stay in the original slot order; cumulative_hours is computed by
    // monday date, so we look each row up.
    let by_date: std::collections::HashMap<NaiveDate, f64> = rows
        .iter()
        .map(|r| (r.monday, r.cumulative_hours))
        .collect();
    assert_eq!(by_date[&d("2025-01-06")], 5.0);
    assert_eq!(by_date[&d("2025-01-13")], 7.0);
    assert_eq!(by_date[&d("2025-01-20")], 6.0);
}

#[test]
fn cumulative_with_reset_zeroes_rows_before_reset() {
    // B16 — reset partway through range.
    let mut rows = vec![
        row("2025-01-06", 5.0),
        row("2025-01-13", 2.0),
        row("2025-01-20", -1.0),
    ];
    compute_cumulative(&mut rows, Some(d("2025-01-13")));
    let by_date: std::collections::HashMap<NaiveDate, f64> = rows
        .iter()
        .map(|r| (r.monday, r.cumulative_hours))
        .collect();
    assert_eq!(by_date[&d("2025-01-06")], 0.0);
    assert_eq!(by_date[&d("2025-01-13")], 2.0);
    assert_eq!(by_date[&d("2025-01-20")], 1.0);
}

#[test]
fn cumulative_reset_after_newest_zeroes_everything() {
    // B17 — reset date in the far future → every row gets 0.
    let mut rows = vec![row("2025-01-06", 5.0), row("2025-01-13", 2.0)];
    compute_cumulative(&mut rows, Some(d("2099-01-01")));
    assert!(rows.iter().all(|r| r.cumulative_hours == 0.0));
}

#[test]
fn cumulative_reset_before_oldest_is_noop() {
    // B15 — reset date earlier than every row → no rows zeroed.
    let mut rows = vec![row("2025-01-06", 5.0), row("2025-01-13", 2.0)];
    compute_cumulative(&mut rows, Some(d("1970-01-01")));
    let by_date: std::collections::HashMap<NaiveDate, f64> = rows
        .iter()
        .map(|r| (r.monday, r.cumulative_hours))
        .collect();
    assert_eq!(by_date[&d("2025-01-06")], 5.0);
    assert_eq!(by_date[&d("2025-01-13")], 7.0);
}

// ─── is_past_week ───────────────────────────────────────────────────────────

#[test]
fn past_week_is_past() {
    let past =
        get_week_start_monday(chrono::Local::now().date_naive()) - chrono::Duration::days(7 * 4);
    assert!(is_past_week(past));
}

#[test]
fn current_week_is_not_past() {
    let current = get_week_start_monday(chrono::Local::now().date_naive());
    assert!(!is_past_week(current));
}

#[test]
fn future_week_is_not_past() {
    let future =
        get_week_start_monday(chrono::Local::now().date_naive()) + chrono::Duration::days(7);
    assert!(!is_past_week(future));
}

// ─── format_hours / format_hours_signed ─────────────────────────────────────

#[test]
fn format_hours_integer() {
    assert_eq!(format_hours(8.0), "8h");
}

#[test]
fn format_hours_with_minutes() {
    assert_eq!(format_hours(8.5), "8h 30m");
}

#[test]
fn format_hours_negative() {
    assert_eq!(format_hours(-1.25), "-1h 15m");
}

#[test]
fn format_hours_zero() {
    assert_eq!(format_hours(0.0), "0h");
}

#[test]
fn format_hours_signed_plus_prefix() {
    assert_eq!(format_hours_signed(2.0), "+2h");
    assert_eq!(format_hours_signed(-2.0), "-2h");
    assert_eq!(format_hours_signed(0.0), "+0h");
}

#[test]
fn format_hours_rounds_to_nearest_minute() {
    // 0.01h = 0.6 min → rounds to 1 min.
    assert_eq!(format_hours(0.01), "0h 1m");
    // 0.004h = 0.24 min → rounds to 0 min.
    assert_eq!(format_hours(0.004), "0h");
}

// ─── format_week_range ──────────────────────────────────────────────────────

#[test]
fn week_range_within_one_month() {
    assert_eq!(format_week_range(d("2025-03-03")), "Mar 3 - Mar 9, 2025");
}

#[test]
fn week_range_crosses_month_boundary() {
    assert_eq!(format_week_range(d("2025-01-27")), "Jan 27 - Feb 2, 2025");
}

#[test]
fn week_range_crosses_year_boundary() {
    // Sunday year wins (consistent with the fn signature, which takes Sunday's year).
    assert_eq!(format_week_range(d("2025-12-29")), "Dec 29 - Jan 4, 2026");
}

// ─── parse_contract_periods ─────────────────────────────────────────────────

#[test]
fn parse_periods_single_entry() {
    let p = parse_contract_periods("2025-01-06:20").unwrap();
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].from, d("2025-01-06"));
    assert_eq!(p[0].weekly_hours, 20.0);
}

#[test]
fn parse_periods_multiple_sorted_by_date() {
    let p = parse_contract_periods("2025-06-02:40,2025-01-06:20").unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].from, d("2025-01-06"));
    assert_eq!(p[1].from, d("2025-06-02"));
}

#[test]
fn parse_periods_accepts_fractional_hours() {
    let p = parse_contract_periods("2025-01-06:37.5").unwrap();
    assert_eq!(p[0].weekly_hours, 37.5);
}

#[test]
fn parse_periods_rejects_negative_hours() {
    let err = parse_contract_periods("2025-01-06:-5").unwrap_err();
    assert!(err.to_string().contains("negative"), "got: {err}");
}

#[test]
fn parse_periods_rejects_empty() {
    assert!(parse_contract_periods("").is_err());
}

#[test]
fn parse_periods_rejects_invalid_date() {
    assert!(parse_contract_periods("not-a-date:20").is_err());
}

#[test]
fn parse_periods_rejects_missing_hours() {
    assert!(parse_contract_periods("2025-01-06:").is_err());
}

#[test]
fn parse_periods_rejects_missing_colon() {
    assert!(parse_contract_periods("2025-01-06").is_err());
}
