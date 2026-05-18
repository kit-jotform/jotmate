//! Sync fork-pipeline tests driven by a scripted `FakeGit`.
//!
//! Covered edge cases:
//!   skip_fork_sync short-circuits
//!   A11  repo path without .git → error (stale cache detection)
//!   A10  repo without upstream remote → skip
//!   A12  git fetch failure → error with clean message (no progress noise)
//!   A18  detect_default_branch fails → skip "no default branch"
//!   up-to-date: local == upstream sha → UpToDate
//!   empty upstream ref → "no upstream ref" skip
//!   A15  final stash pop fails after successful sync → stderr in status
//!   A15b stash push fails with dirty tree → abort before checkout/merge
//!   A16  rebase conflict → rebase --abort + stash pop + Error (if stashed)

mod common;

use common::{fake_repo_dir, FakeGit};
use jotmate::sync::native::fork::{sync_fork, ForkOpts, ForkResult};
use jotmate::tui::app::{ForkStatus, SyncUpdate};
use tempfile::TempDir;
use tokio::sync::mpsc;

fn collect_fork_updates(rx: &mut mpsc::UnboundedReceiver<SyncUpdate>) -> Vec<ForkStatus> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let SyncUpdate::Fork(_, s) = msg {
            out.push(s);
        }
    }
    out
}

fn opts(skip_fork_sync: bool, skip_git_fetch: bool, skip_rebase: bool) -> ForkOpts {
    ForkOpts {
        skip_fork_sync,
        skip_git_fetch,
        skip_rebase,
    }
}

fn last_terminal(updates: &[ForkStatus]) -> &ForkStatus {
    updates
        .iter()
        .rev()
        .find(|s| {
            matches!(
                s,
                ForkStatus::Done
                    | ForkStatus::UpToDate
                    | ForkStatus::Skipped(_)
                    | ForkStatus::Error(_)
            )
        })
        .unwrap_or_else(|| panic!("no terminal status in {updates:?}"))
}

// ─── skip / sentinel paths ──────────────────────────────────────────────────

#[tokio::test]
async fn skip_fork_sync_short_circuits() {
    let git = FakeGit::new();
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(true, false, false)).await;

    assert!(matches!(result, ForkResult::Unchanged));
    let updates = collect_fork_updates(&mut rx);
    matches!(last_terminal(&updates), ForkStatus::Skipped(_));
    // No git calls were made.
    assert!(git.log().is_empty());
}

#[tokio::test]
async fn missing_dotgit_reports_error() {
    // A11 — stale cache pointing at a non-repo.
    let git = FakeGit::new();
    let td = TempDir::new().unwrap(); // no .git subdir
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;

    assert!(matches!(result, ForkResult::Error(_)));
    let updates = collect_fork_updates(&mut rx);
    match last_terminal(&updates) {
        ForkStatus::Error(msg) => assert!(msg.contains("not a git repository"), "got: {msg}"),
        other => panic!("expected Error, got {other:?}"),
    }
    assert!(git.log().is_empty());
}

#[tokio::test]
async fn no_upstream_remote_is_skipped() {
    // A10 — only `origin`, no `upstream`.
    let git = FakeGit::new().on(&["remote"], Ok("origin\n"));
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;

    assert!(matches!(result, ForkResult::Unchanged));
    let updates = collect_fork_updates(&mut rx);
    match last_terminal(&updates) {
        ForkStatus::Skipped(msg) => assert_eq!(msg, "no upstream"),
        other => panic!("expected Skipped(no upstream), got {other:?}"),
    }
}

// ─── fetch failures ────────────────────────────────────────────────────────

#[tokio::test]
async fn fetch_failure_extracts_clean_error() {
    // A12 — fetch stderr contains progress lines + a real error; we surface
    // just the error line.
    let git = FakeGit::new().on(&["remote"], Ok("origin\nupstream\n")).on(
        &["fetch", "upstream"],
        Err("From github.com:foo/bar\nremote: progress\nfatal: could not read Username"),
    );
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;

    assert!(matches!(result, ForkResult::Error(_)));
    let updates = collect_fork_updates(&mut rx);
    match last_terminal(&updates) {
        ForkStatus::Error(msg) => {
            assert!(msg.starts_with("fetch: "), "got: {msg}");
            assert!(msg.contains("fatal: could not read Username"), "got: {msg}");
            assert!(!msg.contains("From github.com"), "progress leaked: {msg}");
        }
        other => panic!("expected Error, got {other:?}"),
    }
}

// ─── default branch detection ──────────────────────────────────────────────

