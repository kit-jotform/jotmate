use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub sync: SyncConfig,
    #[serde(default)]
    pub time: TimeConfig,
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
    /// Upstream repositories to sync (URL + name + enabled flag)
    #[serde(default = "default_upstream_repos")]
    pub upstream_repos: Vec<UpstreamRepo>,
    /// If true, use the repo path cache; if false, always rediscover
    #[serde(default = "default_true")]
    pub use_cache: bool,
    /// Skip the fork sync step (git fetch upstream + merge + push)
    #[serde(default)]
    pub skip_fork_sync: bool,
    /// Skip rebasing the current branch onto the default branch after fork sync
    #[serde(default)]
    pub skip_rebase: bool,
    /// Skip running ./sync in each repo directory
    #[serde(default)]
    pub skip_rds_sync: bool,
    /// When true, skip RDS sync for repos with no upstream changes, clean tree, and nothing ahead/behind origin
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

/// TimeDoctor company ID — hardcoded, not user-configurable
pub const TIMEDOCTOR_COMPANY_ID: &str = "Xms4iFqBgQAEjLy2";

/// Canonical default timezone — used as the fallback when none is configured
/// and as the initial value in the interactive prompt.
pub const DEFAULT_TIMEZONE: &str = "Europe/Istanbul";

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractPeriod {
    pub from: NaiveDate,
    pub weekly_hours: f64,
}

pub(super) fn default_true() -> bool {
    true
}
