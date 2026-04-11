use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::app::{App, TdReportState};
use super::draw::{draw_screen_header, hint_muted, sub_screen_layout};
use super::layout::LayoutEngine;
use super::palette::{C_ACCENT, C_DANGEROUS, C_MUTED, C_PRIMARY, C_SUCCESS, C_TEXT, C_WARN};

use crate::time::compute::{format_hours, format_hours_signed};

pub fn draw_td_report(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let layout = sub_screen_layout(area);
    let engine = LayoutEngine::new(area.x);

    let hint_spans = match &app.td_report {
        TdReportState::Loading | TdReportState::NeedsReauth => hint_muted(&["loading…"]),
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
    // Inset content 3 chars on each side
    let content_area = inset_horizontal(content_area, 3);

    // Fixed "← Back" footer, always visible
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("▸ ", Style::default().fg(C_PRIMARY)),
            Span::styled("← Back", Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
        ])),
        back_area,
    );

    match &app.td_report {
        TdReportState::Loading | TdReportState::NeedsReauth => {
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
            // Split content_area: fixed 2-row header + up to 6-row scrollable data
            const MAX_VISIBLE_ROWS: usize = 6;
            let data_area_height = MAX_VISIBLE_ROWS.min(rows.len()) as u16;
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(2), Constraint::Length(data_area_height)])
                .split(content_area);
            let header_area = chunks[0];
            let data_area = chunks[1];

            // Fixed header (never scrolls) — inset 2 chars on right to match gap + scrollbar
            let header_text_area = Rect { width: header_area.width.saturating_sub(2), ..header_area };
            let divider = Line::from(Span::styled(
                "─".repeat(header_text_area.width as usize),
                Style::default().fg(C_MUTED),
            ));
            f.render_widget(
                Paragraph::new(vec![build_header(*show_cumulative), divider]),
                header_text_area,
            );

            // Scrollable data rows — always reserve 1 char on the right for the scrollbar
            let total_rows = rows.len();
            let data_lines: Vec<Line> = rows.iter().enumerate()
                .map(|(i, r)| build_row(r, *show_cumulative, i + 1, total_rows))
                .collect();
            let total_lines = data_lines.len();
            let visible = data_area.height as usize;
            let max_scroll = total_lines.saturating_sub(visible);
            let scroll = app.td_report_scroll.min(max_scroll);

            // Reserve gap(1) + scrollbar(1) on the right
            let hchunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(1), Constraint::Length(1)])
                .split(data_area);

            f.render_widget(
                Paragraph::new(data_lines).scroll((scroll as u16, 0)),
                hchunks[0],
            );

            if total_lines > visible {
                // Single-char thumb: position proportionally within the track height
                let track_h = hchunks[2].height as usize;
                let thumb_row = scroll * track_h.saturating_sub(1) / max_scroll;
                let lines: Vec<Line> = (0..track_h)
                    .map(|i| {
                        if i == thumb_row {
                            Line::from(Span::styled("▐", Style::default().fg(C_MUTED)))
                        } else {
                            Line::from(Span::styled("│", Style::default().fg(C_MUTED)))
                        }
                    })
                    .collect();
                f.render_widget(Paragraph::new(lines), hchunks[2]);
            }
        }
    }
}

fn inset_horizontal(area: Rect, margin: u16) -> Rect {
    let inset = margin * 2;
    Rect {
        x: area.x + margin,
        width: area.width.saturating_sub(inset),
        ..area
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

const WEEK_W: usize = 26; // 4-char index prefix + 22-char date range
const TOTAL_W: usize = 71; // UI_WIDTH(79) - 6 margins - 1 gap - 1 scrollbar

fn col_widths(show_cumulative: bool) -> (usize, usize) {
    let num_cols = if show_cumulative { 4 } else { 3 };
    let num_w = (TOTAL_W - WEEK_W) / num_cols;
    (WEEK_W, num_w)
}

fn build_header(show_cumulative: bool) -> Line<'static> {
    let (week_w, num_w) = col_widths(show_cumulative);

    let mut spans = vec![
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
            format!("{:>width$}", "Cuml.", width = num_w),
            Style::default().fg(C_MUTED).add_modifier(Modifier::BOLD),
        ));
    }

    Line::from(spans)
}

fn build_row(row: &crate::time::compute::WeekRow, show_cumulative: bool, index: usize, total: usize) -> Line<'static> {
    let (week_w, num_w) = col_widths(show_cumulative);

    let worked_h = row.worked_secs as f64 / 3600.0;
    let balance = row.balance_hours;
    let cumulative = row.cumulative_hours;

    let balance_color = if balance >= 0.0 { C_SUCCESS } else { C_DANGEROUS };
    let cum_color = if cumulative >= 0.0 { C_SUCCESS } else { C_DANGEROUS };

    // Flag current (partial) week differently
    let week_color = if row.from_cache { C_TEXT } else { C_WARN };

    let worked_str = format!("{:>width$}", format_hours(worked_h), width = num_w);
    let target_str = format!("{:>width$}", format_hours(row.target_hours), width = num_w);
    let balance_str = format!("{:>width$}", format_hours_signed(balance), width = num_w);

    // Index prefix width scales with total (e.g. "9. " = 3, "23. " = 4)
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
