use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, Widget},
    Terminal,
};
use std::io::stdout;

// ── Palette ──────────────────────────────────────────────────────────────────

const C_PRIMARY: Color = Color::Rgb(124, 58, 237);  // purple
const C_ACCENT: Color = Color::Rgb(244, 114, 182);  // pink
const C_SUCCESS: Color = Color::Rgb(16, 185, 129);  // green
const C_MUTED: Color = Color::Rgb(107, 114, 128);   // gray
const C_TEXT: Color = Color::Rgb(229, 231, 235);    // near-white

// ── Logo ─────────────────────────────────────────────────────────────────────

const LOGO: [&str; 6] = [
    "     ██╗ ██████╗ ████████╗███╗   ███╗ █████╗ ████████╗███████╗",
    "     ██║██╔═══██╗╚══██╔══╝████╗ ████║██╔══██╗╚══██╔══╝██╔════╝",
    "     ██║██║   ██║   ██║   ██╔████╔██║███████║   ██║   █████╗  ",
    "██   ██║██║   ██║   ██║   ██║╚██╔╝██║██╔══██║   ██║   ██╔══╝  ",
    "╚█████╔╝╚██████╔╝   ██║   ██║ ╚═╝ ██║██║  ██║   ██║   ███████╗",
    " ╚════╝  ╚═════╝    ╚═╝   ╚═╝     ╚═╝╚═╝  ╚═╝   ╚═╝   ╚══════╝",
];

const LOGO_SMALL: [&str; 3] = [
    " ╦╔═╗╔╦╗╔╦╗╔═╗╔╦╗╔═╗",
    "║║ ║ ║ ║║║╠═╣ ║ ║╣ ",
    "╚╝╚═╝ ╩ ╩ ╩╩ ╩ ╩ ╚═╝",
];

// ── Pixel icon ───────────────────────────────────────────────────────────────
// Each entry: (col, row, char, fg_256, bg_256)  — None = terminal default
#[rustfmt::skip]
const ICON_CELLS: &[(u16, u16, char, Option<u8>, Option<u8>)] = &[
    // row 0
    (0,0,'▗',Some(102),None),(1,0,'▅',Some(244),None),(2,0,'▅',Some(243),None),
    (3,0,'▃',Some(244),None),(4,0,' ',Some(244),None),(5,0,' ',Some(244),None),
    (6,0,' ',Some(244),None),(7,0,' ',Some(244),None),(8,0,' ',Some(244),None),
    (9,0,'▕',Some(235),None),(10,0,'▃',Some(243),None),(11,0,'▅',None,None),
    (12,0,'▅',None,None),(13,0,'▖',Some(246),None),
    // row 1
    (0,1,'▍',Some(16),Some(244)),(1,1,'▎',Some(233),Some(173)),(2,1,'▞',Some(173),Some(167)),
    (3,1,'▆',Some(173),Some(237)),(4,1,'▃',Some(179),Some(241)),(5,1,'▔',Some(16),Some(137)),
    (6,1,'▄',Some(179),Some(243)),(7,1,'▄',Some(179),Some(243)),(8,1,'▔',Some(16),Some(102)),
    (9,1,'▃',Some(179),Some(242)),(10,1,'▆',Some(173),Some(238)),(11,1,'▃',Some(167),Some(173)),
    (12,1,'▋',Some(173),Some(233)),(13,1,'▌',Some(145),Some(233)),
    // row 2
    (0,2,'▍',Some(16),Some(244)),(1,2,'▌',Some(237),Some(173)),(2,2,'▂',Some(179),Some(173)),
    (3,2,'▄',Some(180),Some(173)),(4,2,'▌',Some(179),Some(180)),(5,2,'▊',Some(179),Some(150)),
    (6,2,'▎',Some(180),Some(179)),(7,2,'▞',Some(179),Some(150)),(8,2,'▌',Some(180),Some(215)),
    (9,2,'▘',Some(150),Some(179)),(10,2,'▁',Some(180),Some(173)),(11,2,'▃',Some(215),Some(167)),
    (12,2,'▘',Some(179),Some(238)),(13,2,'▌',Some(246),Some(238)),
    // row 3
    (0,3,'▗',Some(246),Some(236)),(1,3,'▗',Some(179),Some(239)),(2,3,'▃',Some(180),Some(215)),
    (3,3,'▗',Some(23),Some(108)),(4,3,'▔',Some(116),Some(238)),(5,3,'▂',Some(23),Some(109)),
    (6,3,'▍',Some(151),Some(215)),(7,3,'▕',Some(151),Some(215)),(8,3,'▞',Some(66),Some(109)),
    (9,3,'▋',Some(66),Some(73)),(10,3,'▎',Some(66),Some(72)),(11,3,'▏',Some(151),Some(179)),
    (12,3,'▞',Some(138),Some(236)),(13,3,'▖',Some(247),Some(233)),
    // row 4
    (0,4,'▖',Some(235),Some(244)),(1,4,'▍',Some(239),Some(137)),(2,4,'▘',Some(179),Some(173)),
    (3,4,'▔',Some(109),Some(173)),(4,4,'▆',Some(179),Some(239)),(5,4,'▆',Some(179),Some(66)),
    (6,4,'▔',Some(179),Some(173)),(7,4,'▔',Some(179),Some(173)),(8,4,'▔',Some(66),Some(179)),
    (9,4,'▆',Some(179),Some(235)),(10,4,'▔',Some(66),Some(173)),(11,4,'▕',Some(137),Some(173)),
    (12,4,'▋',Some(137),Some(237)),(13,4,'▄',Some(247),Some(59)),
    // row 5
    (0,5,'▊',Some(232),Some(145)),(1,5,'▍',Some(244),Some(237)),(2,5,'▝',Some(173),Some(239)),
    (3,5,'▃',Some(237),Some(179)),(4,5,'▂',Some(95),Some(179)),(5,5,'▁',Some(172),Some(215)),
    (6,5,'▂',Some(172),Some(215)),(7,5,'▂',Some(172),Some(215)),(8,5,'▂',Some(172),Some(215)),
    (9,5,'▂',Some(95),Some(180)),(10,5,'▃',Some(236),Some(179)),(11,5,'▘',Some(173),Some(8)),
    (12,5,'▝',Some(243),Some(237)),(13,5,'▌',Some(102),Some(234)),
    // row 6
    (0,6,'▕',Some(240),Some(233)),(1,6,'▎',Some(244),Some(59)),(2,6,'▝',Some(53),Some(60)),
    (3,6,'▘',Some(53),Some(60)),(4,6,'▔',Some(236),Some(54)),(5,6,'▄',Some(8),Some(239)),
    (6,6,'▆',Some(239),Some(130)),(7,6,'▅',Some(239),Some(130)),(8,6,'▗',Some(236),Some(240)),
    (9,6,'▔',Some(234),Some(54)),(10,6,'▕',Some(60),Some(54)),(11,6,'▘',Some(238),Some(60)),
    (12,6,'▝',Some(246),Some(237)),(13,6,'▖',Some(247),Some(235)),
];

