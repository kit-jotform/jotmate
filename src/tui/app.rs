use anyhow::Result;
use chrono::{Local, NaiveDate};
use ratatui::widgets::ListState;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::config::{ContractPeriod, UpstreamRepo};
use crate::time::compute::get_week_start_monday;

// ── Main menu items ────────────────────────────────────────────────────────────

pub const MAIN_ITEMS: &[(&str, &str)] = &[
    ("Sync", "Sync RDS to upstream"),
    ("Time Doctor", "Track your work hours"),
    ("Settings", "Configure jotmate"),
    ("Exit", ""),
];

// ── Timezone options ──────────────────────────────────────────────────────────

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

// ── Screens ──────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Screen {
    MainMenu,
    Settings,
    SyncGeneralSettings,
    RepoManager,
    RemoveRepos,
    TdGeneralSettings,
    TimeDoctorSettings,
    ContractPeriods,
    SyncProgress,
    TimeDoctorReport,
}

// ── Time Doctor report state ─────────────────────────────────────────────────

pub enum TdReportState {
    Loading,
    Ready {
        rows: Vec<crate::time::compute::WeekRow>,
        show_cumulative: bool,
    },
    Error(String),
    NeedsReauth,
}

// ── Sync progress types ─────────────────────────────────────────────────────

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum ForkStatus {
    Pending,
    FetchingUpstream,
    CheckingDiff,
    UpToDate,
    Stashing,
    CheckingOut,
    Merging,
    PushingDefault,
    Rebasing,
    PushingBranch,
    Unstashing,
    Done,
    Skipped(String),
    Error(String),
}

impl ForkStatus {
    pub fn label(&self) -> &str {
        match self {
            ForkStatus::Pending => "waiting…",
            ForkStatus::FetchingUpstream => "fetching upstream…",
            ForkStatus::CheckingDiff => "checking diff…",
            ForkStatus::UpToDate => "up to date",
            ForkStatus::Stashing => "stashing…",
            ForkStatus::CheckingOut => "checking out…",
            ForkStatus::Merging => "merging…",
            ForkStatus::PushingDefault => "pushing default…",
            ForkStatus::Rebasing => "rebasing…",
            ForkStatus::PushingBranch => "pushing branch…",
            ForkStatus::Unstashing => "unstashing…",
            ForkStatus::Done => "done",
            ForkStatus::Skipped(_) => "skipped",
            ForkStatus::Error(_) => "error",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ForkStatus::Done | ForkStatus::UpToDate | ForkStatus::Skipped(_) | ForkStatus::Error(_)
        )
    }

