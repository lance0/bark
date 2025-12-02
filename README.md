# bark

A fast, keyboard-driven TUI for exploring logs from files, Docker containers, Kubernetes pods, and remote servers.

![Rust](https://img.shields.io/badge/rust-stable-orange.svg)
![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)

## Features

- **Multiple log sources**: Local files, Docker containers, Kubernetes pods, SSH remote files
- **Real-time streaming**: Auto-follows new log lines with smart scroll behavior
- **Powerful filtering**: Substring and regex filtering with live preview
- **Search highlighting**: Matching text highlighted, navigate with n/N
- **Bookmarks**: Mark important lines, jump between them with [/]
- **ANSI color preservation**: View colorized logs as intended
- **Log level detection**: Auto-colors ERROR, WARN, INFO, DEBUG, TRACE
- **JSON pretty-printing**: Expand single-line JSON logs for readability
- **Timestamp parsing**: View relative times ("5s ago", "2m ago")
- **Export**: Save filtered lines to a file
- **Configurable**: Persistent settings via config file or environment
- **Themes**: Built-in color themes (default, kawaii, cyber, dracula, monochrome)

## Installation

### From source

```bash
git clone https://github.com/lance0/bark.git
cd bark
cargo build --release
# Binary is at target/release/bark
```

### Using Cargo

```bash
cargo install --git https://github.com/lance0/bark.git
```

## Usage

```bash
# Tail a local file
bark /var/log/syslog

# Follow Docker container logs
bark --docker my-container

# Follow Kubernetes pod logs
bark --k8s my-pod
bark --k8s my-pod -n my-namespace
bark --k8s my-pod -n my-namespace -c my-container

# Tail a remote file via SSH
bark --ssh user@host /var/log/app.log
```

## Keyboard Shortcuts

### Navigation
| Key | Action |
|-----|--------|
| `j` / `k` | Scroll down/up |
| `h` / `l` | Scroll left/right (when wrap off) |
| `H` / `L` | Large horizontal scroll |
| `0` | Scroll to line start |
| `g` / `G` | Go to top/bottom |
| `PgUp` / `PgDn` | Page up/down |
| `Ctrl+u` / `Ctrl+d` | Half page up/down |
| `n` / `N` | Next/previous match |
| `m` | Toggle bookmark |
| `[` / `]` | Previous/next bookmark |

### Filtering
| Key | Action |
|-----|--------|
| `/` | Start filter input |
| `r` | Toggle regex mode |
| `Enter` | Apply filter |
| `Esc` | Clear filter / cancel |
| `s` | Save current filter |
| `e` | Export filtered lines |

### Display
| Key | Action |
|-----|--------|
| `w` | Toggle line wrapping |
| `c` | Toggle level colors |
| `t` | Toggle relative time |
| `J` | Toggle JSON pretty-print |
| `b` | Toggle side panel |
| `Tab` | Cycle panel focus |
| `?` | Show help |
| `q` | Quit |

## Configuration

Bark reads configuration from `~/.config/bark/config.toml`:

```toml
# Maximum lines in the ring buffer
max_lines = 10000

# Enable log level coloring
level_colors = true

# Enable line wrapping by default
line_wrap = false

# Show side panel on startup
show_side_panel = true

# Directory for exported logs
export_dir = "/tmp"

# Color theme: default, kawaii, cyber, dracula, monochrome
theme = "default"
```

### Environment Variables

Environment variables override config file settings:

- `BARK_MAX_LINES` - Maximum lines in buffer
- `BARK_LEVEL_COLORS` - Enable level coloring (1/true or 0/false)
- `BARK_LINE_WRAP` - Enable line wrapping
- `BARK_SIDE_PANEL` - Show side panel
- `BARK_EXPORT_DIR` - Export directory
- `BARK_THEME` - Color theme (default, kawaii, cyber, dracula, monochrome)

## Status Bar Indicators

The status bar shows active modes:
- `[F]` - Follow mode (auto-scroll)
- `[.*]` - Regex filter mode
- `[W]` - Line wrap enabled
- `[C]` - Level colors enabled
- `[T]` - Relative time enabled
- `[J]` - JSON pretty-print enabled
- `[+N]` - Horizontal scroll offset

## Requirements

- **Rust** 1.85+ (for Rust 2024 edition)
- **For Docker**: `docker` command available
- **For Kubernetes**: `kubectl` configured
- **For SSH**: SSH key authentication recommended

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
