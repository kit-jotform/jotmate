use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

use crate::tui::sync_state::{
    DiscoveryResult, ForkStatus, RdsStatus, RepoSyncState, SyncPhase, SyncScreenState, SyncUpdate,
};

use super::screen::Screen;
use super::App;

impl App {
    fn open_sync_screen(
        &mut self,
        repos: Vec<RepoSyncState>,
        phase: SyncPhase,
        setup_error: Option<String>,
    ) -> mpsc::UnboundedSender<SyncUpdate> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.sync_state = Some(SyncScreenState {
            repos,
            tick: 0,
            sync_handle: None,
            update_rx: rx,
            phase,
            setup_error,
            discovery_rx: None,
        });
        self.sync_scroll = 0;
        self.screen = Screen::SyncProgress;
        tx
    }

    pub fn enter_sync_discovering_with(
        &mut self,
        discovery_rx: oneshot::Receiver<DiscoveryResult>,
    ) {
        let _ = self.open_sync_screen(Vec::new(), SyncPhase::Discovering, None);
        if let Some(state) = &mut self.sync_state {
            state.discovery_rx = Some(discovery_rx);
        }
    }

    pub fn take_discovery_result(&mut self) -> Option<DiscoveryResult> {
        let state = self.sync_state.as_mut()?;
        let rx = state.discovery_rx.as_mut()?;
        match rx.try_recv() {
            Ok(result) => {
                state.discovery_rx = None;
                Some(result)
            }
            Err(oneshot::error::TryRecvError::Empty) => None,
            Err(oneshot::error::TryRecvError::Closed) => {
                state.discovery_rx = None;
                Some(Err("discovery task was cancelled".into()))
            }
        }
    }

    /// Launcher supplies the repo list (`App::sync.repos` is UI-only).
    pub fn start_sync(
        &mut self,
        ordered_repos: Vec<(String, PathBuf)>,
    ) -> mpsc::UnboundedSender<SyncUpdate> {
        let repos: Vec<RepoSyncState> = ordered_repos
            .into_iter()
            .map(|(name, path)| RepoSyncState {
                name,
                path,
                fork_status: ForkStatus::Pending,
                rds_status: RdsStatus::Pending,
                started_at: None,
                elapsed_secs: 0.0,
            })
            .collect();
        self.open_sync_screen(repos, SyncPhase::Syncing, None)
    }

    pub fn fail_sync_setup(&mut self, message: impl Into<String>) {
        let _ = self.open_sync_screen(Vec::new(), SyncPhase::Failed, Some(message.into()));
    }

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
                        if !repo.is_complete() {
                            repo.elapsed_secs = secs;
                        }
                    }
                }
            }
        }
    }

    pub fn sync_is_complete(&self) -> bool {
        let Some(state) = self.sync_state.as_ref() else {
            return true;
        };
        match state.phase {
            SyncPhase::Discovering => false,
            SyncPhase::Failed => true,
            SyncPhase::Syncing => state.repos.iter().all(|r| r.is_complete()),
        }
    }

    /// Aborts the background task; used from sync keys and Ctrl-C handling.
    pub fn cancel_sync(&mut self) {
        if let Some(state) = self.sync_state.take() {
            if let Some(handle) = state.sync_handle {
                handle.abort();
            }
        }
    }
}
