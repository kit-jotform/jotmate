use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::layout::{HAlign, LayoutEngine, Widget, UI_WIDTH};
use crate::tui::palette::{C_ACCENT, C_MUTED, C_PRIMARY};
use crate::tui::widgets::LOGO_SMALL;

pub(in crate::tui::draw) fn draw_screen_header(
    f: &mut ratatui::Frame,
    engine: &LayoutEngine,
    logo_area: Rect,
    title_area: Rect,
    divider_area: Rect,
    title: &str,
    hint_spans: Vec<Span<'static>>,
) {
    if logo_area.height > 0 {
        let logo_w = LOGO_SMALL[0].chars().count() as u16;
        let logo_lines: Vec<Line> = LOGO_SMALL
            .iter()
            .map(|l| {
                Line::from(Span::styled(
                    *l,
                    Style::default().fg(C_PRIMARY).add_modifier(Modifier::BOLD),
                ))
            })
            .collect();
        f.render_widget(Paragraph::new(logo_lines), engine.center(logo_w, logo_area));
    }

    let title_row = engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), title_area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            title,
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
        ))),
        title_row,
    );
    f.render_widget(
        Paragraph::new(Line::from(hint_spans)).right_aligned(),
        title_row,
    );

    let divider = "─".repeat(UI_WIDTH as usize);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            divider,
            Style::default().fg(C_MUTED),
        ))),
        engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), divider_area),
    );
}
