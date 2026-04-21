use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
};

use crate::tui::app::{App, CpListRow, InputMode, Screen, WEEKLY_HOURS_OPTIONS};
use crate::tui::palette::{C_ACCENT, C_MUTED, C_PRIMARY, C_WARN};

use super::{
    back_item, blank_item, del_item, divider_item, draw_confirm_dialog, draw_screen_header,
    field_state, fmt_date, fmt_hours, hint_confirm_cancel, hint_navigate_action, inline_field_item,
    sub_screen_setup, FIELD_LABEL_W,
};

pub fn draw_contract_periods(f: &mut ratatui::Frame, app: &App) {
    let (area, engine, layout) = sub_screen_setup(f);

    let cp_rows = app.cp_list_items();
    let selected = app.selected_index(Screen::ContractPeriods);
    let hint_spans = match &app.input_mode {
        InputMode::ConfirmDeletePeriod(_) => hint_confirm_cancel(),
        _ => {
            let action = match cp_rows.get(selected) {
                Some(CpListRow::Period { .. }) => "delete",
                Some(CpListRow::SavePeriod) => "save",
                Some(CpListRow::MondayField) | Some(CpListRow::HoursField) => "change",
                Some(CpListRow::Back) => "enter",
                _ => "select",
            };
            hint_navigate_action(action)
        }
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

    let items: Vec<ListItem> = cp_rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_sel = selected == i;
            match row {
                CpListRow::SectionTitle(title) => ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(*title, Style::default().fg(C_MUTED)),
                ])),
                CpListRow::Blank => blank_item(),
                CpListRow::Separator => divider_item(),
                CpListRow::Back => back_item(is_sel),
                CpListRow::SavePeriod => {
                    if is_sel {
                        ListItem::new(Line::from(vec![
                            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
                            Span::styled(
                                "Save period",
                                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    } else {
                        ListItem::new(Line::from(vec![
                            Span::raw("  "),
                            Span::styled("Save period", Style::default().fg(C_WARN)),
                        ]))
                    }
                }
                CpListRow::MondayField => {
                    let value = fmt_date(app.add_cp.monday);
                    let editing = matches!(app.input_mode, InputMode::EditingCpMonday(_));
                    inline_field_item(
                        "From Monday",
                        &value,
                        field_state(is_sel, editing),
                        FIELD_LABEL_W,
                    )
                }
                CpListRow::HoursField => {
                    let hours_val = WEEKLY_HOURS_OPTIONS[app.add_cp.hours_idx];
                    let editing = matches!(app.input_mode, InputMode::EditingCpHours(_));
                    inline_field_item(
                        "Weekly hours",
                        &fmt_hours(hours_val),
                        field_state(is_sel, editing),
                        FIELD_LABEL_W,
                    )
                }
                CpListRow::Period {
                    from, weekly_hours, ..
                } => {
                    let detail =
                        format!("  {}  {}/week", fmt_date(*from), fmt_hours(*weekly_hours));
                    del_item(is_sel, detail)
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
        if let Some(p) = app.td.contract_periods.get(*idx) {
            draw_confirm_dialog(f, area, &format!("Delete period {}?", fmt_date(p.from)));
        }
    }
}
