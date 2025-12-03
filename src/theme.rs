//! Color theme definitions for the UI.
//!
//! Provides 11 built-in themes: default, kawaii, cyber, dracula, monochrome,
//! matrix, nord, gruvbox, catppuccin, tokyo_night, solarized.
//! Themes can be selected via the `theme` config option or `BARK_THEME` env var.

use ratatui::style::Color;

/// All themeable colors in the application
#[derive(Clone, Debug)]
pub struct Theme {
    // Theme identifier
    name: &'static str,

    // Log levels
    pub level_error: Color,
    pub level_warn: Color,
    pub level_info: Color,
    pub level_debug: Color,
    pub level_trace: Color,

    // UI borders
    pub border_focused: Color,
    pub border_unfocused: Color,

    // Header
    pub header_title: Color,
    pub header_source: Color,
    pub header_bg: Color,

    // Status bar
    pub status_mode_bg: Color,
    pub status_mode_fg: Color,
    pub status_help: Color,
    pub status_bg: Color,

    // Highlights
    pub highlight_match_bg: Color,
    pub highlight_match_fg: Color,

    // Bookmarks
    pub bookmark: Color,

    // Timestamps
    pub timestamp: Color,

    // JSON pretty-print
    pub json: Color,

    // Filters panel
    pub filter_selected: Color,
    pub filter_prefix: Color,

    // Sources panel
    #[allow(dead_code)]
    pub source_current: Color,

    // Empty states / messages
    pub empty_state: Color,
    pub warning_message: Color,

    // Help overlay
    pub help_border: Color,
    pub help_bg: Color,

    // Source colors (for multi-source display)
    pub source_colors: Vec<Color>,
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_theme()
    }
}

impl Theme {
    /// The default theme - modern dark with vibrant accents
    pub fn default_theme() -> Self {
        Self {
            name: "default",

            // Log levels - vibrant but balanced
            level_error: Color::Rgb(255, 85, 85),    // Soft red
            level_warn: Color::Rgb(255, 184, 108),   // Warm orange
            level_info: Color::Rgb(80, 250, 123),    // Bright green
            level_debug: Color::Rgb(139, 233, 253),  // Cyan
            level_trace: Color::Rgb(98, 114, 164),   // Muted purple-gray

            // UI borders
            border_focused: Color::Rgb(80, 250, 123),  // Bright green
            border_unfocused: Color::Rgb(68, 71, 90),  // Subtle gray

            // Header
            header_title: Color::Rgb(80, 250, 123),    // Bright green
            header_source: Color::Rgb(139, 233, 253),  // Cyan
            header_bg: Color::Rgb(40, 42, 54),         // Dark purple-gray

            // Status bar
            status_mode_bg: Color::Rgb(189, 147, 249), // Purple
            status_mode_fg: Color::Rgb(40, 42, 54),    // Dark bg
            status_help: Color::Rgb(98, 114, 164),     // Muted
            status_bg: Color::Rgb(33, 34, 44),         // Darker bg

            // Highlights
            highlight_match_bg: Color::Rgb(255, 184, 108), // Orange
            highlight_match_fg: Color::Rgb(40, 42, 54),

            // Bookmarks
            bookmark: Color::Rgb(255, 121, 198),  // Pink

            // Timestamps
            timestamp: Color::Rgb(98, 114, 164),  // Muted purple-gray

            // JSON
            json: Color::Rgb(139, 233, 253),      // Cyan

            // Filters
            filter_selected: Color::Rgb(255, 184, 108),
            filter_prefix: Color::Rgb(189, 147, 249),  // Purple

            // Sources
            source_current: Color::Rgb(80, 250, 123),

            // Empty states
            empty_state: Color::Rgb(98, 114, 164),
            warning_message: Color::Rgb(255, 184, 108),

            // Help
            help_border: Color::Rgb(189, 147, 249),  // Purple
            help_bg: Color::Rgb(40, 42, 54),

            // Source colors for multi-source display
            source_colors: vec![
                Color::Rgb(139, 233, 253),  // Cyan
                Color::Rgb(255, 184, 108),  // Orange
                Color::Rgb(255, 121, 198),  // Pink
                Color::Rgb(80, 250, 123),   // Green
                Color::Rgb(189, 147, 249),  // Purple
                Color::Rgb(241, 250, 140),  // Yellow
            ],
        }
    }

