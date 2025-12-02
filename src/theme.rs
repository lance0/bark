use ratatui::style::Color;

/// All themeable colors in the application
#[derive(Clone, Debug)]
pub struct Theme {
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
    pub source_current: Color,

    // Empty states / messages
    pub empty_state: Color,
    pub warning_message: Color,

    // Help overlay
    pub help_border: Color,
    pub help_bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_theme()
    }
}

impl Theme {
    /// The default theme (matches original hardcoded colors)
    pub fn default_theme() -> Self {
        Self {
            // Log levels
            level_error: Color::Red,
            level_warn: Color::Yellow,
            level_info: Color::Green,
            level_debug: Color::Blue,
            level_trace: Color::DarkGray,

            // UI borders
            border_focused: Color::Cyan,
            border_unfocused: Color::DarkGray,

            // Header
            header_title: Color::Green,
            header_source: Color::Cyan,
            header_bg: Color::DarkGray,

            // Status bar
            status_mode_bg: Color::Blue,
            status_mode_fg: Color::White,
            status_help: Color::DarkGray,
            status_bg: Color::Black,

            // Highlights
            highlight_match_bg: Color::Yellow,
            highlight_match_fg: Color::Black,

            // Bookmarks
            bookmark: Color::Magenta,

            // Timestamps
            timestamp: Color::DarkGray,

            // JSON
            json: Color::Cyan,

            // Filters
            filter_selected: Color::Yellow,
            filter_prefix: Color::Yellow,

            // Sources
            source_current: Color::Green,

            // Empty states
            empty_state: Color::DarkGray,
            warning_message: Color::Yellow,

            // Help
            help_border: Color::Cyan,
            help_bg: Color::Black,
        }
    }

    /// Kawaii theme - cute pastel colors
    pub fn kawaii() -> Self {
        Self {
            // Log levels - soft pastels
            level_error: Color::Rgb(255, 121, 162),   // Soft pink
            level_warn: Color::Rgb(255, 200, 152),    // Soft peach
            level_info: Color::Rgb(152, 255, 200),    // Soft mint
            level_debug: Color::Rgb(162, 200, 255),   // Soft lavender-blue
            level_trace: Color::Rgb(180, 180, 200),   // Soft gray-lavender

            // UI borders
            border_focused: Color::Rgb(255, 182, 214),  // Pink
            border_unfocused: Color::Rgb(180, 180, 200),

            // Header
            header_title: Color::Rgb(255, 182, 214),   // Pink
            header_source: Color::Rgb(182, 214, 255),  // Light blue
            header_bg: Color::Rgb(60, 50, 70),         // Dark purple-gray

            // Status bar
            status_mode_bg: Color::Rgb(214, 182, 255), // Light purple
            status_mode_fg: Color::Rgb(40, 30, 50),    // Dark
            status_help: Color::Rgb(180, 180, 200),
            status_bg: Color::Rgb(40, 30, 50),

            // Highlights
            highlight_match_bg: Color::Rgb(255, 214, 182), // Soft orange
            highlight_match_fg: Color::Rgb(40, 30, 50),

            // Bookmarks
            bookmark: Color::Rgb(255, 150, 200),       // Hot pink

            // Timestamps
            timestamp: Color::Rgb(180, 180, 200),

            // JSON
            json: Color::Rgb(182, 255, 214),           // Mint

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
        }
    }

    /// Cyber/Futuristic theme - neon on dark
    pub fn cyber() -> Self {
        Self {
            // Log levels - neon colors
            level_error: Color::Rgb(255, 50, 100),    // Neon red-pink
            level_warn: Color::Rgb(255, 200, 0),      // Neon yellow
            level_info: Color::Rgb(0, 255, 150),      // Neon green
            level_debug: Color::Rgb(0, 200, 255),     // Electric blue
            level_trace: Color::Rgb(100, 100, 120),   // Muted gray-blue

            // UI borders
            border_focused: Color::Rgb(0, 255, 255),  // Cyan neon
            border_unfocused: Color::Rgb(60, 60, 80),

            // Header
            header_title: Color::Rgb(255, 0, 255),    // Magenta neon
            header_source: Color::Rgb(0, 255, 255),   // Cyan neon
            header_bg: Color::Rgb(20, 20, 35),        // Very dark blue

            // Status bar
            status_mode_bg: Color::Rgb(255, 0, 255),  // Magenta
            status_mode_fg: Color::Rgb(0, 0, 0),      // Black
            status_help: Color::Rgb(100, 100, 120),
            status_bg: Color::Rgb(10, 10, 20),

            // Highlights
            highlight_match_bg: Color::Rgb(0, 255, 255), // Cyan
            highlight_match_fg: Color::Rgb(0, 0, 0),

            // Bookmarks
            bookmark: Color::Rgb(255, 100, 255),      // Pink-magenta

            // Timestamps
            timestamp: Color::Rgb(100, 100, 120),

            // JSON
            json: Color::Rgb(0, 200, 255),            // Electric blue

            // Filters
            filter_selected: Color::Rgb(255, 200, 0), // Yellow neon
            filter_prefix: Color::Rgb(0, 255, 255),   // Cyan

            // Sources
            source_current: Color::Rgb(0, 255, 150),  // Neon green

            // Empty states
            empty_state: Color::Rgb(100, 100, 120),
            warning_message: Color::Rgb(255, 200, 0),

            // Help
            help_border: Color::Rgb(0, 255, 255),
            help_bg: Color::Rgb(10, 10, 20),
        }
    }

    /// Dracula theme - popular dark theme
    pub fn dracula() -> Self {
        Self {
            // Log levels - Dracula palette
            level_error: Color::Rgb(255, 85, 85),     // Red
            level_warn: Color::Rgb(255, 184, 108),    // Orange
            level_info: Color::Rgb(80, 250, 123),     // Green
            level_debug: Color::Rgb(139, 233, 253),   // Cyan
            level_trace: Color::Rgb(98, 114, 164),    // Comment gray

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
            bookmark: Color::Rgb(255, 121, 198),      // Pink

            // Timestamps
            timestamp: Color::Rgb(98, 114, 164),      // Comment

            // JSON
            json: Color::Rgb(139, 233, 253),          // Cyan

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
        }
    }

    /// Monochrome theme - grayscale only
    pub fn monochrome() -> Self {
        Self {
            // Log levels - varying grays
            level_error: Color::Rgb(255, 255, 255),   // White (stands out)
            level_warn: Color::Rgb(200, 200, 200),    // Light gray
            level_info: Color::Rgb(170, 170, 170),    // Medium-light
            level_debug: Color::Rgb(140, 140, 140),   // Medium
            level_trace: Color::Rgb(100, 100, 100),   // Dark

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
        }
    }

    /// Get a theme by name
    pub fn by_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "kawaii" => Self::kawaii(),
            "cyber" | "futuristic" => Self::cyber(),
            "monochrome" | "mono" => Self::monochrome(),
            "dracula" => Self::dracula(),
            _ => Self::default_theme(),
        }
    }
}
