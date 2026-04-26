//! `App` state, split across submodules by responsibility. Row types, screen,
//! input mode, TD report state, sync state, and shared constants are
//! re-exported below so callers can `use crate::tui::app::{…}` directly.

use anyhow::Result;
use chrono::NaiveDate;
use ratatui::widgets::ListState;
use std::cell::OnceCell;
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
mod update;

pub use constants::WEEKLY_HOURS_OPTIONS;
pub use screen::{MainMenuItem, MainMenuKind, Screen};
pub use td_report::{TdReportState, TD_REPORT_VISIBLE_ROWS};

pub use super::rows::{
    CpListRow, CycleTarget, GeneralToggleRow, InputMode, RemoveRepoRow, RepoManagerRow, SettingRow,
    TimeDoctorField, TimeSettingRow,
};
pub use super::sync_state::{
    ForkStatus, RdsStatus, RepoSyncState, SyncPhase, SyncScreenState, SyncUpdate,
};

use constants::{this_monday, timezone_index};
use navigation::list_state_at;

pub struct SyncSettings {
    pub use_cache: bool,
    pub skip_fork_sync: bool,
    pub skip_rebase: bool,
    pub skip_rds_sync: bool,
    pub smart_sync: bool,
    pub repos: Vec<UpstreamRepo>,
}

pub struct TimeSettings {
    pub email: String,
    pub timezone_idx: usize,
    pub skip_current_week: bool,
    pub use_time_cache: bool,
    pub show_cumulative: bool,
    /// Lazy: each `security` CLI lookup is ~100-300ms; must not block TUI startup.
    pub password_is_set: OnceCell<bool>,
    pub contract_periods: Vec<ContractPeriod>,
}

pub struct AddCpForm {
    pub monday: NaiveDate,
    pub hours_idx: usize,
}

pub struct App {
    pub ctx: crate::ctx::Ctx,
    pub screen: Screen,
    pub list_states: HashMap<Screen, ListState>,
    pub td_report: TdReportState,
    pub td_report_scroll: usize,
    pub sync_scroll: usize,
    pub input_mode: InputMode,
    pub sync: SyncSettings,
    pub td: TimeSettings,
    pub add_cp: AddCpForm,
    pub sync_state: Option<SyncScreenState>,
    pub update_state: Option<crate::tui::update_state::UpdateScreenState>,
    /// `None` = checking; `Some(None)` = up to date; `Some(Some(v))` = newer release `v` available.
    pub available_update: Option<Option<String>>,
    pub update_check_rx: Option<tokio::sync::oneshot::Receiver<Option<String>>>,
    pub auth_error: Option<String>,
    pub config_load_error: Option<String>,
    pub td_report_rx:
        Option<tokio::sync::oneshot::Receiver<Result<Vec<crate::time::compute::WeekRow>, String>>>,
    pub td_report_started_at: Option<std::time::Instant>,
    pub td_report_elapsed_secs: Option<f64>,
}

impl App {
    pub fn password_is_set(&self) -> bool {
        // Treat keychain errors as "not set" so the UI lets the user re-enter; the save path surfaces the real error.
        let kc = self.ctx.keychain.clone();
        *self
            .td
            .password_is_set
            .get_or_init(|| matches!(kc.get_password(), Ok(Some(_))))
    }

    pub fn new(ctx: crate::ctx::Ctx) -> Result<Self> {
        let (config, config_load_error) = match crate::config::load(&ctx.paths) {
            Ok(c) => (c, None),
            Err(e) => (crate::config::Config::default(), Some(format!("{e:#}"))),
        };
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
        let update_check_rx = spawn_update_check();
        let mut contract_periods = config.time.contract_periods.clone().unwrap_or_default();
        contract_periods.sort_by_key(|p| p.from);
        let add_cp_monday = contract_periods
            .last()
            .map(|p| p.from)
            .unwrap_or_else(this_monday);
        let td_tz = config.time.timezone.as_deref().unwrap_or(DEFAULT_TIMEZONE);
        Ok(Self {
            ctx,
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
                password_is_set: OnceCell::new(),
                contract_periods,
            },
            add_cp: AddCpForm {
                monday: add_cp_monday,
                hours_idx: 1, // default 20h
            },
            sync_state: None,
            update_state: None,
            available_update: None,
            update_check_rx,
            auth_error: None,
            config_load_error,
            td_report_rx: None,
            td_report_started_at: None,
            td_report_elapsed_secs: None,
        })
    }

    pub fn main_menu_items(&self) -> Vec<MainMenuItem> {
        let mut items = vec![
            MainMenuItem {
                kind: MainMenuKind::Sync,
                name: "Sync".into(),
                desc: "Sync RDS to upstream".into(),
            },
            MainMenuItem {
                kind: MainMenuKind::TimeDoctor,
                name: "Time Doctor".into(),
                desc: "Track your work hours".into(),
            },
        ];
        if let Some(Some(version)) = &self.available_update {
            items.push(MainMenuItem {
                kind: MainMenuKind::Update,
                name: "Update".into(),
                desc: format!("New release available — v{version}"),
            });
        }
        items.push(MainMenuItem {
            kind: MainMenuKind::Settings,
            name: "Settings".into(),
            desc: "Configure jotmate".into(),
        });
        items.push(MainMenuItem {
            kind: MainMenuKind::Exit,
            name: "Exit".into(),
            desc: String::new(),
        });
        items
    }

    pub fn poll_update_check(&mut self) {
        let Some(rx) = self.update_check_rx.as_mut() else {
            return;
        };
        match rx.try_recv() {
            Ok(result) => {
                self.available_update = Some(result);
                self.update_check_rx = None;
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                self.available_update = Some(None);
                self.update_check_rx = None;
            }
        }
    }
}

// Returns `None` outside a Tokio runtime (e.g. sync unit tests) so `App::new` stays infallible there.
fn spawn_update_check() -> Option<tokio::sync::oneshot::Receiver<Option<String>>> {
    let handle = tokio::runtime::Handle::try_current().ok()?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    handle.spawn(async move {
        let result = match crate::update::check_for_update().await {
            Ok(Some(a)) => Some(a.version),
            _ => None,
        };
        let _ = tx.send(result);
    });
    Some(rx)
}
