//! Time-tracker HTTP tests driven by a mockito server.
//!
//! Covered edge cases:
//!   B4    invalid IANA timezone surfaces a named error
//!   B21   TZ-keyed cache path — separate subdirs for each tz
//!   B22   corrupt cache falls through to a network fetch
//!   B24   `no_cache = true` always hits the network
//!   B27   empty `data[]` → 0 worked hours
//!   Stats URL is respected (sanity check on the injection seam)

mod common;

use chrono::NaiveDate;
use common::TestCtx;
use jotmate::config::TIMEDOCTOR_COMPANY_ID;
use jotmate::time::cache::{read_week_cache, week_cache_path, write_week_cache};
use jotmate::time::fetch_week;
use jotmate::time::FetchOpts;
use mockito::Server;

fn monday(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

fn a_past_monday() -> NaiveDate {
    // Pick something well in the past so `is_past_week` is always true.
    monday("2024-01-01") // actually a Monday
}

fn opts_for(stats_url: String, timezone: &str, no_cache: bool) -> FetchOpts {
    FetchOpts {
        timezone: timezone.to_string(),
        contract_periods: vec![],
        off_weeks: vec![],
        no_cache,
        stats_url,
    }
}

fn stats_body(total_seconds: u64) -> String {
    format!(r#"{{"data":[{{"total":{total_seconds}}}]}}"#)
}

#[tokio::test]
async fn fetch_week_hits_network_and_caches() {
    let mut server = Server::new_async().await;
    let m = server
        .mock("GET", mockito::Matcher::Regex(r".*/stats/total.*".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(stats_body(9 * 3600)) // 9 hours
        .expect(1)
        .create_async()
        .await;

    let tc = TestCtx::with_mock_http(&server.url());
    let client = reqwest::Client::new();
    let opts = opts_for(tc.ctx.http.stats.clone(), "Europe/Istanbul", false);

    let row = fetch_week(tc.paths(), &client, "cookie=x", a_past_monday(), &opts)
        .await
        .unwrap();
    assert_eq!(row.worked_secs, 9 * 3600);
    assert!(!row.from_cache);

    // Past-week result must be written through to the cache.
    let cache_file = week_cache_path(
        tc.paths(),
        TIMEDOCTOR_COMPANY_ID,
        "Europe/Istanbul",
        a_past_monday(),
    );
    assert!(cache_file.exists(), "cache file should exist after fetch");

    m.assert_async().await;
}

#[tokio::test]
async fn fetch_week_uses_cache_when_present() {
    let mut server = Server::new_async().await;
    // Any network call would fail the test because the mock expects 0 hits.
    let m = server
        .mock("GET", mockito::Matcher::Regex(r".*/stats/total.*".into()))
        .with_status(500)
        .expect(0)
        .create_async()
        .await;

    let tc = TestCtx::with_mock_http(&server.url());
    // Seed the cache directly.
    let stats: jotmate::time::api::StatsResponse = serde_json::from_str(&stats_body(3600)).unwrap();
    write_week_cache(
        tc.paths(),
        TIMEDOCTOR_COMPANY_ID,
        "Europe/Istanbul",
        a_past_monday(),
        &stats,
    );

    let client = reqwest::Client::new();
    let opts = opts_for(tc.ctx.http.stats.clone(), "Europe/Istanbul", false);

    let row = fetch_week(tc.paths(), &client, "cookie", a_past_monday(), &opts)
        .await
        .unwrap();
    assert_eq!(row.worked_secs, 3600);
    assert!(row.from_cache);

    m.assert_async().await;
}

#[tokio::test]
async fn fetch_week_no_cache_bypasses_cache() {
    // B24 — when `no_cache=true`, even a populated cache is ignored.
    let mut server = Server::new_async().await;
    let m = server
        .mock("GET", mockito::Matcher::Regex(r".*/stats/total.*".into()))
        .with_status(200)
        .with_body(stats_body(7200))
        .expect(1)
        .create_async()
        .await;

    let tc = TestCtx::with_mock_http(&server.url());
    // Seed a cache entry with a DIFFERENT value so we can verify the fetch
    // result (7200s) won, not the cached 3600s.
    let stats: jotmate::time::api::StatsResponse = serde_json::from_str(&stats_body(3600)).unwrap();
    write_week_cache(
        tc.paths(),
        TIMEDOCTOR_COMPANY_ID,
        "Europe/Istanbul",
        a_past_monday(),
        &stats,
    );

    let client = reqwest::Client::new();
    let opts = opts_for(tc.ctx.http.stats.clone(), "Europe/Istanbul", true);

    let row = fetch_week(tc.paths(), &client, "cookie", a_past_monday(), &opts)
        .await
        .unwrap();
    assert_eq!(row.worked_secs, 7200);
    m.assert_async().await;
}

#[tokio::test]
async fn fetch_week_corrupt_cache_falls_through() {
    // B22 — a malformed cache JSON should be ignored, and the network fetch
    // should run.
    let mut server = Server::new_async().await;
    let m = server
        .mock("GET", mockito::Matcher::Regex(r".*/stats/total.*".into()))
        .with_status(200)
        .with_body(stats_body(60))
        .expect(1)
        .create_async()
        .await;

    let tc = TestCtx::with_mock_http(&server.url());
    let cache_file = week_cache_path(
        tc.paths(),
        TIMEDOCTOR_COMPANY_ID,
        "Europe/Istanbul",
        a_past_monday(),
    );
    std::fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
    std::fs::write(&cache_file, "not json at all").unwrap();

    let client = reqwest::Client::new();
    let opts = opts_for(tc.ctx.http.stats.clone(), "Europe/Istanbul", false);
    let row = fetch_week(tc.paths(), &client, "cookie", a_past_monday(), &opts)
        .await
        .unwrap();
    assert_eq!(row.worked_secs, 60);
    m.assert_async().await;
}

#[tokio::test]
async fn fetch_week_invalid_timezone_errors() {
    // B4 — typoed IANA name → named error, no HTTP call.
    let mut server = Server::new_async().await;
    let m = server
        .mock("GET", mockito::Matcher::Regex(r".*".into()))
        .expect(0)
        .create_async()
        .await;

    let tc = TestCtx::with_mock_http(&server.url());
    let client = reqwest::Client::new();
    let opts = opts_for(tc.ctx.http.stats.clone(), "Europe/Istambul", true);
    // `no_cache=true` skips the cache-read branch that bypasses TZ parsing,
    // so this exercises the parse.

    let err = fetch_week(tc.paths(), &client, "cookie", a_past_monday(), &opts)
        .await
        .unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("Invalid timezone"), "got: {msg}");
    assert!(msg.contains("Europe/Istambul"), "got: {msg}");

    m.assert_async().await;
}

#[tokio::test]
async fn fetch_week_empty_data_yields_zero_hours() {
    // B27 — API returned empty data[].
    let mut server = Server::new_async().await;
    server
        .mock("GET", mockito::Matcher::Regex(r".*/stats/total.*".into()))
        .with_status(200)
        .with_body(r#"{"data":[]}"#)
        .create_async()
        .await;

    let tc = TestCtx::with_mock_http(&server.url());
    let client = reqwest::Client::new();
    let opts = opts_for(tc.ctx.http.stats.clone(), "Europe/Istanbul", true);
    let row = fetch_week(tc.paths(), &client, "cookie", a_past_monday(), &opts)
        .await
        .unwrap();
    assert_eq!(row.worked_secs, 0);
    assert_eq!(row.balance_hours, 0.0); // no contract periods → target 0
}

#[test]
fn cache_paths_are_segregated_by_timezone() {
    // B21 — two different TZs under the same company must land in distinct
    // subdirs so changing TZ doesn't read/write mismatched rows.
    let tc = TestCtx::new();
    let m = a_past_monday();
    let istanbul = week_cache_path(tc.paths(), TIMEDOCTOR_COMPANY_ID, "Europe/Istanbul", m);
    let new_york = week_cache_path(tc.paths(), TIMEDOCTOR_COMPANY_ID, "America/New_York", m);

    assert_ne!(istanbul, new_york);
    assert!(istanbul.to_string_lossy().contains("Europe_Istanbul"));
    assert!(new_york.to_string_lossy().contains("America_New_York"));

    // Writing to one must not populate the other.
    let stats: jotmate::time::api::StatsResponse = serde_json::from_str(&stats_body(42)).unwrap();
    write_week_cache(
        tc.paths(),
        TIMEDOCTOR_COMPANY_ID,
        "Europe/Istanbul",
        m,
        &stats,
    );

    assert!(read_week_cache(tc.paths(), TIMEDOCTOR_COMPANY_ID, "Europe/Istanbul", m).is_some());
    assert!(read_week_cache(tc.paths(), TIMEDOCTOR_COMPANY_ID, "America/New_York", m).is_none());
}
