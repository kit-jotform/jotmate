use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::tui::app::{ForkStatus, RdsStatus, SyncUpdate};

// ── Git helpers ──────────────────────────────────────────────────────────────

async fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(["-C", &repo.to_string_lossy()])
        .args(args)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("exit code {}", output.status.code().unwrap_or(1))
        } else {
            stderr
        })
    }
}

async fn git_ok(repo: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(["-C", &repo.to_string_lossy()])
        .args(args)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Detect upstream default branch (main, master, or from symbolic ref).
async fn detect_default_branch(repo: &Path) -> Option<String> {
    if git_ok(repo, &["rev-parse", "--verify", "upstream/main"]).await {
        return Some("main".to_string());
    }
    if git_ok(repo, &["rev-parse", "--verify", "upstream/master"]).await {
        return Some("master".to_string());
    }
    git(repo, &["symbolic-ref", "refs/remotes/upstream/HEAD"])
        .await
        .ok()
        .map(|s| s.replace("refs/remotes/upstream/", ""))
}

// ── Fork sync for a single repo ─────────────────────────────────────────────

struct ForkOpts {
    skip_fork_sync: bool,
    skip_git_fetch: bool,
    skip_rebase: bool,
}

/// Result of fork sync: Updated, Unchanged (exit 10 in bash), or Error.
#[allow(dead_code)]
enum ForkResult {
    Updated,
    Unchanged,
    Error(String),
}

async fn sync_fork(
    idx: usize,
    repo: &Path,
    tx: &mpsc::UnboundedSender<SyncUpdate>,
    opts: &ForkOpts,
) -> ForkResult {
    if opts.skip_fork_sync {
        let _ = tx.send(SyncUpdate::Fork(
            idx,
            ForkStatus::Skipped("--skip-fork-sync".into()),
        ));
        return ForkResult::Unchanged;
    }

    // Check upstream remote exists
    let remotes = match git(repo, &["remote"]).await {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Error(e)));
            return ForkResult::Error("no remotes".into());
        }
    };
    if !remotes.lines().any(|l| l.trim() == "upstream") {
        let _ = tx.send(SyncUpdate::Fork(
            idx,
            ForkStatus::Skipped("no upstream".into()),
        ));
        return ForkResult::Unchanged;
    }

    // Fetch upstream
    if !opts.skip_git_fetch {
        let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::FetchingUpstream));
        if let Err(e) = git(repo, &["fetch", "upstream"]).await {
            let _ = tx.send(SyncUpdate::Fork(
                idx,
                ForkStatus::Error(format!("fetch: {e}")),
            ));
            return ForkResult::Error(e);
        }
    } else {
        let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::FetchingUpstream));
    }

    // Detect default branch
    let default_branch = match detect_default_branch(repo).await {
        Some(b) => b,
        None => {
            let _ = tx.send(SyncUpdate::Fork(
                idx,
                ForkStatus::Skipped("no default branch".into()),
            ));
            return ForkResult::Unchanged;
        }
    };

    // Check diff
    let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::CheckingDiff));

    let local_commit = git(repo, &["rev-parse", &default_branch])
        .await
        .unwrap_or_default();
    let upstream_ref = format!("upstream/{default_branch}");
    let upstream_commit = git(repo, &["rev-parse", &upstream_ref])
        .await
        .unwrap_or_default();

    if upstream_commit.is_empty() {
        let _ = tx.send(SyncUpdate::Fork(
            idx,
            ForkStatus::Skipped("no upstream ref".into()),
        ));
        return ForkResult::Unchanged;
    }

    if local_commit == upstream_commit {
        let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::UpToDate));
        return ForkResult::Unchanged;
    }

    // Get current branch before switching
    let current_branch = git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .unwrap_or_default();

    // Stash if dirty
    let dirty = git(repo, &["diff-index", "--quiet", "HEAD", "--"])
        .await
        .is_err();
    if dirty {
        let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Stashing));
        let _ = git(
            repo,
            &["stash", "push", "-m", "Auto-stash before fork sync"],
        )
        .await;
    }

    // Checkout default branch
    let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::CheckingOut));
    if let Err(e) = git(repo, &["checkout", &default_branch]).await {
        if dirty {
            let _ = git(repo, &["stash", "pop"]).await;
        }
        let _ = tx.send(SyncUpdate::Fork(
            idx,
            ForkStatus::Error(format!("checkout: {e}")),
        ));
        return ForkResult::Error(e);
    }

    // Merge upstream
    let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Merging));
    if let Err(e) = git(repo, &["merge", &upstream_ref, "--no-edit"]).await {
        if dirty {
            let _ = git(repo, &["stash", "pop"]).await;
        }
        let _ = tx.send(SyncUpdate::Fork(
            idx,
            ForkStatus::Error(format!("merge: {e}")),
        ));
        return ForkResult::Error(e);
    }

    // Push default branch
    let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::PushingDefault));
    if let Err(e) = git(repo, &["push", "origin", &default_branch]).await {
        if dirty {
            let _ = git(repo, &["stash", "pop"]).await;
        }
        let _ = tx.send(SyncUpdate::Fork(
            idx,
            ForkStatus::Error(format!("push: {e}")),
        ));
        return ForkResult::Error(e);
    }

    // If on a different branch, rebase and push
    if !current_branch.is_empty() && current_branch != default_branch {
        if let Err(e) = git(repo, &["checkout", &current_branch]).await {
            if dirty {
                let _ = git(repo, &["stash", "pop"]).await;
            }
            let _ = tx.send(SyncUpdate::Fork(
                idx,
                ForkStatus::Error(format!("checkout back: {e}")),
            ));
            return ForkResult::Error(e);
        }

        if !opts.skip_rebase {
            let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Rebasing));
            if git(repo, &["rebase", &default_branch]).await.is_err() {
                let _ = git(repo, &["rebase", "--abort"]).await;
                if dirty {
                    let _ = git(repo, &["stash", "pop"]).await;
                }
                let _ = tx.send(SyncUpdate::Fork(
                    idx,
                    ForkStatus::Error("rebase conflict".into()),
                ));
                return ForkResult::Error("rebase conflict".into());
            }

            let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::PushingBranch));
            if let Err(e) = git(
                repo,
                &["push", "--force-with-lease", "origin", &current_branch],
            )
            .await
            {
                if dirty {
                    let _ = git(repo, &["stash", "pop"]).await;
                }
                let _ = tx.send(SyncUpdate::Fork(
                    idx,
                    ForkStatus::Error(format!("push branch: {e}")),
                ));
                return ForkResult::Error(e);
            }
        }
    }

    // Unstash
    if dirty {
        let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Unstashing));
        let _ = git(repo, &["stash", "pop"]).await;
    }

    let _ = tx.send(SyncUpdate::Fork(idx, ForkStatus::Done));
    ForkResult::Updated
}

