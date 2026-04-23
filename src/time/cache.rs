use chrono::NaiveDate;
use std::path::PathBuf;

use crate::ctx::Paths;
use crate::time::api::StatsResponse;

/// IANA timezone names contain `/` which can't go in a path segment on its own.
/// Replace with `_` so `Europe/Istanbul` becomes `Europe_Istanbul`.
fn sanitize_tz(tz: &str) -> String {
    tz.replace('/', "_")
}

pub fn week_cache_path(
    paths: &Paths,
    company_id: &str,
    timezone: &str,
    monday: NaiveDate,
) -> PathBuf {
    paths
        .time_cache_root()
        .join(company_id)
        .join(sanitize_tz(timezone))
        .join(format!("{}.json", monday.format("%Y-%m-%d")))
}

pub fn read_week_cache(
    paths: &Paths,
    company_id: &str,
    timezone: &str,
    monday: NaiveDate,
) -> Option<StatsResponse> {
    let path = week_cache_path(paths, company_id, timezone, monday);
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn write_week_cache(
    paths: &Paths,
    company_id: &str,
    timezone: &str,
    monday: NaiveDate,
    stats: &StatsResponse,
) {
    let path = week_cache_path(paths, company_id, timezone, monday);
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    if let Ok(content) = serde_json::to_string(stats) {
        let _ = std::fs::write(&path, content);
    }
}
