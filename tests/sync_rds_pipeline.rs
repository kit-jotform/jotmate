//! Sync RDS-pipeline tests driven by a scripted `FakeGit`.
//!
//! Covered edge cases:
//!   skip_rds_sync short-circuits (with or without ./sync script)
//!   smart_sync skips when fork unchanged + clean + ahead=0
//!   smart_sync proceeds when dirty tree
//!   smart_sync pulls when behind=N, ahead=0
//!   smart_sync pull failure reports via AlreadyReported path
//!   A14  no ./sync script in repo → Skipped
//!   A13  ./sync exits non-zero → Error with stderr

mod common;

use common::{fake_repo_dir, FakeGit};
use jotmate::sync::native::fork::ForkResult;
use jotmate::sync::native::rds::{sync_rds, RdsOpts};
use jotmate::tui::app::{RdsStatus, SyncUpdate};
use tokio::sync::mpsc;

fn collect_rds_updates(rx: &mut mpsc::UnboundedReceiver<SyncUpdate>) -> Vec<RdsStatus> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let SyncUpdate::Rds(_, s) = msg {
            out.push(s);
        }
    }
    out
}

fn opts(skip_rds_sync: bool, smart_sync: bool) -> RdsOpts {
    RdsOpts {
        skip_rds_sync,
        smart_sync,
    }
}

fn last_terminal(updates: &[RdsStatus]) -> &RdsStatus {
    updates
        .iter()
        .rev()
        .find(|s| s.is_terminal())
        .unwrap_or_else(|| panic!("no terminal status in {updates:?}"))
}

// ─── short-circuits ─────────────────────────────────────────────────────────

#[tokio::test]
async fn skip_rds_sync_short_circuits() {
    let git = FakeGit::new();
    let td = fake_repo_dir(true);
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(true, false),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::Skipped(msg) => assert_eq!(msg, "--skip-rds-sync"),
        other => panic!("expected Skipped, got {other:?}"),
    }
    // No git calls, no rds script call.
    assert!(git.log().is_empty());
}

#[tokio::test]
async fn no_sync_script_is_skipped() {
    // A14 — repo doesn't have a ./sync file.
    // smart_sync=false so the sync script branch runs unconditionally.
    let git = FakeGit::new();
    let td = fake_repo_dir(false); // no sync script
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Updated, // fork ran, so smart-sync skip logic is bypassed
        &tx,
        &opts(false, true),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::Skipped(msg) => assert_eq!(msg, "no ./sync"),
        other => panic!("expected Skipped(no ./sync), got {other:?}"),
    }
}

// ─── smart_sync skip logic ──────────────────────────────────────────────────

#[tokio::test]
async fn smart_sync_skips_when_clean_and_ahead_zero() {
    // Fork Unchanged + clean tree + ahead=0 + behind=0 → "no changes".
    let git = FakeGit::new().on(&["status"], Ok("# branch.head main\n# branch.ab +0 -0\n"));
    let td = fake_repo_dir(true);
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::Skipped(msg) => assert_eq!(msg, "no changes"),
        other => panic!("expected Skipped(no changes), got {other:?}"),
    }
    // The ./sync script must NOT have been executed.
    assert_eq!(git.call_count(&["__rds_script__"]), 0);
}

#[tokio::test]
async fn smart_sync_proceeds_when_dirty() {
    // Dirty tree → smart-sync decides to run ./sync anyway.
    let git = FakeGit::new().on(
        &["status"],
        Ok("# branch.head main\n1 M. N... 100644 100644 100644 a a a README.md\n"),
    );
    let td = fake_repo_dir(true);
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
    assert_eq!(git.call_count(&["__rds_script__"]), 1);
}

