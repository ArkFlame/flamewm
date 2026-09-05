use std::collections::BTreeMap;

use flamewm_api::window::{WindowSnapshot, WindowState};
use flamewm_api::{ErrorCode, FlameError, FlameResult, OutputId, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChromeMetrics {
    pub titlebar_height: i32,
    pub border_x: i32,
    pub border_y: i32,
    pub corner_size_x: i32,
    pub corner_size_y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowChrome {
    pub decorated: bool,
    pub titlebar: bool,
    pub border: bool,
    pub minimize_button: bool,
    pub maximize_button: bool,
    pub close_button: bool,
    pub resizable: bool,
    pub rollover_buttons: bool,
}

impl Default for WindowChrome {
    fn default() -> Self {
        Self {
            decorated: true,
            titlebar: true,
            border: true,
            minimize_button: true,
            maximize_button: true,
            close_button: true,
            resizable: true,
            rollover_buttons: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromePolicy {
    revision: u64,
    scales: BTreeMap<OutputId, u16>,
    hide_title_when_maximized: bool,
    hide_decor_when_maximized: bool,
    accent: String,
}

impl Default for ChromePolicy {
    fn default() -> Self {
        Self {
            revision: 1,
            scales: BTreeMap::new(),
            hide_title_when_maximized: false,
            hide_decor_when_maximized: false,
            accent: "#EF4048".to_owned(),
        }
    }
}

impl ChromePolicy {
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn accent(&self) -> &str {
        &self.accent
    }

    pub fn set_accent(&mut self, value: &str) -> FlameResult<()> {
        if !valid_hex_color(value) {
            return Err(FlameError::new(
                ErrorCode::InvalidArgument,
                "accent must be #RRGGBB",
            ));
        }
        if self.accent != value {
            self.accent = value.to_owned();
            self.revision = self.revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn set_scale(&mut self, output: OutputId, percent: u16) -> FlameResult<()> {
        if !matches!(percent, 100 | 125 | 150 | 175 | 200) || !output.is_valid() {
            return Err(FlameError::new(
                ErrorCode::InvalidArgument,
                "invalid output scale",
            ));
        }
        if self.scales.get(&output).copied() != Some(percent) {
            self.scales.insert(output, percent);
            self.revision = self.revision.saturating_add(1);
        }
        Ok(())
    }

    #[must_use]
    pub fn scale_for(&self, output: &OutputId) -> u16 {
        self.scales.get(output).copied().unwrap_or(100)
    }

    #[must_use]
    pub fn metrics_for(&self, output: &OutputId) -> ChromeMetrics {
        let scale = i32::from(self.scale_for(output));
        ChromeMetrics {
            titlebar_height: (32 * scale + 50) / 100,
            border_x: ((scale + 50) / 100).max(1),
            border_y: ((scale + 50) / 100).max(1),
            corner_size_x: (8 * scale + 50) / 100,
            corner_size_y: (8 * scale + 50) / 100,
        }
    }

    #[must_use]
    pub fn chrome_for(&self, window: &WindowSnapshot) -> WindowChrome {
        if matches!(window.state, WindowState::Fullscreen) {
            return WindowChrome {
                decorated: false,
                titlebar: false,
                border: false,
                minimize_button: false,
                maximize_button: false,
                close_button: false,
                resizable: false,
                rollover_buttons: true,
            };
        }
        let mut chrome = WindowChrome::default();
        if matches!(window.state, WindowState::Maximized) {
            if self.hide_title_when_maximized || self.hide_decor_when_maximized {
                chrome.titlebar = false;
            }
            if self.hide_decor_when_maximized {
                chrome.border = false;
            }
            chrome.decorated = chrome.titlebar || chrome.border;
        }
        chrome
    }

    #[must_use]
    pub fn centered_title_rect(
        titlebar: Rect,
        title_width: i32,
        buttons_left_width: i32,
        buttons_right_width: i32,
    ) -> Rect {
        if titlebar.width <= 0 || titlebar.height <= 0 {
            return Rect::new(titlebar.x, titlebar.y, 0, titlebar.height.max(0));
        }
        let left = titlebar.x + buttons_left_width.max(0);
        let right = titlebar.right() - buttons_right_width.max(0);
        let available = right - left;
        if available <= 0 || title_width <= 0 {
            return Rect::new(left, titlebar.y, 0, titlebar.height);
        }
        let width = title_width.min(available);
        let ideal = titlebar.x + (titlebar.width - width) / 2;
        let x = ideal.clamp(left, right - width);
        Rect::new(x, titlebar.y, width, titlebar.height)
    }
}

fn valid_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_is_frame_centered_then_collision_clamped() {
        let titlebar = Rect::new(100, 20, 500, 32);
        assert_eq!(
            ChromePolicy::centered_title_rect(titlebar, 100, 30, 126),
            Rect::new(300, 20, 100, 32)
        );
        assert_eq!(
            ChromePolicy::centered_title_rect(titlebar, 420, 30, 126),
            Rect::new(130, 20, 344, 32)
        );
    }

    #[test]
    fn default_corner_metrics_match_current_native_theme() {
        let metrics = ChromePolicy::default().metrics_for(&OutputId::new("eDP-1"));
        assert_eq!((metrics.corner_size_x, metrics.corner_size_y), (8, 8));
    }
}
