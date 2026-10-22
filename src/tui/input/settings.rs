use std::ops::ControlFlow;

use crossterm::event::KeyCode;

use crate::config::Office;
use crate::tui::app::{App, CycleTarget, GeneralToggleRow, InputMode, Screen, SettingRow};

use super::helpers::{go_to, handle_cycle, list_activate_row};
use super::Action;

pub(super) fn handle_settings(app: &mut App, code: KeyCode) -> Action {
    let row = match list_activate_row(app, code, Screen::MainMenu, Screen::Settings, |a| {
        a.settings_items()
    }) {
        ControlFlow::Break(a) => return a,
        ControlFlow::Continue(r) => r,
    };
    match row {
        Some(SettingRow::Back) => app.screen = Screen::MainMenu,
        Some(SettingRow::ProfileDeveloper) => {
            app.profile.is_developer = !app.profile.is_developer;
            app.persist_profile();
        }
        Some(SettingRow::ProfileOffice) => {
            let index = app
                .profile
                .office
                .and_then(|selected| Office::ALL.iter().position(|office| *office == selected));
            app.profile.office = match index {
                None => Some(Office::Hacettepe),
                Some(index) if index + 1 == Office::ALL.len() => None,
                Some(index) => Some(Office::ALL[index + 1]),
            };
            app.persist_profile();
        }
        Some(SettingRow::ProfilePartTime) => {
            app.profile.is_part_time = !app.profile.is_part_time;
            app.persist_profile();
        }
        Some(SettingRow::ClearPdksSession) => {
            let _ = crate::pdks::clear_saved_session();
        }
        Some(SettingRow::SyncGeneralLink) => go_to(app, Screen::SyncGeneralSettings),
        Some(SettingRow::ManageRepos) => go_to(app, Screen::RepoManager),
        Some(SettingRow::TdGeneralLink) => go_to(app, Screen::TdGeneralSettings),
        Some(SettingRow::TimeDoctorSettings) => go_to(app, Screen::TimeDoctorSettings),
        Some(SettingRow::ContractPeriodsLink) => go_to(app, Screen::ContractPeriods),
        Some(SettingRow::OffWeeksLink) => go_to(app, Screen::OffWeeks),
        _ => {}
    }
    Action::Continue
}

pub(super) fn handle_general_toggles(app: &mut App, code: KeyCode) -> Action {
    if matches!(app.input_mode, InputMode::SelectingTimezone(_)) {
        return handle_cycle(app, code, CycleTarget::Timezone);
    }
    let screen = app.screen;
    let row = match list_activate_row(app, code, Screen::Settings, screen, |a| {
        if screen == Screen::SyncGeneralSettings {
            a.sync_general_items()
        } else {
            a.td_general_items()
        }
    }) {
        ControlFlow::Break(a) => return a,
        ControlFlow::Continue(r) => r,
    };
    match row {
        Some(GeneralToggleRow::Back) => app.screen = Screen::Settings,
        Some(GeneralToggleRow::Toggle { kind, disabled, .. }) => {
            if !disabled {
                app.toggle_by_kind(kind);
            }
        }
        Some(GeneralToggleRow::TimezoneSelector { .. }) => {
            app.enter_cycle(CycleTarget::Timezone);
        }
        _ => {}
    }
    Action::Continue
}