struct IconWidget;

impl Widget for IconWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for &(col, row, ch, fg, bg) in ICON_CELLS {
            let x = area.x + col;
            let y = area.y + row;
            if x >= area.x + area.width || y >= area.y + area.height {
                continue;
            }
            let cell = &mut buf[(x, y)];
            cell.set_char(ch);
            let mut style = Style::default();
            if let Some(f) = fg { style = style.fg(Color::Indexed(f)); }
            if let Some(b) = bg { style = style.bg(Color::Indexed(b)); }
            cell.set_style(style);
        }
    }
}

// ── Screens ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    MainMenu,
    Settings,
}

// ── Main menu ─────────────────────────────────────────────────────────────────

const MAIN_ITEMS: &[&str] = &[
    "Sync        —  Sync repos to upstream",
    "Time Doctor —  Track your work hours",
    "Settings    —  Configure jotmate",
    "Exit",
];

// ── App ───────────────────────────────────────────────────────────────────────

struct App {
    screen: Screen,
    main_state: ListState,
    settings_state: ListState,
    // in-memory settings state
    sync_all: bool,
    use_cache: bool,
    repos: Vec<RepoEntry>,
}

#[derive(Clone)]
struct RepoEntry {
    name: String,
    url: String,
    enabled: bool,
}

impl App {
    fn new() -> Result<Self> {
        let config = crate::config::load()?;
        let mut main_state = ListState::default();
        main_state.select(Some(0));
        let mut settings_state = ListState::default();
        settings_state.select(Some(0));
        let repos = config
            .sync
            .upstream_repos
            .iter()
            .map(|r| RepoEntry { name: r.name.clone(), url: r.url.clone(), enabled: r.enabled })
            .collect();
        Ok(Self {
            screen: Screen::MainMenu,
            main_state,
            settings_state,
            sync_all: config.sync.sync_all_by_default,
            use_cache: config.sync.use_cache,
            repos,
        })
    }

