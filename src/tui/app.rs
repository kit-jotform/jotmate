use anyhow::Result;
use chrono::NaiveDate;
use ratatui::widgets::ListState;

// ── Main menu items ───────────────────────────────────────────────────────────

pub const MAIN_ITEMS: &[(&str, &str)] = &[
    ("Sync", "Sync RDS to upstream"),
    ("Time Doctor", "Track your work hours"),
    ("Settings", "Configure jotmate"),
    ("Exit", ""),
];

// ── Screens ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    MainMenu,
    Settings,
    RepoManager,
    TimeDoctorSettings,
}

// ── Input mode (used inside RepoManager and TimeDoctorSettings) ──────────────

#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    AddingRepo(String),       // buffer holds URL being typed
    ConfirmDelete(String),    // holds the repo name pending deletion
    EditingField {            // editing a text field in TimeDoctorSettings
        field: TimeDoctorField,
        buf: String,
    },
}

/// Which Time Doctor field is being edited
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TimeDoctorField {
    Email,
    Password,
    StartDate,
    Timezone,
    ContractPeriods,
}

// ── Settings row types ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum ToggleKind {
    SyncAll,
    UseCache,
}

#[derive(Clone)]
pub enum SettingRow {
    Toggle {
        kind: ToggleKind,
        label: &'static str,
        hint: &'static str,
        on: bool,
    },
    Separator,
    Blank,
    RepoToggle {
        name: String,
        url: String,
        enabled: bool,
    },
    ManageRepos,
    TimeDoctorSettings,
    Back,
}

impl SettingRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            SettingRow::Toggle { .. }
                | SettingRow::RepoToggle { .. }
                | SettingRow::ManageRepos
                | SettingRow::TimeDoctorSettings
                | SettingRow::Back
        )
    }
}

// ── Time Doctor settings row types ───────────────────────────────────────────

#[derive(Clone)]
pub enum TimeSettingRow {
    /// Editable text field (email, timezone, start date, contract periods)
    EditField {
        field: TimeDoctorField,
        label: &'static str,
        value: String,          // current config value (empty = not set)
        masked: bool,           // true → display as stars
    },
    /// Password row — shows [set] / [not set] badge instead of value
    Password {
        is_set: bool,
    },
    Toggle {
        label: &'static str,
        hint: &'static str,
        on: bool,
    },
    Separator,
    Blank,
    Back,
}

impl TimeSettingRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            TimeSettingRow::EditField { .. }
                | TimeSettingRow::Password { .. }
                | TimeSettingRow::Toggle { .. }
                | TimeSettingRow::Back
        )
    }
}

// ── Repo manager row types ────────────────────────────────────────────────────

#[derive(Clone)]
pub enum RepoManagerRow {
    Blank,
    RepoDelete {
        name: String,
        url: String,
    },
    AddUrl,
    Back,
}

impl RepoManagerRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            RepoManagerRow::RepoDelete { .. } | RepoManagerRow::AddUrl | RepoManagerRow::Back
        )
    }
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct App {
    pub screen: Screen,
    pub main_state: ListState,
    pub settings_state: ListState,
    pub repo_manager_state: ListState,
    pub td_settings_state: ListState,
    pub input_mode: InputMode,
    // in-memory settings state
    pub sync_all: bool,
    pub use_cache: bool,
    pub repos: Vec<RepoEntry>,
    // in-memory Time Doctor settings
    pub td_email: String,
    pub td_timezone: String,
    pub td_start_date: String,
    pub td_skip_current_week: bool,
    pub td_contract_periods: String,
    pub td_password_is_set: bool,
}

#[derive(Clone)]
pub struct RepoEntry {
    pub name: String,
    pub url: String,
    pub enabled: bool,
}