    /// Kawaii theme - cute pastel colors
    pub fn kawaii() -> Self {
        Self {
            name: "kawaii",

            // Log levels - soft pastels
            level_error: Color::Rgb(255, 121, 162), // Soft pink
            level_warn: Color::Rgb(255, 200, 152),  // Soft peach
            level_info: Color::Rgb(152, 255, 200),  // Soft mint
            level_debug: Color::Rgb(162, 200, 255), // Soft lavender-blue
            level_trace: Color::Rgb(180, 180, 200), // Soft gray-lavender

            // UI borders
            border_focused: Color::Rgb(255, 182, 214), // Pink
            border_unfocused: Color::Rgb(180, 180, 200),

            // Header
            header_title: Color::Rgb(255, 182, 214),  // Pink
            header_source: Color::Rgb(182, 214, 255), // Light blue
            header_bg: Color::Rgb(60, 50, 70),        // Dark purple-gray

            // Status bar
            status_mode_bg: Color::Rgb(214, 182, 255), // Light purple
            status_mode_fg: Color::Rgb(40, 30, 50),    // Dark
            status_help: Color::Rgb(180, 180, 200),
            status_bg: Color::Rgb(40, 30, 50),

            // Highlights
            highlight_match_bg: Color::Rgb(255, 214, 182), // Soft orange
            highlight_match_fg: Color::Rgb(40, 30, 50),

            // Bookmarks
            bookmark: Color::Rgb(255, 150, 200), // Hot pink

            // Timestamps
            timestamp: Color::Rgb(180, 180, 200),

            // JSON
            json: Color::Rgb(182, 255, 214), // Mint

            // Filters
            filter_selected: Color::Rgb(255, 214, 182), // Peach
            filter_prefix: Color::Rgb(255, 182, 214),   // Pink

            // Sources
            source_current: Color::Rgb(182, 255, 214), // Mint

            // Empty states
            empty_state: Color::Rgb(180, 180, 200),
            warning_message: Color::Rgb(255, 200, 152),

            // Help
            help_border: Color::Rgb(255, 182, 214),
            help_bg: Color::Rgb(40, 30, 50),

            // Source colors - pastel rainbow
            source_colors: vec![
                Color::Rgb(255, 182, 214), // Pink
                Color::Rgb(182, 255, 214), // Mint
                Color::Rgb(182, 214, 255), // Light blue
                Color::Rgb(255, 214, 182), // Peach
                Color::Rgb(214, 182, 255), // Lavender
                Color::Rgb(255, 255, 182), // Cream yellow
            ],
        }
    }

    /// Cyber/Futuristic theme - neon on dark
    pub fn cyber() -> Self {
        Self {
            name: "cyber",

            // Log levels - neon colors
            level_error: Color::Rgb(255, 50, 100), // Neon red-pink
            level_warn: Color::Rgb(255, 200, 0),   // Neon yellow
            level_info: Color::Rgb(0, 255, 150),   // Neon green
            level_debug: Color::Rgb(0, 200, 255),  // Electric blue
            level_trace: Color::Rgb(100, 100, 120), // Muted gray-blue

            // UI borders
            border_focused: Color::Rgb(0, 255, 255), // Cyan neon
            border_unfocused: Color::Rgb(60, 60, 80),

            // Header
            header_title: Color::Rgb(255, 0, 255), // Magenta neon
            header_source: Color::Rgb(0, 255, 255), // Cyan neon
            header_bg: Color::Rgb(20, 20, 35),     // Very dark blue

            // Status bar
            status_mode_bg: Color::Rgb(255, 0, 255), // Magenta
            status_mode_fg: Color::Rgb(0, 0, 0),     // Black
            status_help: Color::Rgb(100, 100, 120),
            status_bg: Color::Rgb(10, 10, 20),

            // Highlights
            highlight_match_bg: Color::Rgb(0, 255, 255), // Cyan
            highlight_match_fg: Color::Rgb(0, 0, 0),

            // Bookmarks
            bookmark: Color::Rgb(255, 100, 255), // Pink-magenta

            // Timestamps
            timestamp: Color::Rgb(100, 100, 120),

            // JSON
            json: Color::Rgb(0, 200, 255), // Electric blue

            // Filters
            filter_selected: Color::Rgb(255, 200, 0), // Yellow neon
            filter_prefix: Color::Rgb(0, 255, 255),   // Cyan

            // Sources
            source_current: Color::Rgb(0, 255, 150), // Neon green

            // Empty states
            empty_state: Color::Rgb(100, 100, 120),
            warning_message: Color::Rgb(255, 200, 0),

            // Help
            help_border: Color::Rgb(0, 255, 255),
            help_bg: Color::Rgb(10, 10, 20),

            // Source colors - neon palette
            source_colors: vec![
                Color::Rgb(0, 255, 255),   // Cyan
                Color::Rgb(255, 0, 255),   // Magenta
                Color::Rgb(0, 255, 150),   // Neon green
                Color::Rgb(255, 200, 0),   // Yellow
                Color::Rgb(0, 200, 255),   // Electric blue
                Color::Rgb(255, 100, 100), // Neon red
            ],
        }
    }

