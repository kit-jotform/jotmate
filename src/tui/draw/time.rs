use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

use crate::tui::app::{App, InputMode, Screen, TimeDoctorField, TimeSettingRow};
use crate::tui::layout::LayoutEngine;
use crate::tui::palette::{C_DANGEROUS, C_MUTED};

use super::{
    back_item, draw_screen_header, field_state, hint_muted, inline_field_item, sub_screen_layout,
    FIELD_LABEL_W, SEPARATOR_WIDTH,
};

pub fn draw_td_settings(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let engine = LayoutEngine::new(area);
    let layout = sub_screen_layout(engine.clamp_area(area));

    let td_rows = app.td_settings_items();
    let td_selected = app.selected_index(Screen::TimeDoctorSettings);
    let hint_spans = if matches!(&app.input_mode, InputMode::EditingField { .. }) {
        hint_muted(&["↵", " save  •  ", "Esc", " cancel"])
    } else {
        let action = match td_rows.get(td_selected) {
            Some(TimeSettingRow::Back) => "enter",
            _ => "edit",
        };
        hint_muted(&["↑↓", " navigate  •  ", "↵", &format!(" {action:<6}  •  "), "⌫/Esc", " back"])
    };

    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        "Manage Credentials",
        hint_spans,
    );

    let items: Vec<ListItem> = td_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = td_selected == i;
            match row {
                TimeSettingRow::Blank => ListItem::new(Line::raw("")),

                TimeSettingRow::Separator => ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "─".repeat(SEPARATOR_WIDTH),
                        Style::default().fg(C_MUTED),
                    ),
                ])),

                TimeSettingRow::Back => back_item(is_sel),

                TimeSettingRow::Password { is_set } => {
                    let active_buf = match &app.input_mode {
                        InputMode::EditingField {
                            field: TimeDoctorField::Password,
                            buf,
                        } => Some(buf.as_str()),
                        _ => None,
                    };
                    let display_value = if let Some(buf) = active_buf {
                        format!("{}_", "*".repeat(buf.len()))
                    } else if *is_set {
                        "[saved]".to_string()
                    } else {
                        "—".to_string()
                    };
                    let state = field_state(is_sel, active_buf.is_some());
                    inline_field_item("Password", &display_value, state, FIELD_LABEL_W)
                }

                TimeSettingRow::EditField {
                    field,
                    label,
                    value,
                    masked,
                } => {
                    let active_buf = match &app.input_mode {
                        InputMode::EditingField { field: f, buf } if f == field => {
                            Some(buf.as_str())
                        }
                        _ => None,
                    };
                    let display_value = if let Some(buf) = active_buf {
                        if *masked {
                            format!("{}_", "*".repeat(buf.len()))
                        } else {
                            format!("{buf}_")
                        }
                    } else if value.is_empty() {
                        "—".to_string()
                    } else if *masked {
                        "*".repeat(value.len())
                    } else {
                        value.clone()
                    };
                    let state = field_state(is_sel, active_buf.is_some());
                    inline_field_item(label, &display_value, state, FIELD_LABEL_W)
                }
            }
        })
        .collect();

    let list_area = layout.get("list");

    let (error_area, items_area) = if app.auth_error.is_some() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(list_area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, list_area)
    };

    if let (Some(area), Some(msg)) = (error_area, &app.auth_error) {
        let text = format!("  ✗ {msg}");
        f.render_widget(
            Paragraph::new(text).style(Style::default().fg(C_DANGEROUS)),
            area,
        );
    }

    f.render_stateful_widget(
        List::new(items),
        items_area,
        &mut app.list_state(Screen::TimeDoctorSettings).clone(),
    );
}
