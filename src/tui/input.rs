use crossterm::event::KeyCode;

use super::app::{App, Screen, SettingRow};
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
    match code {
        KeyCode::Up | KeyCode::Left => {
            let rows = app.settings_items();
            let i = app.settings_state.selected().unwrap_or(0);
            let mut next = i.saturating_sub(1);
            while next > 0 && !rows[next].is_interactive() {
                next -= 1;
            }
            app.settings_state.select(Some(next));
        }
        KeyCode::Down | KeyCode::Right => {
            let rows = app.settings_items();
            let i = app.settings_state.selected().unwrap_or(0);
            let last = rows.len() - 1;
            let mut next = (i + 1).min(last);
            while next < last && !rows[next].is_interactive() {
                next += 1;
            }
            app.settings_state.select(Some(next));
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let rows = app.settings_items();
            let i = app.settings_state.selected().unwrap_or(0);
            if matches!(rows.get(i), Some(SettingRow::Back)) {
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