// ── RDS sync for a single repo ──────────────────────────────────────────────

struct RdsOpts {
    skip_rds_sync: bool,
    skip_dirty_sync: bool,
    force_sync_all: bool,
}

async fn sync_rds(
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

    // Check if we should skip (mirrors prepare_project_sync from bash)
    if !opts.force_sync_all && matches!(fork_result, ForkResult::Unchanged) {
        // Check working tree status
        let porcelain = git(repo, &["status", "--porcelain"])
            .await
            .unwrap_or_default();
        if !porcelain.is_empty() {
            if opts.skip_dirty_sync {
                let _ = tx.send(SyncUpdate::Rds(
                    idx,
                    RdsStatus::Skipped("dirty + skip-dirty".into()),
                ));
                return;
            }
            // dirty but not skipping → run sync
        } else {
            // Clean repo, fork unchanged — check if behind origin
            let current_branch = git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
                .await
                .unwrap_or_default();
            if !current_branch.is_empty() && current_branch != "HEAD" {
                let origin_ref = format!("origin/{current_branch}");
                if !git_ok(repo, &["rev-parse", "--verify", &origin_ref]).await {
                    let _ = git(repo, &["fetch", "origin", &current_branch]).await;
                }

                if git_ok(repo, &["rev-parse", "--verify", &origin_ref]).await {
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
                        if let Err(e) =
                            git(repo, &["pull", "--ff-only", "origin", &current_branch]).await
                        {
                            let _ = tx
                                .send(SyncUpdate::Rds(idx, RdsStatus::Error(format!("pull: {e}"))));
                            return;
                        }
                    } else {
                        let ahead = git(
                            repo,
                            &["rev-list", "--count", &format!("{origin_ref}..HEAD")],
                        )
                        .await
                        .unwrap_or_default()
                        .parse::<u32>()
                        .unwrap_or(0);
                        if ahead == 0 {
                            let _ = tx.send(SyncUpdate::Rds(
                                idx,
                                RdsStatus::Skipped("no changes".into()),
                            ));
                            return;
                        }
                    }
                }
            }
        }
    }

    // Check if ./sync exists
    let sync_path = repo.join("sync");
    if !sync_path.exists() {
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
                // Take first line only
                stderr.lines().next().unwrap_or("failed").to_string()
            };
            let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Error(msg)));
        }
        Err(e) => {
            let _ = tx.send(SyncUpdate::Rds(idx, RdsStatus::Error(e.to_string())));
        }
    }
}

