//! Main event loop. Per-screen polling modes (TD-report, sync-progress,
//! main-menu, default blocking) keep `tui/mod.rs` thin.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::Stdout;
use std::time::Duration;

use crate::tui::app::{App, Screen};
use crate::tui::draw::draw;
use crate::tui::input::{handle_key, Action};
use crate::tui::sync_launcher::{launch_sync, promote_discovery_if_ready};

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
            Screen::MainMenu => {
                if handle_main_menu_tick(app)? {
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

fn handle_sync_progress_tick(app: &mut App) -> Result<bool> {
    promote_discovery_if_ready(app);

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

fn handle_default_tick(app: &mut App) -> Result<bool> {
    let Some(key) = poll_key(None)? else {
        return Ok(false);
    };
    Ok(dispatch_key(app, key, |_| {}))
}

fn handle_main_menu_tick(app: &mut App) -> Result<bool> {
    let Some(key) = poll_key(Some(Duration::from_millis(60)))? else {
        return Ok(false);
    };
    Ok(dispatch_key(app, key, |_| {}))
}

fn is_ctrl_c(code: KeyCode, modifiers: KeyModifiers) -> bool {
    code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL)
}