    /// Dracula theme - popular dark theme
    pub fn dracula() -> Self {
        Self {
            name: "dracula",

            // Log levels - Dracula palette
            level_error: Color::Rgb(255, 85, 85),   // Red
            level_warn: Color::Rgb(255, 184, 108),  // Orange
            level_info: Color::Rgb(80, 250, 123),   // Green
            level_debug: Color::Rgb(139, 233, 253), // Cyan
            level_trace: Color::Rgb(98, 114, 164),  // Comment gray

            // UI borders
            border_focused: Color::Rgb(189, 147, 249), // Purple
            border_unfocused: Color::Rgb(68, 71, 90),  // Current line

            // Header
            header_title: Color::Rgb(255, 121, 198),  // Pink
            header_source: Color::Rgb(139, 233, 253), // Cyan
            header_bg: Color::Rgb(40, 42, 54),        // Background

            // Status bar
            status_mode_bg: Color::Rgb(189, 147, 249), // Purple
            status_mode_fg: Color::Rgb(40, 42, 54),    // Background
            status_bg: Color::Rgb(33, 34, 44),         // Darker bg
            status_help: Color::Rgb(98, 114, 164),     // Comment

            // Highlights
            highlight_match_bg: Color::Rgb(241, 250, 140), // Yellow
            highlight_match_fg: Color::Rgb(40, 42, 54),

            // Bookmarks
            bookmark: Color::Rgb(255, 121, 198), // Pink

            // Timestamps
            timestamp: Color::Rgb(98, 114, 164), // Comment

            // JSON
            json: Color::Rgb(139, 233, 253), // Cyan

            // Filters
            filter_selected: Color::Rgb(241, 250, 140), // Yellow
            filter_prefix: Color::Rgb(255, 184, 108),   // Orange

            // Sources
            source_current: Color::Rgb(80, 250, 123), // Green

            // Empty states
            empty_state: Color::Rgb(98, 114, 164),
            warning_message: Color::Rgb(255, 184, 108),

            // Help
            help_border: Color::Rgb(189, 147, 249),
            help_bg: Color::Rgb(40, 42, 54),

            // Source colors - Dracula palette
            source_colors: vec![
                Color::Rgb(139, 233, 253), // Cyan
                Color::Rgb(255, 121, 198), // Pink
                Color::Rgb(80, 250, 123),  // Green
                Color::Rgb(255, 184, 108), // Orange
                Color::Rgb(189, 147, 249), // Purple
                Color::Rgb(241, 250, 140), // Yellow
            ],
        }
    }

