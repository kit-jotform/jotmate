//! User-facing config (~/.config/jotmate/config.toml).

mod io;
pub mod parse;
mod prompt;
mod types;

pub use io::{load, save};
pub use prompt::ensure_time_credentials;
pub use types::{
    normalize_repo_url, Config, ContractPeriod, Office, PdksWeeklyTarget, ProfileConfig,
    UpstreamRepo, DATE_FORMAT, DEFAULT_TIMEZONE, PDKS_DAILY_TARGET_HOURS, PDKS_WEEKLY_TARGET_HOURS,
    TIMEDOCTOR_COMPANY_ID,
};
