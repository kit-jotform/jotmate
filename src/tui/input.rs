use crossterm::event::KeyCode;

use super::app::{
    App, CpListRow, GeneralToggleRow, InputMode, RemoveRepoRow, RepoManagerRow, Screen, SettingRow,
    TdReportState, TimeDoctorField, TimeSettingRow,
};

pub enum Action {
    Continue,
    Back,
    StartSync,
}

pub fn handle_key(app: &mut App, code: KeyCode) -> Action {
    match app.screen {
        Screen::MainMenu => handle_main(app, code),
        Screen::Settings => handle_settings(app, code),
        Screen::SyncGeneralSettings | Screen::TdGeneralSettings => {
            handle_general_toggles(app, code)
        }
        Screen::RepoManager => match &app.input_mode {
            InputMode::AddingRepo(_) => handle_repo_input(app, code),
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
        Screen::TimeDoctorReport => handle_td_report(app, code),
    }
}

/// Map a key to a navigation delta, or None if the key is not a navigation key.
fn nav_delta(code: KeyCode) -> Option<i32> {
    match code {
        KeyCode::Up | KeyCode::Left => Some(-1),
        KeyCode::Down | KeyCode::Right => Some(1),
        _ => None,
    }
}

fn is_activate(code: KeyCode) -> bool {
    matches!(code, KeyCode::Enter | KeyCode::Char(' '))
}

fn is_back(code: KeyCode) -> bool {
    matches!(code, KeyCode::Esc | KeyCode::Backspace)
}

/// Enter a sub-screen and select its first interactive row.
fn go_to(app: &mut App, screen: Screen) {
    app.screen = screen;
    app.select_first_interactive(screen);
}

fn handle_main(app: &mut App, code: KeyCode) -> Action {
    if let Some(delta) = nav_delta(code) {
        app.navigate_current(delta);
        return Action::Continue;
    }
    match code {
        KeyCode::Enter => {
            let i = app.selected_index(Screen::MainMenu);
            match i {
                0 => return Action::StartSync,
                1 => {
                    if app.td_email.is_empty() || !app.td_password_is_set {
                        go_to(app, Screen::TimeDoctorSettings);
                        app.auth_error = Some("Email or password not configured".to_string());
                    } else if app.contract_periods.is_empty() {
                        go_to(app, Screen::ContractPeriods);
                    } else {
                        app.launch_td_report();
                    }
                }
                2 => go_to(app, Screen::Settings),
                _ => return Action::Back,
            }
        }
        KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}

fn handle_settings(app: &mut App, code: KeyCode) -> Action {
    if let Some(delta) = nav_delta(code) {
        app.navigate_current(delta);
        return Action::Continue;
    }
    if is_activate(code) {
        let rows = app.settings_items();
        let i = app.selected_index(Screen::Settings);
        match rows.get(i) {
            Some(SettingRow::Back) => app.screen = Screen::MainMenu,
            Some(SettingRow::SyncGeneralLink) => go_to(app, Screen::SyncGeneralSettings),
            Some(SettingRow::ManageRepos) => go_to(app, Screen::RepoManager),
            Some(SettingRow::TdGeneralLink) => go_to(app, Screen::TdGeneralSettings),
            Some(SettingRow::TimeDoctorSettings) => go_to(app, Screen::TimeDoctorSettings),
            Some(SettingRow::ContractPeriodsLink) => go_to(app, Screen::ContractPeriods),
            _ => {}
        }
        return Action::Continue;
    }
    if is_back(code) {
        app.screen = Screen::MainMenu;
    } else if code == KeyCode::Char('q') {
        return Action::Back;
    }
    Action::Continue
}

fn handle_general_toggles(app: &mut App, code: KeyCode) -> Action {
    // Timezone selecting mode intercepts all keys
    if matches!(app.input_mode, InputMode::SelectingTimezone) {
        return handle_timezone_select(app, code);
    }

    if let Some(delta) = nav_delta(code) {
        app.navigate_current(delta);
        return Action::Continue;
    }

    let is_sync = app.screen == Screen::SyncGeneralSettings;
    let rows = if is_sync {
        app.sync_general_items()
    } else {
        app.td_general_items()
    };
    let i = app.selected_index(app.screen);

    if is_activate(code) {
        match rows.get(i) {
            Some(GeneralToggleRow::Back) => app.screen = Screen::Settings,
            Some(GeneralToggleRow::Toggle { kind, disabled, .. }) => {
                if !disabled {
                    app.toggle_by_kind(*kind);
                }
            }
            Some(GeneralToggleRow::TimezoneSelector { .. }) => {
                app.input_mode = InputMode::SelectingTimezone;
            }
            _ => {}
        }
        return Action::Continue;
    }
    if is_back(code) {
        app.screen = Screen::Settings;
    } else if code == KeyCode::Char('q') {
        return Action::Back;
    }
    Action::Continue
}

fn handle_timezone_select(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Right => app.cycle_timezone(-1),
        KeyCode::Down | KeyCode::Left => app.cycle_timezone(1),
        KeyCode::Enter | KeyCode::Esc => app.input_mode = InputMode::Normal,
        _ => {}
    }
    Action::Continue
}

fn handle_repo_manager(app: &mut App, code: KeyCode) -> Action {
    if let Some(delta) = nav_delta(code) {
        app.navigate_current(delta);
        return Action::Continue;
    }
    if is_activate(code) {
        let rows = app.repo_manager_items();
        let i = app.selected_index(Screen::RepoManager);
        match rows.get(i) {
            Some(RepoManagerRow::Back) => app.screen = Screen::Settings,
            Some(RepoManagerRow::RepoToggle { name, .. }) => {
                let name = name.clone();
                app.toggle_repo(&name);
            }
            Some(RepoManagerRow::AddUrl) => {
                app.input_mode = InputMode::AddingRepo(String::new());
            }
            Some(RepoManagerRow::RemoveReposLink) => go_to(app, Screen::RemoveRepos),
            _ => {}
        }
        return Action::Continue;
    }
    if is_back(code) {
        app.screen = Screen::Settings;
    } else if code == KeyCode::Char('q') {
        return Action::Back;
    }
    Action::Continue
}

fn handle_remove_repos(app: &mut App, code: KeyCode) -> Action {
    if let Some(delta) = nav_delta(code) {
        app.navigate_current(delta);
        return Action::Continue;
    }
    if is_activate(code) {
        let rows = app.remove_repo_items();
        let i = app.selected_index(Screen::RemoveRepos);
        match rows.get(i) {
            Some(RemoveRepoRow::Back) => app.screen = Screen::RepoManager,
            Some(RemoveRepoRow::RepoDelete { name, .. }) => {
                let name = name.clone();
                app.confirm_delete_repo(name);
            }
            _ => {}
        }
        return Action::Continue;
    }
    if is_back(code) {
        app.screen = Screen::RepoManager;
    } else if code == KeyCode::Char('q') {
        return Action::Back;
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
    if let Some(delta) = nav_delta(code) {
        app.navigate_current(delta);
        return Action::Continue;
    }
    if is_activate(code) {
        let rows = app.td_settings_items();
        let i = app.selected_index(Screen::TimeDoctorSettings);
        match rows.get(i).cloned() {
            Some(TimeSettingRow::Back) => {
                app.auth_error = None;
                app.screen = Screen::Settings;
            }
            Some(TimeSettingRow::EditField { field, value, .. }) => {
                app.auth_error = None;
                app.input_mode = InputMode::EditingField { field, buf: value };
            }
            Some(TimeSettingRow::Password { .. }) => {
                app.auth_error = None;
                app.input_mode = InputMode::EditingField {
                    field: TimeDoctorField::Password,
                    buf: String::new(),
                };
            }
            _ => {}
        }
        return Action::Continue;
    }
    if is_back(code) {
        app.auth_error = None;
        app.screen = Screen::Settings;
    } else if code == KeyCode::Char('q') {
        return Action::Back;
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
                    if !app.set_td_password(&buf) {
                        app.auth_error = Some("Failed to save password to keychain".to_string());
                    }
                    return Action::Continue;
                }
            }
            app.persist_td_settings();
        }
        KeyCode::Esc => app.input_mode = InputMode::Normal,
        _ => {}
    }
    Action::Continue
}