    /// Monochrome theme - grayscale only
    pub fn monochrome() -> Self {
        Self {
            name: "monochrome",

            // Log levels - varying grays
            level_error: Color::Rgb(255, 255, 255), // White (stands out)
            level_warn: Color::Rgb(200, 200, 200),  // Light gray
            level_info: Color::Rgb(170, 170, 170),  // Medium-light
            level_debug: Color::Rgb(140, 140, 140), // Medium
            level_trace: Color::Rgb(100, 100, 100), // Dark

            // UI borders
            border_focused: Color::Rgb(200, 200, 200),
            border_unfocused: Color::Rgb(80, 80, 80),

            // Header
            header_title: Color::Rgb(255, 255, 255),
            header_source: Color::Rgb(180, 180, 180),
            header_bg: Color::Rgb(50, 50, 50),

            // Status bar
            status_mode_bg: Color::Rgb(200, 200, 200),
            status_mode_fg: Color::Rgb(0, 0, 0),
            status_help: Color::Rgb(120, 120, 120),
            status_bg: Color::Rgb(30, 30, 30),

            // Highlights
            highlight_match_bg: Color::Rgb(200, 200, 200),
            highlight_match_fg: Color::Rgb(0, 0, 0),

            // Bookmarks
            bookmark: Color::Rgb(255, 255, 255),

            // Timestamps
            timestamp: Color::Rgb(120, 120, 120),

            // JSON
            json: Color::Rgb(180, 180, 180),

            // Filters
            filter_selected: Color::Rgb(255, 255, 255),
            filter_prefix: Color::Rgb(180, 180, 180),

            // Sources
            source_current: Color::Rgb(255, 255, 255),

            // Empty states
            empty_state: Color::Rgb(120, 120, 120),
            warning_message: Color::Rgb(200, 200, 200),

            // Help
            help_border: Color::Rgb(180, 180, 180),
            help_bg: Color::Rgb(20, 20, 20),

            // Source colors - varying grays
            source_colors: vec![
                Color::Rgb(220, 220, 220),
                Color::Rgb(180, 180, 180),
                Color::Rgb(140, 140, 140),
                Color::Rgb(200, 200, 200),
                Color::Rgb(160, 160, 160),
                Color::Rgb(240, 240, 240),
            ],
        }
    }

    /// Matrix theme - green on black hacker style
    pub fn matrix() -> Self {
        Self {
            name: "matrix",

            // Log levels - shades of green
            level_error: Color::Rgb(255, 100, 100),   // Red stands out
            level_warn: Color::Rgb(200, 255, 100),    // Yellow-green
            level_info: Color::Rgb(0, 255, 0),        // Bright green
            level_debug: Color::Rgb(0, 200, 0),       // Medium green
            level_trace: Color::Rgb(0, 100, 0),       // Dark green

            // UI borders
            border_focused: Color::Rgb(0, 255, 0),
            border_unfocused: Color::Rgb(0, 80, 0),

            // Header
            header_title: Color::Rgb(0, 255, 0),
            header_source: Color::Rgb(100, 255, 100),
            header_bg: Color::Rgb(0, 20, 0),

            // Status bar
            status_mode_bg: Color::Rgb(0, 200, 0),
            status_mode_fg: Color::Rgb(0, 0, 0),
            status_help: Color::Rgb(0, 100, 0),
            status_bg: Color::Rgb(0, 10, 0),

            // Highlights
            highlight_match_bg: Color::Rgb(0, 255, 0),
            highlight_match_fg: Color::Rgb(0, 0, 0),

            // Bookmarks
            bookmark: Color::Rgb(150, 255, 150),

            // Timestamps
            timestamp: Color::Rgb(0, 120, 0),

            // JSON
            json: Color::Rgb(100, 255, 100),

            // Filters
            filter_selected: Color::Rgb(0, 255, 0),
            filter_prefix: Color::Rgb(0, 200, 0),

            // Sources
            source_current: Color::Rgb(0, 255, 0),

            // Empty states
            empty_state: Color::Rgb(0, 100, 0),
            warning_message: Color::Rgb(200, 255, 100),

            // Help
            help_border: Color::Rgb(0, 255, 0),
            help_bg: Color::Rgb(0, 10, 0),

            // Source colors - green variations
            source_colors: vec![
                Color::Rgb(0, 255, 0),
                Color::Rgb(100, 255, 100),
                Color::Rgb(0, 200, 0),
                Color::Rgb(150, 255, 150),
                Color::Rgb(0, 255, 100),
                Color::Rgb(100, 255, 0),
            ],
        }
    }

