use crossterm::event::KeyCode;

use crate::tui::app::{App, MainMenuKind, Screen};

use super::helpers::go_to;
use super::keys::nav_delta;
use super::Action;

pub(super) fn handle_main(app: &mut App, code: KeyCode) -> Action {
    if let Some(delta) = nav_delta(code) {
        app.navigate_current(delta);
        return Action::Continue;
    }
    match code {
        KeyCode::Enter => {
            let i = app.selected_index(Screen::MainMenu);
            let items = app.main_menu_items();
            let Some(item) = items.get(i) else {
                return Action::Back;
            };
            match item.kind {
                MainMenuKind::Sync => return Action::StartSync,
                MainMenuKind::TimeDoctor => app.launch_td_report(),
                MainMenuKind::Update => app.launch_update(),
                MainMenuKind::Settings => go_to(app, Screen::Settings),
                MainMenuKind::Exit => return Action::Back,
            }
        }
        KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}
