mod bar;
mod tray;
mod widgets;

pub use bar::{BarConfig, BarLayout, BarPosition};
pub use tray::TrayConfig;
pub use widgets::WidgetsConfig;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellConfig {
    pub bar: BarConfig,
    pub tray: TrayConfig,
    pub widgets: WidgetsConfig,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            bar: BarConfig::default(),
            tray: TrayConfig::default(),
            widgets: WidgetsConfig::default(),
        }
    }
}

/// The commented template written to disk the first time CreamShell runs,
/// so the file is comfortable to open and edit by hand right away. Keep this
/// in sync with the `Default` impls above when a field's default changes.
const DEFAULT_SHELL_TOML: &str = r#"# CreamShell configuration.
# Edit this file and restart CreamShell (or, in the future, use the settings
# panel) to apply changes. Missing keys fall back to their defaults, so you
# only need to list what you want to change.

[bar]
# Where the bar sits on screen: "top" or "bottom".
position = "bottom"

[bar.layout]
# Widget ids shown in each section of the bar, left to right. Known ids:
# "logo", "weather", "current_playing", "app_launcher", "control_center",
# "clock". Remove an id to hide it, or move it to another section.
left = ["logo", "weather", "current_playing"]
center = ["app_launcher"]
right = ["control_center", "clock"]

[tray]
# Visibility of each status icon: "always" (shown in the bar), "hidden"
# (not in the bar, but still available when the control center is opened),
# or "off" (never shown).
wifi = "always"
bluetooth = "always"
battery = "always"
volume = "always"
brightness = "always"

[widgets.weather]
enabled = true

[widgets.current_playing]
enabled = true

[widgets.app_launcher]
enabled = true

[widgets.control_center]
enabled = true

[widgets.clock]
enabled = true
# strftime pattern for the bar clock, e.g. "%I:%M %p" for a 12-hour clock.
format = "%H:%M"
"#;

impl ShellConfig {
    /// Loads the shell configuration from disk, creating the default file on
    /// first run and falling back to in-memory defaults if the file cannot
    /// be read or contains invalid TOML (the file itself is left untouched
    /// in that case, so no user edits are lost).
    pub fn load() -> Self {
        let path = config_file_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => config,
                Err(error) => {
                    eprintln!(
                        "config: failed to parse {}: {error}; using defaults",
                        path.display()
                    );
                    ShellConfig::default()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                write_default_config(&path);
                ShellConfig::default()
            }
            Err(error) => {
                eprintln!(
                    "config: failed to read {}: {error}; using defaults",
                    path.display()
                );
                ShellConfig::default()
            }
        }
    }
}

fn write_default_config(path: &std::path::Path) {
    let Some(parent) = path.parent() else {
        return;
    };
    if let Err(error) = std::fs::create_dir_all(parent) {
        eprintln!("config: failed to create {}: {error}", parent.display());
        return;
    }
    if let Err(error) = std::fs::write(path, DEFAULT_SHELL_TOML) {
        eprintln!(
            "config: failed to write default config to {}: {error}",
            path.display()
        );
    }
}

/// Resolves `<config dir>/cream/shell.toml`: `$XDG_CONFIG_HOME` or
/// `~/.config` on Linux/macOS, `%APPDATA%` on Windows.
fn config_file_path() -> PathBuf {
    config_dir().join("cream").join("shell.toml")
}

#[cfg(target_os = "windows")]
fn config_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(not(target_os = "windows"))]
fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_template_matches_in_memory_defaults() {
        let parsed: ShellConfig = toml::from_str(DEFAULT_SHELL_TOML).expect("valid TOML");
        assert_eq!(parsed, ShellConfig::default());
    }

    #[test]
    fn missing_keys_fall_back_to_defaults() {
        let parsed: ShellConfig = toml::from_str("[bar]\nposition = \"top\"\n").unwrap();
        assert_eq!(parsed.bar.position, BarPosition::Top);
        assert_eq!(parsed.bar.layout, BarLayout::default());
        assert_eq!(parsed.tray, TrayConfig::default());
        assert_eq!(parsed.widgets, WidgetsConfig::default());
    }

    #[test]
    fn invalid_toml_is_rejected_rather_than_silently_accepted() {
        let result: Result<ShellConfig, _> = toml::from_str("bar = \"not a table\"");
        assert!(result.is_err());
    }
}
