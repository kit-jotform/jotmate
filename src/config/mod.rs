//! User-facing config (~/.config/jotmate/config.toml).

mod io;
pub mod parse;
mod prompt;
mod types;

pub use io::{load, save};
pub use prompt::ensure_time_credentials;
pub use types::{Config, ContractPeriod, UpstreamRepo, DEFAULT_TIMEZONE, TIMEDOCTOR_COMPANY_ID};
