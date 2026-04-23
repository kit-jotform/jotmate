//! Native (no-bash) sync engine shared by the TUI and the headless CLI.
//!
//! Split by responsibility:
//! - [`git`] — git command helpers and default-branch detection
//! - [`fork`] — per-repo fork sync pipeline (fetch/merge/push/rebase)
//! - [`rds`] — per-repo RDS sync pipeline (./sync invocation + skip logic)
//! - [`elapsed`] — per-repo wall-clock elapsed reporter
//! - [`headless`] — single-line CLI renderer that wraps [`run_tui`]
//!
//! This `mod.rs` owns only the public [`SyncOpts`] + [`run_tui`] entry point
//! and the two-phase orchestration (fork → rds) over all repos.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::tui::app::SyncUpdate;

mod elapsed;
pub mod fork;
pub mod git;
mod headless;
pub mod rds;

pub use git::{GitExec, SubprocessGit};
pub use headless::run_headless;

use elapsed::track_elapsed;
use fork::{sync_fork, ForkOpts};
use rds::{sync_rds, RdsOpts};

#[derive(Debug)]
pub struct SyncOpts {
    pub skip_fork_sync: bool,
    pub skip_git_fetch: bool,
    pub skip_rebase: bool,
    pub skip_rds_sync: bool,
    pub smart_sync: bool,
}

pub async fn run_tui(
    git: Arc<dyn GitExec>,
    repos: Vec<(usize, PathBuf)>,
    tx: mpsc::UnboundedSender<SyncUpdate>,
    opts: SyncOpts,
) {
    let SyncOpts {
        skip_fork_sync,
        skip_git_fetch,
        skip_rebase,
        skip_rds_sync,
        smart_sync,
    } = opts;

    let now = Instant::now();
    let starts: Vec<(usize, Instant)> = repos.iter().map(|&(idx, _)| (idx, now)).collect();
    let elapsed_handle = tokio::spawn(track_elapsed(tx.clone(), starts));

    // Each repo gets a pipeline task: fork sync → immediately followed by its own RDS sync.
    // This means a fast repo's RDS can start while slower repos are still fetching upstream.
    let mut handles = Vec::new();
    for &(idx, ref path) in &repos {
        let tx = tx.clone();
        let path = path.clone();
        let git = git.clone();
        handles.push(tokio::spawn(async move {
            let fork_opts = ForkOpts {
                skip_fork_sync,
                skip_git_fetch,
                skip_rebase,
            };
            let fork_result = sync_fork(git.as_ref(), idx, &path, &tx, &fork_opts).await;

            let rds_opts = RdsOpts {
                skip_rds_sync,
                smart_sync,
            };
            sync_rds(git.as_ref(), idx, &path, &fork_result, &tx, &rds_opts).await;
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }

    elapsed_handle.abort();
}
