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

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::tui::app::{ForkStatus, RdsStatus, RepoSyncState, SyncUpdate};

mod elapsed;
mod fork;
mod git;
mod rds;

use elapsed::track_elapsed;
use fork::{sync_fork, ForkOpts};
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

    let now = Instant::now();
    let starts: Vec<(usize, Instant)> = repos.iter().map(|&(idx, _)| (idx, now)).collect();
    let elapsed_handle = tokio::spawn(track_elapsed(tx.clone(), starts));

    // Each repo gets a pipeline task: fork sync → immediately followed by its own RDS sync.
    // This means a fast repo's RDS can start while slower repos are still fetching upstream.
    let mut handles = Vec::new();
    for &(idx, ref path) in &repos {
        let tx = tx.clone();
        let path = path.clone();
        handles.push(tokio::spawn(async move {
            let fork_opts = ForkOpts {
                skip_fork_sync,
                skip_git_fetch,
                skip_rebase,
            };
            let fork_result = sync_fork(idx, &path, &tx, &fork_opts).await;

            let rds_opts = RdsOpts {
                skip_rds_sync,
                smart_sync,
            };
            sync_rds(idx, &path, &fork_result, &tx, &rds_opts).await;
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }

    elapsed_handle.abort();
}

// ── Headless (single-line) sync ───────────────────────────────────────────────

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];

pub async fn run_headless(repo_paths: Vec<(String, PathBuf)>, opts: SyncOpts) {
    let n = repo_paths.len();

    // Build initial state vec (one entry per repo, in order)
    let mut states: Vec<RepoSyncState> = repo_paths
        .iter()
        .map(|(name, path)| RepoSyncState {
            name: name.clone(),
            path: path.clone(),
            fork_status: ForkStatus::Pending,
            rds_status: RdsStatus::Pending,
            started_at: None,
            elapsed_secs: 0.0,
        })
        .collect();

    let (tx, mut rx) = mpsc::unbounded_channel::<SyncUpdate>();

    let indexed: Vec<(usize, PathBuf)> = repo_paths
        .into_iter()
        .enumerate()
        .map(|(i, (_, p))| (i, p))
        .collect();

    let start = Instant::now();
    let sync_task = tokio::spawn(run_tui(indexed, tx, opts));
    tokio::pin!(sync_task);

    let mut tick: usize = 0;
    let mut sync_done = false;

    let _ = terminal::enable_raw_mode();
    print!("\x1b[?25l"); // hide cursor
    let _ = std::io::stdout().flush();

    loop {
        tokio::select! {
            _ = &mut sync_task, if !sync_done => { sync_done = true; }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
        }

        // Check for 'q' keypress without blocking
        while event::poll(std::time::Duration::ZERO).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    sync_task.abort();
                    let _ = terminal::disable_raw_mode();
                    print!("\x1b[?25h"); // restore cursor
                    println!();
                    return;
                }
            }
        }

        while let Ok(msg) = rx.try_recv() {
            apply_update(&mut states, msg);
        }

        tick += 1;

        let elapsed = start.elapsed().as_secs_f64();

        if sync_done || states.iter().all(|r| r.is_complete()) {
            print_line(&states, n, tick, true, elapsed);
            let _ = terminal::disable_raw_mode();
            print!("\x1b[?25h"); // restore cursor
            println!();
            print_errors(&states);
            break;
        }

        print_line(&states, n, tick, false, elapsed);
    }
}

fn fmt_elapsed(secs: f64) -> String {
    format!("{secs:.1}s")
}

fn apply_update(states: &mut [RepoSyncState], msg: SyncUpdate) {
    match msg {
        SyncUpdate::Fork(idx, status) => {
            if let Some(r) = states.get_mut(idx) {
                r.fork_status = status;
            }
        }
        SyncUpdate::Rds(idx, status) => {
            if let Some(r) = states.get_mut(idx) {
                r.rds_status = status;
            }
        }
        SyncUpdate::Elapsed(idx, secs) => {
            if let Some(r) = states.get_mut(idx) {
                r.elapsed_secs = secs;
            }
        }
    }
}

// ANSI color codes matching palette.rs indexed colors
const ANSI_ACCENT: &str = "\x1b[38;5;51m"; // C_ACCENT  — cyan, in-progress count
const ANSI_SUCCESS: &str = "\x1b[38;5;10m"; // C_SUCCESS — green, done count / ✓
const ANSI_WARN: &str = "\x1b[38;5;11m"; // C_WARN    — yellow, skipped count
const ANSI_DANGER: &str = "\x1b[38;5;9m"; // C_DANGEROUS — red, error count / ✗
const ANSI_MUTED: &str = "\x1b[38;5;243m"; // C_MUTED   — separator •
const ANSI_TEXT: &str = "\x1b[38;5;255m"; // C_TEXT    — labels
const ANSI_RESET: &str = "\x1b[0m";

fn print_line(states: &[RepoSyncState], total: usize, tick: usize, done: bool, elapsed: f64) {
    let complete = states.iter().filter(|r| r.is_complete()).count();
    let errors = states.iter().filter(|r| r.has_error()).count();
    let skipped = states.iter().filter(|r| r.is_skipped()).count();

    let spinner_ch;
    let (icon, icon_color): (&str, &str) = if done {
        if errors > 0 {
            ("✗", ANSI_DANGER)
        } else {
            ("✓", ANSI_SUCCESS)
        }
    } else {
        spinner_ch = SPINNER[tick % SPINNER.len()].to_string();
        (&spinner_ch, ANSI_ACCENT)
    };

    let mut line = format!(
        "{icon_color}{icon}{ANSI_RESET}    {ANSI_ACCENT}{complete}/{total}{ANSI_RESET}{ANSI_TEXT} complete{ANSI_RESET}"
    );

    if errors > 0 {
        line.push_str(&format!(
            "  {ANSI_MUTED}•{ANSI_RESET}  {ANSI_DANGER}{errors}{ANSI_RESET}{ANSI_TEXT} error{ANSI_RESET}"
        ));
    }

    line.push_str(&format!(
        "  {ANSI_MUTED}•{ANSI_RESET}  {ANSI_WARN}{skipped}{ANSI_RESET}{ANSI_TEXT} skipped{ANSI_RESET}"
    ));

    line.push_str(&format!(
        "  {ANSI_MUTED}•{ANSI_RESET}  {ANSI_MUTED}{}{ANSI_RESET}",
        fmt_elapsed(elapsed)
    ));

    print!("\r\x1b[2K{line}");
    let _ = std::io::stdout().flush();
}

fn print_errors(states: &[RepoSyncState]) {
    for repo in states.iter().filter(|r| r.has_error()) {
        let msg = match &repo.fork_status {
            ForkStatus::Error(m) => m.as_str(),
            _ => match &repo.rds_status {
                RdsStatus::Error(m) => m.as_str(),
                _ => "",
            },
        };
        println!(
            "  {ANSI_DANGER}{}{ANSI_RESET}  {ANSI_MUTED}{msg}{ANSI_RESET}",
            repo.name
        );
    }
}
