use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
};

use ratatui::style::Color;

use crate::tui::app::{App, ForkStatus, RdsStatus, RepoSyncState, SyncPhase};
use crate::tui::layout::{HAlign, LayoutEngine, ScreenLayout, Widget, UI_WIDTH};
use crate::tui::palette::{
    C_ACCENT, C_DANGEROUS, C_MUTED, C_PRIMARY, C_SUCCESS, C_TEXT, C_WARN, SPINNER,
};

use super::{draw_screen_header, draw_scroll_table, hint_muted, inset_rect, HINT_RETURN_TO_MENU};

const NAME_W: usize = 14;
const FORK_W: usize = 22;
const RDS_W: usize = 22;
const ELAPSED_W: usize = 7;

const LIST_H_INSET: u16 = 2;

const LIST_VISIBLE: u16 = 6;

pub fn draw_sync_progress(f: &mut ratatui::Frame, app: &App) {
    match app.sync_state.as_ref().map(|s| s.phase.clone()) {
        Some(SyncPhase::Discovering) => draw_discovering(f, app),
        Some(SyncPhase::Failed) => draw_failed(f, app),
        _ => draw_syncing(f, app),
    }
}

fn draw_discovering(f: &mut ratatui::Frame, app: &App) {
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
        .row("hint", 1)
        .margin(1)
        .split(engine.clamp_area(area));

    draw_screen_header(
        f,
        &engine,
        rows.get("logo"),
        rows.get("title"),
        rows.get("divider"),
        "Preparing Sync",
        hint_muted(&["⌫/Esc", " cancel"]),
    );

    let tick = app
        .sync_state
        .as_ref()
        .map(|s| s.tick as usize)
        .unwrap_or(0);
    let spinner = SPINNER[tick % SPINNER.len()].to_string();
    let status_line = Line::from(vec![
        Span::styled(format!("  {spinner}  "), Style::default().fg(C_ACCENT)),
        Span::styled("Discovering repositories…", Style::default().fg(C_TEXT)),
    ]);
    f.render_widget(
        Paragraph::new(status_line),
        engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("status")),
    );

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Scanning your home directory for cloned repos (first run only)…",
            Style::default().fg(C_MUTED),
        ))),
        engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("hint")),
    );
}

fn draw_failed(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let engine = LayoutEngine::new(area);

    let msg = app
        .sync_state
        .as_ref()
        .and_then(|s| s.setup_error.clone())
        .unwrap_or_else(|| "Sync could not start.".to_string());

    let rows = ScreenLayout::new()
        .row("logo", 3)
        .row("blank1", 1)
        .row("title", 1)
        .row("divider", 1)
        .row("blank2", 1)
        .row("error", 2)
        .row("blank3", 1)
        .row("hint", 1)
        .margin(1)
        .split(engine.clamp_area(area));

    draw_screen_header(
        f,
        &engine,
        rows.get("logo"),
        rows.get("title"),
        rows.get("divider"),
        "Sync Unavailable",
        hint_muted(&["↵", " back"]),
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ✗  ", Style::default().fg(C_DANGEROUS)),
            Span::styled(msg, Style::default().fg(C_TEXT)),
        ]))
        .wrap(ratatui::widgets::Wrap { trim: false }),
        engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("error")),
    );

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            HINT_RETURN_TO_MENU,
            Style::default().fg(C_MUTED),
        ))),
        engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("hint")),
    );
}

fn draw_syncing(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();
    let engine = LayoutEngine::new(area);

    let (error_count, repo_count) = app
        .sync_state
        .as_ref()
        .map(|s| {
            (
                s.repos.iter().filter(|r| r.has_error()).count() as u16,
                s.repos.len() as u16,
            )
        })
        .unwrap_or((0, 0));

    let is_scrollable = repo_count > LIST_VISIBLE;

    let visible_rows = repo_count.min(LIST_VISIBLE);
    let list_height = 2 + visible_rows;

    let mut layout = ScreenLayout::new()
        .row("logo", 3)
        .row("blank1", 1)
        .row("title", 1)
        .row("divider", 1)
        .row("blank2", 1)
        .row("summary", 1)
        .row("blank3", 1)
        .row("list", list_height)
        .row("blank4", 2);
    if is_scrollable {
        layout = layout.row("nav_hint", 1).row("blank5", 1);
    }
    let rows = layout
        .row("hint", 1)
        .row("errors", error_count)
        .margin(1)
        .split(engine.clamp_area(area));

    let is_complete = app.sync_is_complete();
    let title = if is_complete {
        "Sync Complete"
    } else {
        "Syncing Repos"
    };
    draw_screen_header(
        f,
        &engine,
        rows.get("logo"),
        rows.get("title"),
        rows.get("divider"),
        title,
        hint_muted(&["⌫/Esc", " cancel"]),
    );

    // ── Summary line ──
    if let Some(state) = &app.sync_state {
        let total = state.repos.len();
        let complete = state.repos.iter().filter(|r| r.is_complete()).count();
        let errors = state.repos.iter().filter(|r| r.has_error()).count();
        let skipped = state.repos.iter().filter(|r| r.is_skipped()).count();

        let mut parts: Vec<Span> = vec![
            Span::styled(format!("{complete}/{total}"), Style::default().fg(C_ACCENT)),
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
        parts.push(Span::styled("  •  ", Style::default().fg(C_MUTED)));
        parts.push(Span::styled(
            format!("{skipped}"),
            Style::default().fg(C_WARN),
        ));
        parts.push(Span::styled(" skipped", Style::default().fg(C_TEXT)));

        let summary_area = inset_rect(
            engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("summary")),
            LIST_H_INSET + 1,
        );
        f.render_widget(Paragraph::new(Line::from(parts)), summary_area);

        // ── Repo list ──
        let tick = state.tick as usize;
        let data_lines: Vec<Line> = state
            .repos
            .iter()
            .map(|repo| repo_row(repo, tick))
            .collect();
        let list_area = inset_rect(
            engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("list")),
            LIST_H_INSET,
        );
        draw_scroll_table(f, list_area, build_header(), data_lines, app.sync_scroll);

        // ── Nav hint (only when list is scrollable) ──
        if is_scrollable {
            f.render_widget(
                Paragraph::new(Line::from(hint_muted(&["↑↓", " scroll"]))),
                engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("nav_hint")),
            );
        }

        // ── Hint ──
        let hint_text = if is_complete {
            HINT_RETURN_TO_MENU
        } else {
            "Syncing..."
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                hint_text,
                Style::default().fg(C_MUTED),
            ))),
            engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("hint")),
        );

        // ── Error details ──
        if error_count > 0 {
            let error_items: Vec<ListItem> = state
                .repos
                .iter()
                .filter(|r| r.has_error())
                .map(|r| {
                    let msg = error_message(r);
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("  {} ", r.name), Style::default().fg(C_DANGEROUS)),
                        Span::styled(msg, Style::default().fg(C_MUTED)),
                    ]))
                })
                .collect();
            f.render_widget(
                List::new(error_items),
                engine.place(&Widget::anon(UI_WIDTH, HAlign::Left), rows.get("errors")),
            );
        }
    }
}

