use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub profile: ProfileConfig,
    #[serde(default)]
    pub sync: SyncConfig,
    #[serde(default)]
    pub time: TimeConfig,
    #[serde(default)]
    pub pdks: PdksConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    #[serde(default = "default_true")]
    pub is_developer: bool,
    #[serde(default)]
    pub office: Option<Office>,
    #[serde(default)]
    pub is_part_time: bool,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            is_developer: true,
            office: None,
            is_part_time: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Office {
    #[default]
    Hacettepe,
    Izmir,
    Odtu,
    Istanbul,
}

impl Office {
    pub const ALL: [Self; 4] = [Self::Hacettepe, Self::Izmir, Self::Odtu, Self::Istanbul];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Hacettepe => "Hacettepe",
            Self::Izmir => "Izmir",
            Self::Odtu => "ODTU",
            Self::Istanbul => "Istanbul",
        }
    }
}

/// Per-week PDKS target-hour exceptions. Weeks not listed here use 45 hours.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PdksConfig {
    #[serde(default)]
    pub weekly_targets: Vec<PdksWeeklyTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdksWeeklyTarget {
    pub week: NaiveDate,
    pub target_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamRepo {
    pub url: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl UpstreamRepo {
    pub fn new(url: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            name: name.into(),
            enabled: true,
        }
    }
}

pub fn normalize_repo_url(url: &str) -> String {
    url.trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_string()
}

fn default_upstream_repos() -> Vec<UpstreamRepo> {
    vec![
        UpstreamRepo::new("https://github.com/jotform/frontend.git", "frontend"),
        UpstreamRepo::new("https://github.com/jotform/vendors.git", "vendors"),
        UpstreamRepo::new("https://github.com/jotform/backend.git", "backend"),
        UpstreamRepo::new("https://github.com/jotform/core.git", "core"),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    #[serde(default = "default_upstream_repos")]
    pub upstream_repos: Vec<UpstreamRepo>,
    #[serde(default = "default_true")]
    pub use_cache: bool,
    #[serde(default)]
    pub skip_fork_sync: bool,
    #[serde(default)]
    pub skip_rebase: bool,
    #[serde(default)]
    pub skip_rds_sync: bool,
    #[serde(default = "default_true")]
    pub smart_sync: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            upstream_repos: default_upstream_repos(),
            use_cache: true,
            skip_fork_sync: false,
            skip_rebase: false,
            skip_rds_sync: false,
            smart_sync: true,
        }
    }
}

pub const TIMEDOCTOR_COMPANY_ID: &str = "Xms4iFqBgQAEjLy2";
pub const DEFAULT_TIMEZONE: &str = "Europe/Istanbul";
pub const DATE_FORMAT: &str = "%Y-%m-%d";
pub const PDKS_DAILY_TARGET_HOURS: f64 = 9.0;
pub const PDKS_WEEKLY_TARGET_HOURS: f64 = 45.0;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeConfig {
    pub email: Option<String>,
    pub timezone: Option<String>,
    pub start_date: Option<NaiveDate>,
    #[serde(default = "default_true")]
    pub skip_current_week: bool,
    #[serde(default = "default_true")]
    pub use_time_cache: bool,
    #[serde(default = "default_true")]
    pub show_cumulative: bool,
    pub contract_periods: Option<Vec<ContractPeriod>>,
    pub reset_cumulative_from_date: Option<NaiveDate>,
    pub off_weeks: Option<Vec<NaiveDate>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractPeriod {
    pub from: NaiveDate,
    pub weekly_hours: f64,
}

pub(super) fn default_true() -> bool {
    true
}
