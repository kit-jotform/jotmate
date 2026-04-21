use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::time::compute::{format_hours, format_hours_signed};
use crate::tui::app::{App, TdReportState};
use crate::tui::layout::{LayoutEngine, UI_WIDTH};
use crate::tui::palette::{C_DANGEROUS, C_MUTED, C_SUCCESS, C_TEXT, C_WARN};

use super::{draw_screen_header, hint_muted, sub_screen_layout, HINT_RETURN_TO_MENU};

use crate::tui::palette::C_ACCENT;

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];

pub fn draw_td_report(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let engine = LayoutEngine::new(area);
    let layout = sub_screen_layout(engine.clamp_area(area));

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

    // Split the list area into scrollable content + total balance + hint row.
    // Clamp to UI_WIDTH so the report doesn't bleed across wide terminals.
    let list_area = clamp_to_ui_width(layout.get("list"), area.x);
    const MAX_VISIBLE_ROWS: usize = 6;
    let visible_row_count = match &app.td_report {
        TdReportState::Ready { rows, .. } => rows.len(),
        TdReportState::PartialReady { rows, pending, .. } => rows.len() + pending,
        _ => 0,
    };
    let content_height = 2 + MAX_VISIBLE_ROWS.min(visible_row_count) as u16;
    let (content_area, total_area, hint_area) = split_content_total_hint(list_area, content_height);
    // Inset content 3 chars on each side
    let content_area = inset_horizontal(content_area, 3);

    // Total balance row
    let total_area = inset_horizontal(total_area, 3);
    let total_line = match &app.td_report {
        TdReportState::PartialReady { tick, .. } => {
            let spinner_ch = SPINNER[(*tick as usize) % SPINNER.len()];
            Line::from(vec![
                Span::styled("Total Weekly: ", Style::default().fg(C_MUTED)),
                Span::styled(spinner_ch.to_string(), Style::default().fg(C_ACCENT)),
            ])
        }
        TdReportState::Ready { rows, .. } => {
            let total: f64 = rows.iter().map(|r| r.balance_hours).sum();
            let balance_color = if total >= 0.0 { C_SUCCESS } else { C_DANGEROUS };
            Line::from(vec![
                Span::styled("Total Weekly: ", Style::default().fg(C_MUTED)),
                Span::styled(
                    format_hours_signed(total),
                    Style::default().fg(balance_color),
                ),
            ])
        }
        _ => Line::default(),
    };
    f.render_widget(Paragraph::new(total_line), total_area);

    // Bottom status hint
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
            render_table(
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
            render_table(
                f,
                content_area,
                *show_cumulative,
                data_lines,
                app.td_report_scroll,
            );
        }
    }
}

/// Render the header + scrollable data lines into `content_area`.
fn render_table(
    f: &mut ratatui::Frame,
    content_area: Rect,
    show_cumulative: bool,
    data_lines: Vec<Line>,
    scroll_pos: usize,
) {
    let data_area_height = content_area.height.saturating_sub(2);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(data_area_height)])
        .split(content_area);
    let header_area = chunks[0];
    let data_area = chunks[1];

    let header_text_area = Rect {
        width: header_area.width.saturating_sub(2),
        ..header_area
    };
    let divider = Line::from(Span::styled(
        "─".repeat(header_text_area.width as usize),
        Style::default().fg(C_MUTED),
    ));
    f.render_widget(
        Paragraph::new(vec![build_header(show_cumulative), divider]),
        header_text_area,
    );

    let total_lines = data_lines.len();
    let visible = data_area.height as usize;
    let max_scroll = total_lines.saturating_sub(visible);
    let scroll = scroll_pos.min(max_scroll);

    let hchunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(data_area);

    f.render_widget(
        Paragraph::new(data_lines).scroll((scroll as u16, 0)),
        hchunks[0],
    );

    if total_lines > visible {
        let track_h = hchunks[2].height as usize;
        let thumb_row = scroll * track_h.saturating_sub(1) / max_scroll;
        let scrollbar: Vec<Line> = (0..track_h)
            .map(|i| {
                if i == thumb_row {
                    Line::from(Span::styled("▐", Style::default().fg(C_MUTED)))
                } else {
                    Line::from(Span::styled("│", Style::default().fg(C_MUTED)))
                }
            })
            .collect();
        f.render_widget(Paragraph::new(scrollbar), hchunks[2]);
    }
}

/// Build the spinner row shown while the live week is still loading.
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

fn clamp_to_ui_width(area: Rect, base_x: u16) -> Rect {
    Rect {
        x: base_x,
        width: UI_WIDTH.min(area.width),
        ..area
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

/// Split `area` into (content, total, hint) with content sized to `content_height`
/// and a blank row between content and total.
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
