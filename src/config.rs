use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::theme::Theme;

/// Configuration for bark
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Maximum number of log lines to keep in the ring buffer
    pub max_lines: usize,
    /// Whether to enable log level coloring by default
    pub level_colors: bool,
    /// Whether to enable line wrapping by default
    pub line_wrap: bool,
    /// Whether to show the side panel by default
    pub show_side_panel: bool,
    /// Default export directory
    pub export_dir: String,
    /// Theme name: "default", "kawaii", "cyber", "dracula", "monochrome"
    pub theme: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_lines: 10_000,
            level_colors: true,
            line_wrap: false,
            show_side_panel: true,
            export_dir: "/tmp".to_string(),
            theme: "default".to_string(),
        }
    }
}

impl Config {
    /// Get the config file path
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("bark").join("config.toml"))
    }

    /// Load configuration from file, env vars, and defaults
    pub fn load() -> Self {
        let mut config = Self::default();

        // Try to load from config file
        if let Some(path) = Self::config_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(file_config) = toml::from_str::<Config>(&content) {
                        config = file_config;
                    }
                }
            }
        }

        // Override with environment variables
        if let Ok(val) = std::env::var("BARK_MAX_LINES") {
            if let Ok(max_lines) = val.parse() {
                config.max_lines = max_lines;
            }
        }
        if let Ok(val) = std::env::var("BARK_LEVEL_COLORS") {
            config.level_colors = val == "1" || val.to_lowercase() == "true";
        }
        if let Ok(val) = std::env::var("BARK_LINE_WRAP") {
            config.line_wrap = val == "1" || val.to_lowercase() == "true";
        }
        if let Ok(val) = std::env::var("BARK_SIDE_PANEL") {
            config.show_side_panel = val == "1" || val.to_lowercase() == "true";
        }
        if let Ok(val) = std::env::var("BARK_EXPORT_DIR") {
            config.export_dir = val;
        }
        if let Ok(val) = std::env::var("BARK_THEME") {
            config.theme = val;
        }

        config
    }

    /// Get the resolved theme based on config
    pub fn get_theme(&self) -> Theme {
        Theme::by_name(&self.theme)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path()
            .ok_or_else(|| "Could not determine config directory".to_string())?;

        // Create config directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Legacy function for compatibility
    pub fn from_env() -> Self {
        Self::load()
    }
}
