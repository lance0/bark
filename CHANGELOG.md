# Changelog

All notable changes to Bark will be documented in this file.

## [1.1.0] - 2025-12-03

### Added
- **Auto-discovery mode** - Start bark without arguments to open source picker immediately
  - `bark` - Opens Docker picker to discover running containers
  - `bark --docker` - Discovers all Docker containers (no name required)
  - `bark --k8s` - Discovers all Kubernetes pods (no pod name required)
  - `bark --all` - Discovers all Docker containers and K8s pods
- **Click-to-select lines** - Click on any log line to select it for yanking
  - Selected line shows `▶` indicator
  - `y` yanks the selected line (or top visible line if none selected)
  - `Esc` clears selection (then clears filter on second press)
  - Clicking in split view also switches focus to that pane
- **Improved source picker** - Shows existing sources as checked, allows deselecting
  - Already-added sources appear pre-checked in picker
  - Uncheck and press Enter to hide sources from view
  - Press Enter without toggling to add highlighted item
- **Multi-source support** - View logs from multiple containers/pods simultaneously
  - CLI: `bark --docker nginx --docker redis` or `bark --k8s frontend --k8s backend`
  - Mixed sources: `bark /var/log/app.log --docker nginx`
- **Runtime source discovery** - Add containers/pods while running
  - `D` - Open Docker container picker (lists running containers)
  - `K` - Open Kubernetes pod picker (lists pods across namespaces)
  - Multi-select support with checkbox toggles
- **Source visibility controls** in Sources panel
  - `Space` - Toggle source visibility
  - `v` - Solo view (show only selected source)
  - `a` - Show all sources
- **Source-colored log prefixes** - Each source gets a unique color in merged view
- **Per-theme source color palettes** - All 5 themes have matching source colors
- **SourceManager** for merging multiple async log streams
- **Pause/resume auto-scroll** - `p` key to toggle following new logs
- **SSH host key checking config** - `BARK_SSH_HOST_KEY_CHECKING` env var (default: strict)
- **High-volume log handling** - Improved performance for fast log streams
  - Batch processing: up to 500 lines processed per frame
  - Throttled UI redraws at ~60fps to prevent overwhelming the terminal
  - Lines/sec throughput indicator shows real-time log rate in status bar
- **Settings page** - `S` key opens settings overlay
  - Configure theme, level colors, line wrap, side panel visibility
  - Changes auto-save to `~/.config/bark/config.toml`
  - Navigate with `j`/`k`, toggle with `Space`, close with `Esc`
- **6 new themes** (11 total):
  - `matrix` - Green on black hacker style
  - `nord` - Arctic, north-bluish colors
  - `gruvbox` - Retro groove warm colors
  - `catppuccin` - Soothing pastel (Mocha variant)
  - `tokyo_night` - Dark theme inspired by Tokyo city lights
  - `solarized` - Precision colors for readability
- **Filter history** - `↑`/`↓` in filter mode to browse recent filters (keeps last 50)
- **Copy to clipboard** - `y` yanks current line to clipboard (strips ANSI codes)
- **Line numbers** - `#` toggles line number display in log view
- **Split view** - View logs in multiple panes with independent filters
  - `Ctrl+W, v` - Vertical split (side-by-side)
  - `Ctrl+W, s` - Horizontal split (stacked)
  - `Ctrl+W, q` - Close current pane
  - `Ctrl+W, w` - Cycle to next pane
  - `Ctrl+W, h/j/k/l` - Navigate between panes (vim-style)
  - Each pane has independent: scroll position, filter, bookmarks, source visibility

### Changed
- AppState now accepts multiple sources at initialization
- Event loop handles multiplexed SourcedLogEvent from all sources
- Sources panel shows `[x]/[ ]` visibility toggles
- CLI help updated with multi-source examples and auto-discovery options
- Non-blocking container/pod discovery (UI stays responsive)
- Tab key now cycles through split panes before moving to sidebar

### Security
- **Input validation** - Reject hostnames/container names starting with `-` to prevent option injection
- **Command separators** - All spawned commands use `--` before user arguments
- **SSH StrictHostKeyChecking** - Default changed to `yes` (strict) to prevent MITM attacks
- **UTF-8 error handling** - Reader loops report errors instead of silently stopping
- **Source cleanup on shutdown** - Spawned tasks properly terminated on exit
- Removed unsafe code in visible_lines function
- 70 unit tests covering security-critical validation functions

