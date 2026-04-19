use std::path::Path;
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::tui::app::{RdsStatus, SyncUpdate};

use super::fork::ForkResult;
use super::git::{git, git_ok};

pub(super) struct RdsOpts {
    pub skip_rds_sync: bool,
    pub smart_sync: bool,
}

pub(super) async fn sync_rds(
    idx: usize,
    repo: &Path,
    fork_result: &ForkResult,
    tx: &mpsc::UnboundedSender<SyncUpdate>,
    opts: &RdsOpts,
) {
    if opts.skip_rds_sync {
        let _ = tx.send(SyncUpdate::Rds(
            idx,
            RdsStatus::Skipped("--skip-rds-sync".into()),
        ));
        return;
    }

    let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Preparing));

    // Decide whether to skip (mirrors prepare_project_sync from bash).
    if opts.smart_sync && matches!(fork_result, ForkResult::Unchanged) {
        match skip_reason(repo, tx, idx).await {
            SkipDecision::Skip(reason) => {
                let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Skipped(reason)));
                return;
            }
            SkipDecision::AlreadyReported => return,
            SkipDecision::Proceed => {}
        }
    }

    // Check if ./sync exists
    if !repo.join("sync").exists() {
        let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Skipped("no ./sync".into())));
        return;
    }

    // Run ./sync
    let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Running));
    let output = Command::new("./sync").current_dir(repo).output().await;

    match output {
        Ok(o) if o.status.success() => {
            let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Done));
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            let msg = if stderr.is_empty() {
                format!("exit {}", o.status.code().unwrap_or(1))
            } else {
                stderr.lines().next().unwrap_or("failed").to_string()
            };
            let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Error(msg)));
        }
        Err(e) => {
            let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Error(e.to_string())));
        }
    }
}

/// Outcome of `skip_reason` — tells the caller whether to proceed, skip with a
/// reason, or bail because a terminal status was already sent on the channel.
enum SkipDecision {
    Proceed,
    Skip(String),
    AlreadyReported,
}

/// When the fork was unchanged and smart sync is on, decide whether RDS sync can be skipped.
/// Dirty repos always proceed. May send `Pulling`/`Error` updates as a side effect.
async fn skip_reason(
    repo: &Path,
    tx: &mpsc::UnboundedSender<SyncUpdate>,
    idx: usize,
) -> SkipDecision {
    let porcelain = git(repo, &["status", "--porcelain"]).await.unwrap_or_default();

    if !porcelain.is_empty() {
        return SkipDecision::Proceed; // dirty → always run sync
    }

    // Clean repo, fork unchanged — check if behind origin
    let current_branch = git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .unwrap_or_default();
    if current_branch.is_empty() || current_branch == "HEAD" {
        return SkipDecision::Proceed;
    }

    let origin_ref = format!("origin/{current_branch}");
    if !git_ok(repo, &["rev-parse", "--verify", &origin_ref]).await {
        let _ = git(repo, &["fetch", "origin", &current_branch]).await;
    }
    if !git_ok(repo, &["rev-parse", "--verify", &origin_ref]).await {
        return SkipDecision::Proceed;
    }

    let behind = git(
        repo,
        &["rev-list", "--count", &format!("HEAD..{origin_ref}")],
    )
    .await
    .unwrap_or_default()
    .parse::<u32>()
    .unwrap_or(0);

    if behind > 0 {
        let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Pulling));
        if let Err(e) = git(repo, &["pull", "--ff-only", "origin", &current_branch]).await {
            let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Error(format!("pull: {e}"))));
            return SkipDecision::AlreadyReported;
        }
        return SkipDecision::Proceed;
    }

    let ahead = git(
        repo,
        &["rev-list", "--count", &format!("{origin_ref}..HEAD")],
    )
    .await
    .unwrap_or_default()
    .parse::<u32>()
    .unwrap_or(0);

    if ahead == 0 {
        SkipDecision::Skip("no changes".into())
    } else {
        SkipDecision::Proceed
    }
}
