use anyhow::Result;
use ratatui::widgets::ListState;

// ── Screens ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    MainMenu,
    Settings,
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
    Back,
}

impl SettingRow {
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            SettingRow::Toggle { .. } | SettingRow::RepoToggle { .. } | SettingRow::Back
        )
    }
}

// ── App ───────────────────────────────────────────────────────────────────────

pub struct App {
    pub screen: Screen,
    pub main_state: ListState,
    pub settings_state: ListState,
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
        rows.push(SettingRow::Back);
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
            _ => {} // Blank, Separator, Back — do nothing
        }
    }

    pub fn persist_settings(&self) {
        if let Ok(mut config) = crate::config::load() {
            config.sync.sync_all_by_default = self.sync_all;
            config.sync.use_cache = self.use_cache;
            for repo in &mut config.sync.upstream_repos {
                if let Some(r) = self.repos.iter().find(|r| r.name == repo.name) {
                    repo.enabled = r.enabled;
                }
            }
            let _ = crate::config::save(&config);
        }
    }
}
