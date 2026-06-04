use ratatui::widgets::{List, ListItem};

use crate::tui::app::{App, GeneralToggleRow, InputMode, Screen, SettingRow};

use super::{
    back_item, blank_item, divider_item, draw_screen_header, hint_navigate_action,
    inline_field_item, link_item, separator_item, sub_screen_setup, toggle_item, FieldState,
    FIELD_LABEL_W_TZ,
};

pub fn draw_settings(f: &mut ratatui::Frame, app: &App) {
    let (_area, engine, layout) = sub_screen_setup(f);

    let setting_rows = app.settings_items();
    let selected = app.selected_index(Screen::Settings);

    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        "Settings",
        hint_navigate_action("enter"),
    );

    let items: Vec<ListItem> = setting_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                SettingRow::Blank => blank_item(),

                SettingRow::Divider => divider_item(),

                SettingRow::Separator => {
                    let sep_count = setting_rows[..i]
                        .iter()
                        .filter(|r| matches!(r, SettingRow::Separator))
                        .count();
                    let label = if sep_count == 0 {
                        "RDS Sync"
                    } else {
                        "Time Doctor"
                    };
                    separator_item(label)
                }

                SettingRow::Back => back_item(is_sel),
                SettingRow::SyncGeneralLink => link_item(is_sel, "→ General settings"),
                SettingRow::ManageRepos => link_item(is_sel, "→ Manage upstream repos"),
                SettingRow::TdGeneralLink => link_item(is_sel, "→ General settings"),
                SettingRow::TimeDoctorSettings => link_item(is_sel, "→ Manage credentials"),
                SettingRow::ContractPeriodsLink => link_item(is_sel, "→ Manage contract periods"),
                SettingRow::OffWeeksLink => link_item(is_sel, "→ Manage off weeks"),
            }
        })
        .collect();

    f.render_stateful_widget(
        List::new(items),
        layout.get("list"),
        &mut app.list_state(Screen::Settings).clone(),
    );
}

pub fn draw_general_toggles(f: &mut ratatui::Frame, app: &App, title: &str, is_sync: bool) {
    let (_area, engine, layout) = sub_screen_setup(f);

    let screen = if is_sync {
        Screen::SyncGeneralSettings
    } else {
        Screen::TdGeneralSettings
    };
    let rows = if is_sync {
        app.sync_general_items()
    } else {
        app.td_general_items()
    };
    let selected = app.selected_index(screen);

    let action = match rows.get(selected) {
        Some(GeneralToggleRow::Toggle { .. }) => "toggle",
        Some(GeneralToggleRow::TimezoneSelector { .. }) => "change",
        _ => "enter",
    };
    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        title,
        hint_navigate_action(action),
    );

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                GeneralToggleRow::Blank => blank_item(),
                GeneralToggleRow::Separator => divider_item(),
                GeneralToggleRow::Back => back_item(is_sel),
                GeneralToggleRow::Toggle {
                    label,
                    hint,
                    on,
                    indent,
                    disabled,
                    ..
                } => {
                    let label_text = if hint.is_empty() {
                        label.to_string()
                    } else {
                        format!("{label}  ({hint})")
                    };
                    toggle_item(is_sel, *on, label_text, *indent, *disabled)
                }
                GeneralToggleRow::TimezoneSelector { value } => {
                    let selecting = matches!(app.input_mode, InputMode::SelectingTimezone(_));
                    let state = if selecting {
                        FieldState::Editing
                    } else if is_sel {
                        FieldState::Selected
                    } else {
                        FieldState::Normal
                    };
                    inline_field_item("Timezone", value, state, FIELD_LABEL_W_TZ)
                }
            }
        })
        .collect();

    f.render_stateful_widget(
        List::new(items),
        layout.get("list"),
        &mut app.list_state(screen).clone(),
    );
}
