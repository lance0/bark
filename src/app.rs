//! Core application state and log processing logic.
//!
//! This module contains the main `AppState` struct that manages:
//! - Log line storage in a ring buffer
//! - Filtering and search functionality
//! - Bookmarks and navigation state
//! - UI mode and panel focus

use arboard::{Clipboard, Error as ClipboardError};
use crate::config::{Config, FILTER_DEBOUNCE_MS};
use crate::discovery::DiscoveredSource;
use crate::filter::{ActiveFilter, MatchRange, SavedFilter};
use crate::sources::LogSourceType;
use crate::theme::Theme;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::time::Instant;
use tui_textarea::TextArea;

/// Common timestamp formats to try parsing
const TIMESTAMP_FORMATS: &[&str] = &[
    "%Y-%m-%dT%H:%M:%S%.fZ",   // ISO 8601 with Z
    "%Y-%m-%dT%H:%M:%S%.f%:z", // ISO 8601 with offset
    "%Y-%m-%dT%H:%M:%S%:z",    // ISO 8601 without millis
    "%Y-%m-%dT%H:%M:%S%.f",    // ISO 8601 no timezone
    "%Y-%m-%dT%H:%M:%S",       // ISO 8601 basic
    "%Y-%m-%d %H:%M:%S%.f",    // Common log format with millis
    "%Y-%m-%d %H:%M:%S",       // Common log format
    "%d/%b/%Y:%H:%M:%S %z",    // Apache/nginx combined
    "%b %d %H:%M:%S",          // Syslog format
];

/// Try to parse a timestamp from the beginning of a line
fn parse_timestamp(line: &str) -> Option<DateTime<Local>> {
    // Extract the first ~35 characters which should contain any timestamp
    let prefix: String = line.chars().take(35).collect();

    for fmt in TIMESTAMP_FORMATS {
        // Try to parse with chrono
        if let Ok(dt) = NaiveDateTime::parse_from_str(&prefix, fmt) {
            return Local.from_local_datetime(&dt).single();
        }
        // Try parsing with timezone info
        if let Ok(dt) = DateTime::parse_from_str(&prefix, fmt) {
            return Some(dt.with_timezone(&Local));
        }
    }

    // Try to find a timestamp pattern anywhere in the first part of the line
    // Look for ISO-like patterns
    for word in prefix.split_whitespace().take(3) {
        for fmt in TIMESTAMP_FORMATS {
            if let Ok(dt) = NaiveDateTime::parse_from_str(word, fmt) {
                return Local.from_local_datetime(&dt).single();
            }
            if let Ok(dt) = DateTime::parse_from_str(word, fmt) {
                return Some(dt.with_timezone(&Local));
            }
        }
    }

    None
}

/// Format a duration as human-readable relative time
fn format_relative_time(dt: DateTime<Local>) -> String {
    let now = Local::now();
    let duration = now.signed_duration_since(dt);

    if duration.num_seconds() < 0 {
        return "future".to_string();
    }

    if duration.num_seconds() < 60 {
        return format!("{}s ago", duration.num_seconds());
    }
    if duration.num_minutes() < 60 {
        return format!("{}m ago", duration.num_minutes());
    }
    if duration.num_hours() < 24 {
        return format!("{}h ago", duration.num_hours());
    }
    if duration.num_days() < 7 {
        return format!("{}d ago", duration.num_days());
    }

    format!("{}w ago", duration.num_weeks())
}

/// Detected log level
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    None,
}

impl LogLevel {
    /// Detect log level from a line of text
    pub fn detect(line: &str) -> Self {
        let upper = line.to_uppercase();
        // Check for common log level patterns
        if upper.contains("ERROR") || upper.contains("[E]") || upper.contains("ERR]") {
            LogLevel::Error
        } else if upper.contains("WARN") || upper.contains("[W]") || upper.contains("WRN]") {
            LogLevel::Warn
        } else if upper.contains("INFO") || upper.contains("[I]") || upper.contains("INF]") {
            LogLevel::Info
        } else if upper.contains("DEBUG") || upper.contains("[D]") || upper.contains("DBG]") {
            LogLevel::Debug
        } else if upper.contains("TRACE") || upper.contains("[T]") || upper.contains("TRC]") {
            LogLevel::Trace
        } else {
            LogLevel::None
        }
    }
}

/// A single log line with optional cached rendering
pub struct LogLine {
    /// The raw log line as received (may contain ANSI codes)
    pub raw: String,
    /// Detected log level
    pub level: LogLevel,
    /// Whether the line contains ANSI escape codes
    pub has_ansi: bool,
    /// Parsed timestamp from the line
    pub timestamp: Option<DateTime<Local>>,
    /// Whether this line is valid JSON
    pub is_json: bool,
    /// Index of the source this line came from
    pub source_id: usize,
}

impl LogLine {
    pub fn new(raw: String) -> Self {
        let level = LogLevel::detect(&raw);
        let has_ansi = raw.contains('\x1b');
        let timestamp = parse_timestamp(&raw);
        let is_json = Self::detect_json(&raw);
        Self {
            raw,
            level,
            has_ansi,
            timestamp,
            is_json,
            source_id: 0,
        }
    }

    /// Set the source ID for this line
    pub fn with_source_id(mut self, source_id: usize) -> Self {
        self.source_id = source_id;
        self
    }

    /// Detect if a line is JSON
    fn detect_json(line: &str) -> bool {
        let trimmed = line.trim();
        (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    }

    /// Get relative time string if timestamp is available
    pub fn relative_time(&self) -> Option<String> {
        self.timestamp.map(format_relative_time)
    }
}

/// Input mode for the application
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    /// Normal navigation mode
    Normal,
    /// Editing the filter text
    FilterEditing,
    /// Selecting a source (future feature)
    #[allow(dead_code)]
    SourceSelect,
    /// Waiting for split command (after Ctrl+W)
    SplitCommand,
}

/// Split direction for dual-pane view
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SplitDirection {
    /// Single pane (no split)
    #[default]
    None,
    /// Vertical split (side-by-side)
    Vertical,
    /// Horizontal split (stacked)
    Horizontal,
}

/// Which panel has focus
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusedPanel {
    LogView,
    Sources,
    Filters,
}

/// View mode for multi-source display
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SourceViewMode {
    /// Show all visible sources merged chronologically
    #[default]
    AllMerged,
    /// Show only a single source
    SingleSource(usize),
}

/// Which picker is currently open
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerMode {
    Docker,
    K8s,
}

/// State for the container/pod picker overlay
#[derive(Debug)]
pub struct PickerState {
    /// Whether the picker is visible
    pub visible: bool,
    /// Which picker mode (Docker or K8s)
    pub mode: PickerMode,
    /// Discovered sources
    pub sources: Vec<DiscoveredSource>,
    /// Currently selected index
    pub selected: usize,
    /// Which items are checked for multi-select
    pub checked: Vec<bool>,
    /// Initial checked state (to detect removals)
    pub initial_checked: Vec<bool>,
    /// Loading state
    pub loading: bool,
    /// Error message if discovery failed
    pub error: Option<String>,
}

