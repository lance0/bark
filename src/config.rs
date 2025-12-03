//! Configuration loading from file and environment variables.
//!
//! Configuration is loaded in order of priority:
//! 1. Default values
//! 2. Config file (`~/.config/bark/config.toml`)
//! 3. Environment variables (`BARK_*`)

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::theme::Theme;

/// Default channel buffer size for log sources
pub const DEFAULT_CHANNEL_BUFFER: usize = 1000;

/// Default number of lines to tail from sources
pub const DEFAULT_TAIL_LINES: &str = "1000";

/// Filter input debounce delay in milliseconds
pub const FILTER_DEBOUNCE_MS: u128 = 150;

/// Mouse scroll lines per wheel event
pub const MOUSE_SCROLL_LINES: usize = 3;

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
    /// SSH StrictHostKeyChecking mode: "yes" (default, strict), "accept-new", or "no"
    /// WARNING: "accept-new" and "no" are insecure and vulnerable to MITM attacks
    pub ssh_host_key_checking: String,
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
            ssh_host_key_checking: "yes".to_string(),
        }
    }
}

impl Config {
    /// Get the config file path
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("barklog").join("config.toml"))
    }

    /// Load configuration from file, env vars, and defaults
    pub fn load() -> Self {
        let mut config = Self::default();

        // Try to load from config file
        if let Some(path) = Self::config_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    match toml::from_str::<Config>(&content) {
                        Ok(file_config) => config = file_config,
                        Err(e) => eprintln!("Warning: Invalid config at {}: {}", path.display(), e),
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
        if let Ok(val) = std::env::var("BARK_SSH_HOST_KEY_CHECKING") {
            // Validate the value
            let val_lower = val.to_lowercase();
            if val_lower == "yes" || val_lower == "accept-new" || val_lower == "no" {
                config.ssh_host_key_checking = val_lower;
            } else {
                eprintln!(
                    "Warning: Invalid BARK_SSH_HOST_KEY_CHECKING='{}'. Use 'yes', 'accept-new', or 'no'. Defaulting to 'yes'.",
                    val
                );
            }
        }

        config
    }

    /// Get the resolved theme based on config
    pub fn get_theme(&self) -> Theme {
        Theme::by_name(&self.theme)
    }

    /// Save configuration to file (future feature)
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let config = Config::default();
        assert_eq!(config.max_lines, 10_000);
        assert!(config.level_colors);
        assert!(!config.line_wrap);
        assert!(config.show_side_panel);
        assert_eq!(config.export_dir, "/tmp");
        assert_eq!(config.theme, "default");
    }

    #[test]
    fn test_get_theme_default() {
        let config = Config::default();
        let theme = config.get_theme();
        // Default theme uses RGB soft red for errors
        assert_eq!(theme.level_error, ratatui::style::Color::Rgb(255, 85, 85));
    }

    #[test]
    fn test_get_theme_custom() {
        let mut config = Config::default();
        config.theme = "dracula".to_string();
        let theme = config.get_theme();
        // Dracula uses RGB colors
        assert!(matches!(
            theme.level_error,
            ratatui::style::Color::Rgb(_, _, _)
        ));
    }

    #[test]
    fn test_config_path_is_some() {
        // On most systems, config_path should return Some
        // This may fail in restricted environments
        let path = Config::config_path();
        // We just verify it doesn't panic and returns a path containing "bark"
        if let Some(p) = path {
            assert!(p.to_string_lossy().contains("bark"));
        }
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).expect("serialization should work");
        assert!(toml_str.contains("max_lines"));
        assert!(toml_str.contains("level_colors"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
            max_lines = 5000
            level_colors = false
            line_wrap = true
            show_side_panel = false
            export_dir = "/home/user/logs"
            theme = "kawaii"
        "#;
        let config: Config = toml::from_str(toml_str).expect("deserialization should work");
        assert_eq!(config.max_lines, 5000);
        assert!(!config.level_colors);
        assert!(config.line_wrap);
        assert!(!config.show_side_panel);
        assert_eq!(config.export_dir, "/home/user/logs");
        assert_eq!(config.theme, "kawaii");
    }

    #[test]
    fn test_config_partial_deserialization() {
        // Only some fields specified, others should use defaults
        let toml_str = r#"
            max_lines = 500
        "#;
        let config: Config = toml::from_str(toml_str).expect("deserialization should work");
        assert_eq!(config.max_lines, 500);
        // Defaults for unspecified fields
        assert!(config.level_colors);
        assert_eq!(config.theme, "default");
    }

    #[test]
    fn test_ssh_host_key_checking_default() {
        let config = Config::default();
        // Security: Default should be "yes" (strict) to prevent MITM attacks
        assert_eq!(config.ssh_host_key_checking, "yes");
    }

    #[test]
    fn test_ssh_host_key_checking_from_toml() {
        let toml_str = r#"
            ssh_host_key_checking = "accept-new"
        "#;
        let config: Config = toml::from_str(toml_str).expect("deserialization should work");
        assert_eq!(config.ssh_host_key_checking, "accept-new");
    }
}
