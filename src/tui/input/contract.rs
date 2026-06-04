use std::ops::ControlFlow;

use crossterm::event::KeyCode;

use crate::tui::app::{App, CpListRow, CycleTarget, Screen};

use super::helpers::{list_activate_row, pending_delete};
use super::Action;

pub(super) fn handle_contract_periods(app: &mut App, code: KeyCode) -> Action {
    let row = match list_activate_row(app, code, Screen::Settings, Screen::ContractPeriods, |a| {
        a.cp_list_items()
    }) {
        ControlFlow::Break(a) => return a,
        ControlFlow::Continue(r) => r,
    };
    match row {
        Some(CpListRow::Back) => app.screen = Screen::Settings,
        Some(CpListRow::MondayField) => app.enter_cycle(CycleTarget::CpMonday),
        Some(CpListRow::HoursField) => app.enter_cycle(CycleTarget::CpHours),
        Some(CpListRow::SavePeriod) => app.save_new_contract_period(),
        Some(CpListRow::Period { index, .. }) => app.confirm_delete_period(index),
        _ => {}
    }
    Action::Continue
}

pending_delete!(
    execute_pending_period_delete,
    ConfirmDeletePeriod,
    execute_delete_period
);
