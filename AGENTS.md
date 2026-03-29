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

## Project folder structure

```
jotmate/
├── .github/
│   └── workflows/
│       └── release.yml          # CI: builds macOS/Linux binaries on v* tag push
├── assets/
│   ├── icon.txt                 # Source art for the pixel icon in the TUI header
│   └── logos.txt                # Source art for LOGO / LOGO_SMALL constants
├── scripts/
│   ├── run-sync.sh              # Sync script — embedded in the binary via include_str!()
│   └── time-checker-node.js     # Original Node.js time checker (reference only, not used)
├── src/
│   ├── main.rs                  # Entry point — parses CLI args, dispatches to tui/sync/time
│   ├── cli.rs                   # Clap structs: Cli, Commands, SyncArgs, TimeArgs
│   ├── config.rs                # Config structs, load/save, ensure_time_credentials prompt
│   ├── error.rs                 # AppError enum (thiserror) — IO, HTTP, auth, keyring, fd
│   ├── sync/
│   │   ├── mod.rs               # run() entry: resolves repo paths, calls runner
│   │   ├── cache.rs             # RepoPathsCache — load/save/invalidate ~/.cache/jotmate/repo_paths.json
│   │   ├── discover.rs          # fd-based git repo discovery; matches repos to upstream URLs
│   │   └── runner.rs            # Patches GITHUB_BASE in embedded script, writes tempfile, execs bash
│   ├── time/
│   │   ├── mod.rs               # run() entry: auth, batch-fetches weeks, computes, displays
│   │   ├── auth.rs              # Keychain read/write for TimeDoctor session cookie; browser login flow
│   │   ├── api.rs               # HTTP client: fetches weekly stats from TimeDoctor API
│   │   ├── cache.rs             # Per-week JSON cache at ~/.cache/jotmate/time/<company_id>/YYYY-MM-DD.json
│   │   ├── compute.rs           # WeekRow, weeks_to_fetch, cumulative balance, target hours logic
│   │   └── display.rs           # ANSI terminal table renderer for WeekRow results
│   └── tui/
│       ├── mod.rs               # Terminal setup/teardown, async event loop, run_interactive/run_settings
│       ├── app.rs               # App state, Screen/InputMode/SettingRow/RepoManagerRow enums, MAIN_ITEMS
│       ├── draw.rs              # Ratatui frame rendering for all three screens + confirm dialog overlay
│       ├── input.rs             # Keyboard event handlers → Action enum (Continue/Back/Run)
│       ├── layout.rs            # ScreenLayout (named rows), LayoutEngine (x placement), UI_WIDTH
│       └── widgets.rs           # IconWidget (pixel art), LOGO, LOGO_SMALL constants
├── Cargo.toml                   # Package manifest and dependencies
├── Cargo.lock                   # Locked dependency versions
├── install.sh                   # Curl-based installer for end users
├── AGENTS.md                    # This file — guidance for AI-assisted development
└── README.md                    # User-facing documentation
```

## Adding a new tool or settings field

**New tool**: Add a variant to the `Screen` enum and/or menu in `app.rs`, handle it in `input.rs` and `draw.rs`, and add a Rust subcommand in `src/cli.rs` + `src/main.rs` if data access is needed.

**New settings field**: Add to the struct in `src/config.rs` with `#[serde(default)]`, add a row in the settings render in `draw.rs`, and handle the toggle in `input.rs` and `app.rs`.

## Coding style

Prefer changes that stay small, coherent, and easy to reason about. When editing or adding code, aim for:

- **Single responsibility** — Each module, type, and function should have one clear job. Split mixed concerns (e.g. parsing vs. I/O vs. UI) instead of growing god-objects or all-in-one handlers. If a function does two unrelated things, extract one of them.

- **Single source of truth** — Define each piece of behavior or data in one place. Config and persistent state should flow from `config` (and related types), not from parallel ad hoc structs or duplicated defaults. Menu labels, keybindings, and domain rules should not diverge across files; centralize constants and enums where they are owned.

- **DRY (don’t repeat yourself)** — Before copying a block, extract a shared helper, type, or constant. Repeated string literals, match arms, or validation logic are signals to consolidate. Duplication across `draw.rs` / `input.rs` / `app.rs` for the same feature usually means one definition should drive the rest.

These principles reinforce each other: one responsibility per unit, one canonical definition per concept, and no unnecessary repetition.

## Key constraints

- The `ansi-to-tui` crate is incompatible with ratatui 0.29 (requires <0.27) — don't add it.
- `scripts/run-sync.sh` is the only script still in active use — it is embedded in the binary.
