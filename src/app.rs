use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::time::Instant;
use ansi_to_tui::IntoText;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use tui_textarea::TextArea;
use crate::config::Config;
use crate::filter::{ActiveFilter, MatchRange, SavedFilter};
use crate::sources::LogSourceType;

/// Common timestamp formats to try parsing
const TIMESTAMP_FORMATS: &[&str] = &[
    "%Y-%m-%dT%H:%M:%S%.fZ",          // ISO 8601 with Z
    "%Y-%m-%dT%H:%M:%S%.f%:z",        // ISO 8601 with offset
    "%Y-%m-%dT%H:%M:%S%:z",           // ISO 8601 without millis
    "%Y-%m-%dT%H:%M:%S%.f",           // ISO 8601 no timezone
    "%Y-%m-%dT%H:%M:%S",              // ISO 8601 basic
    "%Y-%m-%d %H:%M:%S%.f",           // Common log format with millis
    "%Y-%m-%d %H:%M:%S",              // Common log format
    "%d/%b/%Y:%H:%M:%S %z",           // Apache/nginx combined
    "%b %d %H:%M:%S",                 // Syslog format
];

/// Try to parse a timestamp from the beginning of a line
fn parse_timestamp(line: &str) -> Option<DateTime<Local>> {
    // Extract the first ~35 characters which should contain any timestamp
    let prefix: String = line.chars().take(35).collect();

    for fmt in TIMESTAMP_FORMATS {
        // Try to parse with chrono
        if let Ok(dt) = NaiveDateTime::parse_from_str(&prefix, fmt) {
            return Some(Local.from_local_datetime(&dt).single()?);
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
                return Some(Local.from_local_datetime(&dt).single()?);
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

    /// Get the color for this log level
    pub fn color(&self) -> Option<Color> {
        match self {
            LogLevel::Error => Some(Color::Red),
            LogLevel::Warn => Some(Color::Yellow),
            LogLevel::Info => Some(Color::Green),
            LogLevel::Debug => Some(Color::Blue),
            LogLevel::Trace => Some(Color::DarkGray),
            LogLevel::None => None,
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
    /// Cached rendered version with ANSI codes converted to styles
    pub rendered: Option<Text<'static>>,
    /// Parsed timestamp from the line
    pub timestamp: Option<DateTime<Local>>,
    /// Whether this line is valid JSON
    pub is_json: bool,
    /// Pretty-printed JSON (cached)
    pub json_pretty: Option<String>,
}

impl LogLine {
    pub fn new(raw: String) -> Self {
        let level = LogLevel::detect(&raw);
        let has_ansi = raw.contains('\x1b');
        let timestamp = parse_timestamp(&raw);
        let is_json = Self::detect_json(&raw);
        Self { raw, level, has_ansi, rendered: None, timestamp, is_json, json_pretty: None }
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

    /// Get pretty-printed JSON, or None if not JSON
    pub fn get_json_pretty(&mut self) -> Option<&str> {
        if !self.is_json {
            return None;
        }

        if self.json_pretty.is_none() {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&self.raw) {
                if let Ok(pretty) = serde_json::to_string_pretty(&value) {
                    self.json_pretty = Some(pretty);
                }
            }
        }

        self.json_pretty.as_deref()
    }

    /// Get or create the rendered text, optionally applying level coloring
    pub fn get_rendered(&mut self, apply_level_colors: bool) -> &Text<'static> {
        if self.rendered.is_none() {
            let text = if self.has_ansi {
                // Line has ANSI codes - parse them
                self.raw.as_bytes().into_text()
                    .unwrap_or_else(|_| Text::raw(self.raw.clone()))
            } else if apply_level_colors {
                // No ANSI codes - apply level-based coloring
                if let Some(color) = self.level.color() {
                    Text::from(Line::from(Span::styled(
                        self.raw.clone(),
                        Style::default().fg(color),
                    )))
                } else {
                    Text::raw(self.raw.clone())
                }
            } else {
                Text::raw(self.raw.clone())
            };
            self.rendered = Some(text);
        }
        self.rendered.as_ref().unwrap()
    }

    /// Invalidate the cached render (e.g., when settings change)
    pub fn invalidate_render(&mut self) {
        self.rendered = None;
    }
}

/// Input mode for the application
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    /// Normal navigation mode
    Normal,
    /// Editing the filter text
    FilterEditing,
    /// Selecting a source
    SourceSelect,
}

/// Which panel has focus
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusedPanel {
    LogView,
    Sources,
    Filters,
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
}

impl<'a> AppState<'a> {
    pub fn new(config: &Config, source: LogSourceType) -> Self {
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        textarea.set_placeholder_text("type to filter...");
        textarea.set_placeholder_style(Style::default().fg(Color::DarkGray));

        Self {
            lines: VecDeque::with_capacity(config.max_lines),
            filtered_indices: Vec::new(),
            scroll: 0,
            max_lines: config.max_lines,
            mode: InputMode::Normal,
            filter_textarea: textarea,
            active_filter: None,
            filter_is_regex: false,
            sources: vec![source],
            current_source_idx: 0,
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
            if let Some(scroll_pos) = self.filtered_indices.iter().position(|&i| i == next_bookmark) {
                self.scroll = scroll_pos;
                self.stick_to_bottom = false;
                self.status_message = Some(format!(
                    "Bookmark {}/{}",
                    self.bookmarks.iter().position(|&b| b == next_bookmark).unwrap() + 1,
                    self.bookmarks.len()
                ));
                return;
            }
        }

        // Wrap around to first bookmark
        if let Some(&first_bookmark) = self.bookmarks.first() {
            if let Some(scroll_pos) = self.filtered_indices.iter().position(|&i| i == first_bookmark) {
                self.scroll = scroll_pos;
                self.stick_to_bottom = false;
                self.status_message = Some(format!(
                    "Bookmark 1/{} (wrapped)",
                    self.bookmarks.len()
                ));
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
            if let Some(scroll_pos) = self.filtered_indices.iter().position(|&i| i == prev_bookmark) {
                self.scroll = scroll_pos;
                self.stick_to_bottom = false;
                self.status_message = Some(format!(
                    "Bookmark {}/{}",
                    self.bookmarks.iter().position(|&b| b == prev_bookmark).unwrap() + 1,
                    self.bookmarks.len()
                ));
                return;
            }
        }

        // Wrap around to last bookmark
        if let Some(&last_bookmark) = self.bookmarks.last() {
            if let Some(scroll_pos) = self.filtered_indices.iter().position(|&i| i == last_bookmark) {
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
        // Invalidate all cached renders
        for line in &mut self.lines {
            line.invalidate_render();
        }
        self.status_message = Some(format!(
            "Level colors: {}",
            if self.level_colors_enabled { "on" } else { "off" }
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

    /// Add a new source
    pub fn add_source(&mut self, source: LogSourceType) {
        self.sources.push(source);
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
        if self.stick_to_bottom && !self.filtered_indices.is_empty() {
            self.scroll = self.filtered_indices.len().saturating_sub(1);
        }
    }

    /// Check if a line at the given index matches the current filter
    fn matches_filter(&self, index: usize) -> bool {
        match &self.active_filter {
            None => true,
            Some(filter) => {
                if let Some(line) = self.lines.get(index) {
                    filter.matches(&line.raw)
                } else {
                    false
                }
            }
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
        let prev = self.active_filter
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
        const DEBOUNCE_MS: u128 = 150;

        if let Some(last_change) = self.filter_last_change {
            if last_change.elapsed().as_millis() >= DEBOUNCE_MS && self.filter_needs_recompute {
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
            if self.filter_is_regex { "regex" } else { "substring" }
        ));
    }

    /// Get visible lines for rendering
    pub fn visible_lines(&mut self, height: usize) -> Vec<(usize, &mut LogLine)> {
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