    fn settings_items(&self) -> Vec<String> {
        let sa = if self.sync_all { "ON " } else { "OFF" };
        let uc = if self.use_cache { "ON " } else { "OFF" };
        let mut items = vec![
            format!("[{sa}]  Sync all by default  (--sync-all)"),
            format!("[{uc}]  Use repo path cache"),
            "── Upstream Repositories ───────────────────────".to_string(),
        ];
        for r in &self.repos {
            let b = if r.enabled { "ON " } else { "OFF" };
            items.push(format!("[{b}]  {}  <{}>", r.name, r.url));
        }
        items.push("  ← Back".to_string());
        items
    }

    fn settings_item_count(&self) -> usize {
        // 2 toggles + 1 separator + repos + back
        3 + self.repos.len() + 1
    }

    fn toggle_selected_setting(&mut self) {
        let idx = self.settings_state.selected().unwrap_or(0);
        match idx {
            0 => {
                self.sync_all = !self.sync_all;
                self.persist_settings();
            }
            1 => {
                self.use_cache = !self.use_cache;
                self.persist_settings();
            }
            2 => {} // separator — do nothing
            n => {
                let repo_idx = n - 3;
                if repo_idx < self.repos.len() {
                    self.repos[repo_idx].enabled = !self.repos[repo_idx].enabled;
                    self.persist_settings();
                }
                // "← Back" row (last item) is handled by the caller
            }
        }
    }

    fn persist_settings(&self) {
        if let Ok(mut config) = crate::config::load() {
            config.sync.sync_all_by_default = self.sync_all;
            config.sync.use_cache = self.use_cache;
            for repo in &mut config.sync.upstream_repos {
                if let Some(r) = self.repos.iter().find(|r| r.name == repo.name) {
                    repo.enabled = r.enabled;
                }
            }
            let _ = crate::config::save(&config);
        }
    }
}

// ── Terminal setup / teardown ─────────────────────────────────────────────────

fn setup_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    Ok(Terminal::new(backend)?)
}

fn teardown_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
}

// ── Entry points ──────────────────────────────────────────────────────────────

pub async fn run_interactive() -> Result<()> {
    run_tui(Screen::MainMenu).await
}

pub async fn run_settings() -> Result<()> {
    run_tui(Screen::Settings).await
}

async fn run_tui(initial_screen: Screen) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let mut app = App::new()?;
    app.screen = initial_screen;
    if initial_screen == Screen::Settings {
        app.settings_state.select(Some(0));
    }

    let result = event_loop(&mut terminal, &mut app).await;
    teardown_terminal(&mut terminal);

    // If the user selected Sync or Time from the main menu, run them now
    // (after restoring the terminal so their output is visible)
    if let Ok(Some(action)) = result {
        match action.as_str() {
            "sync" => crate::sync::run(Default::default()).await?,
            "time" => crate::time::run(Default::default()).await?,
            _ => {}
        }
    }

    Ok(())
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<Option<String>> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match app.screen {
                Screen::MainMenu => {
                    if let Some(action) = handle_main_key(app, key.code) {
                        return Ok(action);
                    }
                }
                Screen::Settings => {
                    handle_settings_key(app, key.code);
                }
            }
        }
    }
}

// Returns None to keep looping, Some(None) to quit, Some(Some("sync")) etc to run a tool
fn handle_main_key(app: &mut App, code: KeyCode) -> Option<Option<String>> {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            let i = app.main_state.selected().unwrap_or(0);
            app.main_state.select(Some(i.saturating_sub(1)));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let i = app.main_state.selected().unwrap_or(0);
            app.main_state.select(Some((i + 1).min(MAIN_ITEMS.len() - 1)));
        }
        KeyCode::Enter => {
            let i = app.main_state.selected().unwrap_or(0);
            match i {
                0 => return Some(Some("sync".to_string())),
                1 => return Some(Some("time".to_string())),
                2 => {
                    app.screen = Screen::Settings;
                    app.settings_state.select(Some(0));
                }
                _ => return Some(None), // Exit
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => return Some(None),
        _ => {}
    }
    None
}

fn handle_settings_key(app: &mut App, code: KeyCode) {
    let count = app.settings_item_count();
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            let i = app.settings_state.selected().unwrap_or(0);
            // skip separator (idx 2)
            let next = if i == 0 { 0 } else { i - 1 };
            let next = if next == 2 { 1 } else { next };
            app.settings_state.select(Some(next));
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let i = app.settings_state.selected().unwrap_or(0);
            let next = (i + 1).min(count - 1);
            let next = if next == 2 { 3 } else { next };
            app.settings_state.select(Some(next));
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let i = app.settings_state.selected().unwrap_or(0);
            if i == count - 1 {
                // "← Back"
                app.screen = Screen::MainMenu;
            } else {
                app.toggle_selected_setting();
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.screen = Screen::MainMenu;
        }
        _ => {}
    }
}

// ── Drawing ───────────────────────────────────────────────────────────────────