#[tokio::test]
async fn no_default_branch_is_skipped() {
    // A18 — upstream/HEAD unset and neither main nor master tracked.
    let git = FakeGit::new()
        .on(&["remote"], Ok("origin\nupstream\n"))
        .on(&["fetch", "upstream"], Ok(""))
        .on(
            &["symbolic-ref", "refs/remotes/upstream/HEAD"],
            Err("not found"),
        )
        .on(&["for-each-ref"], Ok(""));
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;

    assert!(matches!(result, ForkResult::Unchanged));
    let updates = collect_fork_updates(&mut rx);
    match last_terminal(&updates) {
        ForkStatus::Skipped(msg) => assert_eq!(msg, "no default branch"),
        other => panic!("expected Skipped(no default branch), got {other:?}"),
    }
}

#[tokio::test]
async fn default_branch_detected_via_for_each_ref_fallback() {
    // symbolic-ref fails but upstream/master shows up in for-each-ref.
    let git = FakeGit::new()
        .on(&["remote"], Ok("upstream\n"))
        .on(&["fetch", "upstream"], Ok(""))
        .on(
            &["symbolic-ref", "refs/remotes/upstream/HEAD"],
            Err("no ref"),
        )
        .on(&["for-each-ref"], Ok("upstream/master\n"))
        .on(&["rev-parse", "master"], Ok("aaaa"))
        .on(&["rev-parse", "upstream/master"], Ok("aaaa"));
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;

    // Shas matched → UpToDate.
    assert!(matches!(result, ForkResult::Unchanged));
    let updates = collect_fork_updates(&mut rx);
    matches!(last_terminal(&updates), ForkStatus::UpToDate);
}

// ─── up-to-date / diverged paths ───────────────────────────────────────────

#[tokio::test]
async fn up_to_date_when_shas_match() {
    let git = FakeGit::new()
        .on(&["remote"], Ok("upstream\n"))
        .on(&["fetch", "upstream"], Ok(""))
        .on(&["symbolic-ref"], Ok("refs/remotes/upstream/main"))
        .on(&["rev-parse", "main"], Ok("deadbeef"))
        .on(&["rev-parse", "upstream/main"], Ok("deadbeef"));
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;

    assert!(matches!(result, ForkResult::Unchanged));
    let updates = collect_fork_updates(&mut rx);
    matches!(last_terminal(&updates), ForkStatus::UpToDate);
}

#[tokio::test]
async fn empty_upstream_ref_is_skipped() {
    let git = FakeGit::new()
        .on(&["remote"], Ok("upstream\n"))
        .on(&["fetch", "upstream"], Ok(""))
        .on(&["symbolic-ref"], Ok("refs/remotes/upstream/main"))
        .on(&["rev-parse", "main"], Ok("aaaa"))
        .on(&["rev-parse", "upstream/main"], Ok(""));
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;
    assert!(matches!(result, ForkResult::Unchanged));
    let updates = collect_fork_updates(&mut rx);
    match last_terminal(&updates) {
        ForkStatus::Skipped(msg) => assert_eq!(msg, "no upstream ref"),
        other => panic!("expected Skipped(no upstream ref), got {other:?}"),
    }
}

#[tokio::test]
async fn happy_path_on_default_branch_runs_merge_and_push() {
    // Local behind upstream, user is on main, no dirty tree → merge + push,
    // no rebase (same branch).
    let git = FakeGit::new()
        .on(&["remote"], Ok("upstream\n"))
        .on(&["fetch", "upstream"], Ok(""))
        .on(&["symbolic-ref"], Ok("refs/remotes/upstream/main"))
        .on(&["rev-parse", "main"], Ok("aaaa"))
        .on(&["rev-parse", "upstream/main"], Ok("bbbb"))
        .on(&["rev-parse", "--abbrev-ref", "HEAD"], Ok("main"))
        .on(&["diff-index"], Ok("")) // clean tree
        .on(&["checkout", "main"], Ok(""))
        .on(&["merge"], Ok(""))
        .on(&["push", "origin", "main"], Ok(""));
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;
    assert!(matches!(result, ForkResult::Updated));
    let updates = collect_fork_updates(&mut rx);
    matches!(last_terminal(&updates), ForkStatus::Done);
    assert!(git.was_called(&["merge"]));
    assert!(git.was_called(&["push", "origin", "main"]));
}

// ─── stash pop after sync (A15 / A15b) ─────────────────────────────────────

