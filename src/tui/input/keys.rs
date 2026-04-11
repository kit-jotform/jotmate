//! Key classifiers shared by every input handler.

use crossterm::event::KeyCode;

pub(super) fn nav_delta(code: KeyCode) -> Option<i32> {
    match code {
        KeyCode::Up | KeyCode::Left => Some(-1),
        KeyCode::Down | KeyCode::Right => Some(1),
        _ => None,
    }
}

pub(super) fn cycle_delta(code: KeyCode) -> Option<i32> {
    match code {
        KeyCode::Up | KeyCode::Right => Some(1),
        KeyCode::Down | KeyCode::Left => Some(-1),
        _ => None,
    }
}

pub(super) fn is_activate(code: KeyCode) -> bool {
    matches!(code, KeyCode::Enter)
}

pub(super) fn is_back(code: KeyCode) -> bool {
    matches!(code, KeyCode::Esc | KeyCode::Backspace)
}

pub(super) fn is_yes(code: KeyCode) -> bool {
    matches!(code, KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y'))
}

pub(super) fn is_no(code: KeyCode) -> bool {
    matches!(code, KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N'))
}