#[tokio::test]
async fn smart_sync_pulls_when_behind() {
    // behind=3, ahead=0 → pull --ff-only + proceed.
    let git = FakeGit::new()
        .on(&["status"], Ok("# branch.head main\n# branch.ab +0 -3\n"))
        .on(&["pull", "--ff-only", "origin", "main"], Ok("Fast-forward"));
    let td = fake_repo_dir(true);
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true),
    )
    .await;

    assert!(git.was_called(&["pull", "--ff-only", "origin", "main"]));
    assert_eq!(git.call_count(&["__rds_script__"]), 1);
    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
}

#[tokio::test]
async fn smart_sync_pull_failure_reports_and_stops() {
    let git = FakeGit::new()
        .on(&["status"], Ok("# branch.head main\n# branch.ab +0 -3\n"))
        .on(
            &["pull", "--ff-only", "origin", "main"],
            Err("Aborting: not possible to fast-forward"),
        );
    let td = fake_repo_dir(true);
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true),
    )
    .await;

    // Script must not run when pull failed.
    assert_eq!(git.call_count(&["__rds_script__"]), 0);
    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::Error(msg) => assert!(msg.starts_with("pull:"), "got: {msg}"),
        other => panic!("expected Error(pull:…), got {other:?}"),
    }
}

#[tokio::test]
async fn smart_sync_proceeds_when_ab_missing() {
    // No `# branch.ab` line → fetch origin + proceed.
    let git = FakeGit::new()
        .on(&["status"], Ok("# branch.head main\n"))
        .on(&["fetch", "origin", "main"], Ok(""));
    let td = fake_repo_dir(true);
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true),
    )
    .await;

    assert!(git.was_called(&["fetch", "origin", "main"]));
    assert_eq!(git.call_count(&["__rds_script__"]), 1);
    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
}

#[tokio::test]
async fn smart_sync_proceeds_on_detached_head() {
    // Detached HEAD → proceed, no pull.
    let git = FakeGit::new().on(&["status"], Ok("# branch.head (detached)\n"));
    let td = fake_repo_dir(true);
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, true),
    )
    .await;

    assert_eq!(git.call_count(&["pull"]), 0);
    assert_eq!(git.call_count(&["__rds_script__"]), 1);
    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
}

// ─── script failure (A13) ──────────────────────────────────────────────────

#[tokio::test]
async fn sync_script_failure_reports_error() {
    // A13 — ./sync exits non-zero.
    let git = FakeGit::new();
    git.set_rds_result(Err("build step failed".into()));
    let td = fake_repo_dir(true);
    let (tx, mut rx) = mpsc::unbounded_channel();

    // smart_sync=false so we skip the status parse and go straight to script.
    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Updated,
        &tx,
        &opts(false, true),
    )
    .await;

    let updates = collect_rds_updates(&mut rx);
    match last_terminal(&updates) {
        RdsStatus::Error(msg) => assert_eq!(msg, "build step failed"),
        other => panic!("expected Error(build step failed), got {other:?}"),
    }
}

#[tokio::test]
async fn smart_sync_off_runs_script_unconditionally() {
    // smart_sync=false → no status parse, always run script if it exists.
    let git = FakeGit::new();
    let td = fake_repo_dir(true);
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Unchanged,
        &tx,
        &opts(false, false),
    )
    .await;

    assert_eq!(git.call_count(&["status"]), 0);
    assert_eq!(git.call_count(&["__rds_script__"]), 1);
    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
}

#[tokio::test]
async fn fork_error_still_runs_script_when_smart_sync_enabled() {
    // ForkResult::Error isn't Unchanged → smart-sync skip branch is NOT
    // taken, so the script runs (same as ForkResult::Updated).
    let git = FakeGit::new();
    let td = fake_repo_dir(true);
    let (tx, mut rx) = mpsc::unbounded_channel();

    sync_rds(
        git.as_ref(),
        0,
        td.path(),
        &ForkResult::Error("something".into()),
        &tx,
        &opts(false, true),
    )
    .await;

    assert_eq!(git.call_count(&["status"]), 0);
    assert_eq!(git.call_count(&["__rds_script__"]), 1);
    let updates = collect_rds_updates(&mut rx);
    matches!(last_terminal(&updates), RdsStatus::Done);
}
