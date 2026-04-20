use crossterm::event::KeyCode;

use crate::tui::app::{App, Screen};

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
            match i {
                0 => return Action::StartSync,
                1 => enter_time_doctor(app),
                2 => go_to(app, Screen::Settings),
                _ => return Action::Back,
            }
        }
        KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}

fn enter_time_doctor(app: &mut App) {
    app.launch_td_report();
}
