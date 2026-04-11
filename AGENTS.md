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

Screens managed by an `App` state struct (defined in `app/screen.rs`):

| Screen | Purpose |
|--------|---------|
| **MainMenu** | Navigable list: Sync, Time Doctor, Settings, Exit |
| **Settings** | Entry point into Sync / Time Doctor settings groups |
| **SyncGeneralSettings** / **TdGeneralSettings** | Shared toggle-list screen driven by `draw_general_toggles` |
| **RepoManager** / **RemoveRepos** | Add / toggle / remove upstream repo URLs |
| **TimeDoctorSettings** / **ContractPeriods** | TimeDoctor credentials, timezone, contract periods |
| **TimeDoctorReport** | Live report view (auto-refreshes; drops to foreground for re-auth) |
| **SyncProgress** | In-TUI sync progress screen driven by `sync_state` updates |

- `mod.rs` — terminal setup/teardown, `run_interactive()` / `run_settings()` entry points (thin dispatcher)
- `event_loop.rs` — async event loop; splits per-screen tick handling (`handle_td_report_tick`, `handle_sync_progress_tick`, `handle_default_tick`)
- `sync_launcher.rs` — extracts the "load config → read cache → spawn sync task" flow triggered from the main menu
- `app/` — split `App` module:
  - `mod.rs` (struct + ctor), `screen.rs` (Screen enum), `navigation.rs` (selection/clamping), `mutations.rs` (toggles, edits), `row_builders.rs` (per-screen row derivations), `persistence.rs` (config save helpers), `sync.rs` (sync_state lifecycle), `td_report.rs` (TimeDoctor report state), `constants.rs` (TIMEZONES, WEEKLY_HOURS_OPTIONS, etc.)
- `rows.rs` — row enums (`SettingRow`, `RepoManagerRow`, `CpListRow`, `ToggleKind`, …) and `InputMode`
- `sync_state.rs` — `RepoSyncState`, `ForkStatus`, `RdsStatus`, `SyncUpdate` channel messages
- `draw/` — per-screen renderers + shared helpers:
  - `mod.rs` is a dispatcher; each screen has its own file (`main_menu.rs`, `settings.rs`, `repos.rs`, `time.rs`, `td_report.rs`, `sync_screen.rs`)
  - `common/` holds shared helpers: `hints.rs`, `items.rs` (list-item builders + `FieldState`), `header.rs` (`draw_screen_header`), `dialog.rs` (`draw_confirm_dialog`)
- `input/` — per-screen keyboard handlers:
  - `mod.rs` dispatches; `keys.rs` (key classifiers), `helpers.rs` (nav/cycle/text-input helpers), then one file per screen (`main_menu.rs`, `settings.rs`, `repos.rs`, `time.rs`, `contract.rs`, `td_report.rs`, `sync.rs`)
- `layout.rs` — `ScreenLayout` (named vertical rows), `LayoutEngine` (horizontal placement), `UI_WIDTH` constant
- `palette.rs` — indexed color constants (`C_TEXT`, `C_PRIMARY`, …)
- `widgets.rs` — custom pixel-art `IconWidget`, `LOGO` / `LOGO_SMALL` constants

Selecting Sync or Time from the main menu closes the TUI, restores the terminal, then runs the subcommand so its output is visible in the foreground. Sync can alternatively run in-TUI via the `SyncProgress` screen.

### TUI design system

**Color palette** (all defined as named constants in `tui/palette.rs`):

| Constant | Color index | Role |
|----------|-------------|------|
| `C_TEXT` | 255 (white) | Default foreground text |
| `C_PRIMARY` | 199 (magenta) | Selection arrow `▸`, logo on sub-screens |
| `C_ACCENT` | 51 (cyan) | Section headers, selected item text, input cursor |
| `C_SELECT` | = `C_PRIMARY` | Selected menu item in MainMenu |
| `C_SUCCESS` | 10 (green) | `[ON ]` badge |
| `C_MUTED` | 8 (dark gray) | Dividers, hints, unselected text, `[OFF]` badge |
| `C_LOGO` | = `C_TEXT` | Full logo on MainMenu |
| `C_DANGEROUS` | 9 (red) | `[del]` actions, confirmation dialog border |

All colors use `Color::Indexed(n)` for terminal-safe 256-color values. Never use named `Color::*` variants (e.g. `Color::Red`) — they vary by terminal theme.

**Layout system** (`layout.rs`):

- `UI_WIDTH = 79` — canonical content width; matches icon (14) + gap (2) + logo (63).
- `ScreenLayout` — builder that assigns a fixed height (or `Min(0)` for fill) to each named row; call `.split(area)` to get a `RowMap`.
- `RowMap::get(name)` — returns the `Rect` for a named row; panics with a clear message on unknown names.
- `LayoutEngine::place(widget, row)` / `center(width, row)` — computes `x` offset for left-aligned or horizontally-centred content within `UI_WIDTH`.