    /// Nord theme - arctic, north-bluish colors
    pub fn nord() -> Self {
        Self {
            name: "nord",

            // Log levels - Nord palette
            level_error: Color::Rgb(191, 97, 106),    // Nord11 red
            level_warn: Color::Rgb(235, 203, 139),    // Nord13 yellow
            level_info: Color::Rgb(163, 190, 140),    // Nord14 green
            level_debug: Color::Rgb(129, 161, 193),   // Nord9 blue
            level_trace: Color::Rgb(76, 86, 106),     // Nord3

            // UI borders
            border_focused: Color::Rgb(136, 192, 208),  // Nord8 cyan
            border_unfocused: Color::Rgb(76, 86, 106),  // Nord3

            // Header
            header_title: Color::Rgb(136, 192, 208),    // Nord8
            header_source: Color::Rgb(129, 161, 193),   // Nord9
            header_bg: Color::Rgb(46, 52, 64),          // Nord0

            // Status bar
            status_mode_bg: Color::Rgb(136, 192, 208),  // Nord8
            status_mode_fg: Color::Rgb(46, 52, 64),     // Nord0
            status_help: Color::Rgb(76, 86, 106),       // Nord3
            status_bg: Color::Rgb(59, 66, 82),          // Nord1

            // Highlights
            highlight_match_bg: Color::Rgb(235, 203, 139), // Nord13
            highlight_match_fg: Color::Rgb(46, 52, 64),

            // Bookmarks
            bookmark: Color::Rgb(180, 142, 173),  // Nord15 purple

            // Timestamps
            timestamp: Color::Rgb(76, 86, 106),   // Nord3

            // JSON
            json: Color::Rgb(143, 188, 187),      // Nord7

            // Filters
            filter_selected: Color::Rgb(235, 203, 139),
            filter_prefix: Color::Rgb(136, 192, 208),

            // Sources
            source_current: Color::Rgb(163, 190, 140),

            // Empty states
            empty_state: Color::Rgb(76, 86, 106),
            warning_message: Color::Rgb(235, 203, 139),

            // Help
            help_border: Color::Rgb(136, 192, 208),
            help_bg: Color::Rgb(46, 52, 64),

            // Source colors - Nord accent colors
            source_colors: vec![
                Color::Rgb(136, 192, 208),  // Nord8 cyan
                Color::Rgb(163, 190, 140),  // Nord14 green
                Color::Rgb(235, 203, 139),  // Nord13 yellow
                Color::Rgb(208, 135, 112),  // Nord12 orange
                Color::Rgb(180, 142, 173),  // Nord15 purple
                Color::Rgb(191, 97, 106),   // Nord11 red
            ],
        }
    }

    /// Gruvbox theme - retro groove colors
    pub fn gruvbox() -> Self {
        Self {
            name: "gruvbox",

            // Log levels - Gruvbox palette
            level_error: Color::Rgb(251, 73, 52),     // Red
            level_warn: Color::Rgb(250, 189, 47),     // Yellow
            level_info: Color::Rgb(184, 187, 38),     // Green
            level_debug: Color::Rgb(131, 165, 152),   // Aqua
            level_trace: Color::Rgb(146, 131, 116),   // Gray

            // UI borders
            border_focused: Color::Rgb(254, 128, 25),   // Orange
            border_unfocused: Color::Rgb(80, 73, 69),   // bg2

            // Header
            header_title: Color::Rgb(254, 128, 25),     // Orange
            header_source: Color::Rgb(142, 192, 124),   // Bright green
            header_bg: Color::Rgb(40, 40, 40),          // bg0

            // Status bar
            status_mode_bg: Color::Rgb(254, 128, 25),   // Orange
            status_mode_fg: Color::Rgb(40, 40, 40),     // bg0
            status_help: Color::Rgb(146, 131, 116),     // Gray
            status_bg: Color::Rgb(50, 48, 47),          // bg0_s

            // Highlights
            highlight_match_bg: Color::Rgb(250, 189, 47), // Yellow
            highlight_match_fg: Color::Rgb(40, 40, 40),

            // Bookmarks
            bookmark: Color::Rgb(211, 134, 155),  // Purple

            // Timestamps
            timestamp: Color::Rgb(146, 131, 116), // Gray

            // JSON
            json: Color::Rgb(131, 165, 152),      // Aqua

            // Filters
            filter_selected: Color::Rgb(250, 189, 47),
            filter_prefix: Color::Rgb(254, 128, 25),

            // Sources
            source_current: Color::Rgb(184, 187, 38),

            // Empty states
            empty_state: Color::Rgb(146, 131, 116),
            warning_message: Color::Rgb(250, 189, 47),

            // Help
            help_border: Color::Rgb(254, 128, 25),
            help_bg: Color::Rgb(40, 40, 40),

            // Source colors - Gruvbox bright colors
            source_colors: vec![
                Color::Rgb(254, 128, 25),   // Orange
                Color::Rgb(142, 192, 124),  // Green
                Color::Rgb(250, 189, 47),   // Yellow
                Color::Rgb(131, 165, 152),  // Aqua
                Color::Rgb(211, 134, 155),  // Purple
                Color::Rgb(251, 73, 52),    // Red
            ],
        }
    }

