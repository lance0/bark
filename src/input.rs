//! Keyboard and mouse input handling.
//!
//! Maps key events to application actions based on current input mode.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use regex::Regex;
use tui_textarea::Input;

use crate::app::{AppState, FocusedPanel, InputMode, PickerMode, SourceViewMode};
use crate::config::MOUSE_SCROLL_LINES;

/// Strip ANSI escape codes from a string
fn strip_ansi_codes(s: &str) -> String {
    // Regex to match ANSI escape sequences
    let ansi_regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    ansi_regex.replace_all(s, "").to_string()
}

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
        MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            // Check if click is in any log view pane
            let col = mouse.column;
            let row = mouse.row;

            for (pane_idx, area) in state.log_view_areas.iter().enumerate() {
                if col >= area.x
                    && col < area.x + area.width
                    && row >= area.y
                    && row < area.y + area.height
                {
                    // Click is in this pane
                    // Switch to this pane if in split mode
                    if state.is_split() && pane_idx != state.active_pane {
                        state.active_pane = pane_idx;
                    }

                    // Calculate which line was clicked (relative to pane top)
                    let line_row = (row - area.y) as usize;
                    state.select_line_at_row(line_row);
                    return;
                }
            }

            // Click outside log view - clear selection
            state.clear_selection();
        }
        _ => {}
    }
}

