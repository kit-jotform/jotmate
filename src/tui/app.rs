use anyhow::Result;
use ratatui::widgets::ListState;

// ── Screens ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    MainMenu,
    Settings,
    RepoManager,
}

// ── Input mode (used inside RepoManager) ─────────────────────────────────────

#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    AddingRepo(String),       // buffer holds URL being typed
    ConfirmDelete(String),    // holds the repo name pending deletion
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
    Back,
}

impl SettingRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            SettingRow::Toggle { .. }
                | SettingRow::RepoToggle { .. }
                | SettingRow::ManageRepos
                | SettingRow::Back
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
    pub input_mode: InputMode,
    // in-memory settings state
    pub sync_all: bool,
    pub use_cache: bool,
    pub repos: Vec<RepoEntry>,
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
        Ok(Self {
            screen: Screen::MainMenu,
            main_state,
            settings_state,
            repo_manager_state,
            input_mode: InputMode::Normal,
            sync_all: config.sync.sync_all_by_default,
            use_cache: config.sync.use_cache,
            repos,
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
        rows.push(SettingRow::Back);
        rows
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
}
