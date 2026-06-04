use crate::tui::app::{App, Screen};

mod common;
mod contract;
mod main_menu;
mod off_weeks;
mod repos;
mod settings;
mod sync;
mod td_report;
mod time;
mod update;

pub(in crate::tui::draw) use common::{
    back_item, blank_item, del_item, divider_item, draw_confirm_dialog, draw_screen_header,
    draw_scroll_table, field_state, fmt_date, fmt_hours, hint_confirm_cancel, hint_input_confirm,
    hint_muted, hint_navigate_action, inline_field_item, inset_rect, link_item, separator_item,
    sub_link_item, sub_screen_setup, toggle_item, FieldState, DIVIDER_WIDTH, FIELD_LABEL_W,
    FIELD_LABEL_W_TZ, HINT_RETURN_TO_MENU,
};

use contract::draw_contract_periods;
use main_menu::draw_main_menu;
use off_weeks::draw_off_weeks;
use repos::{draw_remove_repos, draw_repo_manager};
use settings::{draw_general_toggles, draw_settings};
use td_report::draw_td_report;
use time::draw_td_settings;

pub fn draw(f: &mut ratatui::Frame, app: &App) {
    match app.screen {
        Screen::MainMenu => draw_main_menu(f, app),
        Screen::Settings => draw_settings(f, app),
        Screen::SyncGeneralSettings => draw_general_toggles(f, app, "RDS Sync", true),
        Screen::RepoManager => draw_repo_manager(f, app),
        Screen::RemoveRepos => draw_remove_repos(f, app),
        Screen::TdGeneralSettings => draw_general_toggles(f, app, "Time Doctor", false),
        Screen::TimeDoctorSettings => draw_td_settings(f, app),
        Screen::ContractPeriods => draw_contract_periods(f, app),
        Screen::OffWeeks => draw_off_weeks(f, app),
        Screen::TimeDoctorReport => draw_td_report(f, app),
        Screen::SyncProgress => sync::draw_sync_progress(f, app),
        Screen::UpdateProgress => update::draw_update_progress(f, app),
    }
}
