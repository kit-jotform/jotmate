use crossterm::event::KeyCode;

use super::app::{App, InputMode, RepoManagerRow, Screen, SettingRow};
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
        Screen::RepoManager => match &app.input_mode {
            InputMode::AddingRepo(_) => handle_repo_input(app, code),
            InputMode::ConfirmDelete(_) => handle_confirm_delete(app, code),
            InputMode::Normal => handle_repo_manager(app, code),
        },
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
                _ => return Action::Back,
            }
        }
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
            match rows.get(i) {
                Some(SettingRow::Back) => {
                    app.screen = Screen::MainMenu;
                }
                Some(SettingRow::ManageRepos) => {
                    app.screen = Screen::RepoManager;
                    let rm_rows = app.repo_manager_items();
                    let first = rm_rows.iter().position(|r| r.is_interactive()).unwrap_or(0);
                    app.repo_manager_state.select(Some(first));
                }
                _ => {
                    app.toggle_selected_setting();
                }
            }
        }
        KeyCode::Esc | KeyCode::Backspace => {
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}

fn handle_repo_manager(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Left => {
            let rows = app.repo_manager_items();
            let i = app.repo_manager_state.selected().unwrap_or(0);
            let mut next = i.saturating_sub(1);
            while next > 0 && !rows[next].is_interactive() {
                next -= 1;
            }
            app.repo_manager_state.select(Some(next));
        }
        KeyCode::Down | KeyCode::Right => {
            let rows = app.repo_manager_items();
            let i = app.repo_manager_state.selected().unwrap_or(0);
            let last = rows.len() - 1;
            let mut next = (i + 1).min(last);
            while next < last && !rows[next].is_interactive() {
                next += 1;
            }
            app.repo_manager_state.select(Some(next));
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let rows = app.repo_manager_items();
            let i = app.repo_manager_state.selected().unwrap_or(0);
            match rows.get(i) {
                Some(RepoManagerRow::Back) => {
                    app.screen = Screen::Settings;
                }
                Some(RepoManagerRow::AddUrl) => {
                    app.input_mode = InputMode::AddingRepo(String::new());
                }
                Some(RepoManagerRow::RepoDelete { name, .. }) => {
                    let name = name.clone();
                    app.confirm_delete_repo(name);
                }
                _ => {}
            }
        }
        KeyCode::Esc | KeyCode::Backspace => {
            app.screen = Screen::Settings;
        }
        KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}

fn handle_confirm_delete(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            let name = match &app.input_mode {
                InputMode::ConfirmDelete(n) => n.clone(),
                _ => return Action::Continue,
            };
            app.execute_delete_repo(&name);
        }
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
    Action::Continue
}

fn handle_repo_input(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Char(c) => {
            if let InputMode::AddingRepo(buf) = &mut app.input_mode {
                buf.push(c);
            }
        }
        KeyCode::Backspace => {
            if let InputMode::AddingRepo(buf) = &mut app.input_mode {
                buf.pop();
            }
        }
        KeyCode::Enter => {
            let url = match &app.input_mode {
                InputMode::AddingRepo(buf) => buf.clone(),
                _ => String::new(),
            };
            app.input_mode = InputMode::Normal;
            app.add_repo_from_input(url);
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
    Action::Continue
}
