use std::path::Path;
use tokio::sync::mpsc;

use crate::tui::app::{RdsStatus, SyncUpdate};

use super::fork::ForkResult;
use super::git::{detect_default_branch, GitExec};

pub struct RdsOpts {
    pub skip_rds_sync: bool,
    pub smart_sync: bool,
}

pub async fn sync_rds(
    git: &dyn GitExec,
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

    if opts.smart_sync && matches!(fork_result, ForkResult::Unchanged) {
        match skip_reason(git, repo, tx, idx).await {
            SkipDecision::Skip(reason) => {
                let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Skipped(reason)));
                return;
            }
            SkipDecision::AlreadyReported => return,
            SkipDecision::Proceed => {}
        }
    }

    if !repo.join("sync").exists() {
        let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Skipped("no ./sync".into())));
        return;
    }

    let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Running));
    match git.run_rds_script(repo).await {
        Ok(()) => {
            let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Done));
        }
        Err(msg) => {
            let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Error(msg)));
        }
    }
}

enum SkipDecision {
    Proceed,
    Skip(String),
    AlreadyReported,
}

/// One `git status --porcelain=v2 --branch` covers dirty state, branch, and
/// ahead/behind — replaces 4-6 sequential git calls.
async fn skip_reason(
    git: &dyn GitExec,
    repo: &Path,
    tx: &mpsc::UnboundedSender<SyncUpdate>,
    idx: usize,
) -> SkipDecision {
    let status = git
        .git(repo, &["status", "--porcelain=v2", "--branch"])
        .await
        .unwrap_or_default();

    let dirty = status.lines().any(|l| !l.starts_with('#'));
    if dirty {
        return SkipDecision::Proceed;
    }

    let branch = status
        .lines()
        .find(|l| l.starts_with("# branch.head "))
        .and_then(|l| l.strip_prefix("# branch.head "))
        .unwrap_or("")
        .trim()
        .to_string();
    if branch.is_empty() || branch == "(detached)" {
        return SkipDecision::Proceed;
    }

    let ab_line = status.lines().find(|l| l.starts_with("# branch.ab "));
    let (ahead, behind) = match ab_line {
        None => {
            // Missing `branch.ab` means upstream isn't tracked yet; fetch and proceed.
            let _ = git.git(repo, &["fetch", "origin", &branch]).await;
            return SkipDecision::Proceed;
        }
        Some(line) => {
            let mut ahead = 0u32;
            let mut behind = 0u32;
            for token in line.split_whitespace() {
                if let Some(n) = token.strip_prefix('+') {
                    ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = token.strip_prefix('-') {
                    behind = n.parse().unwrap_or(0);
                }
            }
            (ahead, behind)
        }
    };

    if behind > 0 {
        let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Pulling));
        if let Err(e) = git
            .git(repo, &["pull", "--ff-only", "origin", &branch])
            .await
        {
            let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Error(format!("pull: {e}"))));
            return SkipDecision::AlreadyReported;
        }
        return SkipDecision::Proceed;
    }

    if ahead > 0 {
        return SkipDecision::Proceed;
    }

    // Branch may be in sync with its own remote yet still carry commits the
    // default branch hasn't seen — those need to reach RDS too.
    if let Some(default_branch) = detect_default_branch(git, repo).await {
        if default_branch != branch {
            let count = git
                .git(
                    repo,
                    &["rev-list", "--count", &format!("{default_branch}..HEAD")],
                )
                .await
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok())
                .unwrap_or(0);
            if count > 0 {
                return SkipDecision::Proceed;
            }
        }
    }

    SkipDecision::Skip("no changes".into())
}
