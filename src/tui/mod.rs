mod app;
mod draw;
mod input;
mod layout;
mod widgets;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;

use app::{App, Screen};
use draw::draw;
use input::{handle_key, Action};

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
            // Ctrl+C always quits
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(None);
            }
            match handle_key(app, key.code) {
                Action::Back => return Ok(None),
                Action::Run(cmd) => return Ok(Some(cmd)),
                Action::Continue => {}
            }
        }
    }
}
