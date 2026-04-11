use crossterm::event::KeyCode;

use crate::tui::app::{App, Screen};

use super::Action;

pub(super) fn handle_sync_progress(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Enter | KeyCode::Esc | KeyCode::Backspace => {
            if let Some(state) = app.sync_state.take() {
                if let Some(handle) = state.sync_handle {
                    handle.abort();
                }
            }
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}
