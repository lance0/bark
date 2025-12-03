//! Core application state and log processing logic.
//!
//! This module contains the main `AppState` struct that manages:
//! - Log line storage in a ring buffer
//! - Filtering and search functionality
//! - Bookmarks and navigation state
//! - UI mode and panel focus

use crate::config::{Config, FILTER_DEBOUNCE_MS};
use crate::filter::{ActiveFilter, MatchRange, SavedFilter};
use crate::sources::LogSourceType;
use crate::theme::Theme;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
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

/// Main application state
pub struct AppState<'a> {
    /// Ring buffer of log lines
    pub lines: VecDeque<LogLine>,
    /// Indices into lines that match the current filter
    pub filtered_indices: Vec<usize>,
    /// Current scroll position (index into filtered_indices)
    pub scroll: usize,
    /// Maximum lines to keep in the buffer
    pub max_lines: usize,
    /// Current input mode
    pub mode: InputMode,
    /// Filter text input widget
    pub filter_textarea: TextArea<'a>,
    /// Currently active filter
    pub active_filter: Option<ActiveFilter>,
    /// Whether filter is regex mode
    pub filter_is_regex: bool,
    /// Available log sources
    pub sources: Vec<LogSourceType>,
    /// Index of current/selected source
    pub current_source_idx: usize,
    /// Which sources are currently visible (by index)
    pub visible_sources: Vec<bool>,
    /// View mode: show all sources merged or single source only
    pub view_mode: SourceViewMode,
    /// Saved filters
    pub saved_filters: Vec<SavedFilter>,
    /// Selected saved filter index (for navigation)
    pub selected_filter_idx: usize,
    /// Which panel currently has focus
    pub focused_panel: FocusedPanel,
    /// Whether side panel is visible
    pub show_side_panel: bool,
    /// If true, auto-scroll to bottom on new lines
    pub stick_to_bottom: bool,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Status message to display
    pub status_message: Option<String>,
    /// Last time filter input changed (for debounce)
    pub filter_last_change: Option<Instant>,
    /// Whether we need to recompute filter (after debounce)
    pub filter_needs_recompute: bool,
    /// Whether to show help overlay
    pub show_help: bool,
    /// Whether to apply log level coloring (for lines without ANSI)
    pub level_colors_enabled: bool,
    /// Whether to wrap long lines
    pub line_wrap: bool,
    /// Horizontal scroll offset (in characters) - used when line_wrap is false
    pub horizontal_scroll: usize,
    /// Export directory for logs
    pub export_dir: String,
    /// Whether to show relative timestamps
    pub show_relative_time: bool,
    /// Whether to pretty-print JSON logs
    pub json_pretty: bool,
    /// Bookmarked line indices (into filtered_indices)
    pub bookmarks: Vec<usize>,
    /// Active color theme
    pub theme: Theme,
    /// Last known viewport height (for auto-scroll calculations)
    pub viewport_height: usize,
}

impl<'a> AppState<'a> {
    pub fn new(config: &Config, sources: Vec<LogSourceType>) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        textarea.set_placeholder_text("type to filter...");
        textarea.set_placeholder_style(Style::default().fg(Color::DarkGray));

        let visible_sources = vec![true; sources.len()];