    pub fn is_error(&self) -> bool {
        matches!(self, ForkStatus::Error(_))
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum RdsStatus {
    Pending,
    Preparing,
    Pulling,
    Running,
    Done,
    Skipped(String),
    Error(String),
}

impl RdsStatus {
    pub fn label(&self) -> &str {
        match self {
            RdsStatus::Pending => "waiting…",
            RdsStatus::Preparing => "preparing…",
            RdsStatus::Pulling => "pulling…",
            RdsStatus::Running => "running ./sync…",
            RdsStatus::Done => "done",
            RdsStatus::Skipped(_) => "skipped",
            RdsStatus::Error(_) => "error",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            RdsStatus::Done | RdsStatus::Skipped(_) | RdsStatus::Error(_)
        )
    }

    pub fn is_error(&self) -> bool {
        matches!(self, RdsStatus::Error(_))
    }
}

#[derive(Clone, Debug)]
pub struct RepoSyncState {
    pub name: String,
    pub path: PathBuf,
    pub fork_status: ForkStatus,
    pub rds_status: RdsStatus,
    pub started_at: Option<Instant>,
    pub elapsed_secs: f64,
}

impl RepoSyncState {
    pub fn is_complete(&self) -> bool {
        self.fork_status.is_terminal() && self.rds_status.is_terminal()
    }

    pub fn is_active(&self) -> bool {
        !self.fork_status.is_terminal() || !self.rds_status.is_terminal()
    }

    pub fn has_error(&self) -> bool {
        self.fork_status.is_error() || self.rds_status.is_error()
    }

    pub fn is_skipped(&self) -> bool {
        matches!(
            (&self.fork_status, &self.rds_status),
            (
                ForkStatus::Skipped(_) | ForkStatus::UpToDate,
                RdsStatus::Skipped(_)
            )
        )
    }
}

pub enum SyncUpdate {
    Fork(usize, ForkStatus),
    Rds(usize, RdsStatus),
    Elapsed(usize, f64),
}

pub struct SyncScreenState {
    pub repos: Vec<RepoSyncState>,
    pub tick: u8,
    pub sync_handle: Option<JoinHandle<()>>,
    pub update_rx: mpsc::UnboundedReceiver<SyncUpdate>,
}

// ── Input mode (used inside RepoManager and TimeDoctorSettings) ──────────────

#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    AddingRepo(String),         // buffer holds URL being typed
    ConfirmDelete(String),      // holds the repo name pending deletion
    ConfirmDeletePeriod(usize), // holds the period index pending deletion
    SelectingTimezone(usize),   // ↑↓ cycles timezone; stored value = snapshot for cancel
    EditingCpMonday(NaiveDate), // ↑↓ cycles monday date; stored value = snapshot for cancel
    EditingCpHours(usize),      // ↑↓ cycles hours option; stored value = snapshot for cancel
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

// ── Settings row types ────────────────────────────────────────────────────────────

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

// ── Contract periods row types ────────────────────────────────────────────────

#[derive(Clone)]
pub enum CpListRow {
    Period {
        index: usize,
        from: NaiveDate,
        weekly_hours: f64,
    },
    Blank,
    Separator,
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

// ── Repo manager row types ──────────────────────────────────────────────────────

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

// ── Helpers ─────────────────────────────────────────────────────────────────

fn list_state_at(index: usize) -> ListState {
    let mut s = ListState::default();
    s.select(Some(index));
    s
}

fn first_interactive<T>(rows: &[T], is_interactive: impl Fn(&T) -> bool) -> usize {
    rows.iter().position(is_interactive).unwrap_or(0)
}

fn navigate_simple(len: usize, current: usize, delta: i32) -> usize {
    if len == 0 {
        return 0;
    }
    let last = len - 1;
    if delta < 0 {
        if current == 0 { last } else { current - 1 }
    } else if current == last {
        0
    } else {
        current + 1
    }
}

fn navigate_rows<T>(
    rows: &[T],
    current: usize,
    delta: i32,
    is_interactive: impl Fn(&T) -> bool,
) -> usize {
    let len = rows.len();
    if len == 0 {
        return current;
    }
    let mut next = current;
    for _ in 0..len {
        next = navigate_simple(len, next, delta);
        if is_interactive(&rows[next]) {
            return next;
        }
    }
    current
}

fn clamp_to_last_interactive<T>(
    rows: &[T],
    current: usize,
    is_interactive: impl Fn(&T) -> bool,
) -> usize {
    let last = rows.iter().rposition(is_interactive).unwrap_or(0);
    current.min(last)
}

// ── App ─────────────────────────────────────────────────────────────────────

pub struct App {
    pub screen: Screen,
    pub list_states: HashMap<Screen, ListState>,
    pub td_report: TdReportState,
    pub td_report_scroll: usize,
    pub input_mode: InputMode,
    // in-memory settings state
    pub sync_all: bool,
    pub use_cache: bool,
    pub skip_fork_sync: bool,
    pub skip_rebase: bool,
    pub skip_rds_sync: bool,
    pub skip_git_fetch: bool,
    pub skip_dirty_sync: bool,
    pub repos: Vec<UpstreamRepo>,
    // in-memory Time Doctor settings
    pub td_email: String,
    pub td_timezone_idx: usize,
    pub td_skip_current_week: bool,
    pub td_use_time_cache: bool,
    pub td_show_cumulative: bool,
    pub td_password_is_set: bool,
    pub contract_periods: Vec<ContractPeriod>,
    // Add contract period state (inline in ContractPeriods screen)
    pub add_cp_monday: NaiveDate,
    pub add_cp_hours_idx: usize,
    // Sync progress state
    pub sync_state: Option<SyncScreenState>,
    // Auth error message to show on TimeDoctorSettings screen
    pub auth_error: Option<String>,
    // Channel for receiving TD report results from the background fetch task
    pub td_report_rx: Option<tokio::sync::oneshot::Receiver<Result<Vec<crate::time::compute::WeekRow>, String>>>,
}

/// Run a Time Doctor report fetch and map `TokenExpired` to the `TOKEN_EXPIRED` sentinel
/// consumed by `poll_td_report`, so the TUI can drop into foreground re-auth.
async fn fetch_report_result(
    email: &str,
    opts: crate::time::FetchOpts,
) -> std::result::Result<Vec<crate::time::compute::WeekRow>, String> {
    crate::time::fetch_report(email, opts).await.map_err(|e| {
        let is_expired = e
            .downcast_ref::<crate::error::AppError>()
            .map(|ae| matches!(ae, crate::error::AppError::TokenExpired))
            .unwrap_or(false);
        if is_expired {
            let _ = crate::time::auth::delete_token_from_keychain();
            "TOKEN_EXPIRED".to_string()
        } else {
            e.to_string()
        }
    })
}

/// Dispatches an expression across every list-screen's row-provider so both
/// `first_interactive` and `navigate_rows` can share one match arm per screen.
///
/// Usage: `with_rows!(self, screen, rows, is_int => body; default_expr)` —
/// inside `body`, `rows` is the `Vec<RowType>` and `is_int` is the corresponding
/// row type's `is_interactive` function.
macro_rules! with_rows {
    ($self:ident, $screen:expr, $rows:ident, $is_int:ident => $body:expr ; $default:expr) => {
        match $screen {
            Screen::Settings => {
                let $rows = $self.settings_items();
                let $is_int = SettingRow::is_interactive;
                $body
            }
            Screen::SyncGeneralSettings => {
                let $rows = $self.sync_general_items();
                let $is_int = GeneralToggleRow::is_interactive;
                $body
            }
            Screen::TdGeneralSettings => {
                let $rows = $self.td_general_items();
                let $is_int = GeneralToggleRow::is_interactive;
                $body
            }
            Screen::RepoManager => {
                let $rows = $self.repo_manager_items();
                let $is_int = RepoManagerRow::is_interactive;
                $body
            }
            Screen::RemoveRepos => {
                let $rows = $self.remove_repo_items();
                let $is_int = RemoveRepoRow::is_interactive;
                $body
            }
            Screen::TimeDoctorSettings => {
                let $rows = $self.td_settings_items();
                let $is_int = TimeSettingRow::is_interactive;
                $body
            }
            Screen::ContractPeriods => {
                let $rows = $self.cp_list_items();
                let $is_int = CpListRow::is_interactive;
                $body
            }
            _ => $default,
        }
    };
}

fn timezone_index(tz: &str) -> usize {
    TIMEZONES.iter().position(|&t| t == tz).unwrap_or(7) // default: Europe/Istanbul
}

fn this_monday() -> NaiveDate {
    get_week_start_monday(Local::now().date_naive())
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
        let td_tz = config.time.timezone.as_deref().unwrap_or("Europe/Istanbul");
        Ok(Self {
            screen: Screen::MainMenu,
            list_states,
            td_report: TdReportState::Loading,
            td_report_scroll: 0,
            input_mode: InputMode::Normal,
            sync_all: config.sync.sync_all_by_default,
            use_cache: config.sync.use_cache,
            skip_fork_sync: config.sync.skip_fork_sync,
            skip_rebase: config.sync.skip_rebase,
            skip_rds_sync: config.sync.skip_rds_sync,
            skip_git_fetch: config.sync.skip_git_fetch,
            skip_dirty_sync: config.sync.skip_dirty_sync,
            repos: config.sync.upstream_repos.clone(),
            td_email: config.time.email.clone().unwrap_or_default(),
            td_timezone_idx: timezone_index(td_tz),
            td_skip_current_week: config.time.skip_current_week,
            td_use_time_cache: config.time.use_time_cache,
            td_show_cumulative: config.time.show_cumulative,
            td_password_is_set: crate::time::auth::load_password_from_keychain().is_some(),
            contract_periods,
            add_cp_monday,
            add_cp_hours_idx: 1, // default 20h
            sync_state: None,
            auth_error: None,
            td_report_rx: None,
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
        rows.push(CpListRow::Separator);
        rows.push(CpListRow::MondayField);
        rows.push(CpListRow::HoursField);
        rows.push(CpListRow::Blank);
        rows.push(CpListRow::SavePeriod);
        rows.push(CpListRow::Separator);
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

    // ── List state accessors ───────────────────────────────────────────────

    pub fn list_state(&self, screen: Screen) -> &ListState {
        self.list_states
            .get(&screen)
            .unwrap_or_else(|| panic!("no ListState for screen {:?}", screen as u8))
    }

    pub fn list_state_mut(&mut self, screen: Screen) -> &mut ListState {
        self.list_states
            .get_mut(&screen)
            .unwrap_or_else(|| panic!("no ListState for screen {:?}", screen as u8))
    }

    pub fn selected_index(&self, screen: Screen) -> usize {
        self.list_state(screen).selected().unwrap_or(0)
    }

    pub fn select(&mut self, screen: Screen, i: usize) {
        self.list_state_mut(screen).select(Some(i));
    }

    /// Select the first interactive row of the given screen (for use when navigating in).
    pub fn select_first_interactive(&mut self, screen: Screen) {
        let i = with_rows!(self, screen, rows, is_int => first_interactive(&rows, is_int); 0);
        self.select(screen, i);
    }

    /// Navigate the current screen's list by `delta`, skipping non-interactive rows.
    pub fn navigate_current(&mut self, delta: i32) {
        let screen = self.screen;
        let cur = self.selected_index(screen);
        let next = with_rows!(
            self, screen, rows, is_int => navigate_rows(&rows, cur, delta, is_int);
            match screen {
                Screen::MainMenu => navigate_simple(MAIN_ITEMS.len(), cur, delta),
                _ => cur,
            }
        );
        self.select(screen, next);
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

    /// Cancel an in-progress cycle edit by restoring the snapshot stored in the InputMode variant.
    pub fn cancel_cycle_edit(&mut self) {
        match self.input_mode.clone() {
            InputMode::SelectingTimezone(snapshot) => {
                self.td_timezone_idx = snapshot;
                self.persist_td_settings();
            }
            InputMode::EditingCpMonday(snapshot) => {
                self.add_cp_monday = snapshot;
            }
            InputMode::EditingCpHours(snapshot) => {
                self.add_cp_hours_idx = snapshot;
            }
            _ => {}
        }
        self.input_mode = InputMode::Normal;
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
        let rows = self.remove_repo_items();
        let cur = self.selected_index(Screen::RemoveRepos);
        let clamped = clamp_to_last_interactive(&rows, cur, RemoveRepoRow::is_interactive);
        self.select(Screen::RemoveRepos, clamped);
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
        let rows = self.cp_list_items();
        let cur = self.selected_index(Screen::ContractPeriods);
        let clamped = clamp_to_last_interactive(&rows, cur, CpListRow::is_interactive);
        self.select(Screen::ContractPeriods, clamped);
    }

    pub fn save_new_contract_period(&mut self) {
        let entry = ContractPeriod {
            from: self.add_cp_monday,
            weekly_hours: WEEKLY_HOURS_OPTIONS[self.add_cp_hours_idx],
        };
        if let Some(existing) = self.contract_periods.iter_mut().find(|p| p.from == entry.from) {
            existing.weekly_hours = entry.weekly_hours;
        } else {
            self.contract_periods.push(entry);
            self.contract_periods.sort_by_key(|p| p.from);
            // New period added — bump selection to stay on the same row
            let i = self.selected_index(Screen::ContractPeriods);
            self.select(Screen::ContractPeriods, i + 1);
        }
        self.persist_td_settings();
        // Reset add fields for next entry
        self.add_cp_monday = self
            .contract_periods
            .last()
            .map(|p| p.from)
            .unwrap_or_else(this_monday);
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

    /// Initialize sync screen state from enabled repos and resolved paths.
    pub fn start_sync(
        &mut self,
        repo_paths: HashMap<String, PathBuf>,
        update_rx: mpsc::UnboundedReceiver<SyncUpdate>,
    ) {
        let repos: Vec<RepoSyncState> = self
            .repos
            .iter()
            .filter(|r| r.enabled)
            .filter_map(|r| {
                repo_paths.get(&r.name).map(|path| RepoSyncState {
                    name: r.name.clone(),
                    path: path.clone(),
                    fork_status: ForkStatus::Pending,
                    rds_status: RdsStatus::Pending,
                    started_at: None,
                    elapsed_secs: 0.0,
                })
            })
            .collect();
        self.sync_state = Some(SyncScreenState {
            repos,
            tick: 0,
            sync_handle: None,
            update_rx,
        });
        self.screen = Screen::SyncProgress;
    }

    /// Apply a sync update message to the sync state.
    pub fn apply_sync_update(&mut self, update: SyncUpdate) {
        if let Some(state) = &mut self.sync_state {
            match update {
                SyncUpdate::Fork(idx, status) => {
                    if let Some(repo) = state.repos.get_mut(idx) {
                        if repo.started_at.is_none() {
                            repo.started_at = Some(Instant::now());
                        }
                        repo.fork_status = status;
                    }
                }
                SyncUpdate::Rds(idx, status) => {
                    if let Some(repo) = state.repos.get_mut(idx) {
                        repo.rds_status = status;
                    }
                }
                SyncUpdate::Elapsed(idx, secs) => {
                    if let Some(repo) = state.repos.get_mut(idx) {
                        repo.elapsed_secs = secs;
                    }
                }
            }
        }
    }

    /// Check if all sync repos are in a terminal state.
    pub fn sync_is_complete(&self) -> bool {
        self.sync_state
            .as_ref()
            .map(|s| s.repos.iter().all(|r| r.is_complete()))
            .unwrap_or(true)
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
            self.repos.push(UpstreamRepo {
                url,
                name,
                enabled: true,
            });
            self.persist_settings();
        }
    }

    pub fn persist_settings(&self) {
        self.mutate_and_save(|cfg| {
            cfg.sync.sync_all_by_default = self.sync_all;
            cfg.sync.use_cache = self.use_cache;
            cfg.sync.skip_fork_sync = self.skip_fork_sync;
            cfg.sync.skip_rebase = self.skip_rebase;
            cfg.sync.skip_rds_sync = self.skip_rds_sync;
            cfg.sync.skip_git_fetch = self.skip_git_fetch;
            cfg.sync.skip_dirty_sync = self.skip_dirty_sync;
            cfg.sync.upstream_repos = self.repos.clone();
        });
    }

    pub fn persist_td_settings(&self) {
        self.mutate_and_save(|cfg| {
            cfg.time.email = Some(self.td_email.clone()).filter(|e| !e.is_empty());
            cfg.time.timezone = Some(TIMEZONES[self.td_timezone_idx].to_string());
            // start_date auto-derived from first contract period
            cfg.time.start_date = self.contract_periods.first().map(|p| p.from);
            cfg.time.skip_current_week = self.td_skip_current_week;
            cfg.time.use_time_cache = self.td_use_time_cache;
            cfg.time.show_cumulative = self.td_show_cumulative;
            cfg.time.contract_periods = Some(self.contract_periods.clone()).filter(|c| !c.is_empty());
        });
    }

    /// Load the config, apply `mutate`, and save — ignoring errors at either end.
    /// This is the single place where in-memory state is flushed back to disk.
    fn mutate_and_save(&self, mutate: impl FnOnce(&mut crate::config::Config)) {
        if let Ok(mut config) = crate::config::load() {
            mutate(&mut config);
            let _ = crate::config::save(&config);
        }
    }

    /// Navigate to the Time Doctor report screen and kick off a background fetch.
    pub fn launch_td_report(&mut self) {
        self.screen = Screen::TimeDoctorReport;
        self.td_report = TdReportState::Loading;
        self.td_report_scroll = 0;
        self.td_report_rx = None;

        let Some(start_date) = self.contract_periods.first().map(|p| p.from) else {
            self.td_report = TdReportState::Error("No contract periods configured".to_string());
            return;
        };
        let opts = crate::time::FetchOpts {
            timezone: TIMEZONES[self.td_timezone_idx].to_string(),
            contract_periods: self.contract_periods.clone(),
            start_date,
            skip_current: self.td_skip_current_week,
            no_cache: !self.td_use_time_cache,
        };

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.td_report_rx = Some(rx);
        let email = self.td_email.clone();
        tokio::spawn(async move {
            let _ = tx.send(fetch_report_result(&email, opts).await);
        });
    }

    /// Poll for a completed TD report result. Returns true if state changed.
    pub fn poll_td_report(&mut self) -> bool {
        let result = match &mut self.td_report_rx {
            Some(rx) => match rx.try_recv() {
                Ok(v) => v,
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => return false,
                Err(_) => return false,
            },
            None => return false,
        };
        self.td_report_rx = None;
        match result {
            Ok(mut rows) => {
                let reset_from = crate::config::load()
                    .ok()
                    .and_then(|c| c.time.reset_cumulative_from_date);
                crate::time::compute::compute_cumulative(&mut rows, reset_from);
                rows.reverse();
                self.td_report_scroll = rows.len().saturating_sub(6);
                self.td_report = TdReportState::Ready {
                    rows,
                    show_cumulative: self.td_show_cumulative,
                };
            }
            Err(msg) if msg == "TOKEN_EXPIRED" => {
                self.td_report = TdReportState::NeedsReauth;
            }
            Err(msg) => {
                self.td_report = TdReportState::Error(msg);
            }
        }
        true
    }

    /// Save password to keychain and update in-memory flag.
    /// Returns true if the password was saved successfully.
    pub fn set_td_password(&mut self, password: &str) -> bool {
        if password.is_empty() {
            return false;
        }
        // Delete old session token so a fresh login is triggered with the new password
        let _ = crate::time::auth::delete_token_from_keychain();
        match crate::time::auth::save_password_to_keychain(password) {
            Ok(()) => {
                self.td_password_is_set = true;
                true
            }
            Err(_) => false,
        }
    }
}
