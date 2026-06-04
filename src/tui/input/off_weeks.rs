use std::ops::ControlFlow;

use crossterm::event::KeyCode;

use crate::tui::app::{App, CycleTarget, OwListRow, Screen};

use super::helpers::{list_activate_row, pending_delete};
use super::Action;

pub(super) fn handle_off_weeks(app: &mut App, code: KeyCode) -> Action {
    let row = match list_activate_row(app, code, Screen::Settings, Screen::OffWeeks, |a| {
        a.ow_list_items()
    }) {
        ControlFlow::Break(a) => return a,
        ControlFlow::Continue(r) => r,
    };
    match row {
        Some(OwListRow::Back) => app.screen = Screen::Settings,
        Some(OwListRow::MondayField) => app.enter_cycle(CycleTarget::OwMonday),
        Some(OwListRow::SaveOffWeek) => app.save_new_off_week(),
        Some(OwListRow::OffWeek { index, .. }) => app.confirm_delete_off_week(index),
        _ => {}
    }
    Action::Continue
}

pending_delete!(
    execute_pending_off_week_delete,
    ConfirmDeleteOffWeek,
    execute_delete_off_week
);
