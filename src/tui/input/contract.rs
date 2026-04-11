use crossterm::event::KeyCode;

use crate::tui::app::{App, CpListRow, InputMode, Screen};

use super::helpers::handle_list_nav;
use super::keys::is_activate;
use super::Action;

pub(super) fn handle_contract_periods(app: &mut App, code: KeyCode) -> Action {
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

pub(super) fn execute_pending_period_delete(app: &mut App) {
    if let InputMode::ConfirmDeletePeriod(idx) = app.input_mode {
        app.execute_delete_period(idx);
    }
}