impl Default for PickerState {
    fn default() -> Self {
        Self {
            visible: false,
            mode: PickerMode::Docker,
            sources: Vec::new(),
            selected: 0,
            checked: Vec::new(),
            initial_checked: Vec::new(),
            loading: false,
            error: None,
        }
    }
}

impl PickerState {
    /// Open the picker with a specific mode
    pub fn open(&mut self, mode: PickerMode) {
        self.visible = true;
        self.mode = mode;
        self.sources.clear();
        self.selected = 0;
        self.checked.clear();
        self.initial_checked.clear();
        self.loading = true;
        self.error = None;
    }

    /// Close the picker
    pub fn close(&mut self) {
        self.visible = false;
        self.loading = false;
    }

    /// Set discovered sources, marking any that match existing sources as already checked
    pub fn set_sources(&mut self, sources: Vec<DiscoveredSource>, existing_sources: &[LogSourceType]) {
        self.checked = sources
            .iter()
            .map(|discovered| {
                // Check if this discovered source already exists in the app
                existing_sources.iter().any(|existing| match (existing, self.mode) {
                    (LogSourceType::Docker { container }, PickerMode::Docker) => {
                        container == &discovered.name
                    }
                    (LogSourceType::K8s { pod, namespace, .. }, PickerMode::K8s) => {
                        pod == &discovered.name && *namespace == discovered.namespace
                    }
                    _ => false,
                })
            })
            .collect();
        // Save initial state to detect removals later
        self.initial_checked = self.checked.clone();
        self.sources = sources;
        self.selected = 0;
        self.loading = false;
    }

    /// Get sources that were initially checked but are now unchecked (to be removed)
    pub fn get_unchecked_sources(&self) -> Vec<&DiscoveredSource> {
        self.sources
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                // Was initially checked but now unchecked
                self.initial_checked.get(*i).copied().unwrap_or(false)
                    && !self.checked.get(*i).copied().unwrap_or(false)
            })
            .map(|(_, s)| s)
            .collect()
    }

    /// Set error state
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.loading = false;
    }

    /// Navigate up
    pub fn up(&mut self) {
        if !self.sources.is_empty() && self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Navigate down
    pub fn down(&mut self) {
        if !self.sources.is_empty() && self.selected < self.sources.len() - 1 {
            self.selected += 1;
        }
    }

    /// Toggle checkbox on selected item
    pub fn toggle_selected(&mut self) {
        if let Some(checked) = self.checked.get_mut(self.selected) {
            *checked = !*checked;
        }
    }

}

/// State for the settings overlay
#[derive(Debug, Default)]
pub struct SettingsState {
    /// Whether the settings overlay is visible
    pub visible: bool,
    /// Currently selected setting index (0-3)
    pub selected: usize,
}

impl SettingsState {
    /// Number of settings available
    pub const COUNT: usize = 4;

    /// Open the settings overlay
    pub fn open(&mut self) {
        self.visible = true;
        self.selected = 0;
    }

    /// Close the settings overlay
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Navigate up
    pub fn up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Navigate down
    pub fn down(&mut self) {
        if self.selected < Self::COUNT - 1 {
            self.selected += 1;
        }
    }
}

/// State for a single log view pane
pub struct PaneState<'a> {
    // Scrolling
    /// Current scroll position (index into filtered_indices)
    pub scroll: usize,
    /// If true, auto-scroll to bottom on new lines
    pub stick_to_bottom: bool,
    /// Horizontal scroll offset (in characters) - used when line_wrap is false
    pub horizontal_scroll: usize,
    /// Last known viewport height (for auto-scroll calculations)
    pub viewport_height: usize,

    // Filtering
    /// Indices into lines that match the current filter
    pub filtered_indices: Vec<usize>,
    /// Currently active filter
    pub active_filter: Option<ActiveFilter>,
    /// Filter text input widget
    pub filter_textarea: TextArea<'a>,
    /// Whether filter is regex mode
    pub filter_is_regex: bool,
    /// Last time filter input changed (for debounce)
    pub filter_last_change: Option<Instant>,
    /// Whether we need to recompute filter (after debounce)
    pub filter_needs_recompute: bool,
    /// Current position in filter history when browsing (None = not browsing)
    pub filter_history_idx: Option<usize>,

    // Source visibility (per-pane)
    /// Which sources are currently visible (by index)
    pub visible_sources: Vec<bool>,
    /// View mode: show all sources merged or single source only
    pub view_mode: SourceViewMode,

    // Bookmarks (per-pane)
    /// Bookmarked line indices (into the lines buffer)
    pub bookmarks: Vec<usize>,

    // Line selection (for yank/click)
    /// Currently selected line index (into filtered_indices), if any
    pub selected_line: Option<usize>,
}

impl<'a> PaneState<'a> {
    /// Create a new pane with default state
    pub fn new(num_sources: usize) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        textarea.set_placeholder_text("type to filter...");
        textarea.set_placeholder_style(Style::default().fg(Color::DarkGray));

        Self {
            scroll: 0,
            stick_to_bottom: true,
            horizontal_scroll: 0,
            viewport_height: 20,
            filtered_indices: Vec::new(),
            active_filter: None,
            filter_textarea: textarea,
            filter_is_regex: false,
            filter_last_change: None,
            filter_needs_recompute: false,
            filter_history_idx: None,
            visible_sources: vec![true; num_sources],
            view_mode: SourceViewMode::default(),
            bookmarks: Vec::new(),
            selected_line: None,
        }
    }

    /// Clone pane state for creating a split (duplicates current view)
    pub fn clone_for_split(&self) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        textarea.set_placeholder_text("type to filter...");
        textarea.set_placeholder_style(Style::default().fg(Color::DarkGray));
        // Copy the current filter text if any
        let filter_text = self.filter_textarea.lines().join("\n");
        if !filter_text.is_empty() {
            textarea.insert_str(&filter_text);
        }

        Self {
            scroll: self.scroll,
            stick_to_bottom: self.stick_to_bottom,
            horizontal_scroll: self.horizontal_scroll,
            viewport_height: self.viewport_height,
            filtered_indices: self.filtered_indices.clone(),
            active_filter: self.active_filter.clone(),
            filter_textarea: textarea,
            filter_is_regex: self.filter_is_regex,
            filter_last_change: None,
            filter_needs_recompute: false,
            filter_history_idx: None,
            visible_sources: self.visible_sources.clone(),
            view_mode: self.view_mode,
            bookmarks: self.bookmarks.clone(),
            selected_line: None, // Don't copy selection to new pane
        }
    }

}

