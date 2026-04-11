use crossterm::event::KeyCode;

use crate::tui::app::{App, GeneralToggleRow, InputMode, Screen, SettingRow};

use super::helpers::{go_to, handle_cycle, handle_list_nav};
use super::keys::is_activate;
use super::Action;

pub(super) fn handle_settings(app: &mut App, code: KeyCode) -> Action {
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

pub(super) fn handle_general_toggles(app: &mut App, code: KeyCode) -> Action {
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
