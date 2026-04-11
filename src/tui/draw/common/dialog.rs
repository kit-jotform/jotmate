use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::tui::palette::{C_DANGEROUS, C_MUTED, C_TEXT};

pub(in crate::tui::draw) fn draw_confirm_dialog(f: &mut ratatui::Frame, area: Rect, msg: &str) {
    let dialog_w = (msg.len() as u16 + 4).max(26);
    let dialog_h = 5u16;

    let x = area.x + area.width.saturating_sub(dialog_w) / 2;
    let y = area.y + area.height.saturating_sub(dialog_h) / 2;
    let dialog_area = Rect::new(x, y, dialog_w, dialog_h);

    f.render_widget(Clear, dialog_area);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_DANGEROUS))
            .title(Span::styled(" Confirm ", Style::default().fg(C_DANGEROUS))),
        dialog_area,
    );

    let inner = Rect::new(
        dialog_area.x + 1,
        dialog_area.y + 1,
        dialog_area.width - 2,
        dialog_area.height - 2,
    );

    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(msg.to_string(), Style::default().fg(C_TEXT))),
            Line::raw(""),
            Line::from(vec![
                Span::styled(
                    " ↵/y ",
                    Style::default()
                        .fg(C_DANGEROUS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("delete  ", Style::default().fg(C_MUTED)),
                Span::styled(
                    " Esc/n ",
                    Style::default().fg(C_MUTED).add_modifier(Modifier::BOLD),
                ),
                Span::styled("cancel", Style::default().fg(C_MUTED)),
            ]),
        ]),
        inner,
    );
}