/// Main application state
pub struct AppState<'a> {
    // === Shared log data ===
    /// Ring buffer of log lines
    pub lines: VecDeque<LogLine>,
    /// Maximum lines to keep in the buffer
    pub max_lines: usize,
    /// Available log sources
    pub sources: Vec<LogSourceType>,

    // === Pane management ===
    /// Panes (1 or 2)
    pub panes: Vec<PaneState<'a>>,
    /// Currently active pane index (0 or 1)
    pub active_pane: usize,
    /// Split direction
    pub split_direction: SplitDirection,

    // === Global UI state ===
    /// Current input mode
    pub mode: InputMode,
    /// Which panel currently has focus
    pub focused_panel: FocusedPanel,
    /// Index of current/selected source (for Sources panel navigation)
    pub current_source_idx: usize,
    /// Saved filters
    pub saved_filters: Vec<SavedFilter>,
    /// Selected saved filter index (for navigation)
    pub selected_filter_idx: usize,
    /// Whether side panel is visible
    pub show_side_panel: bool,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Status message to display
    pub status_message: Option<String>,
    /// Whether to show help overlay
    pub show_help: bool,
    /// Picker state for adding sources at runtime
    pub picker: PickerState,
    /// Settings overlay state
    pub settings: SettingsState,
    /// Shared clipboard handle (kept alive to avoid X11 drops)
    pub clipboard: Option<Clipboard>,

    // === Display preferences (global) ===
    /// Whether to apply log level coloring (for lines without ANSI)
    pub level_colors_enabled: bool,
    /// Whether to wrap long lines
    pub line_wrap: bool,
    /// Whether to show relative timestamps
    pub show_relative_time: bool,
    /// Whether to pretty-print JSON logs
    pub json_pretty: bool,
    /// Whether to show line numbers in log view
    pub show_line_numbers: bool,
    /// Active color theme
    pub theme: Theme,
    /// Export directory for logs
    pub export_dir: String,

    // === Throughput tracking ===
    /// Lines received in the last second (for throughput display)
    pub lines_per_second: usize,
    /// Counter for lines in current second
    lines_this_second: usize,
    /// When the current second started
    throughput_second_start: Instant,

    // === Filter history (shared across panes) ===
    /// Filter history (recent filters)
    pub filter_history: Vec<String>,

    // === Layout tracking for mouse input ===
    /// Log view area(s) for mouse click handling - one per pane
    pub log_view_areas: Vec<Rect>,
}

impl<'a> AppState<'a> {
    pub fn new(config: &Config, sources: Vec<LogSourceType>) -> Self {
        let num_sources = sources.len();

        // Create initial pane
        let initial_pane = PaneState::new(num_sources);

        Self {
            // Shared log data
            lines: VecDeque::with_capacity(config.max_lines),
            max_lines: config.max_lines,
            sources,

            // Pane management - start with single pane
            panes: vec![initial_pane],
            active_pane: 0,
            split_direction: SplitDirection::None,

            // Global UI state
            mode: InputMode::Normal,
            focused_panel: FocusedPanel::LogView,
            current_source_idx: 0,
            saved_filters: Vec::new(),
            selected_filter_idx: 0,
            show_side_panel: config.show_side_panel,
            should_quit: false,
            status_message: None,
            show_help: false,
            picker: PickerState::default(),
            settings: SettingsState::default(),
            clipboard: None,

            // Display preferences
            level_colors_enabled: config.level_colors,
            line_wrap: config.line_wrap,
            show_relative_time: false,
            json_pretty: false,
            show_line_numbers: false,
            theme: config.get_theme(),
            export_dir: config.export_dir.clone(),

            // Throughput tracking
            lines_per_second: 0,
            lines_this_second: 0,
            throughput_second_start: Instant::now(),

            // Filter history
            filter_history: Vec::new(),

            // Layout tracking
            log_view_areas: vec![Rect::default()],
        }
    }

    /// Check if we're in split mode (have 2 panes)
    pub fn is_split(&self) -> bool {
        self.panes.len() > 1
    }

    /// Get or initialize a shared clipboard handle
    pub fn clipboard(&mut self) -> Result<&mut Clipboard, ClipboardError> {
        if self.clipboard.is_none() {
            self.clipboard = Some(Clipboard::new()?);
        }
        Ok(self.clipboard.as_mut().unwrap())
    }

    /// Create a vertical split (side-by-side panes)
    pub fn split_vertical(&mut self) {
        if self.is_split() {
            self.status_message = Some("Already split (close with Ctrl+W,q)".to_string());
            return;
        }
        let new_pane = self.panes[0].clone_for_split();
        self.panes.push(new_pane);
        self.log_view_areas.push(Rect::default());
        self.split_direction = SplitDirection::Vertical;
        self.active_pane = 1; // Focus the new pane
        self.status_message = Some("Vertical split created".to_string());
    }

    /// Create a horizontal split (stacked panes)
    pub fn split_horizontal(&mut self) {
        if self.is_split() {
            self.status_message = Some("Already split (close with Ctrl+W,q)".to_string());
            return;
        }
        let new_pane = self.panes[0].clone_for_split();
        self.panes.push(new_pane);
        self.log_view_areas.push(Rect::default());
        self.split_direction = SplitDirection::Horizontal;
        self.active_pane = 1; // Focus the new pane
        self.status_message = Some("Horizontal split created".to_string());
    }

    /// Close the current pane (returns to single-pane mode)
    pub fn close_pane(&mut self) {
        if !self.is_split() {
            self.status_message = Some("No split to close".to_string());
            return;
        }
        // Remove the non-active pane, or if active is 1, remove index 1
        if self.active_pane == 0 {
            self.panes.remove(1);
            self.log_view_areas.truncate(1);
        } else {
            self.panes.remove(1);
            self.log_view_areas.truncate(1);
            self.active_pane = 0;
        }
        self.split_direction = SplitDirection::None;
        self.status_message = Some("Split closed".to_string());
    }

    /// Cycle to the next pane
    pub fn cycle_pane(&mut self) {
        if !self.is_split() {
            return;
        }
        self.active_pane = if self.active_pane == 0 { 1 } else { 0 };
        self.status_message = Some(format!("Pane {}", self.active_pane + 1));
    }

    /// Focus left pane (for vertical split)
    pub fn focus_pane_left(&mut self) {
        if self.split_direction == SplitDirection::Vertical && self.active_pane == 1 {
            self.active_pane = 0;
            self.status_message = Some("Pane 1".to_string());
        }
    }

    /// Focus right pane (for vertical split)
    pub fn focus_pane_right(&mut self) {
        if self.split_direction == SplitDirection::Vertical && self.active_pane == 0 {
            self.active_pane = 1;
            self.status_message = Some("Pane 2".to_string());
        }
    }

    /// Focus up pane (for horizontal split)
    pub fn focus_pane_up(&mut self) {
        if self.split_direction == SplitDirection::Horizontal && self.active_pane == 1 {
            self.active_pane = 0;
            self.status_message = Some("Pane 1".to_string());
        }
    }

    /// Focus down pane (for horizontal split)
    pub fn focus_pane_down(&mut self) {
        if self.split_direction == SplitDirection::Horizontal && self.active_pane == 0 {
            self.active_pane = 1;
            self.status_message = Some("Pane 2".to_string());
        }
    }

