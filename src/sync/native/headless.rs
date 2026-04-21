//! Single-line headless CLI output for `jotmate sync`.
//!
//! Owns its own spinner + ANSI rendering and consumes [`SyncUpdate`] messages
//! from the shared sync engine ([`super::run_tui`]).

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::tui::app::{ForkStatus, RdsStatus, RepoSyncState, SyncUpdate};
use crate::tui::palette::{
    ANSI_ACCENT, ANSI_DANGEROUS, ANSI_MUTED, ANSI_RESET, ANSI_SUCCESS, ANSI_TEXT, ANSI_WARN,
    SPINNER,
};

use super::{run_tui, SyncOpts};

pub async fn run_headless(repo_paths: Vec<(String, PathBuf)>, opts: SyncOpts) {
    let n = repo_paths.len();

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
    print!("\x1b[?25l");
    let _ = std::io::stdout().flush();

    loop {
        tokio::select! {
            _ = &mut sync_task, if !sync_done => { sync_done = true; }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
        }

        while event::poll(std::time::Duration::ZERO).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    sync_task.abort();
                    let _ = terminal::disable_raw_mode();
                    print!("\x1b[?25h");
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
            print!("\x1b[?25h");
            println!();
            print_errors(&states);
            println!();
            break;
        }

        print_line(&states, n, tick, false, elapsed);
    }
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

fn print_line(states: &[RepoSyncState], total: usize, tick: usize, done: bool, elapsed: f64) {
    let complete = states.iter().filter(|r| r.is_complete()).count();
    let errors = states.iter().filter(|r| r.has_error()).count();
    let skipped = states.iter().filter(|r| r.is_skipped()).count();

    let spinner_ch;
    let (icon, icon_color): (&str, &str) = if done {
        if errors > 0 {
            ("✗", ANSI_DANGEROUS)
        } else {
            ("✓", ANSI_SUCCESS)
        }
    } else {
        spinner_ch = SPINNER[tick % SPINNER.len()].to_string();
        (&spinner_ch, ANSI_ACCENT)
    };

    let mut line = format!(
        "  {icon_color}{icon}{ANSI_RESET}  {ANSI_ACCENT}{complete}/{total}{ANSI_RESET}{ANSI_TEXT} complete{ANSI_RESET}"
    );

    if errors > 0 {
        line.push_str(&format!(
            "  {ANSI_MUTED}•{ANSI_RESET}  {ANSI_DANGEROUS}{errors}{ANSI_RESET}{ANSI_TEXT} error{ANSI_RESET}"
        ));
    }

    line.push_str(&format!(
        "  {ANSI_MUTED}•{ANSI_RESET}  {ANSI_WARN}{skipped}{ANSI_RESET}{ANSI_TEXT} skipped{ANSI_RESET}"
    ));

    line.push_str(&format!(
        "  {ANSI_MUTED}•{ANSI_RESET}  {ANSI_MUTED}{elapsed:.1}s{ANSI_RESET}"
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
            "  {ANSI_DANGEROUS}{}{ANSI_RESET}  {ANSI_MUTED}{msg}{ANSI_RESET}",
            repo.name
        );
    }
}