#[tokio::test]
async fn stash_pop_failure_on_happy_path_is_surfaced() {
    // Dirty tree → stash succeeds; merge + push succeed; final stash pop fails.
    let git = FakeGit::new()
        .on(&["remote"], Ok("upstream\n"))
        .on(&["fetch", "upstream"], Ok(""))
        .on(&["symbolic-ref"], Ok("refs/remotes/upstream/main"))
        .on(&["rev-parse", "main"], Ok("aaaa"))
        .on(&["rev-parse", "upstream/main"], Ok("bbbb"))
        .on(&["rev-parse", "--abbrev-ref", "HEAD"], Ok("main"))
        .on(&["diff-index"], Err("dirty")) // reports dirty
        .on(&["stash", "push"], Ok(""))
        .on(&["checkout", "main"], Ok(""))
        .on(&["merge"], Ok(""))
        .on(&["push", "origin", "main"], Ok(""))
        .on(&["stash", "pop"], Err("merge conflict in README.md"));
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;

    assert_eq!(result, ForkResult::Error("stash pop failed".into()));
    let updates = collect_fork_updates(&mut rx);
    match last_terminal(&updates) {
        ForkStatus::Error(msg) => {
            assert!(msg.contains("`git stash pop` failed"), "got: {msg}");
            assert!(msg.contains("merge conflict"), "got: {msg}");
        }
        other => panic!("expected stash pop Error, got {other:?}"),
    }
}

#[tokio::test]
async fn stash_push_failure_aborts_without_merge() {
    // Dirty flag set but stash push fails — must not checkout/merge upstream.
    let git = FakeGit::new()
        .on(&["remote"], Ok("upstream\n"))
        .on(&["fetch", "upstream"], Ok(""))
        .on(&["symbolic-ref"], Ok("refs/remotes/upstream/main"))
        .on(&["rev-parse", "main"], Ok("aaaa"))
        .on(&["rev-parse", "upstream/main"], Ok("bbbb"))
        .on(&["rev-parse", "--abbrev-ref", "HEAD"], Ok("main"))
        .on(&["diff-index"], Err("dirty"))
        .on(
            &["stash", "push"],
            Err("fatal: Unable to stash (simulated stash push failure)"),
        );
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;

    assert_eq!(result, ForkResult::Error("stash push failed".into()));
    assert_eq!(git.call_count(&["checkout"]), 0);
    assert_eq!(git.call_count(&["merge"]), 0);

    match last_terminal(&collect_fork_updates(&mut rx)) {
        ForkStatus::Error(msg) => assert!(msg.contains("stash push failed"), "got: {msg}"),
        other => panic!("expected stash push Error, got {other:?}"),
    }
}

#[tokio::test]
async fn stash_push_no_local_changes_skips_pop() {
    // `diff-index` reports dirty (e.g. stale index / submodule drift) but `stash push`
    // creates no entry. We must not attempt `stash pop` afterwards.
    let git = FakeGit::new()
        .on(&["remote"], Ok("upstream\n"))
        .on(&["fetch", "upstream"], Ok(""))
        .on(&["symbolic-ref"], Ok("refs/remotes/upstream/main"))
        .on(&["rev-parse", "main"], Ok("aaaa"))
        .on(&["rev-parse", "upstream/main"], Ok("bbbb"))
        .on(&["rev-parse", "--abbrev-ref", "HEAD"], Ok("main"))
        .on(&["diff-index"], Err("dirty"))
        .on(&["stash", "push"], Ok("No local changes to save"))
        .on(&["checkout", "main"], Ok(""))
        .on(&["merge"], Ok(""))
        .on(&["push", "origin", "main"], Ok(""));
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;

    assert!(matches!(result, ForkResult::Updated));
    assert_eq!(git.call_count(&["stash", "pop"]), 0);
    matches!(
        last_terminal(&collect_fork_updates(&mut rx)),
        ForkStatus::Done
    );
}

// ─── rebase conflict (A16) ─────────────────────────────────────────────────

