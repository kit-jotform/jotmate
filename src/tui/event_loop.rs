//! The TUI's main event loop. Owns the three distinct polling modes used by
//! different screens (TD-report foreground re-auth, sync-progress animation,
//! and the default blocking-read mode) so `tui/mod.rs` can stay small.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::Stdout;
use std::time::Duration;

use crate::tui::app::{App, Screen};
use crate::tui::draw::draw;
use crate::tui::input::{handle_key, Action};
use crate::tui::sync_launcher::launch_sync;

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

/// Read the next key event if one is available within `poll`. Returns `None`
/// when nothing arrived or the event wasn't a press.
///
/// If `poll` is `None`, blocks on `event::read` (used by the default mode).
fn poll_key(poll: Option<Duration>) -> Result<Option<KeyEvent>> {
    let ready = match poll {
        Some(d) => event::poll(d)?,
        None => true,
    };
    if !ready {
        return Ok(None);
    }
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => Ok(Some(key)),
        _ => Ok(None),
    }
}

/// Dispatch a keypress to the input handlers. Returns `true` if the caller
/// should exit the event loop. The `on_exit` hook fires on Ctrl-C or Back
/// before the exit signal is returned — screens that need to cancel in-flight
/// work plug their teardown there.
fn dispatch_key(app: &mut App, key: KeyEvent, on_exit: impl FnOnce(&mut App)) -> bool {
    if is_ctrl_c(key.code, key.modifiers) {
        on_exit(app);
        return true;
    }
    match handle_key(app, key.code) {
        Action::Back => {
            on_exit(app);
            true
        }
        Action::StartSync => {
            launch_sync(app);
            false
        }
        Action::Continue => false,
    }
}

async fn handle_td_report_tick(
    _terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<bool> {
    app.poll_td_report();
    let Some(key) = poll_key(Some(Duration::from_millis(150)))? else {
        return Ok(false);
    };
    Ok(dispatch_key(app, key, |_| {}))
}

/// One tick of the sync-progress screen. Drains pending updates from the sync
/// channel, polls for keys (to drive spinner animation), and falls back to a
/// tick on the spinner when no key arrived. Returns `true` to exit.
fn handle_sync_progress_tick(app: &mut App) -> Result<bool> {
    let mut updates = Vec::new();
    if let Some(state) = &mut app.sync_state {
        while let Ok(update) = state.update_rx.try_recv() {
            updates.push(update);
        }
    }
    for update in updates {
        app.apply_sync_update(update);
    }

    match poll_key(Some(Duration::from_millis(80)))? {
        Some(key) => Ok(dispatch_key(app, key, |app| app.cancel_sync())),
        None => {
            if let Some(state) = &mut app.sync_state {
                state.tick = state.tick.wrapping_add(1);
            }
            Ok(false)
        }
    }
}

/// One tick of the default blocking-read mode used by most list screens.
fn handle_default_tick(app: &mut App) -> Result<bool> {
    let Some(key) = poll_key(None)? else {
        return Ok(false);
    };
    Ok(dispatch_key(app, key, |_| {}))
}

fn is_ctrl_c(code: KeyCode, modifiers: KeyModifiers) -> bool {
    code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL)
}
