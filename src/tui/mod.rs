use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // logo (6 lines)
            Constraint::Length(1), // tagline
            Constraint::Length(1), // spacer
            Constraint::Length(1), // divider
            Constraint::Length(1), // hint
            Constraint::Min(0),    // menu list
        ])
        .margin(1)
        .split(area);

    // ── Logo ──
    let logo_lines: Vec<Line> = LOGO
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(C_TEXT).add_modifier(Modifier::BOLD))))
        .collect();
    f.render_widget(Paragraph::new(logo_lines), chunks[0]);

    // ── Tagline ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "The lazy engineer's Swiss Army knife",
            Style::default().fg(C_MUTED).add_modifier(Modifier::ITALIC),
        ))),
        chunks[1],
    );

    // ── Divider ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─────────────────────────────────────────────────",
            Style::default().fg(C_MUTED),
        ))),
        chunks[3],
    );

    // ── Hint ──
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "↑↓/jk navigate  ·  Enter select  ·  Esc exit",
            Style::default().fg(C_MUTED),
        ))),
        chunks[4],
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

    f.render_stateful_widget(List::new(items), chunks[5], &mut app.main_state.clone());
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
