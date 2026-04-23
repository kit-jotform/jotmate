//! User config at `~/.config/jotmate/config.toml`.
//!
//! - [`types`]   — `Config`, `SyncConfig`, `TimeConfig`, `UpstreamRepo`, `ContractPeriod`
//! - [`io`]      — `config_path`, `load`, `save`
//! - [`parse`]   — `parse_contract_periods`
//! - [`prompt`]  — `ensure_time_credentials` (interactive fill-in)

mod io;
pub mod parse;
mod prompt;
mod types;

pub use io::{load, save};
pub use prompt::ensure_time_credentials;
pub use types::{Config, ContractPeriod, UpstreamRepo, DEFAULT_TIMEZONE, TIMEDOCTOR_COMPANY_ID};