// ── Elapsed time tracker ─────────────────────────────────────────────────────

async fn track_elapsed(tx: mpsc::UnboundedSender<SyncUpdate>, starts: Vec<(usize, Instant)>) {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        for &(idx, start) in &starts {
            let _ = tx.send(SyncUpdate::Elapsed(idx, start.elapsed().as_secs_f64()));
        }
    }
}

// ── Public entry point for TUI ───────────────────────────────────────────────

pub struct SyncOpts {
    pub skip_fork_sync: bool,
    pub skip_git_fetch: bool,
    pub skip_rebase: bool,
    pub skip_rds_sync: bool,
    pub skip_dirty_sync: bool,
    pub force_sync_all: bool,
}

pub async fn run_tui(
    repos: Vec<(usize, PathBuf)>,
    tx: mpsc::UnboundedSender<SyncUpdate>,
    opts: SyncOpts,
) {
    let fork_opts = ForkOpts {
        skip_fork_sync: opts.skip_fork_sync,
        skip_git_fetch: opts.skip_git_fetch,
        skip_rebase: opts.skip_rebase,
    };

    let rds_opts = RdsOpts {
        skip_rds_sync: opts.skip_rds_sync,
        skip_dirty_sync: opts.skip_dirty_sync,
        force_sync_all: opts.force_sync_all,
    };

    // Phase 1: Fork sync — all repos in parallel
    let now = Instant::now();
    let starts: Vec<(usize, Instant)> = repos.iter().map(|&(idx, _)| (idx, now)).collect();
    let elapsed_tx = tx.clone();
    let elapsed_handle = tokio::spawn(track_elapsed(elapsed_tx, starts));

    let mut fork_handles = Vec::new();
    for &(idx, ref path) in &repos {
        let tx = tx.clone();
        let path = path.clone();
        let skip_fork = fork_opts.skip_fork_sync;
        let skip_fetch = fork_opts.skip_git_fetch;
        let skip_rebase = fork_opts.skip_rebase;
        fork_handles.push(tokio::spawn(async move {
            let opts = ForkOpts {
                skip_fork_sync: skip_fork,
                skip_git_fetch: skip_fetch,
                skip_rebase,
            };
            let result = sync_fork(idx, &path, &tx, &opts).await;
            (idx, result)
        }));
    }

    // Wait for all fork syncs
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
        let fork_result = fork_results.iter().find(|(i, _)| *i == idx).map(|(_, r)| r);

        // Determine fork result type for RDS decision
        let was_unchanged = matches!(fork_result, Some(ForkResult::Unchanged));
        let was_error = matches!(fork_result, Some(ForkResult::Error(_)));

        let skip_rds = rds_opts.skip_rds_sync;
        let skip_dirty = rds_opts.skip_dirty_sync;
        let force_all = rds_opts.force_sync_all;

        rds_handles.push(tokio::spawn(async move {
            let fake_result = if was_error {
                ForkResult::Error(String::new())
            } else if was_unchanged {
                ForkResult::Unchanged
            } else {
                ForkResult::Updated
            };
            let opts = RdsOpts {
                skip_rds_sync: skip_rds,
                skip_dirty_sync: skip_dirty,
                force_sync_all: force_all,
            };
            sync_rds(idx, &path, &fake_result, &tx, &opts).await;
        }));
    }

    for handle in rds_handles {
        let _ = handle.await;
    }

    elapsed_handle.abort();
}