fn handle_contract_periods(app: &mut App, code: KeyCode) -> Action {
    if let Some(delta) = nav_delta(code) {
        app.navigate_current(delta);
        return Action::Continue;
    }
    if is_activate(code) {
        let rows = app.cp_list_items();
        let i = app.selected_index(Screen::ContractPeriods);
        match rows.get(i) {
            Some(CpListRow::Back) => app.screen = Screen::Settings,
            Some(CpListRow::MondayField) => app.input_mode = InputMode::EditingCpMonday,
            Some(CpListRow::HoursField) => app.input_mode = InputMode::EditingCpHours,
            Some(CpListRow::SavePeriod) => app.save_new_contract_period(),
            Some(CpListRow::Period { index, .. }) => app.confirm_delete_period(*index),
            _ => {}
        }
        return Action::Continue;
    }
    if is_back(code) {
        app.screen = Screen::Settings;
    } else if code == KeyCode::Char('q') {
        return Action::Back;
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
        KeyCode::Enter | KeyCode::Esc => app.input_mode = InputMode::Normal,
        _ => {}
    }
    Action::Continue
}

fn handle_cp_hours_edit(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Right => app.cycle_add_cp_hours(1),
        KeyCode::Down | KeyCode::Left => app.cycle_add_cp_hours(-1),
        KeyCode::Enter | KeyCode::Esc => app.input_mode = InputMode::Normal,
        _ => {}
    }
    Action::Continue
}

fn handle_sync_progress(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Enter | KeyCode::Esc | KeyCode::Backspace => {
            if let Some(state) = app.sync_state.take() {
                if let Some(handle) = state.sync_handle {
                    handle.abort();
                }
            }
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}

fn handle_td_report(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Left => {
            app.td_report_scroll = app.td_report_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Right => {
            if let TdReportState::Ready { rows, .. } = &app.td_report {
                let max_scroll = rows.len().saturating_sub(6);
                app.td_report_scroll = (app.td_report_scroll + 1).min(max_scroll);
            }
        }
        KeyCode::Esc | KeyCode::Backspace => app.screen = Screen::MainMenu,
        KeyCode::Char('q') => return Action::Back,
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
        KeyCode::Esc => app.input_mode = InputMode::Normal,
        _ => {}
    }
    Action::Continue
}
