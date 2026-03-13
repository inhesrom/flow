# Anvl

A terminal-based multi-workspace manager built with Rust.

## Features

- **Multi-workspace management** with Git integration — branch tracking, status monitoring, and inline diffs
- **Embedded terminal sessions** (agent + shell tabs) via PTY, with full input passthrough
- **Attention system** that detects prompts, errors, and activity in terminal output
- **Session persistence** with a daemon/attach model for long-running workspaces
- **Web UI** with real-time WebSocket updates, served from an embedded HTTP server
- **Mouse support** and terminal scrollback via mouse wheel
- **Vim-style navigation** throughout the interface

## Architecture

Anvl is organized as a Cargo workspace with three crates:

| Crate | Description |
|---|---|
| `protocol` | Serializable types for IPC — workspace routing, attention levels, terminal kinds, and command/event enums |
| `core` | Application state management — workspaces, Git, terminal PTY spawning, attention detection, SSH, and the async event loop |
| `tui` | Terminal UI built with Ratatui — renders home/workspace screens, handles input, and manages sessions |

See [docs/repo-diagram.md](/Users/ianhersom/repo/anvl/docs/repo-diagram.md) for a rendered repo diagram and runtime overview.

## Getting Started

### Install

```sh
curl -fsSL https://raw.githubusercontent.com/inhesrom/anvl/master/install.sh | bash
```

Prebuilt binaries are available for:
- macOS (Apple Silicon)
- Linux (x86_64)

The installer places the `anvl` binary in `~/.local/bin`. Override with `ANVL_INSTALL_DIR`.

### Build from source

#### Prerequisites

- [Rust toolchain](https://rustup.rs/) (stable)

#### Build

```sh
cargo build --release
```

#### Run

```sh
cargo run
# or
./target/release/anvl
```

## Usage

```
anvl                    Local mode (no session)
anvl -s <name>          Create and start a named session
anvl -a <name>          Attach to an existing session
anvl -l                 List sessions
anvl -r <name>          Remove a session
anvl -d                 Detach (use with -s or -a)
```

## Key Bindings

### Global

| Key | Action |
|---|---|
| `q` | Quit |
| `Tab` / `Shift+Tab` | Cycle focus between sections |
| `Esc` | Exit focused section / go back |

### Home Screen

| Key | Action |
|---|---|
| `h` `j` `k` `l` / Arrow keys | Navigate workspaces |
| `Enter` | Open selected workspace |
| `n` | New workspace |
| `D` | Delete workspace |
| `!` | Toggle attention level |
| `g` | Refresh git status |

### Workspace Screen

| Key | Action |
|---|---|
| `1` `2` / `h` `l` | Switch terminal tabs |
| `n` | New shell tab |
| `x` | Close active tab |
| `r` | Rename tab |
| `a` / `A` | Start / stop agent terminal |
| `s` / `S` | Start / stop shell terminal |
| `g` | Refresh git |
| `j` `k` / Arrow keys | Navigate file list |
| `Enter` | Show diff for selected file |
| Mouse wheel | Scroll terminal output |

## Configuration

### Environment Variables

| Variable | Description | Default |
|---|---|---|
| `ANVL_WEB_PORT` | Embedded web server port | `3001` |
| `ANVL_DISABLE_EMBEDDED_WEB` | Disable the embedded web server if set | — |
| `SHELL` | Shell used for terminal sessions | `zsh` |

### Config Paths

Anvl stores configuration under `~/.config/anvl/` (respects `XDG_CONFIG_HOME`):

- `sessions.json` — session registry
- `workspaces.json` — default workspace persistence
- `workspaces.<session-name>.json` — per-session workspace state

## License

MIT