    /// Toggle bookmark at current scroll position
    pub fn toggle_bookmark(&mut self) {
        if self.panes[self.active_pane].filtered_indices.is_empty() {
            return;
        }

        // Get the actual line index at current scroll position
        let line_idx = self.panes[self.active_pane].filtered_indices[self.panes[self.active_pane].scroll];

        if let Some(pos) = self.panes[self.active_pane].bookmarks.iter().position(|&b| b == line_idx) {
            self.panes[self.active_pane].bookmarks.remove(pos);
            self.status_message = Some("Bookmark removed".to_string());
        } else {
            self.panes[self.active_pane].bookmarks.push(line_idx);
            self.panes[self.active_pane].bookmarks.sort_unstable();
            self.status_message = Some("Bookmark added".to_string());
        }
    }

    /// Jump to next bookmark
    pub fn next_bookmark(&mut self) {
        if self.panes[self.active_pane].bookmarks.is_empty() {
            self.status_message = Some("No bookmarks".to_string());
            return;
        }

        let current_line_idx = self.panes[self.active_pane].filtered_indices.get(self.panes[self.active_pane].scroll).copied().unwrap_or(0);

        // Find next bookmark after current position
        if let Some(&next_bookmark) = self.panes[self.active_pane].bookmarks.iter().find(|&&b| b > current_line_idx) {
            // Find this bookmark in filtered_indices
            if let Some(scroll_pos) = self.panes[self.active_pane]
                .filtered_indices
                .iter()
                .position(|&i| i == next_bookmark)
            {
                self.panes[self.active_pane].scroll = scroll_pos;
                self.panes[self.active_pane].stick_to_bottom = false;
                self.status_message = Some(format!(
                    "Bookmark {}/{}",
                    self.panes[self.active_pane].bookmarks
                        .iter()
                        .position(|&b| b == next_bookmark)
                        .unwrap()
                        + 1,
                    self.panes[self.active_pane].bookmarks.len()
                ));
                return;
            }
        }

        // Wrap around to first bookmark
        if let Some(&first_bookmark) = self.panes[self.active_pane].bookmarks.first() {
            if let Some(scroll_pos) = self.panes[self.active_pane]
                .filtered_indices
                .iter()
                .position(|&i| i == first_bookmark)
            {
                self.panes[self.active_pane].scroll = scroll_pos;
                self.panes[self.active_pane].stick_to_bottom = false;
                self.status_message =
                    Some(format!("Bookmark 1/{} (wrapped)", self.panes[self.active_pane].bookmarks.len()));
            }
        }
    }

    /// Jump to previous bookmark
    pub fn prev_bookmark(&mut self) {
        if self.panes[self.active_pane].bookmarks.is_empty() {
            self.status_message = Some("No bookmarks".to_string());
            return;
        }

        let current_line_idx = self.panes[self.active_pane].filtered_indices.get(self.panes[self.active_pane].scroll).copied().unwrap_or(0);

        // Find previous bookmark before current position
        if let Some(&prev_bookmark) = self.panes[self.active_pane].bookmarks.iter().rev().find(|&&b| b < current_line_idx) {
            // Find this bookmark in filtered_indices
            if let Some(scroll_pos) = self.panes[self.active_pane]
                .filtered_indices
                .iter()
                .position(|&i| i == prev_bookmark)
            {
                self.panes[self.active_pane].scroll = scroll_pos;
                self.panes[self.active_pane].stick_to_bottom = false;
                self.status_message = Some(format!(
                    "Bookmark {}/{}",
                    self.panes[self.active_pane].bookmarks
                        .iter()
                        .position(|&b| b == prev_bookmark)
                        .unwrap()
                        + 1,
                    self.panes[self.active_pane].bookmarks.len()
                ));
                return;
            }
        }

        // Wrap around to last bookmark
        if let Some(&last_bookmark) = self.panes[self.active_pane].bookmarks.last() {
            if let Some(scroll_pos) = self.panes[self.active_pane]
                .filtered_indices
                .iter()
                .position(|&i| i == last_bookmark)
            {
                self.panes[self.active_pane].scroll = scroll_pos;
                self.panes[self.active_pane].stick_to_bottom = false;
                self.status_message = Some(format!(
                    "Bookmark {}/{} (wrapped)",
                    self.panes[self.active_pane].bookmarks.len(),
                    self.panes[self.active_pane].bookmarks.len()
                ));
            }
        }
    }

    /// Check if a line index is bookmarked
    #[allow(dead_code)]
    pub fn is_bookmarked(&self, line_idx: usize) -> bool {
        self.panes[self.active_pane].bookmarks.contains(&line_idx)
    }

    /// Toggle JSON pretty-printing
    pub fn toggle_json_pretty(&mut self) {
        self.json_pretty = !self.json_pretty;
        self.status_message = Some(format!(
            "JSON pretty-print: {}",
            if self.json_pretty { "on" } else { "off" }
        ));
    }

    /// Toggle relative timestamp display
    pub fn toggle_relative_time(&mut self) {
        self.show_relative_time = !self.show_relative_time;
        self.status_message = Some(format!(
            "Relative time: {}",
            if self.show_relative_time { "on" } else { "off" }
        ));
    }

    /// Toggle log level coloring
    pub fn toggle_level_colors(&mut self) {
        self.level_colors_enabled = !self.level_colors_enabled;
        self.status_message = Some(format!(
            "Level colors: {}",
            if self.level_colors_enabled {
                "on"
            } else {
                "off"
            }
        ));
    }

    /// Toggle line wrapping
    pub fn toggle_line_wrap(&mut self) {
        self.line_wrap = !self.line_wrap;
        // Reset horizontal scroll when enabling wrapping
        if self.line_wrap {
            self.panes[self.active_pane].horizontal_scroll = 0;
        }
        self.status_message = Some(format!(
            "Line wrap: {}",
            if self.line_wrap { "on" } else { "off" }
        ));
    }

