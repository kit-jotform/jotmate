use crossterm::event::KeyCode;

use super::app::{
    App, CpListRow, GeneralToggleRow, InputMode, RemoveRepoRow, RepoManagerRow, Screen, SettingRow,
    TimeDoctorField, TimeSettingRow, MAIN_ITEMS,
};

fn navigate<T>(
    rows: &[T],
    current: usize,
    delta: i32,
    is_interactive: impl Fn(&T) -> bool,
) -> usize {
    let len = rows.len();
    let mut next = current;
    for _ in 0..len {
        if delta < 0 {
            next = if next == 0 { len - 1 } else { next - 1 };
        } else {
            next = if next == len - 1 { 0 } else { next + 1 };
        }
        if is_interactive(&rows[next]) {
            return next;
        }
    }
    current
}

pub enum Action {
    Continue,
    Back,
    Run(String),
    StartSync,
}

pub fn handle_key(app: &mut App, code: KeyCode) -> Action {
    match app.screen {
        Screen::MainMenu => handle_main(app, code),
        Screen::Settings => handle_settings(app, code),
        Screen::SyncGeneralSettings => handle_general_toggles(app, code, true),
        Screen::TdGeneralSettings => handle_general_toggles(app, code, false),
        Screen::RepoManager => match &app.input_mode {
            InputMode::AddingRepo(_) => handle_repo_input(app, code),
            InputMode::Normal => handle_repo_manager(app, code),
            _ => handle_repo_manager(app, code),
        },
        Screen::RemoveRepos => match &app.input_mode {
            InputMode::ConfirmDelete(_) => handle_confirm_delete(app, code),
            _ => handle_remove_repos(app, code),
        },
        Screen::TimeDoctorSettings => match &app.input_mode {
            InputMode::EditingField { .. } => handle_td_field_input(app, code),
            _ => handle_td_settings(app, code),
        },
        Screen::ContractPeriods => match &app.input_mode {
            InputMode::ConfirmDeletePeriod(_) => handle_confirm_delete_period(app, code),
            InputMode::EditingCpMonday => handle_cp_monday_edit(app, code),
            InputMode::EditingCpHours => handle_cp_hours_edit(app, code),
            _ => handle_contract_periods(app, code),
        },
        Screen::SyncProgress => handle_sync_progress(app, code),
    }
}