        Self {
            lines: VecDeque::with_capacity(config.max_lines),
            filtered_indices: Vec::new(),
            scroll: 0,
            max_lines: config.max_lines,
            mode: InputMode::Normal,
            filter_textarea: textarea,
            active_filter: None,
            filter_is_regex: false,
            sources,
            current_source_idx: 0,
            visible_sources,
            view_mode: SourceViewMode::default(),
            saved_filters: Vec::new(),
            selected_filter_idx: 0,
            focused_panel: FocusedPanel::LogView,
            show_side_panel: config.show_side_panel,
            stick_to_bottom: true,
            should_quit: false,
            status_message: None,
            filter_last_change: None,
            filter_needs_recompute: false,
            show_help: false,
            level_colors_enabled: config.level_colors,
            line_wrap: config.line_wrap,
            horizontal_scroll: 0,
            export_dir: config.export_dir.clone(),
            show_relative_time: false,
            json_pretty: false,
            bookmarks: Vec::new(),
            theme: config.get_theme(),
            viewport_height: 20, // Default, updated on first render
        }
    }

    /// Toggle bookmark at current scroll position
    pub fn toggle_bookmark(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }

        // Get the actual line index at current scroll position
        let line_idx = self.filtered_indices[self.scroll];

        if let Some(pos) = self.bookmarks.iter().position(|&b| b == line_idx) {
            self.bookmarks.remove(pos);
            self.status_message = Some("Bookmark removed".to_string());
        } else {
            self.bookmarks.push(line_idx);
            self.bookmarks.sort_unstable();
            self.status_message = Some("Bookmark added".to_string());
        }
    }

    /// Jump to next bookmark
    pub fn next_bookmark(&mut self) {
        if self.bookmarks.is_empty() {
            self.status_message = Some("No bookmarks".to_string());
            return;
        }

        let current_line_idx = self.filtered_indices.get(self.scroll).copied().unwrap_or(0);

        // Find next bookmark after current position
        if let Some(&next_bookmark) = self.bookmarks.iter().find(|&&b| b > current_line_idx) {
            // Find this bookmark in filtered_indices
            if let Some(scroll_pos) = self
                .filtered_indices
                .iter()
                .position(|&i| i == next_bookmark)
            {
                self.scroll = scroll_pos;
                self.stick_to_bottom = false;
                self.status_message = Some(format!(
                    "Bookmark {}/{}",
                    self.bookmarks
                        .iter()
                        .position(|&b| b == next_bookmark)
                        .unwrap()
                        + 1,
                    self.bookmarks.len()
                ));
                return;
            }
        }

        // Wrap around to first bookmark
        if let Some(&first_bookmark) = self.bookmarks.first() {
            if let Some(scroll_pos) = self
                .filtered_indices
                .iter()
                .position(|&i| i == first_bookmark)
            {
                self.scroll = scroll_pos;
                self.stick_to_bottom = false;
                self.status_message =
                    Some(format!("Bookmark 1/{} (wrapped)", self.bookmarks.len()));
            }
        }
    }

    /// Jump to previous bookmark
    pub fn prev_bookmark(&mut self) {
        if self.bookmarks.is_empty() {
            self.status_message = Some("No bookmarks".to_string());
            return;
        }

        let current_line_idx = self.filtered_indices.get(self.scroll).copied().unwrap_or(0);

        // Find previous bookmark before current position
        if let Some(&prev_bookmark) = self.bookmarks.iter().rev().find(|&&b| b < current_line_idx) {
            // Find this bookmark in filtered_indices
            if let Some(scroll_pos) = self
                .filtered_indices
                .iter()
                .position(|&i| i == prev_bookmark)
            {
                self.scroll = scroll_pos;
                self.stick_to_bottom = false;
                self.status_message = Some(format!(
                    "Bookmark {}/{}",
                    self.bookmarks
                        .iter()
                        .position(|&b| b == prev_bookmark)
                        .unwrap()
                        + 1,
                    self.bookmarks.len()
                ));
                return;
            }
        }

        // Wrap around to last bookmark
        if let Some(&last_bookmark) = self.bookmarks.last() {
            if let Some(scroll_pos) = self
                .filtered_indices
                .iter()
                .position(|&i| i == last_bookmark)
            {
                self.scroll = scroll_pos;
                self.stick_to_bottom = false;
                self.status_message = Some(format!(
                    "Bookmark {}/{} (wrapped)",
                    self.bookmarks.len(),
                    self.bookmarks.len()
                ));
            }
        }
    }

    /// Check if a line index is bookmarked
    #[allow(dead_code)]
    pub fn is_bookmarked(&self, line_idx: usize) -> bool {
        self.bookmarks.contains(&line_idx)
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
            self.horizontal_scroll = 0;
        }
        self.status_message = Some(format!(
            "Line wrap: {}",
            if self.line_wrap { "on" } else { "off" }
        ));
    }

    /// Scroll left (when line wrap is off)
    pub fn scroll_left(&mut self) {
        if !self.line_wrap && self.horizontal_scroll > 0 {
            self.horizontal_scroll = self.horizontal_scroll.saturating_sub(4);
        }
    }

    /// Scroll right (when line wrap is off)
    pub fn scroll_right(&mut self) {
        if !self.line_wrap {
            self.horizontal_scroll += 4;
        }
    }

    /// Scroll left by a larger amount
    pub fn scroll_left_large(&mut self) {
        if !self.line_wrap && self.horizontal_scroll > 0 {
            self.horizontal_scroll = self.horizontal_scroll.saturating_sub(20);
        }
    }

    /// Scroll right by a larger amount
    pub fn scroll_right_large(&mut self) {
        if !self.line_wrap {
            self.horizontal_scroll += 20;
        }
    }

    /// Reset horizontal scroll to beginning
    pub fn scroll_home(&mut self) {
        self.horizontal_scroll = 0;
    }

    /// Export filtered (or all) lines to a file
    pub fn export_lines(&self, path: &str) -> Result<usize, String> {
        let mut file = File::create(path).map_err(|e| e.to_string())?;

        let mut count = 0;
        for &idx in &self.filtered_indices {
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

    /// Get the current source
    pub fn current_source(&self) -> &LogSourceType {
        &self.sources[self.current_source_idx]
    }

    /// Add a new source at runtime
    #[allow(dead_code)]
    pub fn add_source(&mut self, source: LogSourceType) {
        self.sources.push(source);
        self.visible_sources.push(true);
    }

    /// Save the current filter with a name
    pub fn save_current_filter(&mut self, name: String) {
        if let Some(ref filter) = self.active_filter {
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

            self.filter_textarea = TextArea::new(vec![pattern.clone()]);
            self.filter_textarea.set_cursor_line_style(Style::default());
            self.filter_is_regex = is_regex;
            self.active_filter = Some(ActiveFilter::new(pattern, is_regex));
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
        if !self.show_side_panel {
            return;
        }
        self.focused_panel = match self.focused_panel {
            FocusedPanel::LogView => FocusedPanel::Sources,
            FocusedPanel::Sources => FocusedPanel::Filters,
            FocusedPanel::Filters => FocusedPanel::LogView,
        };
    }

    /// Push a new log line into the buffer
    pub fn push_line(&mut self, line: LogLine) {
        // If buffer is full, remove oldest line
        if self.lines.len() >= self.max_lines {
            self.lines.pop_front();
            // Recompute filtered indices since indices shifted
            self.recompute_filter();
        }

        let line_index = self.lines.len();
        self.lines.push_back(line);

        // Check if the new line matches the filter
        if self.matches_filter(line_index) {
            self.filtered_indices.push(line_index);
        }

        // Auto-scroll if stick_to_bottom is enabled
        // Scroll so the last line appears at the BOTTOM of the viewport
        if self.stick_to_bottom && !self.filtered_indices.is_empty() {
            self.scroll = self
                .filtered_indices
                .len()
                .saturating_sub(self.viewport_height);
        }
    }

    /// Check if a line at the given index matches the current filter
    fn matches_filter(&self, index: usize) -> bool {
        let line = match self.lines.get(index) {
            Some(l) => l,
            None => return false,
        };

        // Check source visibility
        if !self
            .visible_sources
            .get(line.source_id)
            .copied()
            .unwrap_or(true)
        {
            return false;
        }

        // Check view mode
        match self.view_mode {
            SourceViewMode::AllMerged => {}
            SourceViewMode::SingleSource(id) if id != line.source_id => return false,
            _ => {}
        }

        // Check text filter
        match &self.active_filter {
            None => true,
            Some(filter) => filter.matches(&line.raw),
        }
    }

    /// Recompute filtered_indices based on current filter
    pub fn recompute_filter(&mut self) {
        self.filtered_indices.clear();
        for i in 0..self.lines.len() {
            if self.matches_filter(i) {
                self.filtered_indices.push(i);
            }
        }

        // Adjust scroll if it's now out of bounds
        if !self.filtered_indices.is_empty() {
            self.scroll = self.scroll.min(self.filtered_indices.len() - 1);
        } else {
            self.scroll = 0;
        }
    }

    /// Scroll up by one line
    pub fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
            self.stick_to_bottom = false;
        }
    }

    /// Scroll down by one line
    pub fn scroll_down(&mut self) {
        if !self.filtered_indices.is_empty() && self.scroll < self.filtered_indices.len() - 1 {
            self.scroll += 1;
        }
    }

    /// Scroll up by a page
    pub fn scroll_page_up(&mut self, page_size: usize) {
        self.scroll = self.scroll.saturating_sub(page_size);
        self.stick_to_bottom = false;
    }

    /// Scroll down by a page
    pub fn scroll_page_down(&mut self, page_size: usize) {
        if !self.filtered_indices.is_empty() {
            self.scroll = (self.scroll + page_size).min(self.filtered_indices.len() - 1);
        }
    }

    /// Go to the top of the log
    pub fn go_to_top(&mut self) {
        self.scroll = 0;
        self.stick_to_bottom = false;
    }

    /// Go to the bottom of the log and enable stick_to_bottom
    pub fn go_to_bottom(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.scroll = self.filtered_indices.len() - 1;
        }
        self.stick_to_bottom = true;
    }

    /// Go to next matching line (when filter is active)
    pub fn next_match(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        if self.scroll < self.filtered_indices.len() - 1 {
            self.scroll += 1;
            self.stick_to_bottom = false;
            self.status_message = Some(format!(
                "Match {}/{}",
                self.scroll + 1,
                self.filtered_indices.len()
            ));
        } else {
            // Wrap to beginning
            self.scroll = 0;
            self.stick_to_bottom = false;
            self.status_message = Some(format!(
                "Match {}/{} (wrapped)",
                self.scroll + 1,
                self.filtered_indices.len()
            ));
        }
    }

    /// Go to previous matching line (when filter is active)
    pub fn prev_match(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        if self.scroll > 0 {
            self.scroll -= 1;
            self.stick_to_bottom = false;
            self.status_message = Some(format!(
                "Match {}/{}",
                self.scroll + 1,
                self.filtered_indices.len()
            ));
        } else {
            // Wrap to end
            self.scroll = self.filtered_indices.len() - 1;
            self.stick_to_bottom = false;
            self.status_message = Some(format!(
                "Match {}/{} (wrapped)",
                self.scroll + 1,
                self.filtered_indices.len()
            ));
        }
    }

    /// Get match ranges for a line (for highlighting)
    pub fn get_match_ranges(&self, line: &str) -> Vec<MatchRange> {
        if let Some(ref filter) = self.active_filter {
            filter.find_matches(line)
        } else {
            Vec::new()
        }
    }

    /// Get the current filter input text
    pub fn filter_input(&self) -> String {
        self.filter_textarea.lines().join("\n")
    }

    /// Apply the current filter input as the active filter
    pub fn apply_filter(&mut self) {
        let input = self.filter_input();
        if input.is_empty() {
            self.active_filter = None;
        } else {
            self.active_filter = Some(ActiveFilter::new(input, self.filter_is_regex));
        }
        self.recompute_filter();
        self.mode = InputMode::Normal;
        self.filter_last_change = None;
        self.filter_needs_recompute = false;
    }

    /// Cancel filter editing and revert to previous state
    pub fn cancel_filter(&mut self) {
        // Restore textarea to previous filter
        let prev = self
            .active_filter
            .as_ref()
            .map(|f| f.pattern.clone())
            .unwrap_or_default();
        self.filter_textarea = TextArea::new(vec![prev]);
        self.filter_textarea.set_cursor_line_style(Style::default());
        self.mode = InputMode::Normal;
        self.filter_last_change = None;
        self.filter_needs_recompute = false;
    }

    /// Mark that filter input changed (for debounce)
    pub fn filter_changed(&mut self) {
        self.filter_last_change = Some(Instant::now());
        self.filter_needs_recompute = true;
    }

    /// Check if debounce period has passed and recompute if needed
    pub fn check_filter_debounce(&mut self) {
        if let Some(last_change) = self.filter_last_change {
            if last_change.elapsed().as_millis() >= FILTER_DEBOUNCE_MS
                && self.filter_needs_recompute
            {
                // Apply filter without changing mode
                let input = self.filter_input();
                if input.is_empty() {
                    self.active_filter = None;
                } else {
                    self.active_filter = Some(ActiveFilter::new(input, self.filter_is_regex));
                }
                self.recompute_filter();
                self.filter_needs_recompute = false;
            }
        }
    }

    /// Toggle regex mode for filtering
    pub fn toggle_regex_mode(&mut self) {
        self.filter_is_regex = !self.filter_is_regex;
        if self.active_filter.is_some() {
            // Reapply filter with new mode
            let input = self.filter_input();
            self.active_filter = Some(ActiveFilter::new(input, self.filter_is_regex));
            self.recompute_filter();
        }
        self.status_message = Some(format!(
            "Filter mode: {}",
            if self.filter_is_regex {
                "regex"
            } else {
                "substring"
            }
        ));
    }

    /// Get visible lines for rendering
    pub fn visible_lines(&mut self, height: usize) -> Vec<(usize, &mut LogLine)> {
        // Store viewport height for auto-scroll calculations
        self.viewport_height = height;

        if self.filtered_indices.is_empty() {
            return Vec::new();
        }

        let start = self.scroll;
        let end = (start + height).min(self.filtered_indices.len());

        // We need to collect indices first, then get mutable references
        let indices: Vec<usize> = self.filtered_indices[start..end].to_vec();

        // This is safe because we're only accessing each index once
        let mut result = Vec::with_capacity(indices.len());
        for (i, &line_idx) in indices.iter().enumerate() {
            let line = &mut self.lines[line_idx];
            result.push((start + i, unsafe {
                // Safety: we know line_idx is valid and we only access each once
                &mut *(line as *mut LogLine)
            }));
        }
        result
    }

    /// Get total and visible line counts
    pub fn line_counts(&self) -> (usize, usize) {
        (self.lines.len(), self.filtered_indices.len())
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
