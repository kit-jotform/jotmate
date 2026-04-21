//! App state module. The `App` struct and its behavior are split across
//! submodules by responsibility:
//!
//! - [`screen`] — `Screen` enum and `MAIN_ITEMS`
//! - [`constants`] — `TIMEZONES`, `WEEKLY_HOURS_OPTIONS`, and small helpers
//! - [`navigation`] — list-state accessors, row navigation, `with_rows!` macro
//! - [`row_builders`] — `App::*_items()` methods that build screen row vectors
//! - [`mutations`] — toggle, cycle, confirm/execute delete, add repo, etc.
//! - [`persistence`] — `persist_settings` / `persist_td_settings` + `mutate_and_save`
//! - [`sync`] — sync-screen lifecycle (`start_sync`, `apply_sync_update`, …)
//! - [`td_report`] — `TdReportState` and async TD report fetch/poll
//!
//! External callers keep importing from `crate::tui::app::{…}`: the row types,
//! screen, input mode, td report state, sync state types, and shared constants
//! are all re-exported from this module.

use anyhow::Result;
use chrono::NaiveDate;
use ratatui::widgets::ListState;
use std::collections::HashMap;

use crate::config::{ContractPeriod, UpstreamRepo, DEFAULT_TIMEZONE};

mod constants;
mod mutations;
mod navigation;
mod persistence;
mod row_builders;
mod screen;
mod sync;
mod td_report;

// ── Public re-exports ─────────────────────────────────────────────────────────
//
// External modules import row types, screen, sync types, and TD report state
// from `crate::tui::app::*`. Keep these flat so the refactor is a pure move.

pub use constants::WEEKLY_HOURS_OPTIONS;
pub use screen::{Screen, MAIN_ITEMS};
pub use td_report::TdReportState;

pub use super::rows::{
    CpListRow, CycleTarget, GeneralToggleRow, InputMode, RemoveRepoRow, RepoManagerRow, SettingRow,
    TimeDoctorField, TimeSettingRow,
};
pub use super::sync_state::{ForkStatus, RdsStatus, RepoSyncState, SyncScreenState, SyncUpdate};

use constants::{this_monday, timezone_index};
use navigation::list_state_at;

// ── Grouped in-memory state ──────────────────────────────────────────────────

/// Mirrors `config.sync` in memory. Flushed back via `App::persist_settings`.
pub struct SyncSettings {
    pub use_cache: bool,
    pub skip_fork_sync: bool,
    pub skip_rebase: bool,
    pub skip_rds_sync: bool,
    pub smart_sync: bool,
    pub repos: Vec<UpstreamRepo>,
}

/// Mirrors `config.time` in memory (plus password-set flag read from keychain).
/// Flushed back via `App::persist_td_settings`.
pub struct TimeSettings {
    pub email: String,
    pub timezone_idx: usize,
    pub skip_current_week: bool,
    pub use_time_cache: bool,
    pub show_cumulative: bool,
    pub password_is_set: bool,
    pub contract_periods: Vec<ContractPeriod>,
}

/// Ephemeral state for the "add new contract period" inline form on the
/// ContractPeriods screen.
pub struct AddCpForm {
    pub monday: NaiveDate,
    pub hours_idx: usize,
}

// ── App struct ────────────────────────────────────────────────────────────────

pub struct App {
    pub screen: Screen,
    pub list_states: HashMap<Screen, ListState>,
    pub td_report: TdReportState,
    pub td_report_scroll: usize,
    pub sync_scroll: usize,
    pub input_mode: InputMode,
    pub sync: SyncSettings,
    pub td: TimeSettings,
    pub add_cp: AddCpForm,
    // Sync progress state
    pub sync_state: Option<SyncScreenState>,
    // Auth error message to show on TimeDoctorSettings screen
    pub auth_error: Option<String>,
    // Channel for receiving TD report results from the background fetch task
    pub td_report_rx:
        Option<tokio::sync::oneshot::Receiver<Result<Vec<crate::time::compute::WeekRow>, String>>>,
}

impl App {
    pub fn new() -> Result<Self> {
        let config = crate::config::load()?;
        let list_states: HashMap<Screen, ListState> = [
            Screen::MainMenu,
            Screen::Settings,
            Screen::SyncGeneralSettings,
            Screen::RepoManager,
            Screen::RemoveRepos,
            Screen::TdGeneralSettings,
            Screen::TimeDoctorSettings,
            Screen::ContractPeriods,
        ]
        .into_iter()
        .map(|s| (s, list_state_at(0)))
        .collect();
        let mut contract_periods = config.time.contract_periods.clone().unwrap_or_default();
        contract_periods.sort_by_key(|p| p.from);
        let add_cp_monday = contract_periods
            .last()
            .map(|p| p.from)
            .unwrap_or_else(this_monday);
        let td_tz = config.time.timezone.as_deref().unwrap_or(DEFAULT_TIMEZONE);
        Ok(Self {
            screen: Screen::MainMenu,
            list_states,
            td_report: TdReportState::Loading,
            td_report_scroll: 0,
            sync_scroll: 0,
            input_mode: InputMode::Normal,
            sync: SyncSettings {
                use_cache: config.sync.use_cache,
                skip_fork_sync: config.sync.skip_fork_sync,
                skip_rebase: config.sync.skip_rebase,
                skip_rds_sync: config.sync.skip_rds_sync,
                smart_sync: config.sync.smart_sync,
                repos: config.sync.upstream_repos.clone(),
            },
            td: TimeSettings {
                email: config.time.email.clone().unwrap_or_default(),
                timezone_idx: timezone_index(td_tz),
                skip_current_week: config.time.skip_current_week,
                use_time_cache: config.time.use_time_cache,
                show_cumulative: config.time.show_cumulative,
                password_is_set: crate::time::auth::load_password_from_keychain().is_some(),
                contract_periods,
            },
            add_cp: AddCpForm {
                monday: add_cp_monday,
                hours_idx: 1, // default 20h
            },
            sync_state: None,
            auth_error: None,
            td_report_rx: None,
        })
    }
}