    /// Catppuccin Mocha theme - soothing pastel colors
    pub fn catppuccin() -> Self {
        Self {
            name: "catppuccin",

            // Log levels - Catppuccin Mocha palette
            level_error: Color::Rgb(243, 139, 168),   // Red
            level_warn: Color::Rgb(249, 226, 175),    // Yellow
            level_info: Color::Rgb(166, 227, 161),    // Green
            level_debug: Color::Rgb(137, 180, 250),   // Blue
            level_trace: Color::Rgb(108, 112, 134),   // Overlay0

            // UI borders
            border_focused: Color::Rgb(203, 166, 247),  // Mauve
            border_unfocused: Color::Rgb(88, 91, 112),  // Surface2

            // Header
            header_title: Color::Rgb(245, 194, 231),    // Pink
            header_source: Color::Rgb(137, 180, 250),   // Blue
            header_bg: Color::Rgb(30, 30, 46),          // Base

            // Status bar
            status_mode_bg: Color::Rgb(203, 166, 247),  // Mauve
            status_mode_fg: Color::Rgb(30, 30, 46),     // Base
            status_help: Color::Rgb(108, 112, 134),     // Overlay0
            status_bg: Color::Rgb(24, 24, 37),          // Mantle

            // Highlights
            highlight_match_bg: Color::Rgb(249, 226, 175), // Yellow
            highlight_match_fg: Color::Rgb(30, 30, 46),

            // Bookmarks
            bookmark: Color::Rgb(245, 194, 231),  // Pink

            // Timestamps
            timestamp: Color::Rgb(108, 112, 134), // Overlay0

            // JSON
            json: Color::Rgb(148, 226, 213),      // Teal

            // Filters
            filter_selected: Color::Rgb(249, 226, 175),
            filter_prefix: Color::Rgb(203, 166, 247),

            // Sources
            source_current: Color::Rgb(166, 227, 161),

            // Empty states
            empty_state: Color::Rgb(108, 112, 134),
            warning_message: Color::Rgb(249, 226, 175),

            // Help
            help_border: Color::Rgb(203, 166, 247),
            help_bg: Color::Rgb(30, 30, 46),

            // Source colors - Catppuccin accent colors
            source_colors: vec![
                Color::Rgb(137, 180, 250),  // Blue
                Color::Rgb(166, 227, 161),  // Green
                Color::Rgb(249, 226, 175),  // Yellow
                Color::Rgb(250, 179, 135),  // Peach
                Color::Rgb(203, 166, 247),  // Mauve
                Color::Rgb(148, 226, 213),  // Teal
            ],
        }
    }

    /// Tokyo Night theme - dark theme inspired by Tokyo city lights
    pub fn tokyo_night() -> Self {
        Self {
            name: "tokyo_night",

            // Log levels - Tokyo Night palette
            level_error: Color::Rgb(247, 118, 142),   // Red
            level_warn: Color::Rgb(224, 175, 104),    // Yellow
            level_info: Color::Rgb(158, 206, 106),    // Green
            level_debug: Color::Rgb(122, 162, 247),   // Blue
            level_trace: Color::Rgb(86, 95, 137),     // Comment

            // UI borders
            border_focused: Color::Rgb(187, 154, 247),  // Purple
            border_unfocused: Color::Rgb(59, 66, 97),   // bg_highlight

            // Header
            header_title: Color::Rgb(187, 154, 247),    // Purple
            header_source: Color::Rgb(125, 207, 255),   // Cyan
            header_bg: Color::Rgb(26, 27, 38),          // bg_dark

            // Status bar
            status_mode_bg: Color::Rgb(187, 154, 247),  // Purple
            status_mode_fg: Color::Rgb(26, 27, 38),     // bg_dark
            status_help: Color::Rgb(86, 95, 137),       // Comment
            status_bg: Color::Rgb(22, 22, 30),          // bg

            // Highlights
            highlight_match_bg: Color::Rgb(224, 175, 104), // Yellow
            highlight_match_fg: Color::Rgb(26, 27, 38),

            // Bookmarks
            bookmark: Color::Rgb(255, 117, 127),  // Magenta

            // Timestamps
            timestamp: Color::Rgb(86, 95, 137),   // Comment

            // JSON
            json: Color::Rgb(125, 207, 255),      // Cyan

            // Filters
            filter_selected: Color::Rgb(224, 175, 104),
            filter_prefix: Color::Rgb(187, 154, 247),

            // Sources
            source_current: Color::Rgb(158, 206, 106),

            // Empty states
            empty_state: Color::Rgb(86, 95, 137),
            warning_message: Color::Rgb(224, 175, 104),

            // Help
            help_border: Color::Rgb(187, 154, 247),
            help_bg: Color::Rgb(26, 27, 38),

            // Source colors - Tokyo Night accent colors
            source_colors: vec![
                Color::Rgb(125, 207, 255),  // Cyan
                Color::Rgb(158, 206, 106),  // Green
                Color::Rgb(224, 175, 104),  // Yellow
                Color::Rgb(255, 158, 100),  // Orange
                Color::Rgb(187, 154, 247),  // Purple
                Color::Rgb(247, 118, 142),  // Red
            ],
        }
    }

