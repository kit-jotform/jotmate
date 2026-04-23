//! Scrollable table: 2-row header, clipped data rows, scrollbar on overflow.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::palette::C_MUTED;

pub(in crate::tui::draw) fn draw_scroll_table(
    f: &mut ratatui::Frame,
    area: Rect,
    header: Line<'static>,
    data_lines: Vec<Line>,
    scroll_pos: usize,
) {
    let data_area_height = area.height.saturating_sub(2);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(data_area_height)])
        .split(area);
    let header_area = chunks[0];
    let data_area = chunks[1];

    // 2-col right margin keeps the divider clear of the scrollbar track.
    let header_w = header_area.width.saturating_sub(2) as usize;
    let divider = Line::from(Span::styled(
        "─".repeat(header_w),
        Style::default().fg(C_MUTED),
    ));
    let header_text_area = Rect {
        width: header_area.width.saturating_sub(2),
        ..header_area
    };
    f.render_widget(Paragraph::new(vec![header, divider]), header_text_area);

    let total = data_lines.len();
    let visible = data_area.height as usize;
    let max_scroll = total.saturating_sub(visible);
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

    if total > visible {
        let track_h = hchunks[2].height as usize;
        let thumb_row = if max_scroll > 0 {
            scroll * track_h.saturating_sub(1) / max_scroll
        } else {
            0
        };
        let scrollbar: Vec<Line> = (0..track_h)
            .map(|i| {
                let ch = if i == thumb_row { "▐" } else { "│" };
                Line::from(Span::styled(ch, Style::default().fg(C_MUTED)))
            })
            .collect();
        f.render_widget(Paragraph::new(scrollbar), hchunks[2]);
    }
}