fn build_header() -> Line<'static> {
    let header_style = Style::default().fg(C_MUTED).add_modifier(Modifier::BOLD);
    let cell = |text: &str, width: usize| {
        Span::styled(format!("{:<width$}", text, width = width), header_style)
    };
    Line::from(vec![
        Span::styled("   ", Style::default()),
        cell("Repo Name", NAME_W),
        cell("Fork Sync", FORK_W),
        cell("RDS Sync", RDS_W),
        Span::styled(
            format!("{:>width$}", "Elapsed", width = ELAPSED_W),
            header_style,
        ),
    ])
}

fn repo_row(repo: &RepoSyncState, tick: usize) -> Line<'static> {
    let (icon, icon_style) = if repo.has_error() {
        ("✗".to_string(), Style::default().fg(C_DANGEROUS))
    } else if repo.is_complete() {
        if repo.is_skipped() {
            ("-".to_string(), Style::default().fg(C_WARN))
        } else {
            ("✓".to_string(), Style::default().fg(C_SUCCESS))
        }
    } else {
        let ch = SPINNER[tick % SPINNER.len()];
        (ch.to_string(), Style::default().fg(C_ACCENT))
    };

    // Name color precedence matches the pre-refactor render:
    //   - in-flight (no error yet) → bold C_PRIMARY alongside spinner
    //   - in-flight but errored on fork/rds → plain C_PRIMARY (still active)
    //   - terminal error → C_DANGEROUS
    //   - terminal skipped → C_MUTED
    //   - terminal success → C_TEXT
    let name_style = if repo.is_active() && !repo.has_error() {
        Style::default().fg(C_PRIMARY).add_modifier(Modifier::BOLD)
    } else if repo.is_active() {
        Style::default().fg(C_PRIMARY)
    } else if repo.has_error() {
        Style::default().fg(C_DANGEROUS)
    } else if repo.is_skipped() {
        Style::default().fg(C_MUTED)
    } else {
        Style::default().fg(C_TEXT)
    };

    let name_padded = format!("{:<width$}", repo.name, width = NAME_W);
    let fork_padded = format!("{:<width$}", repo.fork_status.label(), width = FORK_W);
    let rds_padded = format!("{:<width$}", repo.rds_status.label(), width = RDS_W);
    let elapsed = format!(
        "{:>width$}",
        format!("{:.1}s", repo.elapsed_secs),
        width = ELAPSED_W
    );

    let fork_color = status_color(&repo.fork_status);
    let rds_color = rds_status_color(&repo.rds_status);

    Line::from(vec![
        Span::styled(format!(" {icon} "), icon_style),
        Span::styled(name_padded, name_style),
        Span::styled(fork_padded, Style::default().fg(fork_color)),
        Span::styled(rds_padded, Style::default().fg(rds_color)),
        Span::styled(elapsed, Style::default().fg(C_MUTED)),
    ])
}

fn status_color(status: &ForkStatus) -> Color {
    match status {
        ForkStatus::Pending => C_MUTED,
        ForkStatus::Done | ForkStatus::UpToDate => C_SUCCESS,
        ForkStatus::Skipped(_) => C_WARN,
        ForkStatus::Error(_) => C_DANGEROUS,
        ForkStatus::Stashing
        | ForkStatus::Merging
        | ForkStatus::Rebasing
        | ForkStatus::Unstashing => C_WARN,
        _ => C_ACCENT,
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

fn error_message(repo: &RepoSyncState) -> String {
    if let ForkStatus::Error(msg) = &repo.fork_status {
        return msg.clone();
    }
    match &repo.rds_status {
        RdsStatus::Error(msg) => msg.clone(),
        _ => String::new(),
    }
}