fn draw(f: &mut ratatui::Frame, app: &App) {
    match app.screen {
        Screen::MainMenu => draw_main_menu(f, app),
        Screen::Settings => draw_settings(f, app),
    }
}

fn draw_main_menu(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    // Vertical: header block | divider | hint | menu
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // icon (7 rows) / logo (6 rows) — take max
            Constraint::Length(1), // tagline
            Constraint::Length(1), // divider
            Constraint::Length(1), // hint
            Constraint::Min(0),    // menu list
        ])
        .margin(1)
        .split(area);

    // Header row: icon (14 cols + 2 gap) | logo text
    let header_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14), // icon width
            Constraint::Length(2),  // gap
            Constraint::Min(0),     // logo text
        ])
        .split(rows[0]);

    // ── Icon ──
    f.render_widget(IconWidget, header_cols[0]);

    // ── Logo text (vertically centered: logo is 6 lines, area is 7) ──
    let logo_area = Rect { y: header_cols[2].y + 1, height: 6, ..header_cols[2] };
    let logo_lines: Vec<Line> = LOGO
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(C_TEXT).add_modifier(Modifier::BOLD))))
        .collect();
    f.render_widget(Paragraph::new(logo_lines), logo_area);

    // ── Tagline ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "The lazy engineer's Swiss Army knife",
            Style::default().fg(C_MUTED).add_modifier(Modifier::ITALIC),
        ))),
        rows[1],
    );

    // ── Divider ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─────────────────────────────────────────────────",
            Style::default().fg(C_MUTED),
        ))),
        rows[2],
    );

    // ── Hint ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "↑↓/jk navigate  ·  Enter select  ·  Esc exit",
            Style::default().fg(C_MUTED),
        ))),
        rows[3],
    );

    // ── Menu list ──
    let items: Vec<ListItem> = MAIN_ITEMS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let selected = app.main_state.selected() == Some(i);
            let style = if selected {
                Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(C_TEXT)
            };
            let prefix = if selected { "▸ " } else { "  " };
            ListItem::new(Line::from(Span::styled(format!("{prefix}{label}"), style)))
        })
        .collect();

    f.render_stateful_widget(List::new(items), rows[4], &mut app.main_state.clone());
}

fn draw_settings(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // small logo
            Constraint::Length(1), // "Settings" title
            Constraint::Length(1), // divider
            Constraint::Length(1), // hint
            Constraint::Min(0),    // list
        ])
        .margin(1)
        .split(area);

    // ── Small logo ──
    let logo_lines: Vec<Line> = LOGO_SMALL
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(C_PRIMARY).add_modifier(Modifier::BOLD))))
        .collect();
    f.render_widget(Paragraph::new(logo_lines), chunks[0]);

    // ── Title ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Settings",
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
        ))),
        chunks[1],
    );

    // ── Divider ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─────────────────────────────────────────────────",
            Style::default().fg(C_MUTED),
        ))),
        chunks[2],
    );

    // ── Hint ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "↑↓/jk navigate  ·  Enter/Space toggle  ·  Esc back",
            Style::default().fg(C_MUTED),
        ))),
        chunks[3],
    );

    // ── Settings list ──
    let setting_items = app.settings_items();
    let count = setting_items.len();
    let selected = app.settings_state.selected().unwrap_or(0);

    let items: Vec<ListItem> = setting_items
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if label.starts_with("──") {
                // separator row
                return ListItem::new(Line::from(Span::styled(
                    label.clone(),
                    Style::default().fg(C_MUTED),
                )));
            }
            let is_back = i == count - 1;
            let is_sel = selected == i;

            if is_sel {
                let prefix = "▸ ";
                if label.starts_with('[') {
                    let (badge, rest) = label.split_at(5);
                    return ListItem::new(Line::from(vec![
                        Span::styled(prefix, Style::default().fg(C_PRIMARY)),
                        Span::styled(badge.to_string(), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                        Span::styled(rest.to_string(), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
                    ]));
                }
                return ListItem::new(Line::from(Span::styled(
                    format!("{prefix}{label}"),
                    Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
                )));
            }

            if is_back {
                return ListItem::new(Line::from(Span::styled(
                    format!("  {label}"),
                    Style::default().fg(C_MUTED),
                )));
            }

            // Normal toggle row — color the badge
            let on = label.starts_with("[ON");
            let badge_color = if on { C_SUCCESS } else { C_MUTED };
            let (badge, rest) = label.split_at(5);
            ListItem::new(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(badge.to_string(), Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                Span::styled(rest.to_string(), Style::default().fg(C_TEXT)),
            ]))
        })
        .collect();

    f.render_stateful_widget(List::new(items), chunks[4], &mut app.settings_state.clone());
}
