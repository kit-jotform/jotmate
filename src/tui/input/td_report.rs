use crossterm::event::KeyCode;

use crate::tui::app::{App, Screen, TdReportState};

use super::Action;

pub(super) fn handle_td_report(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Up | KeyCode::Left => {
            app.td_report_scroll = app.td_report_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Right => {
            let max_scroll = match &app.td_report {
                TdReportState::Ready { rows, .. } => rows.len().saturating_sub(6),
                TdReportState::PartialReady { rows, pending, .. } => {
                    (rows.len() + pending).saturating_sub(6)
                }
                _ => 0,
            };
            app.td_report_scroll = (app.td_report_scroll + 1).min(max_scroll);
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