**Selection pattern**:

- Non-interactive rows (`Blank`, `Separator`) are skipped during navigation; `is_interactive()` on `SettingRow` / `RepoManagerRow` drives this.
- Selected rows render with: `▸ ` prefix (in `C_PRIMARY`) + text in `C_ACCENT + BOLD`.
- Unselected rows render with: `  ` indent + text in `C_TEXT` (or `C_MUTED` for nav items like Back).

**Enter-to-activate pattern** (inline value selectors):

Inline value selectors (dates, hours, timezone, etc.) must **not** enter editing mode on focus. Arrow keys navigate between rows; Enter activates editing on the focused field. While editing, ↑↓ cycle the value and Enter/Esc confirms. This keeps navigation and editing separate so users can browse without accidentally changing values. The editing state renders as `< value >` with an inline hint `↑↓ change  •  ↵ confirm`. Examples: `InputMode::SelectingTimezone`, `InputMode::EditingCpMonday`, `InputMode::EditingCpHours`.

**Screen header pattern** (Settings and RepoManager screens share `draw_screen_header`):

- 3-row `LOGO_SMALL` centered at the top, colored `C_PRIMARY`.
- Title row: screen name left-aligned in `C_ACCENT + BOLD`; hint spans right-aligned on the same row.
- Full-width `─` divider in `C_MUTED` below the title.

