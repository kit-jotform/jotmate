use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::tui::sync_state::{ForkStatus, RdsStatus, RepoSyncState, SyncScreenState, SyncUpdate};

use super::screen::Screen;
use super::App;

impl App {
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
}
