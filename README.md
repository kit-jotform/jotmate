# jotmate

Jotform developer productivity CLI — syncs forks with upstream and tracks TimeDoctor work hours. **jotmate is blazingly fast.**

## Installation

**From a release (recommended):**

```sh
curl -fsSL https://raw.githubusercontent.com/kit-jotform/Jotmate/main/install.sh | sh
```

Installs to `~/.local/bin/` by default. Use `--prefix` to change:

```sh
curl -fsSL https://raw.githubusercontent.com/kit-jotform/Jotmate/main/install.sh | sh -s -- --prefix /usr/local
```

**From a local build:**

```sh
./install.sh --local
```

Runs `cargo build --release` and copies the binary from `target/release/`. Same `--prefix` option applies.

The installer always creates a `jf` symlink alongside the `jotmate` binary so both names work. If you still have a legacy `jt` symlink in the install directory, re-running the installer removes it so only `jf` remains on your `PATH`.

## Usage

```
jotmate          # open interactive TUI
jf               # same (short alias)
```

### Headless commands

All subcommands work without opening the TUI — useful in scripts or for a quick one-shot run.

```sh
jf sync          # sync enabled forks with upstream
jf time          # show TimeDoctor work hours
jf settings      # open the settings screen
```

### `jf sync` options

```sh
jf sync --only frontend,backend    # sync specific repos (bypasses enabled/disabled)
jf sync --sync-all                 # sync all repos including disabled ones, bypass smart sync
jf sync --rds-only                 # skip fork sync, run only RDS (./sync) in each repo
jf sync -S                         # --no-smart-sync: run RDS unconditionally, don't skip
jf sync --no-cache                 # ignore repo path cache, rediscover repos
jf sync --skip-fork-sync           # skip fork sync for this run
jf sync --skip-rebase              # skip rebasing current branch after fork sync
jf sync --skip-rds-sync            # skip RDS sync for this run
jf sync --skip-fetch               # skip git fetch upstream (use already-fetched refs)
```

Flags can be combined. `--sync-all` and `--only` are mutually exclusive. `--rds-only` and `--skip-rds-sync` are mutually exclusive.

### `jf time` options

```sh
jf time --no-cache            # bypass week cache, re-fetch from TimeDoctor API
jf time --skip-current-week   # exclude the current incomplete week
```

## Settings

Open the interactive settings screen:

```sh
jf settings
# or launch the full TUI and navigate to Settings
jf
```

### Sync settings

| Setting | Default | Description |
|---------|---------|-------------|
| Use repo path cache | ON | Cache discovered repo paths; skip home-directory scan on subsequent runs |
| Fork sync | ON | Fetch upstream, merge into default branch, push to origin |
| → Rebase | ON | After fork sync, rebase current branch onto default branch |
| RDS sync | ON | Run `./sync` in each repo to sync files to the Remote Dev Server |
| → Smart sync | ON | Skip RDS sync for repos with no upstream changes, clean working tree, and nothing ahead/behind origin |

**Smart sync detail:** when ON, RDS sync is skipped for a repo only if all of the following are true: fork was unchanged, working tree is clean, and local branch is not ahead/behind `origin/<branch>`. Dirty repos always run RDS sync. Turn smart sync OFF (or use `-S`) to always run `./sync` regardless.

Repos to sync are managed under **Manage Repos** — toggle individual repos on/off, add new upstream URLs, or remove repos.

### Time Doctor settings

Configure your email, password (stored in the system keychain), timezone, and contract periods (weekly hour targets) from the TUI settings screen.

## Configuration file

Config lives at the platform config dir — `~/Library/Application Support/jotmate/config.toml` on macOS, `~/.config/jotmate/config.toml` on Linux. It is edited automatically by the settings screen, but manual edits are also fine.

```toml
[sync]
use_cache = true
skip_fork_sync = false
skip_rebase = false
skip_rds_sync = false
smart_sync = true

[[sync.upstream_repos]]
url = "https://github.com/jotform/frontend.git"
name = "frontend"
enabled = true

[time]
email = "you@jotform.com"
timezone = "Europe/Istanbul"
skip_current_week = true
use_time_cache = true
show_cumulative = true
```

TimeDoctor credentials (password / session cookie) are stored in the system keychain, never in the config file.

## Project structure

```
jotmate/
├── scripts/                   # reference examples only; not used by the binary
├── src/
│   ├── main.rs                # entry point: dispatches to tui / sync / time
│   ├── lib.rs                 # library surface so tests/ and main share one tree
│   ├── cli.rs                 # clap CLI definitions
│   ├── ctx.rs                 # composition root: Paths, HttpBase, KeychainStore
│   ├── error.rs               # AppError enum
│   ├── config/                # ~/.config/jotmate/config.toml load/save
│   │   ├── types.rs           # Config, SyncConfig, TimeConfig, ContractPeriod
│   │   ├── io.rs              # load / save
│   │   ├── parse.rs           # contract period parsing
│   │   └── prompt.rs          # interactive fill-in for missing creds
│   ├── sync/                  # git fork sync
│   │   ├── mod.rs             # plan_sync + run() entry
│   │   ├── cache.rs           # repo_paths.json under the platform cache dir
│   │   ├── discover.rs        # native repo discovery (ignore crate walker)
│   │   └── native/            # native Rust sync engine (TUI + headless)
│   │       ├── mod.rs         # SyncOpts + run_tui orchestration (fork → rds)
│   │       ├── headless.rs    # single-line CLI spinner renderer
│   │       ├── fork.rs        # fetch/merge/push upstream
│   │       ├── rds.rs         # ./sync runner + smart sync skip logic
│   │       ├── git.rs         # GitExec trait + SubprocessGit impl
│   │       └── elapsed.rs     # per-repo elapsed timer
│   ├── time/                  # TimeDoctor tracking
│   │   ├── mod.rs             # run() entry + parallel week fetch
│   │   ├── auth.rs            # HTTP login / reauth / password prompt
│   │   ├── keychain.rs        # KeychainStore trait + macOS security CLI impl
│   │   ├── api.rs             # HTTP client + shared td_headers()
│   │   ├── fetch.rs           # weekly fetch + cache orchestration
│   │   ├── cache.rs           # per-week JSON cache
│   │   ├── compute.rs         # WeekRow / balance / target hours
│   │   └── display.rs         # headless spinner + final-line renderer
│   └── tui/                   # Ratatui interactive UI
│       ├── mod.rs             # terminal setup/teardown + entry points
│       ├── event_loop.rs      # async event loop + per-screen tick handlers
│       ├── sync_launcher.rs   # kicks off in-TUI sync (discovery + spawn)
│       ├── sync_state.rs      # RepoSyncState / ForkStatus / RdsStatus / SyncPhase
│       ├── rows.rs            # row enums + InputMode
│       ├── layout.rs          # ScreenLayout / LayoutEngine / UI_WIDTH
│       ├── palette.rs         # indexed color constants + ANSI escapes
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
│       │       ├── mod.rs     #     fmt_date, sub_screen_setup, width constants
│       │       ├── hints.rs   #     hint span builders
│       │       ├── items.rs   #     list-item builders + FieldState
│       │       ├── header.rs  #     screen header (logo + title + divider)
│       │       ├── dialog.rs  #     centered confirm dialog
│       │       └── scroll_table.rs # scrollable table (TD report + sync progress)
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
├── tests/                     # integration tests (use Ctx with TempDir + fakes)
├── Cargo.toml
├── install.sh                 # release + local installer (--local for source builds)
└── AGENTS.md                  # detailed guidance for AI-assisted development
```

See `AGENTS.md` for architecture notes, TUI design system, and conventions.
