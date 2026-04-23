use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::time::compute::{format_hours, format_hours_signed, HOURS_DISPLAY_WIDTH};
use crate::tui::app::{App, TdReportState, TD_REPORT_VISIBLE_ROWS};
use crate::tui::layout::UI_WIDTH;
use crate::tui::palette::{C_DANGEROUS, C_MUTED, C_SUCCESS, C_TEXT, C_WARN};

use super::{
    draw_screen_header, draw_scroll_table, hint_muted, inset_rect, sub_screen_setup,
    HINT_RETURN_TO_MENU,
};

use crate::tui::palette::{C_ACCENT, SPINNER};

pub fn draw_td_report(f: &mut ratatui::Frame, app: &App) {
    let (area, engine, layout) = sub_screen_setup(f);

    let hint_spans = match &app.td_report {
        TdReportState::Ready { .. } | TdReportState::PartialReady { .. } => {
            hint_muted(&["↑↓", " scroll  •  ", "⌫/Esc", " cancel"])
        }
        TdReportState::NoCredentials(_) | TdReportState::NoPeriods => {
            hint_muted(&["↵", " configure  •  ", "⌫/Esc", " cancel"])
        }
        _ => hint_muted(&["⌫/Esc", " cancel"]),
    };

    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        "Time Doctor",
        hint_spans,
    );

    // Clamp to UI_WIDTH so the report doesn't bleed across wide terminals.
    let list_area = clamp_to_ui_width(layout.get("list"), area.x);
    let visible_row_count = match &app.td_report {
        TdReportState::Ready { rows, .. } => rows.len(),
        TdReportState::PartialReady { rows, pending, .. } => rows.len() + pending,
        _ => 0,
    };
    let content_height = 2 + TD_REPORT_VISIBLE_ROWS.min(visible_row_count) as u16;
    let (content_area, total_area, hint_area) = split_content_total_hint(list_area, content_height);
    let content_area = inset_rect(content_area, 3);

    let total_area = inset_rect(total_area, 3);
    let total_line = match &app.td_report {
        TdReportState::PartialReady { tick, .. } => {
            let spinner_ch = SPINNER[(*tick as usize) % SPINNER.len()];
            let elapsed = app
                .td_report_started_at
                .map(|t| t.elapsed().as_secs_f64())
                .unwrap_or(0.0);
            total_weekly_line(&spinner_ch.to_string(), C_ACCENT, Some(elapsed))
        }
        TdReportState::Ready { rows, .. } => {
            let total: f64 = rows.iter().map(|r| r.balance_hours).sum();
            let color = if total >= 0.0 { C_SUCCESS } else { C_DANGEROUS };
            total_weekly_line(
                &format_hours_signed(total),
                color,
                app.td_report_elapsed_secs,
            )
        }
        _ => Line::default(),
    };
    f.render_widget(Paragraph::new(total_line), total_area);

    let bottom_hint = match &app.td_report {
        TdReportState::Loading => "Loading...",
        TdReportState::Ready { .. } => HINT_RETURN_TO_MENU,
        TdReportState::PartialReady { .. } => HINT_RETURN_TO_MENU,
        _ => "",
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            bottom_hint,
            Style::default().fg(C_MUTED),
        ))),
        hint_area,
    );

    match &app.td_report {
        TdReportState::Loading => {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("fetching report…", Style::default().fg(C_MUTED)),
                ])),
                content_area,
            );
        }

        TdReportState::Error(msg) => {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("✗ {msg}"), Style::default().fg(C_DANGEROUS)),
                ])),
                content_area,
            );
        }

        TdReportState::NoCredentials(msg) => {
            let lines = vec![
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(msg.clone(), Style::default().fg(C_TEXT)),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Press ↵ to update credentials.",
                        Style::default().fg(C_MUTED),
                    ),
                ]),
            ];
            f.render_widget(Paragraph::new(lines), content_area);
        }

        TdReportState::NoPeriods => {
            let lines = vec![
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "No contract periods configured.",
                        Style::default().fg(C_TEXT),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Press ↵ to set up contract periods.",
                        Style::default().fg(C_MUTED),
                    ),
                ]),
            ];
            f.render_widget(Paragraph::new(lines), content_area);
        }

        TdReportState::PartialReady {
            rows,
            pending,
            show_cumulative,
            tick,
        } => {
            let spinner_ch = SPINNER[(*tick as usize) % SPINNER.len()];
            let total = rows.len() + pending;
            let mut data_lines: Vec<Line> = rows
                .iter()
                .enumerate()
                .map(|(i, r)| build_row(r, *show_cumulative, i + 1, total))
                .collect();
            for n in 0..*pending {
                data_lines.push(live_spinner_row(spinner_ch, rows.len() + 1 + n));
            }
            draw_report_table(
                f,
                content_area,
                *show_cumulative,
                data_lines,
                app.td_report_scroll,
            );
        }

        TdReportState::Ready {
            rows,
            show_cumulative,
        } => {
            let total = rows.len();
            let data_lines: Vec<Line> = rows
                .iter()
                .enumerate()
                .map(|(i, r)| build_row(r, *show_cumulative, i + 1, total))
                .collect();
            draw_report_table(
                f,
                content_area,
                *show_cumulative,
                data_lines,
                app.td_report_scroll,
            );
        }
    }
}

