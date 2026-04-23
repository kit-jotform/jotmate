use crossterm::event::KeyCode;

use crate::tui::app::{App, Screen, TdReportState, TD_REPORT_VISIBLE_ROWS};

use super::helpers::clamp_scroll;
use super::Action;

pub(super) fn handle_td_report(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Left | KeyCode::Down | KeyCode::Right => {
            let total = match &app.td_report {
                TdReportState::Ready { rows, .. } => rows.len(),
                TdReportState::PartialReady { rows, pending, .. } => rows.len() + pending,
                _ => 0,
            };
            let delta = if matches!(code, KeyCode::Up | KeyCode::Left) {
                -1
            } else {
                1
            };
            app.td_report_scroll =
                clamp_scroll(app.td_report_scroll, delta, total, TD_REPORT_VISIBLE_ROWS);
        }
        KeyCode::Enter => match app.td_report {
            TdReportState::NoPeriods => app.screen = Screen::ContractPeriods,
            TdReportState::NoCredentials(_) => app.screen = Screen::TimeDoctorSettings,
            TdReportState::Ready { .. } | TdReportState::PartialReady { .. } => {
                app.screen = Screen::MainMenu
            }
            _ => {}
        },
        KeyCode::Esc | KeyCode::Backspace => app.screen = Screen::MainMenu,
        KeyCode::Char('q') => return Action::Back,
        _ => {}
    }
    Action::Continue
}
