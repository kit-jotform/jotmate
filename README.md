# jotmate

Jotform developer productivity CLI — syncs forks with upstream and tracks TimeDoctor work hours.

## Installation

```sh
./install.sh --prefix /usr/local
```

Or build from source:

```sh
cargo build --release
sudo cp target/release/jotmate /usr/local/bin/
```

Or without sudo (user-only):

```sh
cargo build --release
cp target/release/jotmate ~/.local/bin/
```

## Usage

```
jotmate            # interactive TUI
jotmate sync       # sync all forks with upstream
jotmate time       # show TimeDoctor work hours
jotmate settings   # edit configuration
```

### sync options

```
jotmate sync --only frontend,backend   # sync specific repos
jotmate sync --sync-all                # force run ./sync for all repos
```

### time options

```
jotmate time --no-cache         # bypass week cache
jotmate time --skip-current-week
```

## Configuration

Config file: `~/.config/jotmate/config.toml`

Edit interactively with `jotmate settings`, or set manually:

```toml
[time]
email = "you@jotform.com"
company_id = "12345"
timezone = "Europe/Istanbul"
start_date = "2025-11-17"
contract_periods = "2025-11-17:20,2026-02-02:28"
skip_current_week = true

[sync]
github_base = "/Users/you/Documents/Github"
```

TimeDoctor credentials are stored in the system keychain (macOS Keychain / Linux secret-service).

## Project structure

```
jotmate/
├── scripts/
│   └── run-sync.sh            # embedded into the binary via include_str!()
├── src/
│   ├── main.rs                # entry point: dispatches to tui / sync / time
│   ├── cli.rs                 # clap CLI definitions
│   ├── error.rs               # AppError enum
│   ├── config/                # ~/.config/jotmate/config.toml load/save
│   │   ├── types.rs           # Config, SyncConfig, TimeConfig, ContractPeriod
│   │   ├── io.rs              # load / save
│   │   ├── parse.rs           # contract period parsing
│   │   └── prompt.rs          # interactive fill-in for missing creds
│   ├── sync/                  # git fork sync
│   │   ├── cache.rs           # ~/.cache/jotmate/repo_paths.json
│   │   ├── discover.rs        # fd-based repo discovery
│   │   ├── runner.rs          # runs embedded run-sync.sh
│   │   └── native/            # in-TUI sync pipeline
│   │       ├── fork.rs        # fetch/merge/push upstream
│   │       ├── rds.rs         # ./sync runner
│   │       ├── git.rs         # git helpers
│   │       └── elapsed.rs     # per-repo elapsed timer
│   ├── time/                  # TimeDoctor tracking
│   │   ├── auth.rs            # system keychain session cookie
│   │   ├── api.rs             # HTTP client
│   │   ├── fetch.rs           # weekly fetch + cache orchestration
│   │   ├── cache.rs           # per-week JSON cache
│   │   ├── compute.rs         # WeekRow / balance / target hours
│   │   └── display.rs         # terminal table renderer
│   └── tui/                   # Ratatui interactive UI
│       ├── mod.rs             # terminal setup/teardown + entry points
│       ├── event_loop.rs      # async event loop + per-screen tick handlers
│       ├── sync_launcher.rs   # kicks off in-TUI sync
│       ├── sync_state.rs      # RepoSyncState / ForkStatus / RdsStatus
│       ├── rows.rs            # row enums + InputMode
│       ├── layout.rs          # ScreenLayout / LayoutEngine / UI_WIDTH
│       ├── palette.rs         # indexed color constants
│       ├── widgets.rs         # IconWidget, LOGO, LOGO_SMALL
│       ├── app/               # App state (split by concern)
│       │   ├── mod.rs         #   struct + constructor
│       │   ├── screen.rs      #   Screen enum
│       │   ├── constants.rs   #   TIMEZONES, WEEKLY_HOURS_OPTIONS
│       │   ├── navigation.rs  #   selection / clamping
│       │   ├── mutations.rs   #   toggles / edits / CRUD
│       │   ├── row_builders.rs#   per-screen row derivation
│       │   ├── persistence.rs #   config save helpers
│       │   ├── sync.rs        #   sync_state lifecycle
│       │   └── td_report.rs   #   Time Doctor report state
│       ├── draw/              # per-frame renderers (one file per screen)
│       │   ├── mod.rs         #   dispatcher on Screen
│       │   ├── main_menu.rs
│       │   ├── settings.rs    #   Settings + general-toggle list
│       │   ├── repos.rs       #   RepoManager + RemoveRepos
│       │   ├── time.rs        #   Time Doctor credentials
│       │   ├── contract.rs    #   ContractPeriods
│       │   ├── td_report.rs   #   Time Doctor report
│       │   ├── sync.rs        #   SyncProgress
│       │   └── common/        #   shared helpers
│       │       ├── hints.rs   #     hint span builders
│       │       ├── items.rs   #     list-item builders + FieldState
│       │       ├── header.rs  #     screen header (logo + title + divider)
│       │       └── dialog.rs  #     centered confirm dialog
│       └── input/             # keyboard handlers (one file per screen)
│           ├── mod.rs         #   handle_key dispatcher + Action enum
│           ├── keys.rs        #   key classifiers
│           ├── helpers.rs     #   nav / cycle / text-input helpers
│           ├── main_menu.rs
│           ├── settings.rs
│           ├── repos.rs
│           ├── time.rs        #   Time Doctor credentials
│           ├── contract.rs    #   ContractPeriods
│           ├── td_report.rs
│           └── sync.rs        #   SyncProgress
├── Cargo.toml
├── install.sh                 # curl-based installer
└── AGENTS.md                  # detailed guidance for AI-assisted development
```

See `AGENTS.md` for architecture notes, TUI design system, and conventions.
