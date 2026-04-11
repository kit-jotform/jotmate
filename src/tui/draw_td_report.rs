use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use super::app::{App, TdReportState};
use super::draw::{draw_screen_header, hint_muted, sub_screen_layout};
use super::layout::LayoutEngine;
use super::palette::{C_ACCENT, C_DANGEROUS, C_MUTED, C_PRIMARY, C_SUCCESS, C_TEXT, C_WARN};

use crate::time::compute::format_hours;

pub fn draw_td_report(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let layout = sub_screen_layout(area);
    let engine = LayoutEngine::new(area.x);

    let hint_spans = match &app.td_report {
        TdReportState::Loading => hint_muted(&["loading…"]),
        TdReportState::Error(_) => hint_muted(&["⌫/Esc", " back"]),
        TdReportState::Ready { .. } => hint_muted(&["↑↓", " scroll  •  ", "⌫/Esc", " back"]),
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

    // Split the list area into scrollable content + fixed back footer
    let list_area = layout.get("list");
    let (content_area, back_area) = split_content_back(list_area);

    // Fixed "← Back" footer, always visible
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
            Span::styled("← Back", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        ])),
        back_area,
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

        TdReportState::Ready { rows, show_cumulative } => {
            // Header row + data rows
            let mut lines: Vec<Line> = Vec::with_capacity(rows.len() + 2);

            // Column header
            let header = build_header(*show_cumulative);
            lines.push(header);
            lines.push(Line::from(Span::styled(
                format!("  {}", "─".repeat(62)),
                Style::default().fg(C_MUTED),
            )));

            for row in rows {
                lines.push(build_row(row, *show_cumulative));
            }

            let total_lines = lines.len();
            let visible = content_area.height as usize;
            let max_scroll = total_lines.saturating_sub(visible);
            let scroll = app.td_report_scroll.min(max_scroll);

            // Render with scrollbar if needed
            if total_lines > visible {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Min(0), Constraint::Length(1)])
                    .split(content_area);

                f.render_widget(
                    Paragraph::new(lines).scroll((scroll as u16, 0)),
                    chunks[0],
                );

                let mut sb_state = ScrollbarState::default()
                    .content_length(max_scroll)
                    .position(scroll);
                f.render_stateful_widget(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight),
                    chunks[1],
                    &mut sb_state,
                );
            } else {
                f.render_widget(
                    Paragraph::new(lines).scroll((scroll as u16, 0)),
                    content_area,
                );
            }
        }
    }
}

/// Split `area` into (content, back_footer) — footer is 2 rows: blank + Back item.
fn split_content_back(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(area);
    (chunks[0], chunks[1])
}

fn build_header(show_cumulative: bool) -> Line<'static> {
    let week_w = 20usize;
    let num_w = 9usize;

    let mut spans = vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<width$}", "Week", width = week_w),
            Style::default().fg(C_MUTED).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:>width$}", "Worked", width = num_w),
            Style::default().fg(C_MUTED).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:>width$}", "Target", width = num_w),
            Style::default().fg(C_MUTED).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:>width$}", "Balance", width = num_w),
            Style::default().fg(C_MUTED).add_modifier(Modifier::BOLD),
        ),
    ];

    if show_cumulative {
        spans.push(Span::styled(
            format!("{:>width$}", "Running", width = num_w),
            Style::default().fg(C_MUTED).add_modifier(Modifier::BOLD),
        ));
    }

    Line::from(spans)
}

fn build_row(row: &crate::time::compute::WeekRow, show_cumulative: bool) -> Line<'static> {
    let week_w = 20usize;
    let num_w = 9usize;

    let worked_h = row.worked_secs as f64 / 3600.0;
    let balance = row.balance_hours;
    let cumulative = row.cumulative_hours;

    let balance_color = if balance >= 0.0 { C_SUCCESS } else { C_DANGEROUS };
    let cum_color = if cumulative >= 0.0 { C_SUCCESS } else { C_DANGEROUS };

    // Flag current (partial) week differently
    let week_color = if row.from_cache { C_TEXT } else { C_WARN };

    let worked_str = format!("{:>width$}", format_hours(worked_h), width = num_w);
    let target_str = format!("{:>width$}", format_hours(row.target_hours), width = num_w);
    let balance_str = format!("{:>width$}", format_hours(balance), width = num_w);

    // Truncate week label if needed
    let label = if row.week_label.len() > week_w {
        row.week_label[..week_w].to_string()
    } else {
        format!("{:<width$}", row.week_label, width = week_w)
    };

    let mut spans = vec![
        Span::raw("  "),
        Span::styled(label, Style::default().fg(week_color)),
        Span::styled(worked_str, Style::default().fg(C_TEXT)),
        Span::styled(target_str, Style::default().fg(C_MUTED)),
        Span::styled(balance_str, Style::default().fg(balance_color)),
    ];

    if show_cumulative {
        let cum_str = format!("{:>width$}", format_hours(cumulative), width = num_w);
        spans.push(Span::styled(cum_str, Style::default().fg(cum_color)));
    }

    Line::from(spans)
}
