use crossterm::event::KeyCode;

use super::app::{App, Screen};
use super::draw::MAIN_ITEM_COUNT;

pub enum Action {
    Continue,
    Back,
    Run(String),
}

pub fn handle_key(app: &mut App, code: KeyCode) -> Action {
    match app.screen {
        Screen::MainMenu => handle_main(app, code),
        Screen::Settings => handle_settings(app, code),
    }
}

fn handle_main(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Left => {
            let i = app.main_state.selected().unwrap_or(0);
            app.main_state.select(Some(i.saturating_sub(1)));
        }
        KeyCode::Down | KeyCode::Right => {
            let i = app.main_state.selected().unwrap_or(0);
            app.main_state
                .select(Some((i + 1).min(MAIN_ITEM_COUNT - 1)));
        }
        KeyCode::Enter => {
            let i = app.main_state.selected().unwrap_or(0);
            match i {
                0 => return Action::Run("sync".to_string()),
                1 => return Action::Run("time".to_string()),
                2 => {
                    app.screen = Screen::Settings;
                    app.settings_state.select(Some(0));
                }
                _ => return Action::Back, // Exit row
            }
        }
        // Esc, Backspace, and q all exit from the main menu
        KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}

fn handle_settings(app: &mut App, code: KeyCode) -> Action {
    let count = app.settings_item_count();
    match code {
        KeyCode::Up | KeyCode::Left => {
            let i = app.settings_state.selected().unwrap_or(0);
            let next = if i == 0 { 0 } else { i - 1 };
            let next = if next == 2 { 1 } else { next };
            app.settings_state.select(Some(next));
        }
        KeyCode::Down | KeyCode::Right => {
            let i = app.settings_state.selected().unwrap_or(0);
            let next = (i + 1).min(count - 1);
            let next = if next == 2 { 3 } else { next };
            app.settings_state.select(Some(next));
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let i = app.settings_state.selected().unwrap_or(0);
            if i == count - 1 {
                // "← Back" row
                app.screen = Screen::MainMenu;
            } else {
                app.toggle_selected_setting();
            }
        }
        // Esc/Backspace go back to main menu; q quits directly
        KeyCode::Esc | KeyCode::Backspace => {
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}
