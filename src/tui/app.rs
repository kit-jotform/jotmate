use anyhow::Result;
use chrono::{Local, NaiveDate};
use ratatui::widgets::ListState;

use crate::config::ContractPeriod;
use crate::time::compute::get_week_start_monday;

// ── Main menu items ─────────────────��─────────────────────────────────────────

pub const MAIN_ITEMS: &[(&str, &str)] = &[
    ("Sync", "Sync RDS to upstream"),
    ("Time Doctor", "Track your work hours"),
    ("Settings", "Configure jotmate"),
    ("Exit", ""),
];

// ── Timezone options ───��─────────────────────────────────────────────────────

pub const TIMEZONES: &[&str] = &[
    "America/New_York",
    "America/Chicago",
    "America/Denver",
    "America/Los_Angeles",
    "America/Sao_Paulo",
    "Europe/London",
    "Europe/Berlin",
    "Europe/Istanbul",
    "Europe/Moscow",
    "Asia/Dubai",
    "Asia/Kolkata",
    "Asia/Shanghai",
    "Asia/Tokyo",
    "Australia/Sydney",
    "Pacific/Auckland",
];

// ── Weekly hours options ─────────────────────────────────────────────────────

pub const WEEKLY_HOURS_OPTIONS: &[f64] = &[16.0, 20.0, 24.0, 28.0];

// ── Screens ────────────��────────────────────────────────���─────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    MainMenu,
    Settings,
    SyncGeneralSettings,
    RepoManager,
    RemoveRepos,
    TdGeneralSettings,
    TimeDoctorSettings,
    ContractPeriods,
}

// ── Input mode (used inside RepoManager and TimeDoctorSettings) ──────────────

#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    AddingRepo(String),         // buffer holds URL being typed
    ConfirmDelete(String),      // holds the repo name pending deletion
    ConfirmDeletePeriod(usize), // holds the period index pending deletion
    SelectingTimezone,          // ↑↓ cycles timezone, Enter/Esc confirms (in GeneralToggle screen)
    EditingCpMonday,            // ↑↓ cycles monday date, Enter/Esc confirms
    EditingCpHours,             // ↑↓ cycles hours option, Enter/Esc confirms
    EditingField {
        // editing a text field in TimeDoctorSettings
        field: TimeDoctorField,
        buf: String,
    },
}

/// Which Time Doctor field is being edited
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TimeDoctorField {
    Email,
    Password,
}

// ── Settings row types ─────────────���────────────────────────────────���─────────

#[derive(Clone, Copy)]
pub enum ToggleKind {
    SyncAll,
    UseCache,
    SkipForkSync,
    SkipRebase,
    SkipRdsSync,
    SkipGitFetch,
    SkipDirtySync,
    SkipCurrentWeek,
    UseTimeCache,
    ShowCumulative,
}

impl ToggleKind {
    pub fn is_sync(self) -> bool {
        matches!(
            self,
            ToggleKind::SyncAll
                | ToggleKind::UseCache
                | ToggleKind::SkipForkSync
                | ToggleKind::SkipRebase
                | ToggleKind::SkipRdsSync
                | ToggleKind::SkipGitFetch
                | ToggleKind::SkipDirtySync
        )
    }
}

#[derive(Clone)]
pub enum SettingRow {
    Separator,
    Blank,
    SyncGeneralLink,
    ManageRepos,
    TdGeneralLink,
    TimeDoctorSettings,
    ContractPeriodsLink,
    Back,
}

impl SettingRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            SettingRow::SyncGeneralLink
                | SettingRow::ManageRepos
                | SettingRow::TdGeneralLink
                | SettingRow::TimeDoctorSettings
                | SettingRow::ContractPeriodsLink
                | SettingRow::Back
        )
    }
}

// ── General toggle row (shared by Sync General & TD General sub-screens) ────

#[derive(Clone)]
pub enum GeneralToggleRow {
    Toggle {
        kind: ToggleKind,
        label: &'static str,
        hint: &'static str,
        on: bool,
        indent: bool,
        disabled: bool,
    },
    TimezoneSelector {
        value: String,
    },
    Blank,
    Back,
}

impl GeneralToggleRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            GeneralToggleRow::Toggle { .. }
                | GeneralToggleRow::TimezoneSelector { .. }
                | GeneralToggleRow::Back
        )
    }
}

// ── Time Doctor settings row types ─────────────────────────────────────────

