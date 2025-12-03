# bark

A fast, keyboard-driven TUI for tailing logs from files, Docker, Kubernetes, and SSH servers.

[![CI](https://github.com/lance0/bark/actions/workflows/ci.yml/badge.svg)](https://github.com/lance0/bark/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/bark.svg)](https://crates.io/crates/bark)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

![bark screenshot](screenshot.png)

## Quick Start

```bash
cargo install bark

# Opens picker to select containers
bark

# Or specify sources directly
bark /var/log/syslog
bark --docker nginx
bark --k8s my-pod -n production
```

## Features

**Sources** - Tail logs from local files, Docker containers, Kubernetes pods, or remote files via SSH. Mix and match multiple sources into a single merged view.

**Filtering** - Type `/` to filter with live preview. Supports substring and regex modes. Filter history with `↑`/`↓`.

**Navigation** - Vim-style keys (`j`/`k`, `g`/`G`, `Ctrl+u`/`Ctrl+d`). Click lines to select, `y` to yank to clipboard.

**Split View** - `Ctrl+W,v` for vertical split, `Ctrl+W,s` for horizontal. Each pane has independent filters and scroll.

**Display Options** - JSON pretty-printing (`J`), relative timestamps (`t`), line numbers (`#`), line wrap (`w`), log level coloring (`c`).

**11 Themes** - default, dracula, nord, gruvbox, catppuccin, tokyo_night, solarized, matrix, cyber, kawaii, monochrome.

## Installation

```bash
# From crates.io
cargo install bark

# From source
git clone https://github.com/lance0/bark.git
cd bark && cargo build --release
```

Pre-built binaries available at [GitHub Releases](https://github.com/lance0/bark/releases).

## Usage

```bash
# Auto-discovery (opens picker)
bark                  # Discover Docker containers
bark --docker         # Discover all Docker containers
bark --k8s            # Discover all Kubernetes pods
bark --all            # Discover all sources

# Direct sources
bark /var/log/syslog
bark --docker nginx
bark --k8s my-pod
bark --k8s my-pod -n namespace -c container
bark --ssh user@host /var/log/app.log

# Multiple sources (merged timeline)
bark --docker nginx --docker redis
bark /var/log/app.log --docker nginx
```

## Key Bindings

| Key | Action |
|-----|--------|
| `j`/`k` | Scroll down/up |
| `g`/`G` | Top/bottom |
| `Ctrl+u`/`Ctrl+d` | Half page up/down |
| `/` | Filter (regex with `r`) |
| `n`/`N` | Next/prev match |
| `m` | Toggle bookmark |
| `[`/`]` | Prev/next bookmark |
| `y` | Yank line to clipboard |
| `p` | Pause/resume follow |
| `?` | Help overlay |
| `q` | Quit |

### Split View

| Key | Action |
|-----|--------|
| `Ctrl+W,v` | Vertical split |
| `Ctrl+W,s` | Horizontal split |
| `Ctrl+W,q` | Close pane |
| `Ctrl+W,w` | Cycle panes |
| `Tab` | Cycle focus |

### Display Toggles

| Key | Toggle |
|-----|--------|
| `w` | Line wrap |
| `c` | Level colors |
| `t` | Relative time |
| `J` | JSON pretty-print |
| `#` | Line numbers |
| `b` | Side panel |
| `S` | Settings |

### Runtime Discovery

| Key | Action |
|-----|--------|
| `D` | Docker picker |
| `K` | Kubernetes picker |

In picker: `j`/`k` navigate, `Space` toggle, `Enter` confirm.

## Configuration

`~/.config/bark/config.toml`:

```toml
max_lines = 10000
level_colors = true
line_wrap = false
show_side_panel = true
export_dir = "/tmp"
theme = "default"
```

Environment variables override config: `BARK_MAX_LINES`, `BARK_THEME`, `BARK_LEVEL_COLORS`, etc.

## Requirements

- **Rust** 1.85+
- **Docker**: `docker` CLI (for container logs)
- **Kubernetes**: `kubectl` configured (for pod logs)
- **SSH**: Key-based auth recommended

## Troubleshooting

**Docker not working?** Check `docker ps` works and container exists.

**K8s not working?** Verify `kubectl cluster-info` and pod exists in namespace.

**SSH not working?** Ensure key auth works: `ssh user@host "tail -1 /path/to/log"`

**High memory?** Reduce buffer: `BARK_MAX_LINES=5000`

## License

MIT OR Apache-2.0
