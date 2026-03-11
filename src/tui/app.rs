use anyhow::Result;
use ratatui::widgets::ListState;

// ── Screens ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    MainMenu,
    Settings,
}

// ── Main menu ─────────────────────────────────────────────────────────────────

pub const MAIN_ITEM_COUNT: usize = 4; // Sync, Time Doctor, Settings, Exit

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
            .map(|r| RepoEntry { name: r.name.clone(), url: r.url.clone(), enabled: r.enabled })
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

    pub fn settings_items(&self) -> Vec<String> {
        let sa = if self.sync_all { "ON " } else { "OFF" };
        let uc = if self.use_cache { "ON " } else { "OFF" };
        let mut items = vec![
            format!("[{sa}]  Sync all by default  (--sync-all)"),
            format!("[{uc}]  Use repo path cache"),
            "── Upstream Repositories ───────────────────────".to_string(),
        ];
        for r in &self.repos {
            let b = if r.enabled { "ON " } else { "OFF" };
            items.push(format!("[{b}]  {}  <{}>", r.name, r.url));
        }
        items.push("  ← Back".to_string());
        items
    }

    pub fn settings_item_count(&self) -> usize {
        // 2 toggles + 1 separator + repos + back
        3 + self.repos.len() + 1
    }

    pub fn toggle_selected_setting(&mut self) {
        let idx = self.settings_state.selected().unwrap_or(0);
        match idx {
            0 => {
                self.sync_all = !self.sync_all;
                self.persist_settings();
            }
            1 => {
                self.use_cache = !self.use_cache;
                self.persist_settings();
            }
            2 => {} // separator — do nothing
            n => {
                let repo_idx = n - 3;
                if repo_idx < self.repos.len() {
                    self.repos[repo_idx].enabled = !self.repos[repo_idx].enabled;
                    self.persist_settings();
                }
                // "← Back" row (last item) is handled by the caller
            }
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
