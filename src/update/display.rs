use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::mpsc;

use crate::tui::palette::{
    ANSI_ACCENT, ANSI_DANGEROUS, ANSI_MUTED, ANSI_RESET, ANSI_SUCCESS, ANSI_WARN, SPINNER,
};

use super::api::current_version;
use super::engine::{run_update, UpdatePhase, UpdateUpdate};

pub async fn run_headless() -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<UpdateUpdate>();
    let task = tokio::spawn(run_update(tx));

    hide_cursor();
    let mut tick: usize = 0;
    let mut phase = UpdatePhase::Checking;
    let mut ticker = tokio::time::interval(Duration::from_millis(80));

    loop {
        tokio::select! {
            biased;
            _ = ticker.tick() => {
                render_progress(&phase, tick);
                tick = tick.wrapping_add(1);
            }
            msg = rx.recv() => {
                match msg {
                    Some(UpdateUpdate::Phase(p)) => {
                        phase = p;
                        if phase.is_terminal() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    let _ = task.await;
    render_final(&phase);
    show_cursor();
    if matches!(phase, UpdatePhase::Failed(_)) {
        std::process::exit(1);
    }
    Ok(())
}

fn render_progress(phase: &UpdatePhase, tick: usize) {
    let spinner = SPINNER[tick % SPINNER.len()];
    let label = phase.label();
    print!("\r {ANSI_ACCENT}{spinner}{ANSI_RESET}  {ANSI_MUTED}{label}{ANSI_RESET}\x1b[K");
    let _ = std::io::stdout().flush();
}

fn render_final(phase: &UpdatePhase) {
    let line = match phase {
        UpdatePhase::UpToDate => format!(
            "\r {ANSI_SUCCESS}✓{ANSI_RESET}  Already on {ANSI_ACCENT}v{}{ANSI_RESET} — no update available\x1b[K",
            current_version()
        ),
        UpdatePhase::Done(version) => format!(
            "\r {ANSI_SUCCESS}✓{ANSI_RESET}  Updated to {ANSI_ACCENT}v{version}{ANSI_RESET}  {ANSI_MUTED}•{ANSI_RESET}  restart to use it\x1b[K"
        ),
        UpdatePhase::Failed(msg) => format!(
            "\r {ANSI_DANGEROUS}✗{ANSI_RESET}  Update failed: {ANSI_WARN}{msg}{ANSI_RESET}\x1b[K"
        ),
        other => format!(
            "\r {ANSI_WARN}…{ANSI_RESET}  Update interrupted during {}\x1b[K",
            other.label()
        ),
    };
    println!("{line}");
}

fn hide_cursor() {
    print!("\x1b[?25l");
    let _ = std::io::stdout().flush();
}

fn show_cursor() {
    print!("\x1b[?25h");
    let _ = std::io::stdout().flush();
}
