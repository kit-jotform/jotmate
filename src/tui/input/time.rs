use crossterm::event::KeyCode;

use crate::tui::app::{App, InputMode, Screen, TimeDoctorField, TimeSettingRow};

use super::helpers::{handle_list_nav, text_buf_mut};
use super::keys::is_activate;
use super::Action;

pub(super) fn handle_td_settings(app: &mut App, code: KeyCode) -> Action {
    if let Some(a) = handle_list_nav(app, code, Screen::Settings) {
        app.auth_error = None;
        return a;
    }
    if !is_activate(code) {
        return Action::Continue;
    }
    app.auth_error = None;
    let rows = app.td_settings_items();
    match rows.get(app.selected_index(Screen::TimeDoctorSettings)).cloned() {
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

/// `handle_text_input` would mostly work here, but the password branch needs to
/// report an auth error on save failure. Keep the explicit form.
pub(super) fn handle_td_field_input(app: &mut App, code: KeyCode) -> Action {
    match code {
        KeyCode::Char(c) => {
            if let Some(buf) = text_buf_mut(app) {
                buf.push(c);
            }
        }
        KeyCode::Backspace => {
            if let Some(buf) = text_buf_mut(app) {
                buf.pop();
            }
        }
        KeyCode::Enter => {
            let (field, buf) = match app.input_mode.clone() {
                InputMode::EditingField { field, buf } => (field, buf),
                _ => return Action::Continue,
            };
            app.input_mode = InputMode::Normal;
            match field {
                TimeDoctorField::Email => {
                    app.td_email = buf;
                    app.persist_td_settings();
                }
                TimeDoctorField::Password => {
                    if !app.set_td_password(&buf) {
                        app.auth_error = Some("Failed to save password to keychain".to_string());
                    }
                }
            }
        }
        KeyCode::Esc => app.input_mode = InputMode::Normal,
        _ => {}
    }
    Action::Continue
}
