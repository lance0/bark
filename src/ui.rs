//! Terminal UI rendering using ratatui.
//!
//! Renders the main log view, side panel, status bar, and help overlay.

use ansi_to_tui::IntoText;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};

use crate::app::{AppState, FocusedPanel, InputMode, LogLevel};
use crate::filter::MatchRange;
use crate::theme::Theme;

const SIDE_PANEL_WIDTH: u16 = 24;

/// Data for rendering a single log line: (raw, has_ansi, level_color, relative_time, is_json, is_bookmarked, source_id)
type LineRenderData = (
    String,
    bool,
    Option<Color>,
    Option<String>,
    bool,
    bool,
    usize,
);

/// Get color for a log level from the theme
fn get_level_color(level: &LogLevel, theme: &Theme) -> Option<Color> {
    match level {
        LogLevel::Error => Some(theme.level_error),
        LogLevel::Warn => Some(theme.level_warn),
        LogLevel::Info => Some(theme.level_info),
        LogLevel::Debug => Some(theme.level_debug),
        LogLevel::Trace => Some(theme.level_trace),
        LogLevel::None => None,
    }
}

/// Apply horizontal scroll offset to a string, returning a substring
fn apply_horizontal_scroll(text: &str, offset: usize) -> String {
    if offset == 0 {
        text.to_string()
    } else {
        // Handle multi-byte characters properly
        text.chars().skip(offset).collect()
    }
}

/// Apply horizontal scroll to a styled Line, preserving styles
fn apply_horizontal_scroll_to_line(line: &Line<'_>, offset: usize) -> Line<'static> {
    if offset == 0 {
        return Line::from(
            line.spans
                .iter()
                .map(|s| Span::styled(s.content.to_string(), s.style))
                .collect::<Vec<_>>(),
        );
    }

    let mut result_spans = Vec::new();
    let mut chars_skipped = 0;

    for span in &line.spans {
        let span_len = span.content.chars().count();

        if chars_skipped + span_len <= offset {
            // Skip this entire span
            chars_skipped += span_len;
        } else if chars_skipped < offset {
            // Partial skip - start of span is cut off
            let skip_in_span = offset - chars_skipped;
            let remaining: String = span.content.chars().skip(skip_in_span).collect();
            result_spans.push(Span::styled(remaining, span.style));
            chars_skipped = offset;
        } else {
            // No skip needed for this span
            result_spans.push(Span::styled(span.content.to_string(), span.style));
        }
    }

    Line::from(result_spans)
}

/// Apply match highlighting to a line, returning styled spans
fn highlight_matches(
    text: &str,
    matches: &[MatchRange],
    base_style: Style,
    theme: &Theme,
) -> Line<'static> {
    if matches.is_empty() {
        return Line::from(Span::styled(text.to_string(), base_style));
    }

    let highlight_style = Style::default()
        .bg(theme.highlight_match_bg)
        .fg(theme.highlight_match_fg)
        .add_modifier(Modifier::BOLD);

    let mut spans = Vec::new();
    let mut last_end = 0;

    for m in matches {
        // Add text before the match
        if m.start > last_end {
            spans.push(Span::styled(
                text[last_end..m.start].to_string(),
                base_style,
            ));
        }
        // Add the highlighted match
        if m.end <= text.len() {
            spans.push(Span::styled(
                text[m.start..m.end].to_string(),
                highlight_style,
            ));
            last_end = m.end;
        }
    }

    // Add remaining text after last match
    if last_end < text.len() {
        spans.push(Span::styled(text[last_end..].to_string(), base_style));
    }

    Line::from(spans)
}

/// Draw the entire UI
pub fn draw(frame: &mut Frame, state: &mut AppState) {
    // Main layout: optional side panel + main content
    let main_chunks = if state.show_side_panel {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(SIDE_PANEL_WIDTH), Constraint::Min(20)])
            .split(frame.area())
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(20)])
            .split(frame.area())
    };

    // Draw side panel if visible
    if state.show_side_panel {
        draw_side_panel(frame, state, main_chunks[0]);
    }

    // Main content area
    let content_area = if state.show_side_panel {
        main_chunks[1]
    } else {
        main_chunks[0]
    };

    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(3),    // Log view
            Constraint::Length(1), // Status bar
            Constraint::Length(1), // Filter bar
        ])
        .split(content_area);

    draw_header(frame, state, content_chunks[0]);
    draw_log_view(frame, state, content_chunks[1]);
    draw_status_bar(frame, state, content_chunks[2]);
    draw_filter_bar(frame, state, content_chunks[3]);

    // Draw help overlay if active
    if state.show_help {
        draw_help_overlay(frame, &state.theme);
    }
}

