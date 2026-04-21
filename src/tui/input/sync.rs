use crossterm::event::KeyCode;

use crate::tui::app::{App, Screen};

use super::helpers::clamp_scroll;
use super::Action;

pub(super) fn handle_sync_progress(app: &mut App, code: KeyCode) -> Action {
    let is_running = !app.sync_is_complete();
    match code {
        KeyCode::Up | KeyCode::Down => {
            let total = app.sync_state.as_ref().map(|s| s.repos.len()).unwrap_or(0);
            let delta = if code == KeyCode::Up { -1 } else { 1 };
            app.sync_scroll = clamp_scroll(app.sync_scroll, delta, total, 6);
        }
        KeyCode::Enter if is_running => {}
        KeyCode::Enter | KeyCode::Esc | KeyCode::Backspace => {
            app.cancel_sync();
            app.sync_scroll = 0;
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}
