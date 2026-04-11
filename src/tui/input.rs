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
        Screen::SyncGeneralSettings | Screen::TdGeneralSettings => handle_general_toggles(app, code),
        Screen::RepoManager => match &app.input_mode {
            InputMode::AddingRepo(_) => handle_text_input(app, code, apply_new_repo_url),
            _ => handle_repo_manager(app, code),
        },
        Screen::RemoveRepos => match &app.input_mode {
            InputMode::ConfirmDelete(_) => handle_yes_no(app, code, execute_pending_repo_delete),
            _ => handle_remove_repos(app, code),
        },
        Screen::TimeDoctorSettings => match &app.input_mode {
            InputMode::EditingField { .. } => handle_td_field_input(app, code),
            _ => handle_td_settings(app, code),
        },
        Screen::ContractPeriods => match &app.input_mode {
            InputMode::ConfirmDeletePeriod(_) => handle_yes_no(app, code, execute_pending_period_delete),
            InputMode::EditingCpMonday(_) => handle_cycle(app, code, App::cycle_add_cp_monday),
            InputMode::EditingCpHours(_) => handle_cycle(app, code, App::cycle_add_cp_hours),
            _ => handle_contract_periods(app, code),
        },
        Screen::SyncProgress => handle_sync_progress(app, code),
        Screen::TimeDoctorReport => handle_td_report(app, code),
    }
}

// ── Key classifiers ─────────────────────────────────────────────────────────

fn nav_delta(code: KeyCode) -> Option<i32> {
    match code {
        KeyCode::Up | KeyCode::Left => Some(-1),
        KeyCode::Down | KeyCode::Right => Some(1),
        _ => None,
    }
}

fn cycle_delta(code: KeyCode) -> Option<i32> {
    match code {
        KeyCode::Up | KeyCode::Right => Some(1),
        KeyCode::Down | KeyCode::Left => Some(-1),
        _ => None,
    }
}

fn is_activate(code: KeyCode) -> bool {
    matches!(code, KeyCode::Enter)
}

fn is_back(code: KeyCode) -> bool {
    matches!(code, KeyCode::Esc | KeyCode::Backspace)
}