#[derive(Clone)]
pub enum TimeSettingRow {
    /// Editable text field (email)
    EditField {
        field: TimeDoctorField,
        label: &'static str,
        value: String,
        masked: bool,
    },
    /// Password row — shows [set] / [not set] badge instead of value
    Password {
        is_set: bool,
    },
    Blank,
    Back,
}

impl TimeSettingRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            TimeSettingRow::EditField { .. }
                | TimeSettingRow::Password { .. }
                | TimeSettingRow::Back
        )
    }
}

// ── Contract periods row types ──────��────────────────────────────────────────

#[derive(Clone)]
pub enum CpListRow {
    Period {
        index: usize,
        from: NaiveDate,
        weekly_hours: f64,
    },
    Blank,
    MondayField,
    HoursField,
    SavePeriod,
    Back,
}

impl CpListRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            CpListRow::Period { .. }
                | CpListRow::MondayField
                | CpListRow::HoursField
                | CpListRow::SavePeriod
                | CpListRow::Back
        )
    }
}

// ── Repo manager row types ────────────���───────────────────────────────────────

#[derive(Clone)]
pub enum RepoManagerRow {
    Blank,
    RepoToggle {
        name: String,
        url: String,
        enabled: bool,
    },
    AddUrl,
    RemoveReposLink,
    Back,
}

impl RepoManagerRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            RepoManagerRow::RepoToggle { .. }
                | RepoManagerRow::AddUrl
                | RepoManagerRow::RemoveReposLink
                | RepoManagerRow::Back
        )
    }
}

// ── Remove repos row types ──────────────────────────────────────────────────

#[derive(Clone)]
pub enum RemoveRepoRow {
    Blank,
    RepoDelete { name: String, url: String },
    Back,
}

impl RemoveRepoRow {
    pub fn is_interactive(&self) -> bool {
        matches!(self, RemoveRepoRow::RepoDelete { .. } | RemoveRepoRow::Back)
    }
}

// ── Contract period entry ─────────────────────────────────────────���──────────

#[derive(Clone)]
pub struct ContractPeriodEntry {
    pub from: NaiveDate,
    pub weekly_hours: f64,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn list_state_at(index: usize) -> ListState {
    let mut s = ListState::default();
    s.select(Some(index));
    s
}

// ── App ─────────────────────────────────────────────────────────────────────

pub struct App {
    pub screen: Screen,
    pub main_state: ListState,
    pub settings_state: ListState,
    pub sync_general_state: ListState,
    pub repo_manager_state: ListState,
    pub remove_repo_state: ListState,
    pub td_general_state: ListState,
    pub td_settings_state: ListState,
    pub cp_list_state: ListState,
    pub input_mode: InputMode,
    // in-memory settings state
    pub sync_all: bool,
    pub use_cache: bool,
    pub skip_fork_sync: bool,
    pub skip_rebase: bool,
    pub skip_rds_sync: bool,
    pub skip_git_fetch: bool,
    pub skip_dirty_sync: bool,
    pub repos: Vec<RepoEntry>,
    // in-memory Time Doctor settings
    pub td_email: String,
    pub td_timezone_idx: usize,
    pub td_skip_current_week: bool,
    pub td_use_time_cache: bool,
    pub td_show_cumulative: bool,
    pub td_password_is_set: bool,
    pub contract_periods: Vec<ContractPeriodEntry>,
    // Add contract period state (inline in ContractPeriods screen)
    pub add_cp_monday: NaiveDate,
    pub add_cp_hours_idx: usize,
}

#[derive(Clone)]
pub struct RepoEntry {
    pub name: String,
    pub url: String,
    pub enabled: bool,
}

fn timezone_index(tz: &str) -> usize {
    TIMEZONES.iter().position(|&t| t == tz).unwrap_or(7) // default: Europe/Istanbul
}

fn next_monday_from_today() -> NaiveDate {
    let today = Local::now().date_naive();
    let monday = get_week_start_monday(today);
    if monday == today {
        monday
    } else {
        monday + chrono::Duration::days(7)
    }
}

impl App {
    pub fn new() -> Result<Self> {
        let config = crate::config::load()?;
        let main_state = list_state_at(0);
        let settings_state = list_state_at(0);
        let sync_general_state = list_state_at(0);
        let repo_manager_state = list_state_at(0);
        let remove_repo_state = list_state_at(0);
        let td_general_state = list_state_at(0);
        let td_settings_state = list_state_at(0);
        let cp_list_state = list_state_at(0);
        let repos = config
            .sync
            .upstream_repos
            .iter()
            .map(|r| RepoEntry {
                name: r.name.clone(),
                url: r.url.clone(),
                enabled: r.enabled,
            })
            .collect();
        let td_email = config.time.email.clone().unwrap_or_default();
        let td_tz = config.time.timezone.as_deref().unwrap_or("Europe/Istanbul");
        let td_timezone_idx = timezone_index(td_tz);
        let td_skip_current_week = config.time.skip_current_week;
        let td_use_time_cache = config.time.use_time_cache;
        let td_show_cumulative = config.time.show_cumulative;
        let contract_periods: Vec<ContractPeriodEntry> = config
            .time
            .contract_periods
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|p| ContractPeriodEntry {
                from: p.from,
                weekly_hours: p.weekly_hours,
            })
            .collect();
        let td_password_is_set = crate::time::auth::load_token_from_keychain().is_some();
        Ok(Self {
            screen: Screen::MainMenu,
            main_state,
            settings_state,
            sync_general_state,
            repo_manager_state,
            remove_repo_state,
            td_general_state,
            td_settings_state,
            cp_list_state,
            input_mode: InputMode::Normal,
            sync_all: config.sync.sync_all_by_default,
            use_cache: config.sync.use_cache,
            skip_fork_sync: config.sync.skip_fork_sync,
            skip_rebase: config.sync.skip_rebase,
            skip_rds_sync: config.sync.skip_rds_sync,
            skip_git_fetch: config.sync.skip_git_fetch,
            skip_dirty_sync: config.sync.skip_dirty_sync,
            repos,
            td_email,
            td_timezone_idx,
            td_skip_current_week,
            td_use_time_cache,
            td_show_cumulative,
            td_password_is_set,
            contract_periods,
            add_cp_monday: next_monday_from_today(),
            add_cp_hours_idx: 1, // default 20h
        })
    }

