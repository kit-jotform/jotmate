//! Native (no-bash) sync engine for the TUI sync screen.
//!
//! Split by responsibility:
//! - [`git`] — git command helpers and default-branch detection
//! - [`fork`] — per-repo fork sync pipeline (fetch/merge/push/rebase)
//! - [`rds`] — per-repo RDS sync pipeline (./sync invocation + skip logic)
//! - [`elapsed`] — per-repo wall-clock elapsed reporter
//!
//! This `mod.rs` owns only the public [`SyncOpts`] + [`run_tui`] entry point
//! and the two-phase orchestration (fork → rds) over all repos.

use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::tui::app::SyncUpdate;

mod elapsed;
mod fork;
mod git;
mod rds;

use elapsed::track_elapsed;
use fork::{sync_fork, ForkOpts, ForkResult};
use rds::{sync_rds, RdsOpts};

pub struct SyncOpts {
    pub skip_fork_sync: bool,
    pub skip_git_fetch: bool,
    pub skip_rebase: bool,
    pub skip_rds_sync: bool,
    pub smart_sync: bool,
}

pub async fn run_tui(
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

    // Phase 1: Fork sync — all repos in parallel
    let now = Instant::now();
    let starts: Vec<(usize, Instant)> = repos.iter().map(|&(idx, _)| (idx, now)).collect();
    let elapsed_handle = tokio::spawn(track_elapsed(tx.clone(), starts));

    let mut fork_handles = Vec::new();
    for &(idx, ref path) in &repos {
        let tx = tx.clone();
        let path = path.clone();
        fork_handles.push(tokio::spawn(async move {
            let opts = ForkOpts {
                skip_fork_sync,
                skip_git_fetch,
                skip_rebase,
            };
            let result = sync_fork(idx, &path, &tx, &opts).await;
            (idx, result)
        }));
    }

    let mut fork_results: Vec<(usize, ForkResult)> = Vec::new();
    for handle in fork_handles {
        if let Ok(result) = handle.await {
            fork_results.push(result);
        }
    }

    // Phase 2: RDS sync — all repos in parallel (after all forks complete)
    let mut rds_handles = Vec::new();
    for &(idx, ref path) in &repos {
        let tx = tx.clone();
        let path = path.clone();

        // Flatten the fork result into a (possibly synthetic) value for the RDS task.
        // We only need to distinguish Unchanged/Error/Updated; the Error payload is not used.
        let fork_slot = fork_results.iter().find(|(i, _)| *i == idx).map(|(_, r)| r);
        let fake_result = match fork_slot {
            Some(ForkResult::Unchanged) => ForkResult::Unchanged,
            Some(ForkResult::Error(_)) => ForkResult::Error(String::new()),
            _ => ForkResult::Updated,
        };

        rds_handles.push(tokio::spawn(async move {
            let opts = RdsOpts {
                skip_rds_sync,
                smart_sync,
            };
            sync_rds(idx, &path, &fake_result, &tx, &opts).await;
        }));
    }

    for handle in rds_handles {
        let _ = handle.await;
    }

    elapsed_handle.abort();
}