/// Handle a key event and update app state accordingly
/// Note: Picker input should be handled by main loop using handle_picker_input
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

    // Settings overlay
    if state.settings.visible {
        handle_settings_input(state, key);
        return;
    }

    // Note: Picker mode is handled separately by main loop

    match state.mode {
        InputMode::Normal => handle_normal_mode(state, key, page_size),
        InputMode::FilterEditing => handle_filter_mode(state, key),
        InputMode::SourceSelect => handle_source_select_mode(state, key),
        InputMode::SplitCommand => handle_split_command(state, key),
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
                if let Some(visible) = state.panes[state.active_pane].visible_sources.get_mut(idx) {
                    *visible = !*visible;
                }
                let is_visible = state.panes[state.active_pane]
                    .visible_sources
                    .get(idx)
                    .copied()
                    .unwrap_or(true);
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
                state.panes[state.active_pane].view_mode =
                    match state.panes[state.active_pane].view_mode {
                        SourceViewMode::SingleSource(id) if id == state.current_source_idx => {
                            SourceViewMode::AllMerged
                        }
                        _ => SourceViewMode::SingleSource(state.current_source_idx),
                    };
                state.recompute_filter();
                state.status_message = Some(match state.panes[state.active_pane].view_mode {
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
                for v in state.panes[state.active_pane].visible_sources.iter_mut() {
                    *v = true;
                }
                state.panes[state.active_pane].view_mode = SourceViewMode::AllMerged;
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
            state.panes[state.active_pane].filter_textarea.select_all();
            state.panes[state.active_pane].filter_textarea.cut();
        }

        // Toggle regex mode
        KeyCode::Char('r') => {
            state.toggle_regex_mode();
        }

        // Next match
        KeyCode::Char('n') => {
            if state.panes[state.active_pane].active_filter.is_some() {
                state.next_match();
            } else {
                state.status_message = Some("No active filter (use / to filter)".to_string());
            }
        }

        // Previous match
        KeyCode::Char('N') => {
            if state.panes[state.active_pane].active_filter.is_some() {
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

        // Toggle line wrap (without modifier)
        KeyCode::Char('w') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
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

        // Toggle pause (stop following new logs)
        KeyCode::Char('p') => {
            state.panes[state.active_pane].stick_to_bottom =
                !state.panes[state.active_pane].stick_to_bottom;
            state.status_message = Some(if state.panes[state.active_pane].stick_to_bottom {
                "Following logs (auto-scroll ON)".to_string()
            } else {
                "PAUSED - press 'p' or 'G' to resume".to_string()
            });
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
            if state.panes[state.active_pane].active_filter.is_some() {
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

        // Clear selection or filter
        KeyCode::Esc => {
            // First clear selection if present
            if state.panes[state.active_pane].selected_line.is_some() {
                state.clear_selection();
            } else if state.panes[state.active_pane].active_filter.is_some() {
                // Then clear filter if no selection
                state.panes[state.active_pane].active_filter = None;
                state.panes[state.active_pane].filter_textarea.select_all();
                state.panes[state.active_pane].filter_textarea.cut();
                state.recompute_filter();
                state.status_message = Some("Filter cleared".to_string());
            }
        }

        // Open Docker picker
        KeyCode::Char('D') => {
            state.picker.open(PickerMode::Docker);
        }

        // Open K8s picker
        KeyCode::Char('K') => {
            state.picker.open(PickerMode::K8s);
        }

        // Open settings
        KeyCode::Char('S') => {
            state.settings.open();
        }

        // Toggle line numbers
        KeyCode::Char('#') => {
            state.toggle_line_numbers();
        }

        // Split view commands (Ctrl+W prefix)
        KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.mode = InputMode::SplitCommand;
        }

        // Yank (copy) current/selected line to clipboard
        KeyCode::Char('y') => {
            if let Some(line) = state.get_current_line_text() {
                match state.clipboard() {
                    Ok(clipboard) => {
                        // Strip ANSI codes for clipboard
                        let clean_line = strip_ansi_codes(&line);
                        match clipboard.set_text(clean_line) {
                            Ok(()) => {
                                state.status_message = Some("Yanked line to clipboard".to_string());
                                state.clear_selection(); // Clear selection after yank
                            }
                            Err(e) => {
                                state.status_message = Some(format!("Clipboard error: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        state.status_message = Some(format!("Clipboard unavailable: {}", e));
                    }
                }
            } else {
                state.status_message = Some("No line to yank".to_string());
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
        // Navigate filter history with Up/Down
        KeyCode::Up => {
            if !state.filter_history_up() {
                // If no history, let textarea handle it
                let input = Input::from(key);
                if state.panes[state.active_pane].filter_textarea.input(input) {
                    state.filter_changed();
                }
            }
        }
        KeyCode::Down => {
            if !state.filter_history_down() {
                // If not browsing history, let textarea handle it
                let input = Input::from(key);
                if state.panes[state.active_pane].filter_textarea.input(input) {
                    state.filter_changed();
                }
            }
        }
        _ => {
            // Forward all other keys to the textarea
            let input = Input::from(key);
            if state.panes[state.active_pane].filter_textarea.input(input) {
                // Text changed, mark for debounce
                state.filter_changed();
                // Reset history browsing when user types
                state.panes[state.active_pane].filter_history_idx = None;
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

/// Handle split command mode (after Ctrl+W)
fn handle_split_command(state: &mut AppState, key: KeyEvent) {
    // Reset to normal mode after any key
    state.mode = InputMode::Normal;

    match key.code {
        // Vertical split (side-by-side)
        KeyCode::Char('v') => {
            state.split_vertical();
        }
        // Horizontal split (stacked)
        KeyCode::Char('s') => {
            state.split_horizontal();
        }
        // Close current pane
        KeyCode::Char('q') => {
            state.close_pane();
        }
        // Cycle to next pane
        KeyCode::Char('w') => {
            state.cycle_pane();
        }
        // Navigate to left pane (for vertical split)
        KeyCode::Char('h') => {
            state.focus_pane_left();
        }
        // Navigate to right pane (for vertical split)
        KeyCode::Char('l') => {
            state.focus_pane_right();
        }
        // Navigate to up pane (for horizontal split)
        KeyCode::Char('k') => {
            state.focus_pane_up();
        }
        // Navigate to down pane (for horizontal split)
        KeyCode::Char('j') => {
            state.focus_pane_down();
        }
        // Escape cancels split command
        KeyCode::Esc => {}
        _ => {
            state.status_message =
                Some("Split: v=vertical s=horizontal q=close w=cycle".to_string());
        }
    }
}

/// Handle input when settings overlay is open
fn handle_settings_input(state: &mut AppState, key: KeyEvent) {
    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => {
            state.settings.down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.settings.up();
        }

        // Toggle selected setting
        KeyCode::Char(' ') | KeyCode::Enter => {
            let selected = state.settings.selected;
            state.toggle_setting(selected);
        }

        // Close settings
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('S') => {
            state.settings.close();
        }

        _ => {}
    }
}

/// Selected source info from picker
#[derive(Debug, Clone)]
pub struct SelectedSource {
    pub name: String,
    pub namespace: Option<String>,
}

/// Picker action to be returned for main loop to handle
#[derive(Debug)]
pub enum PickerAction {
    /// No action needed
    None,
    /// Close the picker
    Close,
    /// Modify sources - returns (sources to add, sources to remove, mode)
    ModifySources {
        add: Vec<SelectedSource>,
        remove: Vec<SelectedSource>,
        mode: PickerMode,
    },
}

/// Handle picker mode input - returns action for main loop
pub fn handle_picker_input(state: &mut AppState, key: KeyEvent) -> PickerAction {
    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => {
            state.picker.down();
            PickerAction::None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.picker.up();
            PickerAction::None
        }

        // Toggle selection
        KeyCode::Char(' ') => {
            state.picker.toggle_selected();
            PickerAction::None
        }

        // Confirm selection - add/remove sources
        KeyCode::Enter => {
            if state.picker.sources.is_empty() {
                state.picker.close();
                return PickerAction::Close;
            }

            let mode = state.picker.mode;

            // Get sources to add (newly checked, not initially checked)
            let add: Vec<SelectedSource> = state
                .picker
                .sources
                .iter()
                .enumerate()
                .filter(|(i, _)| {
                    // Currently checked but wasn't initially checked
                    state.picker.checked.get(*i).copied().unwrap_or(false)
                        && !state
                            .picker
                            .initial_checked
                            .get(*i)
                            .copied()
                            .unwrap_or(false)
                })
                .map(|(_, s)| SelectedSource {
                    name: s.name.clone(),
                    namespace: s.namespace.clone(),
                })
                .collect();

            // Get sources to remove (initially checked but now unchecked)
            let remove: Vec<SelectedSource> = state
                .picker
                .get_unchecked_sources()
                .iter()
                .map(|s| SelectedSource {
                    name: s.name.clone(),
                    namespace: s.namespace.clone(),
                })
                .collect();

            state.picker.close();

            // Fallback: if no explicit changes, add the highlighted item (if not already added)
            if add.is_empty() && remove.is_empty() {
                let selected = state.picker.selected;
                if let Some(source) = state.picker.sources.get(selected) {
                    // Only add if it wasn't initially checked (not already a source)
                    let was_initial = state
                        .picker
                        .initial_checked
                        .get(selected)
                        .copied()
                        .unwrap_or(false);
                    if !was_initial {
                        return PickerAction::ModifySources {
                            add: vec![SelectedSource {
                                name: source.name.clone(),
                                namespace: source.namespace.clone(),
                            }],
                            remove: vec![],
                            mode,
                        };
                    }
                }
                return PickerAction::Close;
            }

            PickerAction::ModifySources { add, remove, mode }
        }

        // Cancel
        KeyCode::Esc | KeyCode::Char('q') => {
            state.picker.close();
            PickerAction::Close
        }

        _ => PickerAction::None,
    }
}
