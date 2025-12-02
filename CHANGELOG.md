# Changelog

All notable changes to Bark will be documented in this file.

## [Unreleased]

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
- **Color themes** - 5 built-in themes: default, kawaii, cyber, dracula, monochrome
  - Set via `theme` config option or `BARK_THEME` environment variable

### Technical
- Async event loop using tokio
- ratatui-based TUI with crossterm backend
- Lazy ANSI-to-styled-text conversion
- Panic hook for clean terminal restoration
- Debounced filter recomputation to avoid lag during typing
- serde-based config serialization

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
- `BARK_THEME` - Color theme (default, kawaii, cyber, dracula, monochrome)