#[tokio::test]
async fn rebase_conflict_aborts_and_reports() {
    // User is on a feature branch; merge + push succeed on main; rebase
    // onto main fails → abort + stash pop + Error.
    let git = FakeGit::new()
        .on(&["remote"], Ok("upstream\n"))
        .on(&["fetch", "upstream"], Ok(""))
        .on(&["symbolic-ref"], Ok("refs/remotes/upstream/main"))
        .on(&["rev-parse", "main"], Ok("aaaa"))
        .on(&["rev-parse", "upstream/main"], Ok("bbbb"))
        .on(&["rev-parse", "--abbrev-ref", "HEAD"], Ok("my-feature"))
        .on(&["diff-index"], Ok("")) // clean
        .on(&["checkout", "main"], Ok(""))
        .on(&["merge"], Ok(""))
        .on(&["push", "origin", "main"], Ok(""))
        .on(&["checkout", "my-feature"], Ok(""))
        .on(&["rebase", "main"], Err("CONFLICT in file.rs"))
        .on(&["rebase", "--abort"], Ok(""));
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;
    assert_eq!(result, ForkResult::Error("rebase conflict".into()));
    let updates = collect_fork_updates(&mut rx);
    match last_terminal(&updates) {
        ForkStatus::Error(msg) => assert_eq!(msg, "rebase conflict"),
        other => panic!("expected Error(rebase conflict), got {other:?}"),
    }
    assert!(git.was_called(&["rebase", "--abort"]));
}

// ─── smart-sync regression: fetch advanced upstream while local aligned ───

#[tokio::test]
async fn fetched_new_commits_returns_updated_even_when_local_aligned() {
    // Pre-fetch upstream/main = aaaa, local main = bbbb (already pulled
    // externally), fetch advances upstream/main to bbbb. local==upstream
    // after fetch, but new commits *did* arrive — must report Updated so
    // smart sync runs RDS.
    let git = FakeGit::new()
        .on(&["remote"], Ok("upstream\n"))
        .on(&["symbolic-ref"], Ok("refs/remotes/upstream/main"))
        .on_once(&["rev-parse", "upstream/main"], Ok("aaaa"))
        .on(&["fetch", "upstream"], Ok(""))
        .on(&["rev-parse", "main"], Ok("bbbb"))
        .on(&["rev-parse", "upstream/main"], Ok("bbbb"));
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;
    assert_eq!(result, ForkResult::Updated);
    let updates = collect_fork_updates(&mut rx);
    matches!(last_terminal(&updates), ForkStatus::UpToDate);
}

#[tokio::test]
async fn fetch_brought_nothing_and_aligned_stays_unchanged() {
    let git = FakeGit::new()
        .on(&["remote"], Ok("upstream\n"))
        .on(&["symbolic-ref"], Ok("refs/remotes/upstream/main"))
        .on(&["rev-parse", "main"], Ok("aaaa"))
        .on(&["rev-parse", "upstream/main"], Ok("aaaa"))
        .on(&["fetch", "upstream"], Ok(""));
    let td = fake_repo_dir(false);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, false)).await;
    assert_eq!(result, ForkResult::Unchanged);
    let updates = collect_fork_updates(&mut rx);
    matches!(last_terminal(&updates), ForkStatus::UpToDate);
}

// ─── skip_git_fetch / skip_rebase flags ────────────────────────────────────

#[tokio::test]
async fn skip_git_fetch_omits_fetch_call() {
    let git = FakeGit::new()
        .on(&["remote"], Ok("upstream\n"))
        .on(&["symbolic-ref"], Ok("refs/remotes/upstream/main"))
        .on(&["rev-parse", "main"], Ok("aaaa"))
        .on(&["rev-parse", "upstream/main"], Ok("aaaa"));
    let td = fake_repo_dir(false);
    let (tx, _rx) = mpsc::unbounded_channel();

    let _ = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, true, false)).await;
    assert_eq!(git.call_count(&["fetch", "upstream"]), 0);
}

#[tokio::test]
async fn skip_rebase_omits_rebase_call() {
    let git = FakeGit::new()
        .on(&["remote"], Ok("upstream\n"))
        .on(&["fetch", "upstream"], Ok(""))
        .on(&["symbolic-ref"], Ok("refs/remotes/upstream/main"))
        .on(&["rev-parse", "main"], Ok("aaaa"))
        .on(&["rev-parse", "upstream/main"], Ok("bbbb"))
        .on(&["rev-parse", "--abbrev-ref", "HEAD"], Ok("my-feature"))
        .on(&["diff-index"], Ok(""))
        .on(&["checkout", "main"], Ok(""))
        .on(&["merge"], Ok(""))
        .on(&["push", "origin", "main"], Ok(""))
        .on(&["checkout", "my-feature"], Ok(""));
    let td = fake_repo_dir(false);
    let (tx, _rx) = mpsc::unbounded_channel();

    let result = sync_fork(git.as_ref(), 0, td.path(), &tx, &opts(false, false, true)).await;
    assert!(matches!(result, ForkResult::Updated));
    assert_eq!(git.call_count(&["rebase"]), 0);
    assert_eq!(git.call_count(&["push", "--force-with-lease"]), 0);
}
