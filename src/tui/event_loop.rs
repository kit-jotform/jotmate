//! The TUI's main event loop. Owns the three distinct polling modes used by
//! different screens (TD-report foreground re-auth, sync-progress animation,
//! and the default blocking-read mode) so `tui/mod.rs` can stay small.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::Stdout;
use std::time::Duration;

use crate::tui::app::{self, App, Screen};
use crate::tui::draw::draw;
use crate::tui::input::{handle_key, Action};
use crate::tui::sync_launcher::launch_sync;
use crate::tui::{setup_terminal, teardown_terminal};

pub(super) async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        match app.screen {
            Screen::TimeDoctorReport => {
                if handle_td_report_tick(terminal, app).await? {
                    return Ok(());
                }
            }
            Screen::SyncProgress => {
                if handle_sync_progress_tick(app)? {
                    return Ok(());
                }
            }
            _ => {
                if handle_default_tick(app)? {
                    return Ok(());
                }
            }
        }
    }
}

/// One tick of the TD-report screen. Polls for the background fetch result and
/// for key events, and drops into foreground re-auth when the session expires.
/// Returns `true` if the event loop should exit.
async fn handle_td_report_tick(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<bool> {
    app.poll_td_report();

    // Session expired — teardown TUI, re-auth in foreground, restart fetch
    if matches!(app.td_report, app::TdReportState::NeedsReauth) {
        teardown_terminal(terminal);
        let email = app.td_email.clone();
        match crate::time::auth::reauth(&email).await {
            Ok(_) => {
                eprintln!("Re-authenticated. Reloading report...");
                std::thread::sleep(std::time::Duration::from_millis(800));
            }
            Err(e) => {
                eprintln!("Re-authentication failed: {e}");
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }
        *terminal = setup_terminal()?;
        app.td_report = app::TdReportState::Loading;
        app.launch_td_report();
    }

    if event::poll(Duration::from_millis(150))? {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(false);
            }
            if is_ctrl_c(key.code, key.modifiers) {
                return Ok(true);
            }
            match handle_key(app, key.code) {
                Action::Back => return Ok(true),
                Action::StartSync | Action::Continue => {}
            }
        }
    }
    Ok(false)
}

/// One tick of the sync-progress screen. Drains pending updates from the sync
/// channel, polls for keys (to drive spinner animation), and falls back to a
/// tick on the spinner when no key arrived. Returns `true` to exit.
fn handle_sync_progress_tick(app: &mut App) -> Result<bool> {
    // Drain pending sync updates
    let mut updates = Vec::new();
    if let Some(state) = &mut app.sync_state {
        while let Ok(update) = state.update_rx.try_recv() {
            updates.push(update);
        }
    }
    for update in updates {
        app.apply_sync_update(update);
    }

    if event::poll(Duration::from_millis(80))? {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(false);
            }
            if is_ctrl_c(key.code, key.modifiers) {
                if let Some(state) = app.sync_state.take() {
                    if let Some(handle) = state.sync_handle {
                        handle.abort();
                    }
                }
                return Ok(true);
            }
            match handle_key(app, key.code) {
                Action::Back => return Ok(true),
                Action::StartSync | Action::Continue => {}
            }
        }
    } else if let Some(state) = &mut app.sync_state {
        // No event — tick spinner
        state.tick = state.tick.wrapping_add(1);
    }
    Ok(false)
}

/// One tick of the default blocking-read mode used by most list screens.
fn handle_default_tick(app: &mut App) -> Result<bool> {
    if let Event::Key(key) = event::read()? {
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        if is_ctrl_c(key.code, key.modifiers) {
            return Ok(true);
        }
        match handle_key(app, key.code) {
            Action::Back => return Ok(true),
            Action::StartSync => launch_sync(app),
            Action::Continue => {}
        }
    }
    Ok(false)
}

fn is_ctrl_c(code: KeyCode, modifiers: KeyModifiers) -> bool {
    code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL)
}