    /// Solarized Dark theme - precision colors for readability
    pub fn solarized() -> Self {
        Self {
            name: "solarized",

            // Log levels - Solarized accent colors
            level_error: Color::Rgb(220, 50, 47),     // Red
            level_warn: Color::Rgb(181, 137, 0),      // Yellow
            level_info: Color::Rgb(133, 153, 0),      // Green
            level_debug: Color::Rgb(38, 139, 210),    // Blue
            level_trace: Color::Rgb(88, 110, 117),    // base01

            // UI borders
            border_focused: Color::Rgb(42, 161, 152),   // Cyan
            border_unfocused: Color::Rgb(7, 54, 66),    // base02

            // Header
            header_title: Color::Rgb(203, 75, 22),      // Orange
            header_source: Color::Rgb(38, 139, 210),    // Blue
            header_bg: Color::Rgb(0, 43, 54),           // base03

            // Status bar
            status_mode_bg: Color::Rgb(42, 161, 152),   // Cyan
            status_mode_fg: Color::Rgb(0, 43, 54),      // base03
            status_help: Color::Rgb(88, 110, 117),      // base01
            status_bg: Color::Rgb(7, 54, 66),           // base02

            // Highlights
            highlight_match_bg: Color::Rgb(181, 137, 0), // Yellow
            highlight_match_fg: Color::Rgb(0, 43, 54),

            // Bookmarks
            bookmark: Color::Rgb(211, 54, 130),  // Magenta

            // Timestamps
            timestamp: Color::Rgb(88, 110, 117), // base01

            // JSON
            json: Color::Rgb(42, 161, 152),      // Cyan

            // Filters
            filter_selected: Color::Rgb(181, 137, 0),
            filter_prefix: Color::Rgb(42, 161, 152),

            // Sources
            source_current: Color::Rgb(133, 153, 0),

            // Empty states
            empty_state: Color::Rgb(88, 110, 117),
            warning_message: Color::Rgb(181, 137, 0),

            // Help
            help_border: Color::Rgb(42, 161, 152),
            help_bg: Color::Rgb(0, 43, 54),

            // Source colors - Solarized accent colors
            source_colors: vec![
                Color::Rgb(42, 161, 152),   // Cyan
                Color::Rgb(133, 153, 0),    // Green
                Color::Rgb(181, 137, 0),    // Yellow
                Color::Rgb(203, 75, 22),    // Orange
                Color::Rgb(211, 54, 130),   // Magenta
                Color::Rgb(108, 113, 196),  // Violet
            ],
        }
    }

