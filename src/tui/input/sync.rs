use crossterm::event::KeyCode;

use crate::tui::app::{App, Screen};

use super::Action;

pub(super) fn handle_sync_progress(app: &mut App, code: KeyCode) -> Action {
    let is_running = !app.sync_is_complete();
    match code {
        KeyCode::Up => {
            app.sync_scroll = app.sync_scroll.saturating_sub(1);
        }
        KeyCode::Down => {
            let total = app.sync_state.as_ref().map(|s| s.repos.len()).unwrap_or(0);
            let max_scroll = total.saturating_sub(6);
            app.sync_scroll = (app.sync_scroll + 1).min(max_scroll);
        }
        KeyCode::Enter if is_running => {}
        KeyCode::Enter | KeyCode::Esc | KeyCode::Backspace => {
            if let Some(state) = app.sync_state.take() {
                if let Some(handle) = state.sync_handle {
                    handle.abort();
                }
            }
            app.sync_scroll = 0;
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}