    pub fn settings_items(&self) -> Vec<SettingRow> {
        vec![
            SettingRow::Separator, // "RDS Sync"
            SettingRow::Blank,
            SettingRow::SyncGeneralLink,
            SettingRow::ManageRepos,
            SettingRow::Blank,
            SettingRow::Separator, // "Time Doctor"
            SettingRow::Blank,
            SettingRow::TdGeneralLink,
            SettingRow::TimeDoctorSettings,
            SettingRow::ContractPeriodsLink,
            SettingRow::Blank,
            SettingRow::Back,
        ]
    }

    pub fn sync_general_items(&self) -> Vec<GeneralToggleRow> {
        vec![
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SyncAll,
                label: "Sync all by default",
                hint: "--sync-all",
                on: self.sync_all,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Toggle {
                kind: ToggleKind::UseCache,
                label: "Use repo path cache",
                hint: "",
                on: self.use_cache,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Blank,
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipForkSync,
                label: "Fork sync",
                hint: "fetch+merge+push upstream",
                on: !self.skip_fork_sync,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipGitFetch,
                label: "Git fetch",
                hint: "git fetch upstream",
                on: !self.skip_git_fetch,
                indent: true,
                disabled: self.skip_fork_sync,
            },
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipRebase,
                label: "Rebase",
                hint: "rebase branch after merge",
                on: !self.skip_rebase,
                indent: true,
                disabled: self.skip_fork_sync,
            },
            GeneralToggleRow::Blank,
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipRdsSync,
                label: "RDS sync",
                hint: "./sync in each repo",
                on: !self.skip_rds_sync,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipDirtySync,
                label: "Dirty repo sync",
                hint: "./sync on uncommitted changes",
                on: !self.skip_dirty_sync,
                indent: true,
                disabled: self.skip_rds_sync,
            },
            GeneralToggleRow::Blank,
            GeneralToggleRow::Back,
        ]
    }

    pub fn td_general_items(&self) -> Vec<GeneralToggleRow> {
        vec![
            GeneralToggleRow::Toggle {
                kind: ToggleKind::SkipCurrentWeek,
                label: "Include current week",
                hint: "show incomplete week",
                on: !self.td_skip_current_week,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Toggle {
                kind: ToggleKind::UseTimeCache,
                label: "Use time cache",
                hint: "",
                on: self.td_use_time_cache,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Toggle {
                kind: ToggleKind::ShowCumulative,
                label: "Show cumulative balance",
                hint: "running hour balance",
                on: self.td_show_cumulative,
                indent: false,
                disabled: false,
            },
            GeneralToggleRow::Blank,
            GeneralToggleRow::TimezoneSelector {
                value: TIMEZONES[self.td_timezone_idx].to_string(),
            },
            GeneralToggleRow::Blank,
            GeneralToggleRow::Back,
        ]
    }

    pub fn td_settings_items(&self) -> Vec<TimeSettingRow> {
        vec![
            TimeSettingRow::EditField {
                field: TimeDoctorField::Email,
                label: "Email",
                value: self.td_email.clone(),
                masked: false,
            },
            TimeSettingRow::Password {
                is_set: self.td_password_is_set,
            },
            TimeSettingRow::Blank,
            TimeSettingRow::Back,
        ]
    }

    pub fn cp_list_items(&self) -> Vec<CpListRow> {
        let mut rows: Vec<CpListRow> = self
            .contract_periods
            .iter()
            .enumerate()
            .map(|(i, p)| CpListRow::Period {
                index: i,
                from: p.from,
                weekly_hours: p.weekly_hours,
            })
            .collect();
        rows.push(CpListRow::Blank);
        rows.push(CpListRow::MondayField);
        rows.push(CpListRow::HoursField);
        rows.push(CpListRow::SavePeriod);
        rows.push(CpListRow::Blank);
        rows.push(CpListRow::Back);
        rows
    }

    pub fn repo_manager_items(&self) -> Vec<RepoManagerRow> {
        let mut rows: Vec<RepoManagerRow> = vec![];
        for r in &self.repos {
            rows.push(RepoManagerRow::RepoToggle {
                name: r.name.clone(),
                url: r.url.clone(),
                enabled: r.enabled,
            });
        }
        rows.push(RepoManagerRow::Blank);
        rows.push(RepoManagerRow::AddUrl);
        rows.push(RepoManagerRow::Blank);
        rows.push(RepoManagerRow::RemoveReposLink);
        rows.push(RepoManagerRow::Blank);
        rows.push(RepoManagerRow::Back);
        rows
    }

    pub fn remove_repo_items(&self) -> Vec<RemoveRepoRow> {
        let mut rows: Vec<RemoveRepoRow> = vec![];
        for r in &self.repos {
            rows.push(RemoveRepoRow::RepoDelete {
                name: r.name.clone(),
                url: r.url.clone(),
            });
        }
        rows.push(RemoveRepoRow::Blank);
        rows.push(RemoveRepoRow::Back);
        rows
    }

    pub fn toggle_by_kind(&mut self, kind: ToggleKind) {
        let flag = match kind {
            ToggleKind::SyncAll => &mut self.sync_all,
            ToggleKind::UseCache => &mut self.use_cache,
            ToggleKind::SkipForkSync => &mut self.skip_fork_sync,
            ToggleKind::SkipRebase => &mut self.skip_rebase,
            ToggleKind::SkipRdsSync => &mut self.skip_rds_sync,
            ToggleKind::SkipGitFetch => &mut self.skip_git_fetch,
            ToggleKind::SkipDirtySync => &mut self.skip_dirty_sync,
            ToggleKind::SkipCurrentWeek => &mut self.td_skip_current_week,
            ToggleKind::UseTimeCache => &mut self.td_use_time_cache,
            ToggleKind::ShowCumulative => &mut self.td_show_cumulative,
        };
        *flag = !*flag;
        if kind.is_sync() {
            self.persist_settings();
        } else {
            self.persist_td_settings();
        }
    }

    pub fn cycle_timezone(&mut self, delta: i32) {
        let len = TIMEZONES.len();
        if delta > 0 {
            self.td_timezone_idx = (self.td_timezone_idx + 1) % len;
        } else {
            self.td_timezone_idx = (self.td_timezone_idx + len - 1) % len;
        }
        self.persist_td_settings();
    }

    pub fn toggle_repo(&mut self, name: &str) {
        if let Some(repo) = self.repos.iter_mut().find(|r| r.name == name) {
            repo.enabled = !repo.enabled;
            self.persist_settings();
        }
    }

    pub fn confirm_delete_repo(&mut self, name: String) {
        self.input_mode = InputMode::ConfirmDelete(name);
    }

    pub fn execute_delete_repo(&mut self, name: &str) {
        self.repos.retain(|r| r.name != name);
        self.persist_settings();
        self.input_mode = InputMode::Normal;
        // Clamp cursor to a valid interactive row
        let rows = self.remove_repo_items();
        let last_interactive = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.is_interactive())
            .map(|(i, _)| i)
            .next_back()
            .unwrap_or(0);
        let cur = self.remove_repo_state.selected().unwrap_or(0);
        if cur > last_interactive {
            self.remove_repo_state.select(Some(last_interactive));
        }
    }

    pub fn confirm_delete_period(&mut self, index: usize) {
        self.input_mode = InputMode::ConfirmDeletePeriod(index);
    }

    pub fn execute_delete_period(&mut self, index: usize) {
        if index < self.contract_periods.len() {
            self.contract_periods.remove(index);
            self.persist_td_settings();
        }
        self.input_mode = InputMode::Normal;
        // Clamp cursor
        let rows = self.cp_list_items();
        let last_interactive = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.is_interactive())
            .map(|(i, _)| i)
            .next_back()
            .unwrap_or(0);
        let cur = self.cp_list_state.selected().unwrap_or(0);
        if cur > last_interactive {
            self.cp_list_state.select(Some(last_interactive));
        }
    }

    pub fn save_new_contract_period(&mut self) {
        let entry = ContractPeriodEntry {
            from: self.add_cp_monday,
            weekly_hours: WEEKLY_HOURS_OPTIONS[self.add_cp_hours_idx],
        };
        self.contract_periods.push(entry);
        self.contract_periods.sort_by_key(|p| p.from);
        self.persist_td_settings();
        // Reset add fields for next entry
        self.add_cp_monday = next_monday_from_today();
        self.add_cp_hours_idx = 1;
    }

    pub fn cycle_add_cp_monday(&mut self, delta: i32) {
        let days = if delta > 0 { 7 } else { -7 };
        self.add_cp_monday += chrono::Duration::days(days);
    }

    pub fn cycle_add_cp_hours(&mut self, delta: i32) {
        let len = WEEKLY_HOURS_OPTIONS.len();
        if delta > 0 {
            self.add_cp_hours_idx = (self.add_cp_hours_idx + 1) % len;
        } else {
            self.add_cp_hours_idx = (self.add_cp_hours_idx + len - 1) % len;
        }
    }

    /// Derive a short name from a URL (last path component, stripped of .git).
    fn name_from_url(url: &str) -> String {
        url.trim_end_matches('/')
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .unwrap_or(url)
            .to_string()
    }

    pub fn add_repo_from_input(&mut self, url: String) {
        let url = url.trim().to_string();
        if url.is_empty() {
            return;
        }
        let name = Self::name_from_url(&url);
        if !self.repos.iter().any(|r| r.url == url) {
            self.repos.push(RepoEntry {
                name,
                url,
                enabled: true,
            });
            self.persist_settings();
        }
    }

    pub fn persist_settings(&self) {
        if let Ok(mut config) = crate::config::load() {
            config.sync.sync_all_by_default = self.sync_all;
            config.sync.use_cache = self.use_cache;
            config.sync.skip_fork_sync = self.skip_fork_sync;
            config.sync.skip_rebase = self.skip_rebase;
            config.sync.skip_rds_sync = self.skip_rds_sync;
            config.sync.skip_git_fetch = self.skip_git_fetch;
            config.sync.skip_dirty_sync = self.skip_dirty_sync;
            config.sync.upstream_repos = self
                .repos
                .iter()
                .map(|r| crate::config::UpstreamRepo {
                    url: r.url.clone(),
                    name: r.name.clone(),
                    enabled: r.enabled,
                })
                .collect();
            let _ = crate::config::save(&config);
        }
    }

    pub fn persist_td_settings(&self) {
        if let Ok(mut config) = crate::config::load() {
            config.time.email = if self.td_email.is_empty() {
                None
            } else {
                Some(self.td_email.clone())
            };
            config.time.timezone = Some(TIMEZONES[self.td_timezone_idx].to_string());
            // start_date auto-derived from first contract period
            config.time.start_date = self.contract_periods.first().map(|p| p.from);
            config.time.skip_current_week = self.td_skip_current_week;
            config.time.use_time_cache = self.td_use_time_cache;
            config.time.show_cumulative = self.td_show_cumulative;
            config.time.contract_periods = if self.contract_periods.is_empty() {
                None
            } else {
                Some(
                    self.contract_periods
                        .iter()
                        .map(|p| ContractPeriod {
                            from: p.from,
                            weekly_hours: p.weekly_hours,
                        })
                        .collect(),
                )
            };
            let _ = crate::config::save(&config);
        }
    }

    /// Save password to keychain and update in-memory flag.
    pub fn set_td_password(&mut self, password: &str) {
        if password.is_empty() {
            return;
        }
        // Delete old session token so a fresh login is triggered with the new password
        let _ = crate::time::auth::delete_token_from_keychain();
        if let Ok(entry) = keyring::Entry::new("jotmate-timedoctor", "password") {
            let _ = entry.set_password(password);
            self.td_password_is_set = true;
        }
    }
}