    /// Get a theme by name
    pub fn by_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "kawaii" => Self::kawaii(),
            "cyber" | "futuristic" => Self::cyber(),
            "monochrome" | "mono" => Self::monochrome(),
            "dracula" => Self::dracula(),
            "matrix" | "hacker" => Self::matrix(),
            "nord" => Self::nord(),
            "gruvbox" => Self::gruvbox(),
            "catppuccin" | "mocha" => Self::catppuccin(),
            "tokyo_night" | "tokyo" | "tokyonight" => Self::tokyo_night(),
            "solarized" => Self::solarized(),
            _ => Self::default_theme(),
        }
    }

    /// Get the color for a source by index (cycles through available colors)
    pub fn get_source_color(&self, source_id: usize) -> Color {
        self.source_colors[source_id % self.source_colors.len()]
    }

    /// Get the theme name
    pub fn name(&self) -> &'static str {
        self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_by_name_default() {
        let theme = Theme::by_name("default");
        assert_eq!(theme.level_error, Color::Rgb(255, 85, 85));
    }

    #[test]
    fn test_by_name_kawaii() {
        let theme = Theme::by_name("kawaii");
        // Kawaii uses RGB colors, not standard
        assert!(matches!(theme.level_error, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_by_name_cyber() {
        let theme = Theme::by_name("cyber");
        assert!(matches!(theme.level_error, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_by_name_cyber_alias() {
        let theme = Theme::by_name("futuristic");
        // Should be same as cyber
        let cyber = Theme::by_name("cyber");
        assert_eq!(
            format!("{:?}", theme.level_error),
            format!("{:?}", cyber.level_error)
        );
    }

    #[test]
    fn test_by_name_dracula() {
        let theme = Theme::by_name("dracula");
        assert!(matches!(theme.level_error, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_by_name_monochrome() {
        let theme = Theme::by_name("monochrome");
        // Monochrome uses grayscale RGB
        assert!(matches!(theme.level_error, Color::Rgb(255, 255, 255)));
    }

    #[test]
    fn test_by_name_mono_alias() {
        let theme = Theme::by_name("mono");
        let monochrome = Theme::by_name("monochrome");
        assert_eq!(
            format!("{:?}", theme.level_error),
            format!("{:?}", monochrome.level_error)
        );
    }

    #[test]
    fn test_by_name_unknown_returns_default() {
        let theme = Theme::by_name("unknown_theme");
        let default = Theme::default_theme();
        assert_eq!(theme.level_error, default.level_error);
    }

    #[test]
    fn test_by_name_case_insensitive() {
        let lower = Theme::by_name("kawaii");
        let upper = Theme::by_name("KAWAII");
        let mixed = Theme::by_name("KaWaIi");
        assert_eq!(
            format!("{:?}", lower.level_error),
            format!("{:?}", upper.level_error)
        );
        assert_eq!(
            format!("{:?}", lower.level_error),
            format!("{:?}", mixed.level_error)
        );
    }

    #[test]
    fn test_default_trait() {
        let theme = Theme::default();
        let default = Theme::default_theme();
        assert_eq!(theme.level_error, default.level_error);
    }

    #[test]
    fn test_by_name_matrix() {
        let theme = Theme::by_name("matrix");
        assert_eq!(theme.name(), "matrix");
        // Matrix has green borders
        assert!(matches!(theme.border_focused, Color::Rgb(0, 255, 0)));
    }

    #[test]
    fn test_by_name_matrix_hacker_alias() {
        let theme = Theme::by_name("hacker");
        assert_eq!(theme.name(), "matrix");
    }

    #[test]
    fn test_by_name_nord() {
        let theme = Theme::by_name("nord");
        assert_eq!(theme.name(), "nord");
    }

    #[test]
    fn test_by_name_gruvbox() {
        let theme = Theme::by_name("gruvbox");
        assert_eq!(theme.name(), "gruvbox");
    }

    #[test]
    fn test_by_name_catppuccin() {
        let theme = Theme::by_name("catppuccin");
        assert_eq!(theme.name(), "catppuccin");
    }

    #[test]
    fn test_by_name_catppuccin_mocha_alias() {
        let theme = Theme::by_name("mocha");
        assert_eq!(theme.name(), "catppuccin");
    }

    #[test]
    fn test_by_name_tokyo_night() {
        let theme = Theme::by_name("tokyo_night");
        assert_eq!(theme.name(), "tokyo_night");
    }

    #[test]
    fn test_by_name_tokyo_night_aliases() {
        assert_eq!(Theme::by_name("tokyo").name(), "tokyo_night");
        assert_eq!(Theme::by_name("tokyonight").name(), "tokyo_night");
    }

    #[test]
    fn test_by_name_solarized() {
        let theme = Theme::by_name("solarized");
        assert_eq!(theme.name(), "solarized");
    }

    #[test]
    fn test_all_themes_have_source_colors() {
        let themes = ["default", "kawaii", "cyber", "dracula", "monochrome",
                     "matrix", "nord", "gruvbox", "catppuccin", "tokyo_night", "solarized"];
        for name in themes {
            let theme = Theme::by_name(name);
            assert!(!theme.source_colors.is_empty(), "{} has no source colors", name);
        }
    }
}
