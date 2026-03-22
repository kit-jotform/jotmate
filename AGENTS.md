# AGENTS.md

This file provides guidance for AI-assisted development in this repository.

## Commands

```bash
cargo build --release        # Build optimized binary → target/release/jotmate
cargo build                  # Debug build
cargo check                  # Fast type-check without linking
cargo clippy                 # Lint
cargo fmt                    # Format
cargo run                    # Launch interactive TUI
cargo run -- sync            # Run sync subcommand
cargo run -- time            # Run time subcommand
cargo run -- settings        # Open settings screen
```

There is no test suite. Release is triggered by pushing a `v*` git tag (CI in `.github/workflows/release.yml` builds macOS arm64/x86_64 binaries and creates a GitHub Release).

## Architecture

All UI and data logic live entirely in Rust. The TUI is built with Ratatui 0.29 + Crossterm 0.28.

```
jotmate (no args)   → Ratatui interactive main menu
jotmate sync        → sync::run() — git fork sync
jotmate time        → time::run() — TimeDoctor report
jotmate settings    → Ratatui settings screen
```

### TUI (src/tui/)

Two screens managed by an `App` state struct:

| Screen | Purpose |
|--------|---------|
| **MainMenu** | Navigable list: Sync, Time Doctor, Settings, Exit |
| **Settings** | Toggle booleans and repo enabled flags |

- `mod.rs` — terminal setup/teardown, async event loop, `run_interactive()` / `run_settings()` entry points
- `app.rs` — `App` struct, `Screen` enum, in-memory state, config persistence on toggle
- `draw.rs` — frame rendering for both screens
- `input.rs` — keyboard event handlers (↑↓ navigate, Enter/Space toggle, Esc back, Q/Ctrl+C quit)
- `widgets.rs` — custom pixel-art `IconWidget`, logo constants

Selecting Sync or Time from the main menu closes the TUI, restores the terminal, then runs the subcommand so its output is visible in the foreground.

### Sync (src/sync/)

`scripts/run-sync.sh` is embedded via `include_str!()` in `sync/runner.rs` and the `GITHUB_BASE` line is patched at runtime before execution. Repos are discovered via `fd -H -t d "^\.git$" ~` matched against upstream URLs, with results cached at `~/.cache/jotmate/repo_paths.json`.

### Time tracking (src/time/)

TimeDoctor uses cookie-based auth stored in the system keychain (macOS Keychain / Linux secret-service). No plaintext fallback. Weekly data cached at `~/.cache/jotmate/time/<company_id>/YYYY-MM-DD.json`.

### Config

`~/.config/jotmate/config.toml` — sync repos (with `enabled` flags), time credentials, contract periods. Settings toggled in the TUI are saved immediately via `config::save()`.

## Adding a new tool or settings field

**New tool**: Add a variant to the `Screen` enum and/or menu in `app.rs`, handle it in `input.rs` and `draw.rs`, and add a Rust subcommand in `src/cli.rs` + `src/main.rs` if data access is needed.

**New settings field**: Add to the struct in `src/config.rs` with `#[serde(default)]`, add a row in the settings render in `draw.rs`, and handle the toggle in `input.rs` and `app.rs`.


## Key constraints

- The `ansi-to-tui` crate is incompatible with ratatui 0.29 (requires <0.27) — don't add it.
- `scripts/run-sync.sh` is the only script still in active use — it is embedded in the binary.
