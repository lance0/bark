# Bark Roadmap

## Milestone 1: Minimal Viable TUI ✅

- [x] Project scaffolding (Cargo.toml, git, module structure)
- [x] Test log generator script with ANSI colors
- [x] Core data structures (LogLine, AppState, ring buffer)
- [x] File source implementation (tail -F)
- [x] Docker source implementation (docker logs -f)
- [x] Basic UI rendering (log view, header, status bar)
- [x] Main event loop with async coordination
- [x] Navigation (j/k, arrows, g/G, PageUp/Down)
- [x] Smart follow mode (auto-scroll, manual scroll breaks follow)
- [x] ANSI color preservation via ansi-to-tui

## Milestone 2: Filtering ✅

- [x] Integrate tui-textarea for filter input
- [x] Filter debounce (150ms delay before recompute)
- [x] Regex filter toggle (r key in normal mode, Ctrl+r in filter mode)
- [x] Live filter preview while typing (debounced)

## Milestone 3: Complete v1 ✅

- [x] Side panel with source list (toggle with `b`)
- [x] Panel focus cycling (`Tab` key)
- [x] Saved filters (save with `s`, apply with `Enter`, delete with `x`)
- [x] Filter quick-select from saved list
- [x] Help overlay (`?` key)
- [x] Config file support (~/.config/bark/config.toml)

## Milestone 4: Enhanced Features ✅

- [x] Search highlighting (matched text highlighted in yellow)
- [x] Match navigation (n/N for next/prev match)
- [x] Horizontal scrolling (h/l, H/L, 0 for line start)
- [x] Export filtered lines to file (e key)
- [x] Log level detection and coloring (c to toggle)
- [x] Line wrapping toggle (w to toggle)
- [x] Mouse wheel scrolling
- [x] Timestamp parsing and relative time display (t to toggle)
- [x] JSON log pretty-printing (J to toggle)
- [x] Bookmarks (m to toggle, [/] to navigate)

## Milestone 5: Multiple Sources ✅

- [x] Kubernetes pod log source (kubectl logs -f)
- [x] SSH remote file tailing (ssh + tail -F)

## Milestone 6: Themes ✅

- [x] Themeable color system
- [x] Built-in themes (11 total): default, kawaii, cyber, dracula, monochrome, matrix, nord, gruvbox, catppuccin, tokyo_night, solarized
- [x] Theme selection via config file and environment variable

## Release Prep ✅

- [x] Unit tests for core modules (filter, app, theme, config) - 51 tests
- [x] Module-level documentation (doc comments)
- [x] Centralized constants (buffer sizes, debounce, scroll lines)
- [x] Improved error messages with actionable hints
- [x] Config error reporting (warns on invalid TOML)
- [x] CLI help with examples and environment variables
- [x] README troubleshooting section
- [x] CI workflow fix (main → master branch)

## Milestone 7: Multi-Source Support ✅

- [x] Multiple sources via CLI (`--docker nginx --docker redis`)
- [x] Source manager for merging multiple log streams
- [x] Source identification on each log line
- [x] Merged timeline view (all sources interleaved chronologically)
- [x] Source visibility toggles (Space to toggle in Sources panel)
- [x] Solo view mode (v to show only selected source)
- [x] Show all sources (a key)
- [x] Source-colored prefixes in log view
- [x] Per-theme source color palettes
- [x] Runtime container discovery (D for Docker, K for K8s)
- [x] Container/pod picker UI with multi-select
- [x] Non-blocking discovery (async with spawn_blocking)

## Milestone 8: Security Hardening ✅

- [x] SSH command injection prevention (hostname validation)
- [x] SSH StrictHostKeyChecking default to "yes" (configurable)
- [x] Option injection prevention (`--` separators in all commands)
- [x] Input validation for container/pod names
- [x] Remove unsafe code in visible_lines
- [x] Source cleanup on shutdown (Drop impl)
- [x] UTF-8 error handling in reader loops
- [x] Bookmark index adjustment on buffer wrap
- [x] Incremental filter optimization
- [x] 70 unit tests including security validation

## Milestone 9: UX Improvements ✅

- [x] Pause/resume auto-scroll (`p` key)
- [x] Fix auto-scroll on initial load
- [x] High-volume log handling
  - [x] Batch log line processing (up to 500 lines per frame)
  - [x] Throttled UI redraws (~60fps)
  - [x] Lines/sec throughput indicator in status bar

## Milestone 10: Settings Page ✅

- [x] Settings overlay (`S` key)
- [x] Theme selection (cycles through 11 themes)
- [x] Toggle level colors, line wrap, side panel
- [x] Auto-save to config file on change

## Milestone 11: UX Quick Wins ✅

- [x] Filter history (`↑`/`↓` in filter mode to browse last 50 filters)
- [x] Copy to clipboard (`y` yanks current line)
- [x] Line numbers toggle (`#` key)

## Milestone 12: Split View ✅

- [x] Split panes (up to 2 panes)
- [x] Vertical split (`Ctrl+W, v` - side-by-side)
- [x] Horizontal split (`Ctrl+W, s` - stacked)
- [x] Close pane (`Ctrl+W, q`)
- [x] Cycle panes (`Ctrl+W, w`)
- [x] Navigate panes (`Ctrl+W, h/j/k/l` - vim-style)
- [x] Independent scroll position per pane
- [x] Independent filter per pane
- [x] Independent bookmarks per pane
- [x] Independent source visibility per pane
- [x] Active pane indicator (highlighted border)
- [x] Pane indicator in status bar (`[1/2]`, `[2/2]`)

## Milestone 13: Discovery & Selection UX ✅

- [x] Auto-discovery mode (`bark` with no args opens picker)
- [x] `bark --docker` discovers all Docker containers
- [x] `bark --k8s` discovers all K8s pods
- [x] `bark --all` discovers all Docker + K8s sources
- [x] Picker shows already-added sources as checked
- [x] Deselect sources in picker to hide them
- [x] Click-to-select lines for yanking
  - [x] `▶` indicator on selected line
  - [x] `y` yanks selected line
  - [x] `Esc` clears selection
  - [x] Click in split switches pane focus
- [x] Tab cycles through split panes before sidebar

## Future Ideas
- [ ] Log correlation/linking between sources
- [ ] Named bookmark groups
- [ ] Custom theme definitions in config
- [ ] Plugin system for custom sources
