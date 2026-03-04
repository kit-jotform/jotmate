# jotmate

Jotform developer productivity CLI — syncs forks with upstream and tracks TimeDoctor work hours.

## Installation

### Local (dev)

```sh
cargo install --path .
```

### Global (from release)

```sh
curl -fsSL https://raw.githubusercontent.com/jotform/jotmate/main/install.sh | sh
```

Installs to `~/.local/bin` by default. For a system-wide install:

```sh
./install.sh --prefix /usr/local
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

## Building from source

```sh
cargo build --release
# binary at: target/release/jotmate
```
