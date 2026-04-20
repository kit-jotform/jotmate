//! Shared dispatcher helpers used by multiple per-screen handlers.

use crossterm::event::KeyCode;

use crate::tui::app::{App, CycleTarget, InputMode, Screen};

use super::keys::{cycle_delta, is_back, is_no, is_yes, nav_delta};
use super::Action;

pub(super) fn go_to(app: &mut App, screen: Screen) {
    app.screen = screen;
    app.select_first_interactive(screen);
}

/// Handle the common nav/back/quit keys shared by every list screen.
/// Returns `Some(action)` when consumed, `None` if the caller should handle
/// the key (typically an activate).
pub(super) fn handle_list_nav(app: &mut App, code: KeyCode, parent: Screen) -> Option<Action> {
    if let Some(delta) = nav_delta(code) {
        app.navigate_current(delta);
        return Some(Action::Continue);
    }
    if is_back(code) {
        app.screen = parent;
        return Some(Action::Continue);
    }
    if code == KeyCode::Char('q') {
        return Some(Action::Back);
    }
    None
}

/// Handle a yes/no confirm dialog. Runs `on_yes` when confirmed, clears input mode on cancel.
pub(super) fn handle_yes_no(app: &mut App, code: KeyCode, on_yes: fn(&mut App)) -> Action {
    if is_yes(code) {
        on_yes(app);
    } else if is_no(code) {
        app.input_mode = InputMode::Normal;
    }
    Action::Continue
}

/// Handle the ↑↓ cycle / Enter-Esc-Backspace confirm/cancel pattern for inline value editors.
/// Enter confirms (keeps the cycled value). Esc/Backspace cancels (restores the snapshot).
pub(super) fn handle_cycle(app: &mut App, code: KeyCode, target: CycleTarget) -> Action {
    if let Some(delta) = cycle_delta(code) {
        app.cycle(target, delta);
    } else if matches!(code, KeyCode::Enter) {
        app.input_mode = InputMode::Normal;
    } else if is_back(code) {
        app.cancel_cycle_edit();
    } else if code == KeyCode::Char('q') {
        app.cancel_cycle_edit();
        return Action::Back;
    }
    Action::Continue
}

/// Handle a single-line text input. `on_enter` is called with the buffer when Enter is pressed.
pub(super) fn handle_text_input(
    app: &mut App,
    code: KeyCode,
    on_enter: fn(&mut App, String),
) -> Action {
    match code {
        KeyCode::Char(c) => {
            if let Some(buf) = text_buf_mut(app) {
                buf.push(c);
            }
        }
        KeyCode::Backspace => {
            if let Some(buf) = text_buf_mut(app) {
                buf.pop();
            }
        }
        KeyCode::Enter => {
            let buf = text_buf_take(app).unwrap_or_default();
            app.input_mode = InputMode::Normal;
            on_enter(app, buf);
        }
        KeyCode::Esc => app.input_mode = InputMode::Normal,
        _ => {}
    }
    Action::Continue
}

/// Returns a mutable reference to the buffer inside the active text-editing input mode.
pub(super) fn text_buf_mut(app: &mut App) -> Option<&mut String> {
    match &mut app.input_mode {
        InputMode::AddingRepo(buf) => Some(buf),
        InputMode::EditingField { buf, .. } => Some(buf),
        _ => None,
    }
}

/// Clones the current text buffer value without changing input mode.
pub(super) fn text_buf_take(app: &App) -> Option<String> {
    match &app.input_mode {
        InputMode::AddingRepo(buf) => Some(buf.clone()),
        InputMode::EditingField { buf, .. } => Some(buf.clone()),
        _ => None,
    }
}
