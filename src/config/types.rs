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
    /// Default projects to sync when --only is not passed
    pub default_only: Option<Vec<String>>,
    /// If true, run with --sync-all by default
    #[serde(default)]
    pub sync_all_by_default: bool,
    /// If true, use the repo path cache; if false, always rediscover
    #[serde(default = "default_true")]
    pub use_cache: bool,
    /// Skip the fork sync step (git fetch upstream + merge + push)
    #[serde(default = "default_true")]
    pub skip_fork_sync: bool,
    /// Skip rebasing the current branch onto the default branch after fork sync
    #[serde(default = "default_true")]
    pub skip_rebase: bool,
    /// Skip running ./sync in each repo directory
    #[serde(default = "default_true")]
    pub skip_rds_sync: bool,
    /// Skip git fetch upstream (useful for offline work)
    #[serde(default = "default_true")]
    pub skip_git_fetch: bool,
    /// Skip RDS sync when repo has uncommitted changes
    #[serde(default = "default_true")]
    pub skip_dirty_sync: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            upstream_repos: default_upstream_repos(),
            default_only: None,
            sync_all_by_default: false,
            use_cache: true,
            skip_fork_sync: true,
            skip_rebase: true,
            skip_rds_sync: true,
            skip_git_fetch: true,
            skip_dirty_sync: true,
        }
    }
}

/// TimeDoctor company ID — hardcoded, not user-configurable
pub const TIMEDOCTOR_COMPANY_ID: &str = "Xms4iFqBgQAEjLy2";

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
