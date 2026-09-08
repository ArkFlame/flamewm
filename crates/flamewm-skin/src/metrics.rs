//! Centralized exact dimensions promoted from RWR 0.0.9 CSS.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopMetrics {
    pub width: u16,
    pub height: u16,
    pub icon: u16,
    pub label_width: u16,
    pub label_height: u16,
    pub gap: u16,
    pub radius: u16,
    pub padding_x: u16,
    pub padding_y: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowChromeMetrics {
    pub radius: u16,
    pub titlebar: u16,
    pub title_side: u16,
    pub controls: u16,
    pub button_width: u16,
    pub button_height: u16,
    pub icon: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsMetrics {
    pub width: u16,
    pub height: u16,
    pub nav_width: u16,
    pub toggle_width: u16,
    pub toggle_height: u16,
    pub knob: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartMetrics {
    pub width: u16,
    pub min_height: u16,
    pub app_row: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskbarMetrics {
    pub height: u16,
    pub pager_width: u16,
    pub pager_height: u16,
    pub button_width: u16,
    pub workspace_button_width: u16,
    pub workspace_button_height: u16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickyMetrics {
    pub width: u16,
    pub height: u16,
    pub padding: u16,
    pub gap: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinMetrics {
    pub desktop: DesktopMetrics,
    pub chrome: WindowChromeMetrics,
    pub settings: SettingsMetrics,
    pub start: StartMetrics,
    pub taskbar: TaskbarMetrics,
    pub sticky: StickyMetrics,
}

impl SkinMetrics {
    pub const RWR_0_0_9: Self = Self {
        desktop: DesktopMetrics {
            width: 82,
            height: 82,
            icon: 44,
            label_width: 76,
            label_height: 15,
            gap: 4,
            radius: 4,
            padding_x: 3,
            padding_y: 4,
        },
        chrome: WindowChromeMetrics {
            radius: 6,
            titlebar: 31,
            title_side: 114,
            controls: 114,
            button_width: 38,
            button_height: 31,
            icon: 13,
        },
        settings: SettingsMetrics {
            width: 720,
            height: 480,
            nav_width: 192,
            toggle_width: 40,
            toggle_height: 22,
            knob: 18,
        },
        start: StartMetrics {
            width: 292,
            min_height: 349,
            app_row: 36,
        },
        taskbar: TaskbarMetrics {
            height: 44,
            pager_width: 51,
            pager_height: 44,
            button_width: 42,
            workspace_button_width: 22,
            workspace_button_height: 16,
        },
        sticky: StickyMetrics {
            width: 190,
            height: 190,
            padding: 12,
            gap: 8,
        },
    };
}

impl Default for SkinMetrics {
    fn default() -> Self {
        Self::RWR_0_0_9
    }
}
