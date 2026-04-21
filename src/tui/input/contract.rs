use std::ops::ControlFlow;

use crossterm::event::KeyCode;

use crate::tui::app::{App, CpListRow, CycleTarget, InputMode, Screen};

use super::helpers::{execute_if_confirmed, list_activate_row};
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

pub(super) fn execute_pending_period_delete(app: &mut App) {
    execute_if_confirmed(
        app,
        |m| {
            if let InputMode::ConfirmDeletePeriod(i) = m {
                Some(*i)
            } else {
                None
            }
        },
        |a, idx| a.execute_delete_period(idx),
    );
}
