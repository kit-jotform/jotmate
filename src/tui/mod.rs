pub mod app;
mod draw;
mod event_loop;
mod input;
mod layout;
pub(crate) mod palette;
mod rows;
mod sync_launcher;
mod sync_state;
mod widgets;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;

use app::{App, Screen};
use event_loop::event_loop;

pub(super) fn setup_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    Ok(Terminal::new(backend)?)
}

pub(super) fn teardown_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
}

pub async fn run_interactive(ctx: crate::ctx::Ctx) -> Result<()> {
    run_tui(ctx, Screen::MainMenu).await
}

pub async fn run_settings(ctx: crate::ctx::Ctx) -> Result<()> {
    run_tui(ctx, Screen::Settings).await
}

async fn run_tui(ctx: crate::ctx::Ctx, initial_screen: Screen) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let mut app = App::new(ctx)?;
    app.screen = initial_screen;
    if initial_screen == Screen::Settings {
        app.select_first_interactive(Screen::Settings);
    }

    event_loop(&mut terminal, &mut app).await?;
    teardown_terminal(&mut terminal);

    Ok(())
}