fn draw_report_table(
    f: &mut ratatui::Frame,
    area: Rect,
    show_cumulative: bool,
    data_lines: Vec<Line>,
    scroll: usize,
) {
    draw_scroll_table(f, area, build_header(show_cumulative), data_lines, scroll);
}

fn live_spinner_row(spinner_ch: char, index: usize) -> Line<'static> {
    let (week_w, _) = col_widths(true); // use widest layout to align with header
    let idx_w = index.to_string().len() + 2;
    let date_w = week_w.saturating_sub(idx_w);
    let idx_str = format!("{:>width$}. ", index, width = index.to_string().len());
    let placeholder = format!("{:<width$}", "fetching…", width = date_w);
    Line::from(vec![
        Span::styled(format!("{spinner_ch} "), Style::default().fg(C_ACCENT)),
        Span::styled(idx_str, Style::default().fg(C_MUTED)),
        Span::styled(placeholder, Style::default().fg(C_MUTED)),
    ])
}

fn total_weekly_line(
    value: &str,
    value_color: ratatui::style::Color,
    elapsed_secs: Option<f64>,
) -> Line<'static> {
    let padded = format!("{:<width$}", value, width = HOURS_DISPLAY_WIDTH);
    let mut spans = vec![
        Span::styled("Total Weekly: ", Style::default().fg(C_MUTED)),
        Span::styled(padded, Style::default().fg(value_color)),
    ];
    if let Some(elapsed) = elapsed_secs {
        spans.push(Span::styled(" •  ", Style::default().fg(C_MUTED)));
        spans.push(Span::styled(
            format!("{elapsed:.1}s"),
            Style::default().fg(C_MUTED),
        ));
    }
    Line::from(spans)
}

fn clamp_to_ui_width(area: Rect, base_x: u16) -> Rect {
    Rect {
        x: base_x,
        width: UI_WIDTH.min(area.width),
        ..area
    }
}

fn split_content_total_hint(area: Rect, content_height: u16) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(content_height),
            Constraint::Length(1), // blank
            Constraint::Length(1), // total
            Constraint::Length(1), // blank
            Constraint::Length(1), // hint
        ])
        .split(area);
    (chunks[0], chunks[2], chunks[4])
}

const WEEK_W: usize = 26; // 4-char index prefix + 22-char date range
const TOTAL_W: usize = 71; // UI_WIDTH(79) - 6 margins - 1 gap - 1 scrollbar

fn col_widths(show_cumulative: bool) -> (usize, usize) {
    let num_cols = if show_cumulative { 4 } else { 3 };
    let num_w = (TOTAL_W - WEEK_W) / num_cols;
    (WEEK_W, num_w)
}

fn build_header(show_cumulative: bool) -> Line<'static> {
    let (week_w, num_w) = col_widths(show_cumulative);
    let header_style = Style::default().fg(C_MUTED).add_modifier(Modifier::BOLD);
    let cell = |text: &str, width: usize, left: bool| {
        let s = if left {
            format!("{:<width$}", text, width = width)
        } else {
            format!("{:>width$}", text, width = width)
        };
        Span::styled(s, header_style)
    };

    let mut spans = vec![
        cell("Week", week_w, true),
        cell("Worked", num_w, false),
        cell("Target", num_w, false),
        cell("Balance", num_w, false),
    ];
    if show_cumulative {
        spans.push(cell("Cuml.", num_w, false));
    }
    Line::from(spans)
}

fn build_row(
    row: &crate::time::compute::WeekRow,
    show_cumulative: bool,
    index: usize,
    total: usize,
) -> Line<'static> {
    let (week_w, num_w) = col_widths(show_cumulative);

    let worked_h = row.worked_secs as f64 / 3600.0;
    let balance = row.balance_hours;
    let cumulative = row.cumulative_hours;

    let balance_color = if balance >= 0.0 {
        C_SUCCESS
    } else {
        C_DANGEROUS
    };
    let cum_color = if cumulative >= 0.0 {
        C_SUCCESS
    } else {
        C_DANGEROUS
    };

    let week_color = if row.from_cache { C_TEXT } else { C_WARN };

    let worked_str = format!("{:>width$}", format_hours(worked_h), width = num_w);
    let target_value = if row.from_cache {
        format!("• {}", format_hours(row.target_hours))
    } else {
        format_hours(row.target_hours)
    };
    let target_str = format!("{:>width$}", target_value, width = num_w);
    let balance_str = format!("{:>width$}", format_hours_signed(balance), width = num_w);

    let idx_w = total.to_string().len() + 2; // digits + ". "
    let date_w = week_w.saturating_sub(idx_w);
    let idx_str = format!("{:>width$}. ", index, width = total.to_string().len());
    let date_label = if row.week_label.len() > date_w {
        format!("{:<width$}", &row.week_label[..date_w], width = date_w)
    } else {
        format!("{:<width$}", row.week_label, width = date_w)
    };

    let mut spans = vec![
        Span::styled(idx_str, Style::default().fg(C_MUTED)),
        Span::styled(date_label, Style::default().fg(week_color)),
        Span::styled(worked_str, Style::default().fg(C_TEXT)),
        Span::styled(target_str, Style::default().fg(C_MUTED)),
        Span::styled(balance_str, Style::default().fg(balance_color)),
    ];

    if show_cumulative {
        let cum_str = format!("{:>width$}", format_hours_signed(cumulative), width = num_w);
        spans.push(Span::styled(cum_str, Style::default().fg(cum_color)));
    }

    Line::from(spans)
}
