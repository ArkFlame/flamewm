use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WidgetsConfig {
    pub weather: ToggleWidgetConfig,
    pub current_playing: ToggleWidgetConfig,
    pub app_launcher: ToggleWidgetConfig,
    pub control_center: ToggleWidgetConfig,
    pub clock: ClockWidgetConfig,
}

impl Default for WidgetsConfig {
    fn default() -> Self {
        Self {
            weather: ToggleWidgetConfig::default(),
            current_playing: ToggleWidgetConfig::default(),
            app_launcher: ToggleWidgetConfig::default(),
            control_center: ToggleWidgetConfig::default(),
            clock: ClockWidgetConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToggleWidgetConfig {
    pub enabled: bool,
}

impl Default for ToggleWidgetConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockWidgetConfig {
    pub enabled: bool,
    /// `chrono::format::strftime` pattern used for the bar clock and the
    /// expanded clock panel.
    pub format: String,
}

impl Default for ClockWidgetConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            format: "%H:%M".to_owned(),
        }
    }
}