impl App {
    pub fn new() -> Result<Self> {
        let config = crate::config::load()?;
        let mut main_state = ListState::default();
        main_state.select(Some(0));
        let mut settings_state = ListState::default();
        settings_state.select(Some(0));
        let mut repo_manager_state = ListState::default();
        repo_manager_state.select(Some(0));
        let mut td_settings_state = ListState::default();
        td_settings_state.select(Some(0));
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
        let td_timezone = config
            .time
            .timezone
            .clone()
            .unwrap_or_else(|| "Europe/Istanbul".to_string());
        let td_start_date = config
            .time
            .start_date
            .map(|d| d.to_string())
            .unwrap_or_default();
        let td_skip_current_week = config.time.skip_current_week;
        let td_contract_periods = config
            .time
            .contract_periods
            .as_deref()
            .map(|ps| {
                ps.iter()
                    .map(|p| format!("{}:{}", p.from, p.weekly_hours))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();
        let td_password_is_set = crate::time::auth::load_token_from_keychain().is_some();
        Ok(Self {
            screen: Screen::MainMenu,
            main_state,
            settings_state,
            repo_manager_state,
            td_settings_state,
            input_mode: InputMode::Normal,
            sync_all: config.sync.sync_all_by_default,
            use_cache: config.sync.use_cache,
            repos,
            td_email,
            td_timezone,
            td_start_date,
            td_skip_current_week,
            td_contract_periods,
            td_password_is_set,
        })
    }

    pub fn settings_items(&self) -> Vec<SettingRow> {
        let mut rows = vec![
            SettingRow::Toggle {
                kind: ToggleKind::SyncAll,
                label: "Sync all by default",
                hint: "--sync-all",
                on: self.sync_all,
            },
            SettingRow::Toggle {
                kind: ToggleKind::UseCache,
                label: "Use repo path cache",
                hint: "",
                on: self.use_cache,
            },
            SettingRow::Blank,
            SettingRow::Separator,
            SettingRow::Blank,
        ];
        for r in &self.repos {
            rows.push(SettingRow::RepoToggle {
                name: r.name.clone(),
                url: r.url.clone(),
                enabled: r.enabled,
            });
        }
        rows.push(SettingRow::Blank);
        rows.push(SettingRow::ManageRepos);
        rows.push(SettingRow::Blank);
        rows.push(SettingRow::Separator);
        rows.push(SettingRow::Blank);
        rows.push(SettingRow::TimeDoctorSettings);
        rows.push(SettingRow::Blank);
        rows.push(SettingRow::Back);
        rows
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
            TimeSettingRow::Separator,
            TimeSettingRow::Blank,
            TimeSettingRow::EditField {
                field: TimeDoctorField::Timezone,
                label: "Timezone",
                value: self.td_timezone.clone(),
                masked: false,
            },
            TimeSettingRow::EditField {
                field: TimeDoctorField::StartDate,
                label: "Start date",
                value: self.td_start_date.clone(),
                masked: false,
            },
            TimeSettingRow::Toggle {
                label: "Skip current week",
                hint: "exclude incomplete week",
                on: self.td_skip_current_week,
            },
            TimeSettingRow::EditField {
                field: TimeDoctorField::ContractPeriods,
                label: "Contract periods",
                value: self.td_contract_periods.clone(),
                masked: false,
            },
            TimeSettingRow::Blank,
            TimeSettingRow::Back,
        ]
    }

    pub fn repo_manager_items(&self) -> Vec<RepoManagerRow> {
        let mut rows: Vec<RepoManagerRow> = vec![];
        for r in &self.repos {
            rows.push(RepoManagerRow::RepoDelete {
                name: r.name.clone(),
                url: r.url.clone(),
            });
        }
        rows.push(RepoManagerRow::Blank);
        rows.push(RepoManagerRow::AddUrl);
        rows.push(RepoManagerRow::Blank);
        rows.push(RepoManagerRow::Back);
        rows
    }

    pub fn toggle_selected_setting(&mut self) {
        let idx = self.settings_state.selected().unwrap_or(0);
        match self.settings_items().get(idx) {
            Some(SettingRow::Toggle {
                kind: ToggleKind::SyncAll,
                ..
            }) => {
                self.sync_all = !self.sync_all;
                self.persist_settings();
            }
            Some(SettingRow::Toggle {
                kind: ToggleKind::UseCache,
                ..
            }) => {
                self.use_cache = !self.use_cache;
                self.persist_settings();
            }
            Some(SettingRow::RepoToggle { name, .. }) => {
                let name = name.clone();
                if let Some(repo) = self.repos.iter_mut().find(|r| r.name == name) {
                    repo.enabled = !repo.enabled;
                    self.persist_settings();
                }
            }
            _ => {}
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
        let rows = self.repo_manager_items();
        let last_interactive = rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.is_interactive())
            .map(|(i, _)| i)
            .next_back()
            .unwrap_or(0);
        let cur = self.repo_manager_state.selected().unwrap_or(0);
        if cur > last_interactive {
            self.repo_manager_state.select(Some(last_interactive));
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
            config.time.timezone = if self.td_timezone.is_empty() {
                None
            } else {
                Some(self.td_timezone.clone())
            };
            config.time.start_date = NaiveDate::parse_from_str(&self.td_start_date, "%Y-%m-%d").ok();
            config.time.skip_current_week = self.td_skip_current_week;
            config.time.contract_periods = if self.td_contract_periods.is_empty() {
                None
            } else {
                crate::config::parse_contract_periods(&self.td_contract_periods).ok()
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
