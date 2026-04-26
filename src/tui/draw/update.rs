use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::app::App;
use crate::tui::layout::{HAlign, LayoutEngine, ScreenLayout, Widget, UI_WIDTH};
use crate::tui::palette::{C_ACCENT, C_DANGEROUS, C_MUTED, C_SUCCESS, C_TEXT, SPINNER};
use crate::update::{api::current_version, UpdatePhase};

use super::{draw_screen_header, hint_muted, HINT_RETURN_TO_MENU};

pub fn draw_update_progress(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let engine = LayoutEngine::new(area);

    let rows = ScreenLayout::new()
        .row("logo", 3)
        .row("blank1", 1)
        .row("title", 1)
        .row("divider", 1)
        .row("blank2", 1)
        .row("status", 1)
        .row("blank3", 1)
        .row("detail", 2)
        .row("blank4", 1)
        .row("hint", 1)
        .margin(1)
        .split(engine.clamp_area(area));

    let phase = app
        .update_state
        .as_ref()
        .map(|s| s.phase.clone())
        .unwrap_or(UpdatePhase::Checking);
    let tick = app
        .update_state
        .as_ref()
        .map(|s| s.tick as usize)
        .unwrap_or(0);

    let (title, hint) = title_and_hint(&phase);
    draw_screen_header(
        f,
        &engine,
        rows.get("logo"),
        rows.get("title"),
        rows.get("divider"),
        title,
        hint,
    );

    let status_line = Line::from(vec![
        Span::raw("  "),
        status_icon(&phase, tick),
        Span::raw("  "),
        Span::styled(status_label(&phase), Style::default().fg(C_TEXT)),
    ]);
    f.render_widget(
        Paragraph::new(status_line),
        engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("status")),
    );

    f.render_widget(
        Paragraph::new(detail_line(&phase)).wrap(ratatui::widgets::Wrap { trim: false }),
        engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("detail")),
    );

    let hint_text = match &phase {
        UpdatePhase::Done(_) => "Press Enter to relaunch jotmate",
        p if p.is_terminal() => HINT_RETURN_TO_MENU,
        _ => "Updating…",
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hint_text,
            Style::default().fg(C_MUTED),
        ))),
        engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("hint")),
    );
}

fn title_and_hint(phase: &UpdatePhase) -> (&'static str, Vec<Span<'static>>) {
    match phase {
        UpdatePhase::Done(_) => ("Update Complete", hint_muted(&["↵", " relaunch"])),
        UpdatePhase::UpToDate => ("No Update Needed", hint_muted(&["↵", " back"])),
        UpdatePhase::Failed(_) => ("Update Failed", hint_muted(&["↵", " back"])),
        _ => ("Updating JotMate", hint_muted(&["⌫/Esc", " cancel"])),
    }
}

fn status_icon(phase: &UpdatePhase, tick: usize) -> Span<'static> {
    match phase {
        UpdatePhase::Done(_) | UpdatePhase::UpToDate => {
            Span::styled("✓".to_string(), Style::default().fg(C_SUCCESS))
        }
        UpdatePhase::Failed(_) => Span::styled("✗".to_string(), Style::default().fg(C_DANGEROUS)),
        _ => Span::styled(
            SPINNER[tick % SPINNER.len()].to_string(),
            Style::default().fg(C_ACCENT),
        ),
    }
}

fn status_label(phase: &UpdatePhase) -> String {
    match phase {
        UpdatePhase::Checking => "Checking for updates…".into(),
        UpdatePhase::Downloading => "Downloading new release…".into(),
        UpdatePhase::Extracting => "Extracting binary…".into(),
        UpdatePhase::Replacing => "Installing…".into(),
        UpdatePhase::Done(version) => format!("Updated to v{version}"),
        UpdatePhase::UpToDate => "Already up to date".into(),
        UpdatePhase::Failed(_) => "Update failed".into(),
    }
}

fn detail_line(phase: &UpdatePhase) -> Line<'static> {
    match phase {
        UpdatePhase::Done(version) => Line::from(Span::styled(
            format!("Updated to v{version}. Press Enter to relaunch."),
            Style::default().fg(C_MUTED),
        )),
        UpdatePhase::UpToDate => Line::from(Span::styled(
            format!("You are already on v{}.", current_version()),
            Style::default().fg(C_MUTED),
        )),
        UpdatePhase::Failed(msg) => {
            Line::from(Span::styled(msg.clone(), Style::default().fg(C_DANGEROUS)))
        }
        _ => Line::from(Span::styled(
            "Fetching the latest release from GitHub…",
            Style::default().fg(C_MUTED),
        )),
    }
}
