use crossterm::event::KeyCode;

use crate::tui::app::{App, Screen};
use crate::update::UpdatePhase;

use super::Action;

pub(super) fn handle_update_progress(app: &mut App, code: KeyCode) -> Action {
    let is_running = !app.update_is_terminal();
    let succeeded = matches!(
        app.update_state.as_ref().map(|s| &s.phase),
        Some(UpdatePhase::Done(_))
    );
    match code {
        KeyCode::Enter if is_running => {}
        KeyCode::Enter | KeyCode::Esc | KeyCode::Backspace => {
            app.cancel_update();
            if succeeded {
                return Action::Restart;
            }
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}
