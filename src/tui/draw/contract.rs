use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
};

use crate::tui::app::{App, CpListRow, InputMode, Screen, WEEKLY_HOURS_OPTIONS};
use crate::tui::layout::LayoutEngine;
use crate::tui::palette::{C_DANGEROUS, C_MUTED, C_TEXT};

use super::{
    back_item, draw_confirm_dialog, draw_screen_header, field_state, fmt_date,
    hint_confirm_cancel, hint_select_back, inline_field_item, link_item, sub_screen_layout,
    DIVIDER_WIDTH, FIELD_LABEL_W,
};

pub fn draw_contract_periods(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let engine = LayoutEngine::new(area);
    let layout = sub_screen_layout(engine.clamp_area(area));

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
