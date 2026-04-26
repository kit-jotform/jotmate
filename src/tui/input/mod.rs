//! Keyboard dispatcher. Per-screen handlers live in submodules.

use crossterm::event::KeyCode;

use super::app::{App, CycleTarget, InputMode, Screen};

mod contract;
mod helpers;
mod keys;
mod main_menu;
mod repos;
mod settings;
mod sync;
mod td_report;
mod time;
mod update;

use contract::{execute_pending_period_delete, handle_contract_periods};
use helpers::{handle_cycle, handle_text_input, handle_yes_no};
use main_menu::handle_main;
use repos::{
    apply_new_repo_url, execute_pending_repo_delete, handle_remove_repos, handle_repo_manager,
};
use settings::{handle_general_toggles, handle_settings};
use sync::handle_sync_progress;
use td_report::handle_td_report;
use time::{handle_td_field_input, handle_td_settings};
use update::handle_update_progress;

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
            InputMode::ConfirmDeletePeriod(_) => {
                handle_yes_no(app, code, execute_pending_period_delete)
            }
            InputMode::EditingCpMonday(_) => handle_cycle(app, code, CycleTarget::CpMonday),
            InputMode::EditingCpHours(_) => handle_cycle(app, code, CycleTarget::CpHours),
            _ => handle_contract_periods(app, code),
        },
        Screen::SyncProgress => handle_sync_progress(app, code),
        Screen::TimeDoctorReport => handle_td_report(app, code),
        Screen::UpdateProgress => handle_update_progress(app, code),
    }
}