**Confirmation dialog** (`draw_confirm_dialog` in `draw/common/dialog.rs`): centered overlay with `Clear` + `Block` border in `C_DANGEROUS`; rendered on top of the active list.

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
│   ├── error.rs                 # AppError enum (thiserror) — IO, HTTP, auth, keyring, fd
│   ├── config/                  # Config module (see ### Config)
│   │   ├── mod.rs               # Re-exports the public surface: load, save, types, ensure_time_credentials
│   │   ├── types.rs             # Config, SyncConfig, TimeConfig, UpstreamRepo, ContractPeriod
│   │   ├── io.rs                # config_path, load, save
│   │   ├── parse.rs             # parse_contract_periods
│   │   └── prompt.rs            # ensure_time_credentials (interactive fill-in)
│   ├── sync/
│   │   ├── mod.rs               # run() entry: resolves repo paths, calls runner
│   │   ├── cache.rs             # RepoPathsCache — load/save/invalidate ~/.cache/jotmate/repo_paths.json
│   │   ├── discover.rs          # fd-based git repo discovery; matches repos to upstream URLs
│   │   ├── runner.rs            # Patches GITHUB_BASE in embedded script, writes tempfile, execs bash
│   │   └── native/              # Native (non-script) in-TUI sync pipeline
│   │       ├── mod.rs           # SyncOpts + run_tui coordinator (fork phase → rds phase)
│   │       ├── git.rs           # git/git_ok/detect_default_branch helpers
│   │       ├── fork.rs          # fork-sync pipeline (stash/fetch/merge/push)
│   │       ├── rds.rs           # rds-sync pipeline (./sync runner with skip rules)
│   │       └── elapsed.rs       # per-repo elapsed-time ticker
│   ├── time/
│   │   ├── mod.rs               # run() entry: auth, batch-fetches weeks, computes, displays
│   │   ├── auth.rs              # Keychain read/write for TimeDoctor session cookie; browser login flow
│   │   ├── api.rs               # HTTP client: fetches weekly stats from TimeDoctor API
│   │   ├── cache.rs             # Per-week JSON cache at ~/.cache/jotmate/time/<company_id>/YYYY-MM-DD.json
│   │   ├── fetch.rs             # High-level weekly fetch + cache orchestration
│   │   ├── compute.rs           # WeekRow, weeks_to_fetch, cumulative balance, target hours logic
│   │   └── display.rs           # ANSI terminal table renderer for WeekRow results
│   └── tui/
│       ├── mod.rs               # Terminal setup/teardown, run_interactive/run_settings entry points
│       ├── event_loop.rs        # Async event loop + per-screen tick handlers
│       ├── sync_launcher.rs     # Kick off in-TUI sync (load config, read cache, spawn task)
│       ├── sync_state.rs        # RepoSyncState, ForkStatus, RdsStatus, SyncUpdate messages
│       ├── rows.rs              # Row enums + InputMode
│       ├── layout.rs            # ScreenLayout (named rows), LayoutEngine (x placement), UI_WIDTH
│       ├── palette.rs           # Indexed color constants (C_TEXT, C_PRIMARY, ...)
│       ├── widgets.rs           # IconWidget (pixel art), LOGO, LOGO_SMALL constants
│       ├── app/                 # Split App state module
│       │   ├── mod.rs           # App struct + constructor
│       │   ├── screen.rs        # Screen enum
│       │   ├── constants.rs     # TIMEZONES, WEEKLY_HOURS_OPTIONS, this_monday helper
│       │   ├── navigation.rs    # Selection indices, clamping to interactive rows
│       │   ├── mutations.rs     # Toggles, cycle edits, repo/period CRUD
│       │   ├── row_builders.rs  # Per-screen row derivation (drives draw + input)
│       │   ├── persistence.rs   # persist_settings / persist_td_settings (save to config)
│       │   ├── sync.rs          # sync_state lifecycle (start/update/cancel)
│       │   └── td_report.rs     # TimeDoctor report state machine
│       ├── draw/                # Per-frame rendering
│       │   ├── mod.rs           # Dispatcher on Screen; re-exports common helpers
│       │   ├── main_menu.rs     # MainMenu renderer
│       │   ├── settings.rs      # Settings + general-toggle list renderer
│       │   ├── repos.rs         # RepoManager + RemoveRepos renderers
│       │   ├── time.rs          # TimeDoctorSettings + ContractPeriods renderers
│       │   ├── td_report.rs     # TimeDoctorReport renderer
│       │   ├── sync_screen.rs   # SyncProgress renderer
│       │   └── common/          # Shared draw helpers
│       │       ├── mod.rs       # fmt_date, sub_screen_layout, width constants
│       │       ├── hints.rs     # Hint-span builders (navigate/toggle, select/back, ...)
│       │       ├── items.rs     # ListItem builders + FieldState (Normal/Selected/Editing)
│       │       ├── header.rs    # draw_screen_header (logo + title + divider)
│       │       └── dialog.rs    # draw_confirm_dialog (centered overlay)
│       └── input/               # Keyboard event handlers
│           ├── mod.rs           # handle_key dispatcher + Action enum
│           ├── keys.rs          # Key classifiers (nav_delta, cycle_delta, is_activate, ...)
│           ├── helpers.rs       # go_to, handle_list_nav, handle_cycle, handle_text_input
│           ├── main_menu.rs     # handle_main
│           ├── settings.rs      # handle_settings, handle_general_toggles
│           ├── repos.rs         # handle_repo_manager, handle_remove_repos
│           ├── time.rs          # handle_td_settings (field input + auth-error flow)
│           ├── contract.rs      # handle_contract_periods
│           ├── td_report.rs     # handle_td_report
│           └── sync.rs          # handle_sync_progress
├── Cargo.toml                   # Package manifest and dependencies
├── Cargo.lock                   # Locked dependency versions
├── install.sh                   # Curl-based installer for end users
├── AGENTS.md                    # This file — guidance for AI-assisted development
└── README.md                    # User-facing documentation
```

## Adding a new tool or settings field

**New tool**: Add a variant to the `Screen` enum in `src/tui/app/screen.rs`, add a new renderer file under `src/tui/draw/` and wire it into `draw/mod.rs`, add a new handler file under `src/tui/input/` and wire it into `input/mod.rs`, and add a Rust subcommand in `src/cli.rs` + `src/main.rs` if CLI access is needed.

**New settings field**: Add the field to the relevant struct in `src/config/types.rs` with `#[serde(default)]`, mirror the field on `App` in `src/tui/app/mod.rs`, extend `persist_settings` / `persist_td_settings` in `src/tui/app/persistence.rs`, add a row via `src/tui/app/row_builders.rs` (and `ToggleKind` in `src/tui/rows.rs` if it's a toggle), and let the existing `draw_general_toggles` + `handle_general_toggles` machinery render/handle it.

## Coding style

Prefer changes that stay small, coherent, and easy to reason about. When editing or adding code, aim for:

- **Single responsibility** — Each module, type, and function should have one clear job. Split mixed concerns (e.g. parsing vs. I/O vs. UI) instead of growing god-objects or all-in-one handlers. If a function does two unrelated things, extract one of them.

- **Single source of truth** — Define each piece of behavior or data in one place. Config and persistent state should flow from `config` (and related types), not from parallel ad hoc structs or duplicated defaults. Menu labels, keybindings, and domain rules should not diverge across files; centralize constants and enums where they are owned.

- **DRY (don’t repeat yourself)** — Before copying a block, extract a shared helper, type, or constant. Repeated string literals, match arms, or validation logic are signals to consolidate. Duplication across `draw.rs` / `input.rs` / `app.rs` for the same feature usually means one definition should drive the rest.

These principles reinforce each other: one responsibility per unit, one canonical definition per concept, and no unnecessary repetition.

## Key constraints

- The `ansi-to-tui` crate is incompatible with ratatui 0.29 (requires <0.27) — don't add it.
- `scripts/run-sync.sh` is the only script still in active use — it is embedded in the binary.
