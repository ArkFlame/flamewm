use serde::{Deserialize, Serialize};

/// How a tray/status icon is exposed to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrayVisibility {
    /// Always shown in the collapsed bar strip.
    Always,
    /// Not shown in the bar, but still available in the control center panel.
    Hidden,
    /// Never shown anywhere.
    Off,
}

impl Default for TrayVisibility {
    fn default() -> Self {
        TrayVisibility::Always
    }
}

impl TrayVisibility {
    pub fn shows_in_bar(self) -> bool {
        matches!(self, TrayVisibility::Always)
    }

    pub fn shows_in_panel(self) -> bool {
        !matches!(self, TrayVisibility::Off)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrayConfig {
    pub wifi: TrayVisibility,
    pub bluetooth: TrayVisibility,
    pub battery: TrayVisibility,
    pub volume: TrayVisibility,
    pub brightness: TrayVisibility,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            wifi: TrayVisibility::default(),
            bluetooth: TrayVisibility::default(),
            battery: TrayVisibility::default(),
            volume: TrayVisibility::default(),
            brightness: TrayVisibility::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TrayVisibility;

    #[test]
    fn always_shows_everywhere() {
        assert!(TrayVisibility::Always.shows_in_bar());
        assert!(TrayVisibility::Always.shows_in_panel());
    }

    #[test]
    fn hidden_skips_the_bar_but_keeps_the_panel() {
        assert!(!TrayVisibility::Hidden.shows_in_bar());
        assert!(TrayVisibility::Hidden.shows_in_panel());
    }

    #[test]
    fn off_skips_everywhere() {
        assert!(!TrayVisibility::Off.shows_in_bar());
        assert!(!TrayVisibility::Off.shows_in_panel());
    }
}
