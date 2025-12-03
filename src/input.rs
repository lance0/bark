//! Keyboard and mouse input handling.
//!
//! Maps key events to application actions based on current input mode.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use tui_textarea::Input;

use crate::app::{AppState, FocusedPanel, InputMode, SourceViewMode};
use crate::config::MOUSE_SCROLL_LINES;

/// Handle a mouse event
pub fn handle_mouse(state: &mut AppState, mouse: MouseEvent, _page_size: usize) {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            for _ in 0..MOUSE_SCROLL_LINES {
                state.scroll_up();
            }
        }
        MouseEventKind::ScrollDown => {
            for _ in 0..MOUSE_SCROLL_LINES {
                state.scroll_down();
            }
        }
        _ => {}
    }
}

/// Handle a key event and update app state accordingly
pub fn handle_key(state: &mut AppState, key: KeyEvent, page_size: usize) {
    // Help overlay takes priority
    if state.show_help {
        if matches!(
            key.code,
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')
        ) {
            state.show_help = false;
        }
        return;
    }

    match state.mode {
        InputMode::Normal => handle_normal_mode(state, key, page_size),
        InputMode::FilterEditing => handle_filter_mode(state, key),
        InputMode::SourceSelect => handle_source_select_mode(state, key),
    }
}

fn handle_normal_mode(state: &mut AppState, key: KeyEvent, page_size: usize) {
    match key.code {
        // Quit
        KeyCode::Char('q') => {
            state.should_quit = true;
        }
        // Ctrl+C also quits
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.should_quit = true;
        }

        // Help
        KeyCode::Char('?') => {
            state.show_help = true;
        }

        // Toggle side panel
        KeyCode::Char('b') => {
            state.toggle_side_panel();
        }

        // Cycle focus between panels
        KeyCode::Tab => {
            state.cycle_focus();
        }

        // Navigation (context-dependent on focused panel)
        KeyCode::Char('j') | KeyCode::Down => match state.focused_panel {
            FocusedPanel::LogView => state.scroll_down(),
            FocusedPanel::Sources => {
                if state.current_source_idx < state.sources.len() - 1 {
                    state.current_source_idx += 1;
                }
            }
            FocusedPanel::Filters => {
                if !state.saved_filters.is_empty()
                    && state.selected_filter_idx < state.saved_filters.len() - 1
                {
                    state.selected_filter_idx += 1;
                }
            }
        },
        KeyCode::Char('k') | KeyCode::Up => match state.focused_panel {
            FocusedPanel::LogView => state.scroll_up(),
            FocusedPanel::Sources => {
                if state.current_source_idx > 0 {
                    state.current_source_idx -= 1;
                }
            }
            FocusedPanel::Filters => {
                if state.selected_filter_idx > 0 {
                    state.selected_filter_idx -= 1;
                }
            }
        },

        // Horizontal scrolling (when line wrap is off)
        KeyCode::Char('h') | KeyCode::Left => {
            if state.focused_panel == FocusedPanel::LogView {
                state.scroll_left();
            }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if state.focused_panel == FocusedPanel::LogView {
                state.scroll_right();
            }
        }
        KeyCode::Char('H') => {
            state.scroll_left_large();
        }
        KeyCode::Char('L') => {
            state.scroll_right_large();
        }
        KeyCode::Char('0') => {
            state.scroll_home();
        }

        // Enter to apply selected saved filter
        KeyCode::Enter => {
            if state.focused_panel == FocusedPanel::Filters && !state.saved_filters.is_empty() {
                state.apply_saved_filter(state.selected_filter_idx);
            }
        }

        // Space - toggle source visibility (in Sources panel)
        KeyCode::Char(' ') => {
            if state.focused_panel == FocusedPanel::Sources {
                let idx = state.current_source_idx;
                if let Some(visible) = state.visible_sources.get_mut(idx) {
                    *visible = !*visible;
                }
                let is_visible = state.visible_sources.get(idx).copied().unwrap_or(true);
                state.recompute_filter();
                let source_name = state.sources[idx].name();
                state.status_message = Some(format!(
                    "{} {}",
                    source_name,
                    if is_visible { "shown" } else { "hidden" }
                ));
            }
        }

        // 'v' - solo view (show only selected source) or toggle back to all
        KeyCode::Char('v') => {
            if state.focused_panel == FocusedPanel::Sources {
                state.view_mode = match state.view_mode {
                    SourceViewMode::SingleSource(id) if id == state.current_source_idx => {
                        SourceViewMode::AllMerged
                    }
                    _ => SourceViewMode::SingleSource(state.current_source_idx),
                };
                state.recompute_filter();
                state.status_message = Some(match state.view_mode {
                    SourceViewMode::AllMerged => "Showing all sources".to_string(),
                    SourceViewMode::SingleSource(id) => {
                        format!("Solo: {}", state.sources[id].name())
                    }
                });
            }
        }

        // 'a' - show all sources
        KeyCode::Char('a') => {
            if state.focused_panel == FocusedPanel::Sources {
                for v in state.visible_sources.iter_mut() {
                    *v = true;
                }
                state.view_mode = SourceViewMode::AllMerged;
                state.recompute_filter();
                state.status_message = Some("All sources visible".to_string());
            }
        }

        KeyCode::PageDown | KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.scroll_page_down(page_size);
        }
        KeyCode::PageUp | KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.scroll_page_up(page_size);
        }
        KeyCode::Char('g') => {
            state.go_to_top();
        }
        KeyCode::Char('G') => {
            state.go_to_bottom();
        }

        // Enter filter mode
        KeyCode::Char('/') => {
            state.mode = InputMode::FilterEditing;
            state.focused_panel = FocusedPanel::LogView;
            // Clear textarea for new filter input
            state.filter_textarea.select_all();
            state.filter_textarea.cut();
        }

        // Toggle regex mode
        KeyCode::Char('r') => {
            state.toggle_regex_mode();
        }

        // Next match
        KeyCode::Char('n') => {
            if state.active_filter.is_some() {
                state.next_match();
            } else {
                state.status_message = Some("No active filter (use / to filter)".to_string());
            }
        }

        // Previous match
        KeyCode::Char('N') => {
            if state.active_filter.is_some() {
                state.prev_match();
            } else {
                state.status_message = Some("No active filter (use / to filter)".to_string());
            }
        }

        // Toggle bookmark
        KeyCode::Char('m') => {
            state.toggle_bookmark();
        }

        // Next bookmark
        KeyCode::Char(']') => {
            state.next_bookmark();
        }

        // Previous bookmark
        KeyCode::Char('[') => {
            state.prev_bookmark();
        }

        // Toggle line wrap
        KeyCode::Char('w') => {
            state.toggle_line_wrap();
        }

        // Toggle level colors
        KeyCode::Char('c') => {
            state.toggle_level_colors();
        }

        // Toggle relative time display
        KeyCode::Char('t') => {
            state.toggle_relative_time();
        }

        // Toggle JSON pretty-printing
        KeyCode::Char('J') => {
            state.toggle_json_pretty();
        }

        // Export filtered lines to file
        KeyCode::Char('e') => {
            let path = state.default_export_path();
            match state.export_lines(&path) {
                Ok(count) => {
                    state.status_message = Some(format!("Exported {} lines to {}", count, path));
                }
                Err(e) => {
                    state.status_message = Some(format!("Export failed: {}", e));
                }
            }
        }

        // Save current filter
        KeyCode::Char('s') => {
            if state.active_filter.is_some() {
                // Simple auto-naming based on pattern
                let pattern = state.filter_input();
                let name = if pattern.len() > 10 {
                    format!("{}...", &pattern[..10])
                } else {
                    pattern
                };
                state.save_current_filter(name);
            } else {
                state.status_message = Some("No active filter to save".to_string());
            }
        }

        // Delete selected saved filter
        KeyCode::Char('x') | KeyCode::Delete => {
            if state.focused_panel == FocusedPanel::Filters && !state.saved_filters.is_empty() {
                state.saved_filters.remove(state.selected_filter_idx);
                if state.selected_filter_idx >= state.saved_filters.len()
                    && state.selected_filter_idx > 0
                {
                    state.selected_filter_idx -= 1;
                }
                state.status_message = Some("Filter deleted".to_string());
            }
        }

        // Clear filter
        KeyCode::Esc => {
            if state.active_filter.is_some() {
                state.active_filter = None;
                state.filter_textarea.select_all();
                state.filter_textarea.cut();
                state.recompute_filter();
                state.status_message = Some("Filter cleared".to_string());
            }
        }

        _ => {}
    }
}

fn handle_filter_mode(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            state.apply_filter();
        }
        KeyCode::Esc => {
            state.cancel_filter();
        }
        // Toggle regex mode with Ctrl+R
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.toggle_regex_mode();
        }
        _ => {
            // Forward all other keys to the textarea
            let input = Input::from(key);
            if state.filter_textarea.input(input) {
                // Text changed, mark for debounce
                state.filter_changed();
            }
        }
    }
}

fn handle_source_select_mode(state: &mut AppState, key: KeyEvent) {
    // Future: handle up/down for source selection
    if key.code == KeyCode::Esc {
        state.mode = InputMode::Normal;
    }
}