fn handle_main(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Left => {
            let i = app.main_state.selected().unwrap_or(0);
            let last = MAIN_ITEMS.len() - 1;
            app.main_state
                .select(Some(if i == 0 { last } else { i - 1 }));
        }
        KeyCode::Down | KeyCode::Right => {
            let i = app.main_state.selected().unwrap_or(0);
            let last = MAIN_ITEMS.len() - 1;
            app.main_state
                .select(Some(if i == last { 0 } else { i + 1 }));
        }
        KeyCode::Enter => {
            let i = app.main_state.selected().unwrap_or(0);
            match i {
                0 => return Action::StartSync,
                1 => return Action::Run("time".to_string()),
                2 => {
                    app.screen = Screen::Settings;
                    let rows = app.settings_items();
                    let first = rows.iter().position(|r| r.is_interactive()).unwrap_or(0);
                    app.settings_state.select(Some(first));
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
    let rows = app.settings_items();
    let i = app.settings_state.selected().unwrap_or(0);
    let current_row = rows.get(i);

    match code {
        KeyCode::Up | KeyCode::Left => {
            let rows = app.settings_items();
            app.settings_state
                .select(Some(navigate(&rows, i, -1, SettingRow::is_interactive)));
        }
        KeyCode::Down | KeyCode::Right => {
            let rows = app.settings_items();
            app.settings_state
                .select(Some(navigate(&rows, i, 1, SettingRow::is_interactive)));
        }
        KeyCode::Enter | KeyCode::Char(' ') => match current_row {
            Some(SettingRow::Back) => {
                app.screen = Screen::MainMenu;
            }
            Some(SettingRow::SyncGeneralLink) => {
                app.screen = Screen::SyncGeneralSettings;
                let rows = app.sync_general_items();
                let first = rows.iter().position(|r| r.is_interactive()).unwrap_or(0);
                app.sync_general_state.select(Some(first));
            }
            Some(SettingRow::ManageRepos) => {
                app.screen = Screen::RepoManager;
                let rm_rows = app.repo_manager_items();
                let first = rm_rows.iter().position(|r| r.is_interactive()).unwrap_or(0);
                app.repo_manager_state.select(Some(first));
            }
            Some(SettingRow::TdGeneralLink) => {
                app.screen = Screen::TdGeneralSettings;
                let rows = app.td_general_items();
                let first = rows.iter().position(|r| r.is_interactive()).unwrap_or(0);
                app.td_general_state.select(Some(first));
            }
            Some(SettingRow::TimeDoctorSettings) => {
                app.screen = Screen::TimeDoctorSettings;
                let td_rows = app.td_settings_items();
                let first = td_rows.iter().position(|r| r.is_interactive()).unwrap_or(0);
                app.td_settings_state.select(Some(first));
            }
            Some(SettingRow::ContractPeriodsLink) => {
                app.screen = Screen::ContractPeriods;
                let cp_rows = app.cp_list_items();
                let first = cp_rows.iter().position(|r| r.is_interactive()).unwrap_or(0);
                app.cp_list_state.select(Some(first));
            }
            _ => {}
        },
        KeyCode::Esc | KeyCode::Backspace => {
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}

fn handle_general_toggles(app: &mut App, code: KeyCode, is_sync: bool) -> Action {
    // Timezone selecting mode intercepts all keys
    if matches!(app.input_mode, InputMode::SelectingTimezone) {
        return handle_timezone_select(app, code);
    }

    let rows = if is_sync {
        app.sync_general_items()
    } else {
        app.td_general_items()
    };
    let state = if is_sync {
        &app.sync_general_state
    } else {
        &app.td_general_state
    };
    let i = state.selected().unwrap_or(0);

    match code {
        KeyCode::Up | KeyCode::Left => {
            let next = navigate(&rows, i, -1, GeneralToggleRow::is_interactive);
            if is_sync {
                app.sync_general_state.select(Some(next));
            } else {
                app.td_general_state.select(Some(next));
            }
        }
        KeyCode::Down | KeyCode::Right => {
            let next = navigate(&rows, i, 1, GeneralToggleRow::is_interactive);
            if is_sync {
                app.sync_general_state.select(Some(next));
            } else {
                app.td_general_state.select(Some(next));
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') => match rows.get(i) {
            Some(GeneralToggleRow::Back) => {
                app.screen = Screen::Settings;
            }
            Some(GeneralToggleRow::Toggle { kind, disabled, .. }) => {
                if !disabled {
                    app.toggle_by_kind(*kind);
                }
            }
            Some(GeneralToggleRow::TimezoneSelector { .. }) => {
                app.input_mode = InputMode::SelectingTimezone;
            }
            _ => {}
        },
        KeyCode::Esc | KeyCode::Backspace => {
            app.screen = Screen::Settings;
        }
        KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}

fn handle_timezone_select(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Right => {
            app.cycle_timezone(-1);
        }
        KeyCode::Down | KeyCode::Left => {
            app.cycle_timezone(1);
        }
        KeyCode::Enter | KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
    Action::Continue
}

fn handle_repo_manager(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Left => {
            let rows = app.repo_manager_items();
            let i = app.repo_manager_state.selected().unwrap_or(0);
            app.repo_manager_state.select(Some(navigate(
                &rows,
                i,
                -1,
                RepoManagerRow::is_interactive,
            )));
        }
        KeyCode::Down | KeyCode::Right => {
            let rows = app.repo_manager_items();
            let i = app.repo_manager_state.selected().unwrap_or(0);
            app.repo_manager_state.select(Some(navigate(
                &rows,
                i,
                1,
                RepoManagerRow::is_interactive,
            )));
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let rows = app.repo_manager_items();
            let i = app.repo_manager_state.selected().unwrap_or(0);
            match rows.get(i) {
                Some(RepoManagerRow::Back) => {
                    app.screen = Screen::Settings;
                }
                Some(RepoManagerRow::RepoToggle { name, .. }) => {
                    let name = name.clone();
                    app.toggle_repo(&name);
                }
                Some(RepoManagerRow::AddUrl) => {
                    app.input_mode = InputMode::AddingRepo(String::new());
                }
                Some(RepoManagerRow::RemoveReposLink) => {
                    app.screen = Screen::RemoveRepos;
                    let rows = app.remove_repo_items();
                    let first = rows.iter().position(|r| r.is_interactive()).unwrap_or(0);
                    app.remove_repo_state.select(Some(first));
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

fn handle_remove_repos(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Left => {
            let rows = app.remove_repo_items();
            let i = app.remove_repo_state.selected().unwrap_or(0);
            app.remove_repo_state.select(Some(navigate(
                &rows,
                i,
                -1,
                RemoveRepoRow::is_interactive,
            )));
        }
        KeyCode::Down | KeyCode::Right => {
            let rows = app.remove_repo_items();
            let i = app.remove_repo_state.selected().unwrap_or(0);
            app.remove_repo_state.select(Some(navigate(
                &rows,
                i,
                1,
                RemoveRepoRow::is_interactive,
            )));
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let rows = app.remove_repo_items();
            let i = app.remove_repo_state.selected().unwrap_or(0);
            match rows.get(i) {
                Some(RemoveRepoRow::Back) => {
                    app.screen = Screen::RepoManager;
                }
                Some(RemoveRepoRow::RepoDelete { name, .. }) => {
                    let name = name.clone();
                    app.confirm_delete_repo(name);
                }
                _ => {}
            }
        }
        KeyCode::Esc | KeyCode::Backspace => {
            app.screen = Screen::RepoManager;
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

fn handle_td_settings(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Left => {
            let rows = app.td_settings_items();
            let i = app.td_settings_state.selected().unwrap_or(0);
            app.td_settings_state.select(Some(navigate(
                &rows,
                i,
                -1,
                TimeSettingRow::is_interactive,
            )));
        }
        KeyCode::Down | KeyCode::Right => {
            let rows = app.td_settings_items();
            let i = app.td_settings_state.selected().unwrap_or(0);
            app.td_settings_state.select(Some(navigate(
                &rows,
                i,
                1,
                TimeSettingRow::is_interactive,
            )));
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let rows = app.td_settings_items();
            let i = app.td_settings_state.selected().unwrap_or(0);
            match rows.get(i).cloned() {
                Some(TimeSettingRow::Back) => {
                    app.screen = Screen::Settings;
                }
                Some(TimeSettingRow::EditField { field, value, .. }) => {
                    app.input_mode = InputMode::EditingField { field, buf: value };
                }
                Some(TimeSettingRow::Password { .. }) => {
                    app.input_mode = InputMode::EditingField {
                        field: TimeDoctorField::Password,
                        buf: String::new(),
                    };
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

fn handle_td_field_input(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Char(c) => {
            if let InputMode::EditingField { buf, .. } = &mut app.input_mode {
                buf.push(c);
            }
        }
        KeyCode::Backspace => {
            if let InputMode::EditingField { buf, .. } = &mut app.input_mode {
                buf.pop();
            }
        }
        KeyCode::Enter => {
            let (field, buf) = match app.input_mode.clone() {
                InputMode::EditingField { field, buf } => (field, buf),
                _ => return Action::Continue,
            };
            app.input_mode = InputMode::Normal;
            match field {
                TimeDoctorField::Email => app.td_email = buf,
                TimeDoctorField::Password => {
                    app.set_td_password(&buf);
                    return Action::Continue;
                }
            }
            app.persist_td_settings();
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
    Action::Continue
}

fn handle_contract_periods(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Left => {
            let rows = app.cp_list_items();
            let i = app.cp_list_state.selected().unwrap_or(0);
            app.cp_list_state
                .select(Some(navigate(&rows, i, -1, CpListRow::is_interactive)));
        }
        KeyCode::Down | KeyCode::Right => {
            let rows = app.cp_list_items();
            let i = app.cp_list_state.selected().unwrap_or(0);
            app.cp_list_state
                .select(Some(navigate(&rows, i, 1, CpListRow::is_interactive)));
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let rows = app.cp_list_items();
            let i = app.cp_list_state.selected().unwrap_or(0);
            match rows.get(i) {
                Some(CpListRow::Back) => {
                    app.screen = Screen::Settings;
                }
                Some(CpListRow::MondayField) => {
                    app.input_mode = InputMode::EditingCpMonday;
                }
                Some(CpListRow::HoursField) => {
                    app.input_mode = InputMode::EditingCpHours;
                }
                Some(CpListRow::SavePeriod) => {
                    app.save_new_contract_period();
                }
                Some(CpListRow::Period { index, .. }) => {
                    app.confirm_delete_period(*index);
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

fn handle_confirm_delete_period(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            let idx = match &app.input_mode {
                InputMode::ConfirmDeletePeriod(i) => *i,
                _ => return Action::Continue,
            };
            app.execute_delete_period(idx);
        }
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
    Action::Continue
}

fn handle_cp_monday_edit(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Right => app.cycle_add_cp_monday(1),
        KeyCode::Down | KeyCode::Left => app.cycle_add_cp_monday(-1),
        KeyCode::Enter | KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
    Action::Continue
}

fn handle_cp_hours_edit(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Right => app.cycle_add_cp_hours(1),
        KeyCode::Down | KeyCode::Left => app.cycle_add_cp_hours(-1),
        KeyCode::Enter | KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
    Action::Continue
}

fn handle_sync_progress(app: &mut App, code: KeyCode) -> Action {
    let is_complete = app.sync_is_complete();
    match code {
        KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Backspace => {
            if is_complete {
                // Clean up and go back to main menu
                if let Some(state) = app.sync_state.take() {
                    if let Some(handle) = state.sync_handle {
                        handle.abort();
                    }
                }
                app.screen = Screen::MainMenu;
            } else {
                // Cancel: abort running tasks and go back
                if let Some(state) = app.sync_state.take() {
                    if let Some(handle) = state.sync_handle {
                        handle.abort();
                    }
                }
                app.screen = Screen::MainMenu;
            }
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