    /// Toggle line numbers display
    pub fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
        self.status_message = Some(format!(
            "Line numbers: {}",
            if self.show_line_numbers { "on" } else { "off" }
        ));
    }

    /// Maximum number of filters to keep in history
    const MAX_FILTER_HISTORY: usize = 50;

    /// Add a filter to history (called when filter is applied)
    pub fn add_to_filter_history(&mut self, pattern: String) {
        if pattern.is_empty() {
            return;
        }
        // Remove duplicates - if this pattern exists, remove it first
        self.filter_history.retain(|p| p != &pattern);
        // Add to front of history
        self.filter_history.insert(0, pattern);
        // Trim to max size
        if self.filter_history.len() > Self::MAX_FILTER_HISTORY {
            self.filter_history.truncate(Self::MAX_FILTER_HISTORY);
        }
        // Reset history browsing position
        self.panes[self.active_pane].filter_history_idx = None;
    }

    /// Navigate up in filter history (older filters)
    /// Returns true if history was navigated
    pub fn filter_history_up(&mut self) -> bool {
        if self.filter_history.is_empty() {
            return false;
        }
        match self.panes[self.active_pane].filter_history_idx {
            None => {
                // Start browsing from most recent
                self.panes[self.active_pane].filter_history_idx = Some(0);
                self.set_filter_text(&self.filter_history[0].clone());
                true
            }
            Some(idx) if idx + 1 < self.filter_history.len() => {
                // Go to older entry
                let new_idx = idx + 1;
                self.panes[self.active_pane].filter_history_idx = Some(new_idx);
                self.set_filter_text(&self.filter_history[new_idx].clone());
                true
            }
            _ => false, // Already at oldest
        }
    }

    /// Navigate down in filter history (newer filters)
    /// Returns true if history was navigated
    pub fn filter_history_down(&mut self) -> bool {
        match self.panes[self.active_pane].filter_history_idx {
            Some(0) => {
                // Back to empty/current input
                self.panes[self.active_pane].filter_history_idx = None;
                self.set_filter_text("");
                true
            }
            Some(idx) => {
                // Go to newer entry
                let new_idx = idx - 1;
                self.panes[self.active_pane].filter_history_idx = Some(new_idx);
                self.set_filter_text(&self.filter_history[new_idx].clone());
                true
            }
            None => false, // Not browsing history
        }
    }

    /// Set filter textarea text (helper for history navigation)
    fn set_filter_text(&mut self, text: &str) {
        self.panes[self.active_pane].filter_textarea.select_all();
        self.panes[self.active_pane].filter_textarea.cut();
        self.panes[self.active_pane].filter_textarea.insert_str(text);
    }

    /// Get the raw text of the selected line (if any) or the line at scroll position
    pub fn get_current_line_text(&self) -> Option<String> {
        if self.panes[self.active_pane].filtered_indices.is_empty() {
            return None;
        }
        // Prefer selected line, otherwise use scroll position
        let idx = self.panes[self.active_pane]
            .selected_line
            .unwrap_or(self.panes[self.active_pane].scroll);
        let line_idx = self.panes[self.active_pane].filtered_indices.get(idx)?;
        self.lines.get(*line_idx).map(|l| l.raw.clone())
    }

    /// Select a line at the given viewport-relative row (0 = top of log view)
    pub fn select_line_at_row(&mut self, row: usize) {
        let pane = &mut self.panes[self.active_pane];
        let target_idx = pane.scroll + row;
        if target_idx < pane.filtered_indices.len() {
            pane.selected_line = Some(target_idx);
        }
    }

    /// Clear line selection
    pub fn clear_selection(&mut self) {
        self.panes[self.active_pane].selected_line = None;
    }

    /// Available theme names in cycle order
    const THEMES: &'static [&'static str] = &[
        "default", "kawaii", "cyber", "dracula", "monochrome",
        "matrix", "nord", "gruvbox", "catppuccin", "tokyo_night", "solarized",
    ];

    /// Cycle to the next theme
    pub fn cycle_theme(&mut self) {
        let current = self.theme.name();
        let idx = Self::THEMES.iter().position(|&t| t == current).unwrap_or(0);
        let next = Self::THEMES[(idx + 1) % Self::THEMES.len()];
        self.theme = Theme::by_name(next);
        self.status_message = Some(format!("Theme: {}", next));
    }

    /// Save current display settings to config file
    pub fn save_settings(&mut self) {
        let config = Config {
            theme: self.theme.name().to_string(),
            level_colors: self.level_colors_enabled,
            line_wrap: self.line_wrap,
            show_side_panel: self.show_side_panel,
            ..Config::default()
        };
        match config.save() {
            Ok(()) => {
                self.status_message = Some("Settings saved".to_string());
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to save: {}", e));
            }
        }
    }

    /// Toggle a setting by index (used by settings overlay)
    pub fn toggle_setting(&mut self, index: usize) {
        match index {
            0 => self.cycle_theme(),
            1 => {
                self.level_colors_enabled = !self.level_colors_enabled;
                self.status_message = Some(format!(
                    "Level colors: {}",
                    if self.level_colors_enabled { "on" } else { "off" }
                ));
            }
            2 => {
                self.line_wrap = !self.line_wrap;
                if self.line_wrap {
                    self.panes[self.active_pane].horizontal_scroll = 0;
                }
                self.status_message = Some(format!(
                    "Line wrap: {}",
                    if self.line_wrap { "on" } else { "off" }
                ));
            }
            3 => {
                self.show_side_panel = !self.show_side_panel;
                if !self.show_side_panel {
                    self.focused_panel = FocusedPanel::LogView;
                }
                self.status_message = Some(format!(
                    "Side panel: {}",
                    if self.show_side_panel { "on" } else { "off" }
                ));
            }
            _ => {}
        }
        self.save_settings();
    }

    /// Scroll left (when line wrap is off)
    pub fn scroll_left(&mut self) {
        if !self.line_wrap && self.panes[self.active_pane].horizontal_scroll > 0 {
            self.panes[self.active_pane].horizontal_scroll = self.panes[self.active_pane].horizontal_scroll.saturating_sub(4);
        }
    }

    /// Scroll right (when line wrap is off)
    pub fn scroll_right(&mut self) {
        if !self.line_wrap {
            self.panes[self.active_pane].horizontal_scroll += 4;
        }
    }

    /// Scroll left by a larger amount
    pub fn scroll_left_large(&mut self) {
        if !self.line_wrap && self.panes[self.active_pane].horizontal_scroll > 0 {
            self.panes[self.active_pane].horizontal_scroll = self.panes[self.active_pane].horizontal_scroll.saturating_sub(20);
        }
    }

    /// Scroll right by a larger amount
    pub fn scroll_right_large(&mut self) {
        if !self.line_wrap {
            self.panes[self.active_pane].horizontal_scroll += 20;
        }
    }

    /// Reset horizontal scroll to beginning
    pub fn scroll_home(&mut self) {
        self.panes[self.active_pane].horizontal_scroll = 0;
    }

    /// Export filtered (or all) lines to a file
    pub fn export_lines(&self, path: &str) -> Result<usize, String> {
        let mut file = File::create(path).map_err(|e| e.to_string())?;

        let mut count = 0;
        for &idx in &self.panes[self.active_pane].filtered_indices {
            if let Some(line) = self.lines.get(idx) {
                writeln!(file, "{}", line.raw).map_err(|e| e.to_string())?;
                count += 1;
            }
        }

        Ok(count)
    }

    /// Generate default export filename
    pub fn default_export_path(&self) -> String {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        format!("{}/bark_export_{}.log", self.export_dir, timestamp)
    }

    /// Get the current source (if any)
    pub fn current_source(&self) -> Option<&LogSourceType> {
        self.sources.get(self.current_source_idx)
    }

    /// Add a new source at runtime
    #[allow(dead_code)]
    pub fn add_source(&mut self, source: LogSourceType) {
        self.sources.push(source);
        self.panes[self.active_pane].visible_sources.push(true);
    }

    /// Save the current filter with a name
    pub fn save_current_filter(&mut self, name: String) {
        if let Some(ref filter) = self.panes[self.active_pane].active_filter {
            self.saved_filters.push(SavedFilter {
                name,
                pattern: filter.pattern.clone(),
                is_regex: filter.is_regex,
            });
            self.status_message = Some("Filter saved".to_string());
        }
    }

    /// Apply a saved filter by index
    pub fn apply_saved_filter(&mut self, idx: usize) {
        if let Some(saved) = self.saved_filters.get(idx) {
            let pattern = saved.pattern.clone();
            let is_regex = saved.is_regex;
            let name = saved.name.clone();

            self.panes[self.active_pane].filter_textarea = TextArea::new(vec![pattern.clone()]);
            self.panes[self.active_pane].filter_textarea.set_cursor_line_style(Style::default());
            self.panes[self.active_pane].filter_is_regex = is_regex;
            self.panes[self.active_pane].active_filter = Some(ActiveFilter::new(pattern, is_regex));
            self.recompute_filter();
            self.status_message = Some(format!("Applied filter: {}", name));
        }
    }

    /// Toggle side panel visibility
    pub fn toggle_side_panel(&mut self) {
        self.show_side_panel = !self.show_side_panel;
        if !self.show_side_panel {
            self.focused_panel = FocusedPanel::LogView;
        }
    }

    /// Cycle focus between panels
    pub fn cycle_focus(&mut self) {
        // When focused on LogView with splits, cycle through panes first
        if self.focused_panel == FocusedPanel::LogView && self.is_split() {
            // If we're on pane 0, go to pane 1
            if self.active_pane == 0 {
                self.active_pane = 1;
                return;
            }
            // If we're on pane 1 and side panel is visible, go to Sources
            // Otherwise cycle back to pane 0
            if self.show_side_panel {
                self.active_pane = 0; // Reset to pane 0 when leaving LogView
                self.focused_panel = FocusedPanel::Sources;
            } else {
                self.active_pane = 0;
            }
            return;
        }

        if !self.show_side_panel {
            // No side panel - just cycle panes if split
            if self.is_split() {
                self.active_pane = if self.active_pane == 0 { 1 } else { 0 };
            }
            return;
        }

        self.focused_panel = match self.focused_panel {
            FocusedPanel::LogView => FocusedPanel::Sources,
            FocusedPanel::Sources => FocusedPanel::Filters,
            FocusedPanel::Filters => FocusedPanel::LogView,
        };
    }

    /// Push multiple log lines efficiently (batched)
    /// Only updates scroll position once at the end
    /// Updates ALL panes (not just active) so split view stays in sync
    pub fn push_lines(&mut self, lines: Vec<LogLine>) {
        let count = lines.len();
        if count == 0 {
            return;
        }

        let num_panes = self.panes.len();

        for line in lines {
            // If buffer is full, remove oldest line
            if self.lines.len() >= self.max_lines {
                self.lines.pop_front();

                // Adjust bookmark and filtered indices for ALL panes
                for pane in &mut self.panes {
                    pane.bookmarks.retain_mut(|idx| {
                        if *idx == 0 {
                            return false;
                        }
                        *idx -= 1;
                        true
                    });

                    pane.filtered_indices.retain_mut(|idx| {
                        if *idx == 0 {
                            return false;
                        }
                        *idx -= 1;
                        true
                    });
                }
            }

            let line_index = self.lines.len();
            self.lines.push_back(line);

            // Check if the new line matches the filter for EACH pane
            for pane_idx in 0..num_panes {
                if self.matches_filter_for_pane(pane_idx, line_index) {
                    self.panes[pane_idx].filtered_indices.push(line_index);
                }
            }
        }

        // Update scroll for ALL panes
        for pane in &mut self.panes {
            if pane.stick_to_bottom && !pane.filtered_indices.is_empty() {
                pane.scroll = pane
                    .filtered_indices
                    .len()
                    .saturating_sub(pane.viewport_height);
            }

            // Adjust scroll if it's now out of bounds
            if !pane.filtered_indices.is_empty() {
                pane.scroll = pane.scroll.min(pane.filtered_indices.len() - 1);
            } else {
                pane.scroll = 0;
            }
        }

        // Update throughput tracking
        self.track_throughput(count);
    }

    /// Update throughput tracking
    fn track_throughput(&mut self, count: usize) {
        let elapsed = self.throughput_second_start.elapsed();
        if elapsed.as_secs() >= 1 {
            // New second started - save count and reset
            self.lines_per_second = self.lines_this_second;
            self.lines_this_second = count;
            self.throughput_second_start = Instant::now();
        } else {
            self.lines_this_second += count;
        }
    }

    /// Check if a line at the given index matches the current filter (active pane)
    fn matches_filter(&self, index: usize) -> bool {
        self.matches_filter_for_pane(self.active_pane, index)
    }

    /// Check if a line at the given index matches the filter for a specific pane
    fn matches_filter_for_pane(&self, pane_idx: usize, index: usize) -> bool {
        let line = match self.lines.get(index) {
            Some(l) => l,
            None => return false,
        };

        let pane = match self.panes.get(pane_idx) {
            Some(p) => p,
            None => return false,
        };

        // Check source visibility
        if !pane
            .visible_sources
            .get(line.source_id)
            .copied()
            .unwrap_or(true)
        {
            return false;
        }

        // Check view mode
        match pane.view_mode {
            SourceViewMode::AllMerged => {}
            SourceViewMode::SingleSource(id) if id != line.source_id => return false,
            _ => {}
        }

        // Check text filter
        match &pane.active_filter {
            None => true,
            Some(filter) => filter.matches(&line.raw),
        }
    }

    /// Recompute filtered_indices based on current filter
    pub fn recompute_filter(&mut self) {
        self.panes[self.active_pane].filtered_indices.clear();
        for i in 0..self.lines.len() {
            if self.matches_filter(i) {
                self.panes[self.active_pane].filtered_indices.push(i);
            }
        }

        // Adjust scroll if it's now out of bounds
        if !self.panes[self.active_pane].filtered_indices.is_empty() {
            self.panes[self.active_pane].scroll = self.panes[self.active_pane].scroll.min(self.panes[self.active_pane].filtered_indices.len() - 1);
        } else {
            self.panes[self.active_pane].scroll = 0;
        }
    }

    /// Scroll up by one line
    pub fn scroll_up(&mut self) {
        if self.panes[self.active_pane].scroll > 0 {
            self.panes[self.active_pane].scroll -= 1;
            self.panes[self.active_pane].stick_to_bottom = false;
        }
    }

    /// Scroll down by one line
    pub fn scroll_down(&mut self) {
        if !self.panes[self.active_pane].filtered_indices.is_empty() && self.panes[self.active_pane].scroll < self.panes[self.active_pane].filtered_indices.len() - 1 {
            self.panes[self.active_pane].scroll += 1;
        }
    }

    /// Scroll up by a page
    pub fn scroll_page_up(&mut self, page_size: usize) {
        self.panes[self.active_pane].scroll = self.panes[self.active_pane].scroll.saturating_sub(page_size);
        self.panes[self.active_pane].stick_to_bottom = false;
    }

    /// Scroll down by a page
    pub fn scroll_page_down(&mut self, page_size: usize) {
        if !self.panes[self.active_pane].filtered_indices.is_empty() {
            self.panes[self.active_pane].scroll = (self.panes[self.active_pane].scroll + page_size).min(self.panes[self.active_pane].filtered_indices.len() - 1);
        }
    }

    /// Go to the top of the log
    pub fn go_to_top(&mut self) {
        self.panes[self.active_pane].scroll = 0;
        self.panes[self.active_pane].stick_to_bottom = false;
    }

    /// Go to the bottom of the log and enable stick_to_bottom
    pub fn go_to_bottom(&mut self) {
        if !self.panes[self.active_pane].filtered_indices.is_empty() {
            self.panes[self.active_pane].scroll = self.panes[self.active_pane].filtered_indices.len() - 1;
        }
        self.panes[self.active_pane].stick_to_bottom = true;
    }

    /// Go to next matching line (when filter is active)
    pub fn next_match(&mut self) {
        if self.panes[self.active_pane].filtered_indices.is_empty() {
            return;
        }
        if self.panes[self.active_pane].scroll < self.panes[self.active_pane].filtered_indices.len() - 1 {
            self.panes[self.active_pane].scroll += 1;
            self.panes[self.active_pane].stick_to_bottom = false;
            self.status_message = Some(format!(
                "Match {}/{}",
                self.panes[self.active_pane].scroll + 1,
                self.panes[self.active_pane].filtered_indices.len()
            ));
        } else {
            // Wrap to beginning
            self.panes[self.active_pane].scroll = 0;
            self.panes[self.active_pane].stick_to_bottom = false;
            self.status_message = Some(format!(
                "Match {}/{} (wrapped)",
                self.panes[self.active_pane].scroll + 1,
                self.panes[self.active_pane].filtered_indices.len()
            ));
        }
    }

    /// Go to previous matching line (when filter is active)
    pub fn prev_match(&mut self) {
        if self.panes[self.active_pane].filtered_indices.is_empty() {
            return;
        }
        if self.panes[self.active_pane].scroll > 0 {
            self.panes[self.active_pane].scroll -= 1;
            self.panes[self.active_pane].stick_to_bottom = false;
            self.status_message = Some(format!(
                "Match {}/{}",
                self.panes[self.active_pane].scroll + 1,
                self.panes[self.active_pane].filtered_indices.len()
            ));
        } else {
            // Wrap to end
            self.panes[self.active_pane].scroll = self.panes[self.active_pane].filtered_indices.len() - 1;
            self.panes[self.active_pane].stick_to_bottom = false;
            self.status_message = Some(format!(
                "Match {}/{} (wrapped)",
                self.panes[self.active_pane].scroll + 1,
                self.panes[self.active_pane].filtered_indices.len()
            ));
        }
    }

    /// Get match ranges for a line (for highlighting)
    pub fn get_match_ranges(&self, line: &str) -> Vec<MatchRange> {
        if let Some(ref filter) = self.panes[self.active_pane].active_filter {
            filter.find_matches(line)
        } else {
            Vec::new()
        }
    }

    /// Get the current filter input text
    pub fn filter_input(&self) -> String {
        self.panes[self.active_pane].filter_textarea.lines().join("\n")
    }

    /// Apply the current filter input as the active filter
    pub fn apply_filter(&mut self) {
        let input = self.filter_input();
        if input.is_empty() {
            self.panes[self.active_pane].active_filter = None;
        } else {
            // Add to history before applying
            self.add_to_filter_history(input.clone());
            self.panes[self.active_pane].active_filter = Some(ActiveFilter::new(input, self.panes[self.active_pane].filter_is_regex));
        }
        self.recompute_filter();
        self.mode = InputMode::Normal;
        self.panes[self.active_pane].filter_last_change = None;
        self.panes[self.active_pane].filter_needs_recompute = false;
        self.panes[self.active_pane].filter_history_idx = None; // Reset history browsing
    }

    /// Cancel filter editing and revert to previous state
    pub fn cancel_filter(&mut self) {
        // Restore textarea to previous filter
        let prev = self.panes[self.active_pane]
            .active_filter
            .as_ref()
            .map(|f| f.pattern.clone())
            .unwrap_or_default();
        self.panes[self.active_pane].filter_textarea = TextArea::new(vec![prev]);
        self.panes[self.active_pane].filter_textarea.set_cursor_line_style(Style::default());
        self.mode = InputMode::Normal;
        self.panes[self.active_pane].filter_last_change = None;
        self.panes[self.active_pane].filter_needs_recompute = false;
        self.panes[self.active_pane].filter_history_idx = None; // Reset history browsing
    }

    /// Mark that filter input changed (for debounce)
    pub fn filter_changed(&mut self) {
        self.panes[self.active_pane].filter_last_change = Some(Instant::now());
        self.panes[self.active_pane].filter_needs_recompute = true;
    }

    /// Check if debounce period has passed and recompute if needed
    pub fn check_filter_debounce(&mut self) {
        if let Some(last_change) = self.panes[self.active_pane].filter_last_change {
            if last_change.elapsed().as_millis() >= FILTER_DEBOUNCE_MS
                && self.panes[self.active_pane].filter_needs_recompute
            {
                // Apply filter without changing mode
                let input = self.filter_input();
                if input.is_empty() {
                    self.panes[self.active_pane].active_filter = None;
                } else {
                    self.panes[self.active_pane].active_filter = Some(ActiveFilter::new(input, self.panes[self.active_pane].filter_is_regex));
                }
                self.recompute_filter();
                self.panes[self.active_pane].filter_needs_recompute = false;
            }
        }
    }

    /// Toggle regex mode for filtering
    pub fn toggle_regex_mode(&mut self) {
        self.panes[self.active_pane].filter_is_regex = !self.panes[self.active_pane].filter_is_regex;
        if self.panes[self.active_pane].active_filter.is_some() {
            // Reapply filter with new mode
            let input = self.filter_input();
            self.panes[self.active_pane].active_filter = Some(ActiveFilter::new(input, self.panes[self.active_pane].filter_is_regex));
            self.recompute_filter();
        }
        self.status_message = Some(format!(
            "Filter mode: {}",
            if self.panes[self.active_pane].filter_is_regex {
                "regex"
            } else {
                "substring"
            }
        ));
    }

    /// Get total and visible line counts
    pub fn line_counts(&self) -> (usize, usize) {
        (self.lines.len(), self.panes[self.active_pane].filtered_indices.len())
    }

    /// Get visible lines for a specific pane
    pub fn visible_lines_for_pane(&mut self, pane_idx: usize, height: usize) -> Vec<(usize, &LogLine)> {
        if pane_idx >= self.panes.len() {
            return Vec::new();
        }

        let height_changed = self.panes[pane_idx].viewport_height != height;
        self.panes[pane_idx].viewport_height = height;

        if self.panes[pane_idx].filtered_indices.is_empty() {
            return Vec::new();
        }

        if height_changed && self.panes[pane_idx].stick_to_bottom {
            self.panes[pane_idx].scroll = self.panes[pane_idx]
                .filtered_indices
                .len()
                .saturating_sub(self.panes[pane_idx].viewport_height);
        }

        let start = self.panes[pane_idx].scroll;
        let end = (start + height).min(self.panes[pane_idx].filtered_indices.len());

        self.panes[pane_idx].filtered_indices[start..end]
            .iter()
            .enumerate()
            .filter_map(|(i, &line_idx)| {
                self.lines.get(line_idx).map(|line| (start + i, line))
            })
            .collect()
    }

    /// Get line counts for a specific pane
    pub fn line_counts_for_pane(&self, pane_idx: usize) -> (usize, usize) {
        if pane_idx >= self.panes.len() {
            return (self.lines.len(), 0);
        }
        (self.lines.len(), self.panes[pane_idx].filtered_indices.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // LogLevel::detect() tests

    #[test]
    fn test_detect_error_level() {
        assert_eq!(LogLevel::detect("ERROR: something failed"), LogLevel::Error);
        assert_eq!(LogLevel::detect("error: lowercase"), LogLevel::Error);
        assert_eq!(LogLevel::detect("Error: mixed case"), LogLevel::Error);
    }

    #[test]
    fn test_detect_error_bracket_patterns() {
        assert_eq!(LogLevel::detect("[E] some message"), LogLevel::Error);
        assert_eq!(LogLevel::detect("2024-01-01 [ERR] failed"), LogLevel::Error);
    }

    #[test]
    fn test_detect_warn_level() {
        assert_eq!(LogLevel::detect("WARN: something happened"), LogLevel::Warn);
        assert_eq!(LogLevel::detect("WARNING: be careful"), LogLevel::Warn);
        assert_eq!(LogLevel::detect("[W] warning message"), LogLevel::Warn);
        assert_eq!(LogLevel::detect("[WRN] another warning"), LogLevel::Warn);
    }

    #[test]
    fn test_detect_info_level() {
        assert_eq!(LogLevel::detect("INFO: informational"), LogLevel::Info);
        assert_eq!(LogLevel::detect("[I] info bracket"), LogLevel::Info);
        assert_eq!(LogLevel::detect("[INF] info message"), LogLevel::Info);
    }

    #[test]
    fn test_detect_debug_level() {
        assert_eq!(LogLevel::detect("DEBUG: some message"), LogLevel::Debug);
        assert_eq!(LogLevel::detect("[D] debug bracket"), LogLevel::Debug);
        assert_eq!(LogLevel::detect("[DBG] debug msg"), LogLevel::Debug);
    }

    #[test]
    fn test_detect_trace_level() {
        assert_eq!(LogLevel::detect("TRACE: some message"), LogLevel::Trace);
        assert_eq!(LogLevel::detect("[T] trace bracket"), LogLevel::Trace);
        assert_eq!(LogLevel::detect("[TRC] trace msg"), LogLevel::Trace);
    }

    #[test]
    fn test_detect_none_level() {
        assert_eq!(LogLevel::detect("just a regular line"), LogLevel::None);
        assert_eq!(LogLevel::detect("no level here"), LogLevel::None);
        assert_eq!(LogLevel::detect(""), LogLevel::None);
    }

    // LogLine::detect_json() tests

    #[test]
    fn test_detect_json_object() {
        assert!(LogLine::detect_json(r#"{"key": "value"}"#));
        assert!(LogLine::detect_json(r#"  {"key": "value"}  "#)); // with whitespace
    }

    #[test]
    fn test_detect_json_array() {
        assert!(LogLine::detect_json(r#"[1, 2, 3]"#));
        assert!(LogLine::detect_json(r#"  ["a", "b"]  "#));
    }

    #[test]
    fn test_detect_json_not_json() {
        assert!(!LogLine::detect_json("just plain text"));
        assert!(!LogLine::detect_json("{incomplete"));
        assert!(!LogLine::detect_json("[incomplete"));
        assert!(!LogLine::detect_json("starts with { but ends wrong"));
    }

    // parse_timestamp() tests

    #[test]
    fn test_parse_timestamp_iso8601() {
        let result = parse_timestamp("2024-01-15T10:30:00 some log message");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_timestamp_iso8601_with_millis() {
        let result = parse_timestamp("2024-01-15T10:30:00.123 some log message");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_timestamp_with_space_separator() {
        // Note: This format requires the timestamp to be extractable
        // The parser tries to find timestamps in the first 35 chars
        let result = parse_timestamp("2024-01-15T10:30:00 INFO some log message");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_timestamp_none() {
        let result = parse_timestamp("no timestamp here");
        assert!(result.is_none());
    }

    // format_relative_time() tests

    #[test]
    fn test_format_relative_time_seconds() {
        let now = Local::now();
        let past = now - chrono::Duration::seconds(30);
        let result = format_relative_time(past);
        assert!(result.contains("s ago"));
    }

    #[test]
    fn test_format_relative_time_minutes() {
        let now = Local::now();
        let past = now - chrono::Duration::minutes(5);
        let result = format_relative_time(past);
        assert!(result.contains("m ago"));
    }

    #[test]
    fn test_format_relative_time_hours() {
        let now = Local::now();
        let past = now - chrono::Duration::hours(3);
        let result = format_relative_time(past);
        assert!(result.contains("h ago"));
    }

    #[test]
    fn test_format_relative_time_days() {
        let now = Local::now();
        let past = now - chrono::Duration::days(2);
        let result = format_relative_time(past);
        assert!(result.contains("d ago"));
    }

    #[test]
    fn test_format_relative_time_weeks() {
        let now = Local::now();
        let past = now - chrono::Duration::weeks(2);
        let result = format_relative_time(past);
        assert!(result.contains("w ago"));
    }

    #[test]
    fn test_format_relative_time_future() {
        let now = Local::now();
        let future = now + chrono::Duration::hours(1);
        let result = format_relative_time(future);
        assert_eq!(result, "future");
    }

    // LogLine::new() tests

    #[test]
    fn test_logline_detects_level() {
        let line = LogLine::new("ERROR: something failed".to_string());
        assert_eq!(line.level, LogLevel::Error);
    }

    #[test]
    fn test_logline_detects_ansi() {
        let line = LogLine::new("\x1b[31mred text\x1b[0m".to_string());
        assert!(line.has_ansi);

        let line_no_ansi = LogLine::new("plain text".to_string());
        assert!(!line_no_ansi.has_ansi);
    }

    #[test]
    fn test_logline_detects_json() {
        let line = LogLine::new(r#"{"level": "error", "msg": "failed"}"#.to_string());
        assert!(line.is_json);

        let line_not_json = LogLine::new("plain text".to_string());
        assert!(!line_not_json.is_json);
    }
}
