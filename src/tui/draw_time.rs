use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

fn field_state(is_sel: bool, editing: bool) -> FieldState {
    if editing {
        FieldState::Editing
    } else if is_sel {
        FieldState::Selected
    } else {
        FieldState::Normal
    }
}

use super::app::{App, CpListRow, InputMode, Screen, TimeDoctorField, TimeSettingRow, WEEKLY_HOURS_OPTIONS};
use super::draw::{
    back_item, draw_confirm_dialog, draw_screen_header, fmt_date, hint_confirm_cancel,
    hint_muted, hint_select_back, inline_field_item, link_item, sub_screen_layout, FieldState,
    DIVIDER_WIDTH, FIELD_LABEL_W,
};
use super::layout::LayoutEngine;
use super::palette::{C_DANGEROUS, C_MUTED, C_TEXT};

pub fn draw_td_settings(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let layout = sub_screen_layout(area);
    let engine = LayoutEngine::new(area.x);

    let hint_spans = if matches!(&app.input_mode, InputMode::EditingField { .. }) {
        hint_muted(&["↵", " save  •  ", "Esc", " cancel"])
    } else {
        hint_muted(&["↵", " edit  •  ", "⌫/Esc", " back"])
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

    let td_rows = app.td_settings_items();
    let selected = app.selected_index(Screen::TimeDoctorSettings);

    let items: Vec<ListItem> = td_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                TimeSettingRow::Blank => ListItem::new(Line::raw("")),

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

pub fn draw_contract_periods(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let layout = sub_screen_layout(area);
    let engine = LayoutEngine::new(area.x);

    let hint_spans = match &app.input_mode {
        InputMode::ConfirmDeletePeriod(_) => hint_confirm_cancel(),
        _ => hint_select_back(),
    };

    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        "Contract Periods",
        hint_spans,
    );

    let cp_rows = app.cp_list_items();
    let selected = app.selected_index(Screen::ContractPeriods);

    let items: Vec<ListItem> = cp_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                CpListRow::Blank => ListItem::new(Line::raw("")),
                CpListRow::Separator => ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "─".repeat(DIVIDER_WIDTH as usize),
                        Style::default().fg(C_MUTED),
                    ),
                ])),
                CpListRow::Back => back_item(is_sel),
                CpListRow::SavePeriod => link_item(is_sel, "Save period"),
                CpListRow::MondayField => {
                    let value = fmt_date(app.add_cp_monday);
                    let editing = matches!(app.input_mode, InputMode::EditingCpMonday(_));
                    inline_field_item(
                        "From Monday",
                        &value,
                        field_state(is_sel, editing),
                        FIELD_LABEL_W,
                    )
                }
                CpListRow::HoursField => {
                    let hours_val = WEEKLY_HOURS_OPTIONS[app.add_cp_hours_idx];
                    let value = if hours_val.fract() == 0.0 {
                        format!("{}h", hours_val as u32)
                    } else {
                        format!("{hours_val}h")
                    };
                    let editing = matches!(app.input_mode, InputMode::EditingCpHours(_));
                    inline_field_item(
                        "Weekly hours",
                        &value,
                        field_state(is_sel, editing),
                        FIELD_LABEL_W,
                    )
                }
                CpListRow::Period {
                    from, weekly_hours, ..
                } => {
                    let hours_display = if weekly_hours.fract() == 0.0 {
                        format!("{}h/week", *weekly_hours as u32)
                    } else {
                        format!("{weekly_hours}h/week")
                    };
                    let detail = format!("{}  {}", fmt_date(*from), hours_display);
                    if is_sel {
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_DANGEROUS)),
                            Span::styled(
                                "[del]",
                                Style::default()
                                    .fg(C_DANGEROUS)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(format!("  {detail}"), Style::default().fg(C_TEXT)),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled("[del]", Style::default().fg(C_MUTED)),
                            Span::styled(format!("  {detail}"), Style::default().fg(C_TEXT)),
                        ]))
                    }
                }
            }
        })
        .collect();

    f.render_stateful_widget(
        List::new(items),
        layout.get("list"),
        &mut app.list_state(Screen::ContractPeriods).clone(),
    );

    if let InputMode::ConfirmDeletePeriod(idx) = &app.input_mode {
        if let Some(p) = app.contract_periods.get(*idx) {
            draw_confirm_dialog(f, area, &format!("Delete period {}?", fmt_date(p.from)));
        }
    }
}
