use crossterm::event::KeyCode;

use super::app::{App, Screen, MAIN_ITEMS};

// Returns None to keep looping, Some(None) to quit, Some(Some("sync")) etc to run a tool
pub fn handle_main_key(app: &mut App, code: KeyCode) -> Option<Option<String>> {
    match code {
        KeyCode::Up | KeyCode::Left => {
            let i = app.main_state.selected().unwrap_or(0);
            app.main_state.select(Some(i.saturating_sub(1)));
        }
        KeyCode::Down | KeyCode::Right => {
            let i = app.main_state.selected().unwrap_or(0);
            app.main_state.select(Some((i + 1).min(MAIN_ITEMS.len() - 1)));
        }
        KeyCode::Enter => {
            let i = app.main_state.selected().unwrap_or(0);
            match i {
                0 => return Some(Some("sync".to_string())),
                1 => return Some(Some("time".to_string())),
                2 => {
                    app.screen = Screen::Settings;
                    app.settings_state.select(Some(0));
                }
                _ => return Some(None), // Exit
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => return Some(None),
        _ => {}
    }
    None
}

pub fn handle_settings_key(app: &mut App, code: KeyCode) {
    let count = app.settings_item_count();
    match code {
        KeyCode::Up | KeyCode::Left => {
            let i = app.settings_state.selected().unwrap_or(0);
            // skip separator (idx 2)
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
                // "← Back"
                app.screen = Screen::MainMenu;
            } else {
                app.toggle_selected_setting();
            }
        }
        KeyCode::Esc | KeyCode::Backspace => {
            app.screen = Screen::MainMenu;
        }
        _ => {}
    }
}
