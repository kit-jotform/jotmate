use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

use ratatui::style::Color;

use crate::tui::app::{App, ForkStatus, RdsStatus, RepoSyncState};
use crate::tui::layout::{HAlign, LayoutEngine, ScreenLayout, Widget, UI_WIDTH};
use crate::tui::palette::{C_ACCENT, C_DANGEROUS, C_MUTED, C_PRIMARY, C_SUCCESS, C_TEXT, C_WARN};

use super::{draw_screen_header, hint_muted};

// ── Spinner frames ──────────────────────────────────────────────────────────

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];

// ── Column widths ───────────────────────────────────────────────────────────

const NAME_W: usize = 14;
const FORK_W: usize = 24;
const RDS_W: usize = 18;

pub fn draw_sync_progress(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    let rows = ScreenLayout::new()
        .row("logo", 3)
        .row("blank1", 1)
        .row("title", 1)
        .row("divider", 1)
        .row("blank2", 1)
        .row("summary", 1)
        .row("blank3", 1)
        .row("list", 0)
        .row("blank4", 1)
        .row("hint", 1)
        .margin(1)
        .split(area);

    let engine = LayoutEngine::new(area.x);

    let is_complete = app.sync_is_complete();
    let (title, hint_spans) = if is_complete {
        ("Sync Complete", hint_muted(&["↵/Esc", " dismiss"]))
    } else {
        ("Syncing Repos", hint_muted(&["q/Esc", " cancel"]))
    };
    draw_screen_header(
        f,
        &engine,
        rows.get("logo"),
        rows.get("title"),
        rows.get("divider"),
        title,
        hint_spans,
    );

    // ── Summary line ──
    if let Some(state) = &app.sync_state {
        let total = state.repos.len();
        let complete = state.repos.iter().filter(|r| r.is_complete()).count();
        let errors = state.repos.iter().filter(|r| r.has_error()).count();
        let skipped = state.repos.iter().filter(|r| r.is_skipped()).count();

        let mut parts: Vec<Span> = vec![
            Span::styled(
                format!("{complete}/{total}"),
                Style::default().fg(if complete == total {
                    C_SUCCESS
                } else {
                    C_ACCENT
                }),
            ),
            Span::styled(" complete", Style::default().fg(C_TEXT)),
        ];
        if errors > 0 {
            parts.push(Span::styled("  •  ", Style::default().fg(C_MUTED)));
            parts.push(Span::styled(
                format!("{errors}"),
                Style::default().fg(C_DANGEROUS),
            ));
            parts.push(Span::styled(" error", Style::default().fg(C_TEXT)));
        }
        if skipped > 0 {
            parts.push(Span::styled("  •  ", Style::default().fg(C_MUTED)));
            parts.push(Span::styled(
                format!("{skipped}"),
                Style::default().fg(C_WARN),
            ));
            parts.push(Span::styled(" skipped", Style::default().fg(C_TEXT)));
        }

        f.render_widget(
            Paragraph::new(Line::from(parts)),
            engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("summary")),
        );

        // ── Repo list ──
        let tick = state.tick as usize;
        let items: Vec<ListItem> = state
            .repos
            .iter()
            .map(|repo| {
                let (icon, icon_color) = if repo.has_error() {
                    ("✗", C_DANGEROUS)
                } else if repo.is_complete() {
                    if repo.is_skipped() {
                        ("-", C_WARN)
                    } else {
                        ("✓", C_SUCCESS)
                    }
                } else {
                    let ch = SPINNER[tick % SPINNER.len()];
                    // Return spinner as &str workaround
                    return repo_line_spinner(ch, repo);
                };

                let name_color = if repo.is_active() {
                    C_PRIMARY
                } else if repo.has_error() {
                    C_DANGEROUS
                } else if repo.is_skipped() {
                    C_MUTED
                } else {
                    C_TEXT
                };

                let fork_color = status_color(&repo.fork_status);
                let rds_color = rds_status_color(&repo.rds_status);

                let name_padded = format!("{:<width$}", repo.name, width = NAME_W);
                let fork_padded = format!("{:<width$}", repo.fork_status.label(), width = FORK_W);
                let rds_padded = format!("{:<width$}", repo.rds_status.label(), width = RDS_W);
                let elapsed = format!("{:>5.1}s", repo.elapsed_secs);

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{icon} "), Style::default().fg(icon_color)),
                    Span::styled(name_padded, Style::default().fg(name_color)),
                    Span::styled(fork_padded, Style::default().fg(fork_color)),
                    Span::styled(rds_padded, Style::default().fg(rds_color)),
                    Span::styled(elapsed, Style::default().fg(C_MUTED)),
                ]))
            })
            .collect();

        f.render_widget(
            List::new(items),
            engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("list")),
        );
    }

    // ── Bottom hint ──
    let hint_text = if is_complete {
        "Press Enter or Esc to return to the main menu"
    } else {
        "Sync is running…"
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hint_text,
            Style::default().fg(C_MUTED),
        ))),
        rows.get("hint"),
    );
}

fn repo_line_spinner(ch: char, repo: &RepoSyncState) -> ListItem<'static> {
    let name_padded = format!("{:<width$}", repo.name, width = NAME_W);
    let fork_padded = format!("{:<width$}", repo.fork_status.label(), width = FORK_W);
    let rds_padded = format!("{:<width$}", repo.rds_status.label(), width = RDS_W);
    let elapsed = format!("{:>5.1}s", repo.elapsed_secs);

    let fork_color = status_color(&repo.fork_status);
    let rds_color = rds_status_color(&repo.rds_status);

    ListItem::new(Line::from(vec![
        Span::styled(format!("{ch} "), Style::default().fg(C_ACCENT)),
        Span::styled(
            name_padded,
            Style::default().fg(C_PRIMARY).add_modifier(Modifier::BOLD),
        ),
        Span::styled(fork_padded, Style::default().fg(fork_color)),
        Span::styled(rds_padded, Style::default().fg(rds_color)),
        Span::styled(elapsed, Style::default().fg(C_MUTED)),
    ]))
}

fn status_color(status: &ForkStatus) -> Color {
    match status {
        ForkStatus::Pending => C_MUTED,
        ForkStatus::Done => C_SUCCESS,
        ForkStatus::UpToDate => C_SUCCESS,
        ForkStatus::Skipped(_) => C_WARN,
        ForkStatus::Error(_) => C_DANGEROUS,
        _ => C_ACCENT, // active states
    }
}

fn rds_status_color(status: &RdsStatus) -> Color {
    match status {
        RdsStatus::Pending => C_MUTED,
        RdsStatus::Done => C_SUCCESS,
        RdsStatus::Skipped(_) => C_WARN,
        RdsStatus::Error(_) => C_DANGEROUS,
        _ => C_ACCENT,
    }
}