fn is_yes(code: KeyCode) -> bool {
    matches!(code, KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y'))
}

fn is_no(code: KeyCode) -> bool {
    matches!(code, KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N'))
}

// ── Shared dispatchers ──────────────────────────────────────────────────────

fn go_to(app: &mut App, screen: Screen) {
    app.screen = screen;
    app.select_first_interactive(screen);
}

/// Handle the common nav/back/quit keys shared by every list screen.
/// Returns `Some(action)` when consumed, `None` if the caller should handle
/// the key (typically an activate).
fn handle_list_nav(app: &mut App, code: KeyCode, parent: Screen) -> Option<Action> {
    if let Some(delta) = nav_delta(code) {
        app.navigate_current(delta);
        return Some(Action::Continue);
    }
    if is_back(code) {
        app.screen = parent;
        return Some(Action::Continue);
    }
    if code == KeyCode::Char('q') {
        return Some(Action::Back);
    }
    None
}

/// Handle a yes/no confirm dialog. Runs `on_yes` when confirmed, clears input mode on cancel.
fn handle_yes_no(app: &mut App, code: KeyCode, on_yes: fn(&mut App)) -> Action {
    if is_yes(code) {
        on_yes(app);
    } else if is_no(code) {
        app.input_mode = InputMode::Normal;
    }
    Action::Continue
}

/// Handle the ↑↓ cycle / Enter-Esc-Backspace confirm/cancel pattern for inline value editors.
/// Enter confirms (keeps the cycled value). Esc/Backspace cancels (restores the snapshot).
fn handle_cycle(app: &mut App, code: KeyCode, cycle: fn(&mut App, i32)) -> Action {
    if let Some(delta) = cycle_delta(code) {
        cycle(app, delta);
    } else if matches!(code, KeyCode::Enter) {
        app.input_mode = InputMode::Normal;
    } else if is_back(code) {
        app.cancel_cycle_edit();
    } else if code == KeyCode::Char('q') {
        app.cancel_cycle_edit();
        return Action::Back;
    }
    Action::Continue
}

/// Handle a single-line text input. `on_enter` is called with the buffer when Enter is pressed.
fn handle_text_input(app: &mut App, code: KeyCode, on_enter: fn(&mut App, String)) -> Action {
    match code {
        KeyCode::Char(c) => {
            if let Some(buf) = text_buf_mut(app) {
                buf.push(c);
            }
        }
        KeyCode::Backspace => {
            if let Some(buf) = text_buf_mut(app) {
                buf.pop();
            }
        }
        KeyCode::Enter => {
            let buf = text_buf_take(app).unwrap_or_default();
            app.input_mode = InputMode::Normal;
            on_enter(app, buf);
        }
        KeyCode::Esc => app.input_mode = InputMode::Normal,
        _ => {}
    }
    Action::Continue
}

/// Returns a mutable reference to the buffer inside the active text-editing input mode.
fn text_buf_mut(app: &mut App) -> Option<&mut String> {
    match &mut app.input_mode {
        InputMode::AddingRepo(buf) => Some(buf),
        InputMode::EditingField { buf, .. } => Some(buf),
        _ => None,
    }
}

/// Clones and returns the current text buffer value without changing input mode.
fn text_buf_take(app: &App) -> Option<String> {
    match &app.input_mode {
        InputMode::AddingRepo(buf) => Some(buf.clone()),
        InputMode::EditingField { buf, .. } => Some(buf.clone()),
        _ => None,
    }
}

// ── Confirm callbacks ───────────────────────────────────────────────────────

fn execute_pending_repo_delete(app: &mut App) {
    if let InputMode::ConfirmDelete(name) = app.input_mode.clone() {
        app.execute_delete_repo(&name);
    }
}

fn execute_pending_period_delete(app: &mut App) {
    if let InputMode::ConfirmDeletePeriod(idx) = app.input_mode {
        app.execute_delete_period(idx);
    }
}

fn apply_new_repo_url(app: &mut App, url: String) {
    app.add_repo_from_input(url);
}

// ── Per-screen handlers ─────────────────────────────────────────────────────

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
    if app.td_email.is_empty() || !app.td_password_is_set {
        go_to(app, Screen::TimeDoctorSettings);
        app.auth_error = Some("Email or password not configured".to_string());
    } else if app.contract_periods.is_empty() {
        go_to(app, Screen::ContractPeriods);
    } else {
        app.launch_td_report();
    }
}

fn handle_settings(app: &mut App, code: KeyCode) -> Action {
    if let Some(a) = handle_list_nav(app, code, Screen::MainMenu) {
        return a;
    }
    if !is_activate(code) {
        return Action::Continue;
    }
    let rows = app.settings_items();
    match rows.get(app.selected_index(Screen::Settings)) {
        Some(SettingRow::Back) => app.screen = Screen::MainMenu,
        Some(SettingRow::SyncGeneralLink) => go_to(app, Screen::SyncGeneralSettings),
        Some(SettingRow::ManageRepos) => go_to(app, Screen::RepoManager),
        Some(SettingRow::TdGeneralLink) => go_to(app, Screen::TdGeneralSettings),
        Some(SettingRow::TimeDoctorSettings) => go_to(app, Screen::TimeDoctorSettings),
        Some(SettingRow::ContractPeriodsLink) => go_to(app, Screen::ContractPeriods),
        _ => {}
    }
    Action::Continue
}

fn handle_general_toggles(app: &mut App, code: KeyCode) -> Action {
    if matches!(app.input_mode, InputMode::SelectingTimezone(_)) {
        return handle_cycle(app, code, App::cycle_timezone);
    }
    if let Some(a) = handle_list_nav(app, code, Screen::Settings) {
        return a;
    }
    if !is_activate(code) {
        return Action::Continue;
    }
    let rows = if app.screen == Screen::SyncGeneralSettings {
        app.sync_general_items()
    } else {
        app.td_general_items()
    };
    match rows.get(app.selected_index(app.screen)) {
        Some(GeneralToggleRow::Back) => app.screen = Screen::Settings,
        Some(GeneralToggleRow::Toggle { kind, disabled, .. }) => {
            if !disabled {
                app.toggle_by_kind(*kind);
            }
        }
        Some(GeneralToggleRow::TimezoneSelector { .. }) => {
            app.input_mode = InputMode::SelectingTimezone(app.td_timezone_idx);
        }
        _ => {}
    }
    Action::Continue
}

fn handle_repo_manager(app: &mut App, code: KeyCode) -> Action {
    if let Some(a) = handle_list_nav(app, code, Screen::Settings) {
        return a;
    }
    if !is_activate(code) {
        return Action::Continue;
    }
    let rows = app.repo_manager_items();
    match rows.get(app.selected_index(Screen::RepoManager)) {
        Some(RepoManagerRow::Back) => app.screen = Screen::Settings,
        Some(RepoManagerRow::RepoToggle { name, .. }) => {
            let name = name.clone();
            app.toggle_repo(&name);
        }
        Some(RepoManagerRow::AddUrl) => app.input_mode = InputMode::AddingRepo(String::new()),
        Some(RepoManagerRow::RemoveReposLink) => go_to(app, Screen::RemoveRepos),
        _ => {}
    }
    Action::Continue
}

fn handle_remove_repos(app: &mut App, code: KeyCode) -> Action {
    if let Some(a) = handle_list_nav(app, code, Screen::RepoManager) {
        return a;
    }
    if !is_activate(code) {
        return Action::Continue;
    }
    let rows = app.remove_repo_items();
    match rows.get(app.selected_index(Screen::RemoveRepos)) {
        Some(RemoveRepoRow::Back) => app.screen = Screen::RepoManager,
        Some(RemoveRepoRow::RepoDelete { name, .. }) => {
            let name = name.clone();
            app.confirm_delete_repo(name);
        }
        _ => {}
    }
    Action::Continue
}

fn handle_td_settings(app: &mut App, code: KeyCode) -> Action {
    if let Some(a) = handle_list_nav(app, code, Screen::Settings) {
        app.auth_error = None;
        return a;
    }
    if !is_activate(code) {
        return Action::Continue;
    }
    app.auth_error = None;
    let rows = app.td_settings_items();
    match rows.get(app.selected_index(Screen::TimeDoctorSettings)).cloned() {
        Some(TimeSettingRow::Back) => app.screen = Screen::Settings,
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
    Action::Continue
}

fn handle_td_field_input(app: &mut App, code: KeyCode) -> Action {
    // `handle_text_input` would work here if not for the password branch needing to
    // report an auth error. Keep the explicit form for the Enter case only.
    match code {
        KeyCode::Char(c) => {
            if let Some(buf) = text_buf_mut(app) {
                buf.push(c);
            }
        }
        KeyCode::Backspace => {
            if let Some(buf) = text_buf_mut(app) {
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
                TimeDoctorField::Email => {
                    app.td_email = buf;
                    app.persist_td_settings();
                }
                TimeDoctorField::Password => {
                    if !app.set_td_password(&buf) {
                        app.auth_error = Some("Failed to save password to keychain".to_string());
                    }
                }
            }
        }
        KeyCode::Esc => app.input_mode = InputMode::Normal,
        _ => {}
    }
    Action::Continue
}

fn handle_contract_periods(app: &mut App, code: KeyCode) -> Action {
    if let Some(a) = handle_list_nav(app, code, Screen::Settings) {
        return a;
    }
    if !is_activate(code) {
        return Action::Continue;
    }
    let rows = app.cp_list_items();
    match rows.get(app.selected_index(Screen::ContractPeriods)) {
        Some(CpListRow::Back) => app.screen = Screen::Settings,
        Some(CpListRow::MondayField) => app.input_mode = InputMode::EditingCpMonday(app.add_cp_monday),
        Some(CpListRow::HoursField) => app.input_mode = InputMode::EditingCpHours(app.add_cp_hours_idx),
        Some(CpListRow::SavePeriod) => app.save_new_contract_period(),
        Some(CpListRow::Period { index, .. }) => app.confirm_delete_period(*index),
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
