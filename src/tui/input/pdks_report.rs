use crossterm::event::KeyCode;

use crate::tui::app::{App, Screen};

use super::Action;

pub(super) fn handle_pdks_report(app: &mut App, code: KeyCode) -> Action {
    if app.pdks_target_editing {
        match code {
            KeyCode::Left | KeyCode::Down => app.adjust_pdks_target(-1),
            KeyCode::Right | KeyCode::Up => app.adjust_pdks_target(1),
            KeyCode::Enter | KeyCode::Esc => app.pdks_target_editing = false,
            KeyCode::Char('q') => return Action::Back,
            _ => {}
        }
        return Action::Continue;
    }

    match code {
        KeyCode::Left | KeyCode::Up => app.shift_pdks_week(-1),
        KeyCode::Right | KeyCode::Down => app.shift_pdks_week(1),
        KeyCode::Enter => app.pdks_target_editing = true,
        KeyCode::Esc | KeyCode::Backspace => app.screen = Screen::MainMenu,
        KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}
