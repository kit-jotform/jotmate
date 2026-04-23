use std::ops::ControlFlow;

use crossterm::event::KeyCode;

use crate::tui::app::{App, InputMode, Screen, TimeDoctorField, TimeSettingRow};

use super::helpers::{handle_text_input, list_activate_row};
use super::Action;

pub(super) fn handle_td_settings(app: &mut App, code: KeyCode) -> Action {
    let row = match list_activate_row(
        app,
        code,
        Screen::Settings,
        Screen::TimeDoctorSettings,
        |a| a.td_settings_items(),
    ) {
        ControlFlow::Break(a) => {
            // Only clear auth_error on real navigation keys; idle keys must not dismiss the error.
            if super::keys::nav_delta(code).is_some()
                || super::keys::is_back(code)
                || code == KeyCode::Char('q')
            {
                app.auth_error = None;
            }
            return a;
        }
        ControlFlow::Continue(r) => r,
    };
    app.auth_error = None;
    match row {
        Some(TimeSettingRow::Back) => app.screen = Screen::Settings,
        Some(TimeSettingRow::EditField { field, value, .. }) => {
            app.input_mode = InputMode::EditingField { field, buf: value };
        }
        Some(TimeSettingRow::Password { .. }) => {
            app.input_mode = InputMode::EditingField {
                field: TimeDoctorField::Password,
                buf: String::new(),
            };
        }
        _ => {}
    }
    Action::Continue
}

pub(super) fn handle_td_field_input(app: &mut App, code: KeyCode) -> Action {
    // Snapshot the field so on_enter can route to the right save path.
    let field = match &app.input_mode {
        InputMode::EditingField { field, .. } => *field,
        _ => return Action::Continue,
    };
    handle_text_input(app, code, move |app, buf| match field {
        TimeDoctorField::Email => {
            if !is_valid_email(&buf) {
                app.auth_error = Some("Invalid email address.".to_string());
            } else {
                let _ = app.ctx.keychain.delete_token();
                app.td.email = buf;
                app.persist_td_settings();
            }
        }
        TimeDoctorField::Password => {
            if !app.set_td_password(&buf) {
                app.auth_error = Some("Failed to save password to keychain".to_string());
            }
        }
    })
}

fn is_valid_email(s: &str) -> bool {
    let s = s.trim();
    matches!(s.find('@'), Some(at) if at > 0 && s[at + 1..].contains('.'))
}