/// Draw the side panel with sources and saved filters
fn draw_side_panel(frame: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(state.sources.len() as u16 + 2), // Sources section
            Constraint::Min(3),                                 // Filters section
        ])
        .split(area);

    draw_sources_panel(frame, state, chunks[0]);
    draw_filters_panel(frame, state, chunks[1]);
}

/// Draw the sources list
fn draw_sources_panel(frame: &mut Frame, state: &AppState, area: Rect) {
    use crate::app::SourceViewMode;

    let focused = state.focused_panel == FocusedPanel::Sources;
    let border_style = if focused {
        Style::default().fg(state.theme.border_focused)
    } else {
        Style::default().fg(state.theme.border_unfocused)
    };

    let title = if state.sources.len() > 1 {
        " Sources (Space:toggle) "
    } else {
        " Sources "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let items: Vec<ListItem> = state
        .sources
        .iter()
        .enumerate()
        .map(|(i, source)| {
            let is_selected = i == state.current_source_idx;
            let is_visible = state.visible_sources.get(i).copied().unwrap_or(true);
            let is_solo = matches!(state.view_mode, SourceViewMode::SingleSource(id) if id == i);

            let prefix = if is_selected { ">" } else { " " };
            let visibility = if is_solo {
                "[*]"
            } else if is_visible {
                "[x]"
            } else {
                "[ ]"
            };

            let color = state.theme.get_source_color(i);

            // Truncate long names
            let name = source.name();
            let display_name = if name.len() > 12 {
                format!("{}...", &name[..9])
            } else {
                name
            };

            let style = if is_selected {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else if is_visible {
                Style::default().fg(color)
            } else {
                Style::default().fg(state.theme.empty_state)
            };

            ListItem::new(format!("{} {} {}", prefix, visibility, display_name)).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

/// Draw the saved filters list
fn draw_filters_panel(frame: &mut Frame, state: &AppState, area: Rect) {
    let focused = state.focused_panel == FocusedPanel::Filters;
    let border_style = if focused {
        Style::default().fg(state.theme.border_focused)
    } else {
        Style::default().fg(state.theme.border_unfocused)
    };

    let block = Block::default()
        .title(" Saved Filters ")
        .borders(Borders::ALL)
        .border_style(border_style);

    if state.saved_filters.is_empty() {
        let msg = Paragraph::new("  (none)")
            .style(Style::default().fg(state.theme.empty_state))
            .block(block);
        frame.render_widget(msg, area);
    } else {
        let items: Vec<ListItem> = state
            .saved_filters
            .iter()
            .enumerate()
            .map(|(i, filter)| {
                let prefix = if i == state.selected_filter_idx {
                    "▶ "
                } else {
                    "  "
                };
                let indicator = if filter.is_regex { " [.*]" } else { "" };
                let style = if i == state.selected_filter_idx {
                    Style::default().fg(state.theme.filter_selected)
                } else {
                    Style::default()
                };
                ListItem::new(format!("{}{}{}", prefix, filter.name, indicator)).style(style)
            })
            .collect();

        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }
}

/// Draw the header showing the current source
fn draw_header(frame: &mut Frame, state: &AppState, area: Rect) {
    let source_name = state.current_source().name();
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " bark ",
            Style::default()
                .fg(state.theme.header_title)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("| "),
        Span::styled(source_name, Style::default().fg(state.theme.header_source)),
    ]))
    .style(Style::default().bg(state.theme.header_bg));

    frame.render_widget(header, area);
}

/// Draw the main log view
fn draw_log_view(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let focused = state.focused_panel == FocusedPanel::LogView;
    let border_style = if focused && state.show_side_panel {
        Style::default().fg(state.theme.border_focused)
    } else {
        Style::default().fg(state.theme.border_unfocused)
    };

    let block = Block::default()
        .borders(if state.show_side_panel {
            Borders::LEFT
        } else {
            Borders::NONE
        })
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let height = inner.height as usize;
    if height == 0 {
        return;
    }

    // Get visible lines
    let level_colors = state.level_colors_enabled;
    let show_relative = state.show_relative_time;
    let json_pretty_enabled = state.json_pretty;
    let scroll_pos = state.scroll;
    let bookmarks = state.bookmarks.clone();
    let filtered_indices = state.filtered_indices.clone();
    let theme = state.theme.clone();
    let visible = state.visible_lines(height);

    // Collect line data first (to avoid borrow issues)
    // Also track which line indices are bookmarked
    let line_data: Vec<LineRenderData> = visible
        .iter()
        .enumerate()
        .map(|(visible_idx, (_scroll_idx, line))| {
            let actual_line_idx = filtered_indices
                .get(scroll_pos + visible_idx)
                .copied()
                .unwrap_or(0);
            let is_bookmarked = bookmarks.contains(&actual_line_idx);
            let level_color = if level_colors {
                get_level_color(&line.level, &theme)
            } else {
                None
            };
            (
                line.raw.clone(),
                line.has_ansi,
                level_color,
                if show_relative {
                    line.relative_time()
                } else {
                    None
                },
                line.is_json,
                is_bookmarked,
                line.source_id,
            )
        })
        .collect();

    // Pre-compute pretty JSON if needed
    let json_cache: Vec<Option<String>> = if json_pretty_enabled {
        line_data
            .iter()
            .map(|(raw, _, _, _, is_json, _, _)| {
                if *is_json {
                    serde_json::from_str::<serde_json::Value>(raw)
                        .ok()
                        .and_then(|v| serde_json::to_string_pretty(&v).ok())
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![None; line_data.len()]
    };

    // Check if we need to show source prefixes (only when multiple sources)
    let show_source_prefix = state.sources.len() > 1;

    // Build the paragraph content with highlighting
    let mut lines_content: Vec<Line<'_>> = Vec::with_capacity(height);
    let h_scroll = if state.line_wrap {
        0
    } else {
        state.horizontal_scroll
    };

    for (idx, (raw, has_ansi, level_color, relative_time, _is_json, is_bookmarked, source_id)) in
        line_data.iter().enumerate()
    {
        // Check if we have pretty JSON for this line
        let display_text = json_cache
            .get(idx)
            .and_then(|j| j.as_ref())
            .map(|s| s.as_str())
            .unwrap_or(raw);

        // Build source prefix if multiple sources
        let source_prefix: Option<Span> = if show_source_prefix {
            let source_name = state
                .sources
                .get(*source_id)
                .map(|s| {
                    let name = s.name();
                    // Truncate long names
                    if name.len() > 10 {
                        format!("{}...", &name[..7])
                    } else {
                        name
                    }
                })
                .unwrap_or_else(|| "?".to_string());
            let color = theme.get_source_color(*source_id);
            Some(Span::styled(
                format!("[{:>10}] ", source_name),
                Style::default().fg(color),
            ))
        } else {
            None
        };

        // Build bookmark prefix if bookmarked
        let bookmark_prefix: Option<Span> = if *is_bookmarked {
            Some(Span::styled(
                "* ",
                Style::default()
                    .fg(theme.bookmark)
                    .add_modifier(Modifier::BOLD),
            ))
        } else {
            None
        };

        // Build relative time prefix if enabled
        let time_prefix: Option<Vec<Span>> = relative_time.as_ref().map(|rt| {
            vec![Span::styled(
                format!("{:>8} ", rt),
                Style::default().fg(theme.timestamp),
            )]
        });

        // Handle multi-line display (for pretty JSON)
        let display_lines: Vec<&str> = display_text.lines().collect();
        let is_multiline = display_lines.len() > 1;

        for (line_idx, display_line) in display_lines.iter().enumerate() {
            // Only show time prefix on first line of multi-line
            let show_prefix = line_idx == 0;

            if *has_ansi && !is_multiline {
                // Line has ANSI codes - use the standard rendering without highlighting
                // (ANSI parsing is complex and highlighting would interfere)
                if let Ok(text) = raw.as_bytes().into_text() {
                    for text_line in text.lines.iter() {
                        // Apply horizontal scroll to ANSI lines by rebuilding spans
                        let mut scrolled_line = if h_scroll > 0 {
                            apply_horizontal_scroll_to_line(text_line, h_scroll)
                        } else {
                            Line::from(
                                text_line
                                    .spans
                                    .iter()
                                    .map(|s| Span::styled(s.content.to_string(), s.style))
                                    .collect::<Vec<_>>(),
                            )
                        };

                        // Add prefixes (source, bookmark, time)
                        if show_prefix {
                            let mut prefix_spans = Vec::new();
                            if let Some(ref sp) = source_prefix {
                                prefix_spans.push(sp.clone());
                            }
                            if let Some(ref bm) = bookmark_prefix {
                                prefix_spans.push(bm.clone());
                            }
                            if let Some(ref tp) = time_prefix {
                                prefix_spans.extend(tp.clone());
                            }
                            if !prefix_spans.is_empty() {
                                prefix_spans.extend(scrolled_line.spans);
                                scrolled_line = Line::from(prefix_spans);
                            }
                        }

                        lines_content.push(scrolled_line);
                        if lines_content.len() >= height {
                            break;
                        }
                    }
                } else {
                    let scrolled = apply_horizontal_scroll(raw, h_scroll);
                    let mut line = Line::from(scrolled);
                    if show_prefix {
                        let mut prefix_spans = Vec::new();
                        if let Some(ref sp) = source_prefix {
                            prefix_spans.push(sp.clone());
                        }
                        if let Some(ref bm) = bookmark_prefix {
                            prefix_spans.push(bm.clone());
                        }
                        if let Some(ref tp) = time_prefix {
                            prefix_spans.extend(tp.clone());
                        }
                        if !prefix_spans.is_empty() {
                            prefix_spans.extend(line.spans);
                            line = Line::from(prefix_spans);
                        }
                    }
                    lines_content.push(line);
                }
            } else {
                // No ANSI codes or multi-line JSON - we can safely apply highlighting
                let base_style = if is_multiline {
                    // JSON gets themed coloring
                    Style::default().fg(theme.json)
                } else {
                    level_color
                        .map(|c| Style::default().fg(c))
                        .unwrap_or_default()
                };

                // Apply horizontal scroll before highlighting
                let scrolled = apply_horizontal_scroll(display_line, h_scroll);
                // Adjust match ranges for the scroll offset (only for original text)
                let matches = if !is_multiline && h_scroll > 0 {
                    state
                        .get_match_ranges(raw)
                        .into_iter()
                        .filter_map(|m| {
                            if m.end <= h_scroll {
                                None // Match is entirely scrolled away
                            } else if m.start < h_scroll {
                                // Match starts before scroll, adjust
                                Some(MatchRange {
                                    start: 0,
                                    end: m.end - h_scroll,
                                })
                            } else {
                                // Match is visible, adjust for scroll
                                Some(MatchRange {
                                    start: m.start - h_scroll,
                                    end: m.end - h_scroll,
                                })
                            }
                        })
                        .collect()
                } else if !is_multiline {
                    state.get_match_ranges(raw)
                } else {
                    Vec::new() // No highlighting for pretty JSON lines
                };

                let mut highlighted_line =
                    highlight_matches(&scrolled, &matches, base_style, &theme);

                // Add prefixes (source, bookmark, time) - only on first line
                if show_prefix {
                    let mut prefix_spans = Vec::new();
                    if let Some(ref sp) = source_prefix {
                        prefix_spans.push(sp.clone());
                    }
                    if let Some(ref bm) = bookmark_prefix {
                        prefix_spans.push(bm.clone());
                    }
                    if let Some(ref tp) = time_prefix {
                        prefix_spans.extend(tp.clone());
                    }
                    if !prefix_spans.is_empty() {
                        prefix_spans.extend(highlighted_line.spans);
                        highlighted_line = Line::from(prefix_spans);
                    }
                }

                lines_content.push(highlighted_line);
            }

            if lines_content.len() >= height {
                break;
            }
        }

        if lines_content.len() >= height {
            break;
        }
    }

    // Pad with empty lines if needed
    while lines_content.len() < height {
        lines_content.push(Line::default());
    }

    let mut paragraph = Paragraph::new(lines_content);
    if state.line_wrap {
        paragraph = paragraph.wrap(Wrap { trim: false });
    }
    frame.render_widget(paragraph, inner);

    // Draw scrollbar if there are more lines than visible
    let (total, filtered) = state.line_counts();
    if filtered > height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        let mut scrollbar_state = ScrollbarState::new(filtered).position(state.scroll);

        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }

    // Show "no lines" message if empty
    if total == 0 {
        let msg = Paragraph::new("Waiting for log lines...")
            .style(Style::default().fg(theme.empty_state));
        frame.render_widget(msg, inner);
    } else if filtered == 0 && state.active_filter.is_some() {
        let msg = Paragraph::new("No lines match the current filter")
            .style(Style::default().fg(theme.warning_message));
        frame.render_widget(msg, inner);
    }
}

/// Draw the status bar
fn draw_status_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let (total, filtered) = state.line_counts();

    let mode_str = match state.mode {
        InputMode::Normal => "NORMAL",
        InputMode::FilterEditing => "FILTER",
        InputMode::SourceSelect => "SOURCE",
    };

    let follow_indicator = if state.stick_to_bottom { "[F]" } else { "" };
    let regex_indicator = if state.filter_is_regex { "[.*]" } else { "" };
    let wrap_indicator = if state.line_wrap { "[W]" } else { "" };
    let color_indicator = if state.level_colors_enabled {
        "[C]"
    } else {
        ""
    };
    let time_indicator = if state.show_relative_time { "[T]" } else { "" };
    let json_indicator = if state.json_pretty { "[J]" } else { "" };
    let hscroll_indicator = if !state.line_wrap && state.horizontal_scroll > 0 {
        format!("[+{}]", state.horizontal_scroll)
    } else {
        String::new()
    };

    // Combine indicators
    let mut indicators: Vec<String> = [
        follow_indicator,
        regex_indicator,
        wrap_indicator,
        color_indicator,
        time_indicator,
        json_indicator,
    ]
    .iter()
    .filter(|s| !s.is_empty())
    .map(|s| s.to_string())
    .collect();
    if !hscroll_indicator.is_empty() {
        indicators.push(hscroll_indicator);
    }
    let indicators_str = if indicators.is_empty() {
        String::new()
    } else {
        format!(" {}", indicators.join(" "))
    };

    let filter_str = state
        .active_filter
        .as_ref()
        .map(|f| format!(" | filter: {}", f.pattern))
        .unwrap_or_default();

    let help_text = match state.mode {
        InputMode::FilterEditing => " Enter:apply  Esc:cancel  Ctrl+r:regex ",
        _ => " ?:help  w:wrap  c:colors ",
    };

    let status = Line::from(vec![
        Span::styled(
            format!(" {} ", mode_str),
            Style::default()
                .bg(state.theme.status_mode_bg)
                .fg(state.theme.status_mode_fg),
        ),
        Span::raw(format!(
            " {}/{} lines{}{} ",
            filtered, total, indicators_str, filter_str
        )),
        Span::styled(help_text, Style::default().fg(state.theme.status_help)),
    ]);

    let paragraph = Paragraph::new(status).style(Style::default().bg(state.theme.status_bg));

    frame.render_widget(paragraph, area);
}

/// Draw the filter input bar
fn draw_filter_bar(frame: &mut Frame, state: &mut AppState, area: Rect) {
    match state.mode {
        InputMode::FilterEditing => {
            // Create a layout with "/" prefix and textarea
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(1), // "/" prefix
                    Constraint::Min(1),    // textarea
                ])
                .split(area);

            let prefix = Paragraph::new("/").style(Style::default().fg(state.theme.filter_prefix));
            frame.render_widget(prefix, chunks[0]);

            frame.render_widget(&state.filter_textarea, chunks[1]);
        }
        _ => {
            if let Some(msg) = &state.status_message {
                let content = Line::from(Span::styled(
                    msg.as_str(),
                    Style::default().fg(state.theme.warning_message),
                ));
                let paragraph = Paragraph::new(content);
                frame.render_widget(paragraph, area);
            }
        }
    }
}

/// Draw the help overlay
fn draw_help_overlay(frame: &mut Frame, theme: &Theme) {
    let area = frame.area();

    // Center the help box
    let width = 50.min(area.width.saturating_sub(4));
    let height = 30.min(area.height.saturating_sub(4));
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let help_area = Rect::new(x, y, width, height);

    // Clear background
    frame.render_widget(Clear, help_area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  j/k, ↑/↓     Scroll up/down"),
        Line::from("  h/l, ←/→     Scroll left/right"),
        Line::from("  H/L          Scroll left/right (large)"),
        Line::from("  0            Scroll to line start"),
        Line::from("  g/G          Go to top/bottom"),
        Line::from("  PgUp/PgDn    Page up/down"),
        Line::from("  n/N          Next/prev match"),
        Line::from("  m            Toggle bookmark"),
        Line::from("  [/]          Prev/next bookmark"),
        Line::from("  Mouse wheel  Scroll"),
        Line::from(""),
        Line::from("Filtering:"),
        Line::from("  /            Start filter input"),
        Line::from("  r            Toggle regex mode"),
        Line::from("  s            Save current filter"),
        Line::from("  e            Export filtered lines"),
        Line::from("  Esc          Clear filter"),
        Line::from(""),
        Line::from("Display:"),
        Line::from("  w            Toggle line wrapping"),
        Line::from("  c            Toggle level colors"),
        Line::from("  t            Toggle relative time"),
        Line::from("  J            Toggle JSON pretty-print"),
        Line::from("  b            Toggle side panel"),
        Line::from("  Tab          Cycle panel focus"),
        Line::from("  ?            Toggle this help"),
        Line::from("  q            Quit"),
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.help_border))
        .style(Style::default().bg(theme.help_bg));

    let paragraph = Paragraph::new(help_text).block(block);
    frame.render_widget(paragraph, help_area);
}
