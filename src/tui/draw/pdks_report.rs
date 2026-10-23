use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::config::PDKS_DAILY_TARGET_HOURS;
use crate::time::compute::{format_hours, format_hours_signed};
use crate::tui::app::{App, PdksReportState};
use crate::tui::palette::{balance_color, C_ACCENT, C_DANGEROUS, C_MUTED, C_TEXT, SPINNER};

use super::{
    draw_screen_header, hint_cancel, hint_muted, inset_rect, sub_screen_setup, HINT_RETURN_TO_MENU,
};

pub fn draw_pdks_report(f: &mut ratatui::Frame, app: &App) {
    let (_, engine, layout) = sub_screen_setup(f);
    let hint = match app.pdks_report {
        PdksReportState::Loading { .. } => hint_cancel(),
        _ if app.pdks_target_editing => hint_muted(&["←/→", " adjust target", "  ↵", " done"]),
        _ => hint_muted(&["←/→", " week", "  ↵", " target", "  Esc", " return"]),
    };
    draw_screen_header(
        f,
        &engine,
        layout.get("logo"),
        layout.get("title"),
        layout.get("divider"),
        "Ar-Ge Portal",
        hint,
    );
    let area = inset_rect(layout.get("list"), 3);
    let lines = match &app.pdks_report {
        PdksReportState::Loading { tick } => vec![Line::from(vec![
            Span::styled(
                format!("{} ", SPINNER[*tick as usize % SPINNER.len()]),
                Style::default().fg(C_ACCENT),
            ),
            Span::styled("Fetching PDKS report…", Style::default().fg(C_MUTED)),
        ])],
        PdksReportState::Error(error) => vec![
            Line::from(Span::styled(
                format!("✗ {error}"),
                Style::default().fg(C_DANGEROUS),
            )),
            Line::from(Span::styled(
                "Press Enter to return to the menu.",
                Style::default().fg(C_MUTED),
            )),
        ],
        PdksReportState::Ready(report) => {
            let end = report.week + chrono::Duration::days(6);
            let mut lines = vec![
                Line::from(Span::styled(
                    format!(
                        "{} – {}",
                        report.week.format("%d.%m.%Y"),
                        end.format("%d.%m.%Y")
                    ),
                    Style::default().fg(C_TEXT).add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![
                    Span::styled("Target: ", Style::default().fg(C_MUTED)),
                    Span::styled(
                        format_hours(report.target_hours),
                        Style::default().fg(if app.pdks_target_editing {
                            C_ACCENT
                        } else {
                            C_TEXT
                        }),
                    ),
                    Span::styled(
                        if app.pdks_target_editing {
                            "  (←/→ changes by 9h)"
                        } else {
                            "  (press Enter to change)"
                        },
                        Style::default().fg(C_MUTED),
                    ),
                ]),
                Line::default(),
            ];
            for day in &report.days {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{:<12}", day.date.format("%a %d.%m")),
                        Style::default().fg(C_TEXT),
                    ),
                    Span::styled(
                        if day.is_still_in_office {
                            format!("{} – now", day.first.format("%H:%M"))
                        } else {
                            format!(
                                "{} – {}",
                                day.first.format("%H:%M"),
                                day.last.format("%H:%M")
                            )
                        },
                        Style::default().fg(C_TEXT),
                    ),
                    Span::styled(
                        format!("  {}", format_hours(day.worked_secs as f64 / 3600.0)),
                        Style::default().fg(C_ACCENT),
                    ),
                    Span::styled(
                        format!(
                            "  ({})",
                            format_hours_signed(
                                day.worked_secs as f64 / 3600.0 - PDKS_DAILY_TARGET_HOURS
                            )
                        ),
                        Style::default().fg(balance_color(
                            day.worked_secs as f64 / 3600.0 - PDKS_DAILY_TARGET_HOURS,
                        )),
                    ),
                    Span::styled(
                        if day.is_still_in_office {
                            "  in office"
                        } else {
                            ""
                        },
                        Style::default().fg(C_MUTED),
                    ),
                ]));
            }
            let worked = report.worked_secs as f64 / 3600.0;
            let balance = worked - report.target_hours;
            lines.extend([
                Line::default(),
                Line::from(vec![
                    Span::styled("Total: ", Style::default().fg(C_MUTED)),
                    Span::styled(format_hours(worked), Style::default().fg(C_TEXT)),
                    Span::styled(
                        format!(" / {}  ", format_hours(report.target_hours)),
                        Style::default().fg(C_MUTED),
                    ),
                    Span::styled(
                        format_hours_signed(balance),
                        Style::default().fg(balance_color(balance)),
                    ),
                ]),
                Line::default(),
                Line::from(Span::styled(
                    if app.pdks_target_editing {
                        "Use ←/→ to change this week's target in 9-hour steps."
                    } else {
                        HINT_RETURN_TO_MENU
                    },
                    Style::default().fg(C_MUTED),
                )),
            ]);
            lines
        }
    };
    f.render_widget(Paragraph::new(lines), area);
}
