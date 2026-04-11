pub(crate) mod app;
mod draw;
mod draw_main_menu;
mod draw_repos;
mod draw_settings;
mod draw_time;
mod input;
mod layout;
mod palette;
mod sync_screen;
mod widgets;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::time::Duration;
use tokio::sync::mpsc;

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
        let rows = app.settings_items();
        let first = rows.iter().position(|r| r.is_interactive()).unwrap_or(0);
        app.settings_state.select(Some(first));
    }

    loop {
        let result = event_loop(&mut terminal, &mut app).await;
        teardown_terminal(&mut terminal);

        // If the user selected Time from the main menu, run it now
        // (after restoring the terminal so its output is visible)
        match result {
            Ok(Some(action)) if action == "time" => {
                // Pre-flight: redirect to credentials screen if not configured
                if app.td_email.is_empty() || !app.td_password_is_set {
                    terminal = setup_terminal()?;
                    app.screen = Screen::TimeDoctorSettings;
                    let td_rows = app.td_settings_items();
                    let first = td_rows.iter().position(|r| r.is_interactive()).unwrap_or(0);
                    app.td_settings_state.select(Some(first));
                    app.auth_error = Some("Email or password not configured".to_string());
                    continue;
                }

                match crate::time::run(Default::default()).await {
                    Ok(()) => break,
                    Err(e) => {
                        // On auth failure, re-enter the TUI on credentials screen
                        let is_auth_err = e
                            .downcast_ref::<crate::error::AppError>()
                            .map(|ae| {
                                matches!(
                                    ae,
                                    crate::error::AppError::AuthFailed(_)
                                        | crate::error::AppError::TokenExpired
                                )
                            })
                            .unwrap_or(false);

                        if is_auth_err {
                            // Reload password state (user may have changed it)
                            app.td_password_is_set =
                                crate::time::auth::load_password_from_keychain().is_some();
                            let msg = e.to_string();
                            terminal = setup_terminal()?;
                            app.screen = Screen::TimeDoctorSettings;
                            let td_rows = app.td_settings_items();
                            let first =
                                td_rows.iter().position(|r| r.is_interactive()).unwrap_or(0);
                            app.td_settings_state.select(Some(first));
                            app.auth_error = Some(msg);
                            continue;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
            _ => break,
        }
    }

    Ok(())
}

// ── Sync launcher ────────────────────────────────────────────────────────────

fn launch_sync(app: &mut App) {
    let config = match crate::config::load() {
        Ok(c) => c,
        Err(_) => return,
    };

    let upstream_repos = &config.sync.upstream_repos;
    let enabled: Vec<_> = upstream_repos.iter().filter(|r| r.enabled).collect();
    if enabled.is_empty() {
        return;
    }

    // Resolve repo paths (blocking, but fast if cached)
    let paths = {
        let enabled_names: Vec<&str> = enabled.iter().map(|r| r.name.as_str()).collect();
        if config.sync.use_cache {
            if let Some(cached) = crate::sync::cache::load() {
                if crate::sync::cache::is_valid(&cached, &enabled_names) {
                    cached.paths
                } else {
                    // Can't discover inside TUI (fd is slow + prints) — use cache or bail
                    return;
                }
            } else {
                return;
            }
        } else {
            return;
        }
    };

    let (tx, rx) = mpsc::unbounded_channel();
    app.start_sync(paths.clone(), rx);

    // Build repo list with indices matching sync_state.repos order
    let repo_list: Vec<(usize, std::path::PathBuf)> = app
        .sync_state
        .as_ref()
        .unwrap()
        .repos
        .iter()
        .enumerate()
        .map(|(idx, r)| (idx, r.path.clone()))
        .collect();

    let opts = crate::sync::native::SyncOpts {
        skip_fork_sync: app.skip_fork_sync,
        skip_git_fetch: app.skip_git_fetch,
        skip_rebase: app.skip_rebase,
        skip_rds_sync: app.skip_rds_sync,
        skip_dirty_sync: app.skip_dirty_sync,
        force_sync_all: app.sync_all,
    };

    let handle = tokio::spawn(async move {
        crate::sync::native::run_tui(repo_list, tx, opts).await;
    });

    if let Some(state) = &mut app.sync_state {
        state.sync_handle = Some(handle);
    }
}

// ── Event loop ───────────────────────────────────────────────────────────────

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<Option<String>> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        // When on sync screen, use poll-based loop for animation + channel drain
        if app.screen == Screen::SyncProgress {
            // Drain pending sync updates
            {
                let mut updates = Vec::new();
                if let Some(state) = &mut app.sync_state {
                    while let Ok(update) = state.update_rx.try_recv() {
                        updates.push(update);
                    }
                }
                for update in updates {
                    app.apply_sync_update(update);
                }
            }

            // Poll for keyboard events with timeout (drives spinner animation)
            if event::poll(Duration::from_millis(80))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        // Clean up sync state
                        if let Some(state) = app.sync_state.take() {
                            if let Some(handle) = state.sync_handle {
                                handle.abort();
                            }
                        }
                        return Ok(None);
                    }
                    match handle_key(app, key.code) {
                        Action::Back => return Ok(None),
                        Action::Run(cmd) => return Ok(Some(cmd)),
                        Action::StartSync => {}
                        Action::Continue => {}
                    }
                }
            } else {
                // No event — tick spinner
                if let Some(state) = &mut app.sync_state {
                    state.tick = state.tick.wrapping_add(1);
                }
            }
        } else {
            // Normal blocking event read for other screens
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(None);
                }
                match handle_key(app, key.code) {
                    Action::Back => return Ok(None),
                    Action::Run(cmd) => return Ok(Some(cmd)),
                    Action::StartSync => {
                        launch_sync(app);
                    }
                    Action::Continue => {}
                }
            }
        }
    }
}
