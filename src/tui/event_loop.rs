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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LoopOutcome {
    Exit,
    Restart,
}

pub(super) async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<LoopOutcome> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        let tick = match app.screen {
            Screen::TimeDoctorReport => handle_td_report_tick(terminal, app).await?,
            Screen::SyncProgress => handle_sync_progress_tick(app)?,
            Screen::UpdateProgress => handle_update_progress_tick(app)?,
            Screen::MainMenu => handle_main_menu_tick(app)?,
            _ => handle_default_tick(app)?,
        };
        if let Some(outcome) = tick {
            return Ok(outcome);
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

fn dispatch_key(
    app: &mut App,
    key: KeyEvent,
    on_exit: impl FnOnce(&mut App),
) -> Option<LoopOutcome> {
    if is_ctrl_c(key.code, key.modifiers) {
        on_exit(app);
        return Some(LoopOutcome::Exit);
    }
    match handle_key(app, key.code) {
        Action::Back => {
            on_exit(app);
            Some(LoopOutcome::Exit)
        }
        Action::Restart => {
            on_exit(app);
            Some(LoopOutcome::Restart)
        }
        Action::StartSync => {
            launch_sync(app);
            None
        }
        Action::Continue => None,
    }
}

async fn handle_td_report_tick(
    _terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<Option<LoopOutcome>> {
    app.poll_td_report();
    let Some(key) = poll_key(Some(Duration::from_millis(150)))? else {
        return Ok(None);
    };
    Ok(dispatch_key(app, key, |_| {}))
}

fn handle_sync_progress_tick(app: &mut App) -> Result<Option<LoopOutcome>> {
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
            Ok(None)
        }
    }
}

fn handle_default_tick(app: &mut App) -> Result<Option<LoopOutcome>> {
    let Some(key) = poll_key(None)? else {
        return Ok(None);
    };
    Ok(dispatch_key(app, key, |_| {}))
}

fn handle_main_menu_tick(app: &mut App) -> Result<Option<LoopOutcome>> {
    app.poll_update_check();
    let Some(key) = poll_key(Some(Duration::from_millis(60)))? else {
        return Ok(None);
    };
    Ok(dispatch_key(app, key, |_| {}))
}

fn handle_update_progress_tick(app: &mut App) -> Result<Option<LoopOutcome>> {
    let mut events = Vec::new();
    if let Some(state) = &mut app.update_state {
        while let Ok(event) = state.update_rx.try_recv() {
            events.push(event);
        }
    }
    for event in events {
        app.apply_update_event(event);
    }

    match poll_key(Some(Duration::from_millis(80)))? {
        Some(key) => Ok(dispatch_key(app, key, |app| app.cancel_update())),
        None => {
            if let Some(state) = &mut app.update_state {
                state.tick = state.tick.wrapping_add(1);
            }
            Ok(None)
        }
    }
}

fn is_ctrl_c(code: KeyCode, modifiers: KeyModifiers) -> bool {
    code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL)
}