### Fixed
- Bookmark indices now adjust correctly when buffer wraps
- K8s namespace preserved when selecting pods from picker
- Auto-scroll position corrected on viewport resize
- Incremental filter optimization (O(m) instead of O(n) on buffer wrap)

## [1.0.0] - 2025-12-02

### Added
- Initial project structure with modular architecture
- File log source using `tail -F`
- Docker log source using `docker logs -f`
- **Kubernetes pod log source** using `kubectl logs -f` with namespace/container support
- **SSH remote file tailing** using `ssh` + `tail -F`
- Ring buffer with configurable max lines (default 10,000, via `BARK_MAX_LINES`)
- ANSI color code preservation and rendering
- Smart follow mode (auto-scroll to bottom, manual scroll disables follow)
- Basic navigation: j/k, arrows, g/G, PageUp/PageDown, Ctrl+u/d
- **Horizontal scrolling**: h/l or arrows, H/L for large jumps, 0 for line start
- Substring filtering with `/` key
- Regex filter toggle (`r` in normal mode, `Ctrl+r` in filter mode)
- **Search highlighting** - matching text highlighted in yellow
- **Match navigation** - n/N to jump between matches
- Filter clear with `Esc`
- Filter debounce (150ms) for live preview while typing
- **Export filtered lines** to file with `e` key
- Status bar showing mode, line counts, and active filter
- Scrollbar for large log buffers
- Test log generator script (`scripts/gen_logs.sh`)
- **tui-textarea** integration for filter input with full editing support
- Side panel with sources list and saved filters (toggle with `b`)
- Panel focus cycling with `Tab` key
- Save filters with `s`, apply with `Enter`, delete with `x`
- Help overlay showing all keyboard shortcuts (`?`)
- **Log level detection** - auto-colors ERROR (red), WARN (yellow), INFO (green), DEBUG (blue), TRACE (gray)
- **Line wrapping** - toggle with `w` key
- **Mouse wheel scrolling** - scroll up/down with mouse wheel
- **Timestamp parsing** - auto-detects common timestamp formats (ISO 8601, syslog, etc.)
- **Relative time display** - toggle with `t` to show "5s ago", "2m ago" format
- **JSON log pretty-printing** - toggle with `J` to expand JSON logs
- **Bookmarks** - mark lines with `m`, navigate with `[`/`]`
- **Config file support** - `~/.config/bark/config.toml` for persistent settings
- **Color themes** - 11 built-in themes: default, kawaii, cyber, dracula, monochrome, matrix, nord, gruvbox, catppuccin, tokyo_night, solarized
  - Set via `theme` config option or `BARK_THEME` environment variable

### Technical
- Async event loop using tokio
- ratatui-based TUI with crossterm backend
- Lazy ANSI-to-styled-text conversion
- Panic hook for clean terminal restoration
- Debounced filter recomputation to avoid lag during typing
- serde-based config serialization
- **Unit tests** for filter, app, theme, and config modules (51 tests)
- **Module documentation** with doc comments across all source files
- **Centralized constants** for buffer sizes, debounce timing, and scroll behavior
- **Improved error messages** with actionable hints for Docker, Kubernetes, and SSH sources
- **Config error reporting** - warns on invalid TOML instead of silent failure

## Configuration

Bark can be configured via `~/.config/bark/config.toml`:

```toml
max_lines = 10000
level_colors = true
line_wrap = false
show_side_panel = true
export_dir = "/tmp"
theme = "default"
```

Environment variables (override config file):
- `BARK_MAX_LINES` - Maximum lines in buffer
- `BARK_LEVEL_COLORS` - Enable/disable level coloring (1/true or 0/false)
- `BARK_LINE_WRAP` - Enable/disable line wrapping
- `BARK_SIDE_PANEL` - Show/hide side panel
- `BARK_EXPORT_DIR` - Directory for exported logs
- `BARK_THEME` - Color theme (11 options: default, kawaii, cyber, dracula, monochrome, matrix, nord, gruvbox, catppuccin, tokyo_night, solarized)
