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
            // Nav/back consumed the key — clear stale auth errors. Idle keys
            // (e.g. random letters) instead return Continue via is_activate=false;
            // distinguish them so they don't dismiss the error.
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
    // Snapshot which field is being edited so the on_enter closure can route to
    // the matching save path (email validation vs keychain password save).
    let field = match &app.input_mode {
        InputMode::EditingField { field, .. } => *field,
        _ => return Action::Continue,
    };
    handle_text_input(app, code, move |app, buf| match field {
        TimeDoctorField::Email => {
            if !is_valid_email(&buf) {
                app.auth_error = Some("Invalid email address.".to_string());
            } else {
                let _ = crate::time::keychain::delete_token_from_keychain();
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
