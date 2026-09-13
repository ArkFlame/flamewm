use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BarPosition {
    Top,
    Bottom,
}

impl Default for BarPosition {
    fn default() -> Self {
        BarPosition::Bottom
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BarConfig {
    pub position: BarPosition,
    pub layout: BarLayout,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            position: BarPosition::default(),
            layout: BarLayout::default(),
        }
    }
}

/// Widget ids placed in each section of the bar, left to right. Unknown ids
/// are ignored (with a warning); a widget missing from every section simply
/// does not appear in the bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BarLayout {
    pub left: Vec<String>,
    pub center: Vec<String>,
    pub right: Vec<String>,
}

impl Default for BarLayout {
    fn default() -> Self {
        Self {
            left: vec!["logo".into(), "weather".into(), "current_playing".into()],
            center: vec!["app_launcher".into()],
            right: vec!["control_center".into(), "clock".into()],
        }
    }
}
