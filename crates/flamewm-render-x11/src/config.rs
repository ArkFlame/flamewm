use super::*;
#[derive(Clone, Debug)]
pub struct X11Config {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub title: String,
    pub role: X11WindowRole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X11WindowRole {
    Normal,
    Desktop,
    Dock,
    PopupMenu,
    DropdownMenu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceRole {
    Normal,
    Desktop,
    Dock,
    PopupMenu,
    DropdownMenu,
}

impl From<SurfaceRole> for X11WindowRole {
    fn from(role: SurfaceRole) -> Self {
        match role {
            SurfaceRole::Normal => Self::Normal,
            SurfaceRole::Desktop => Self::Desktop,
            SurfaceRole::Dock => Self::Dock,
            SurfaceRole::PopupMenu => Self::PopupMenu,
            SurfaceRole::DropdownMenu => Self::DropdownMenu,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub role: SurfaceRole,
    pub initially_visible: bool,
    pub x: i32,
    pub y: i32,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            width: 1350,
            height: 641,
            title: "FlameWM Render 0.0.9".to_string(),
            role: SurfaceRole::Normal,
            initially_visible: true,
            x: 0,
            y: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SurfaceId(pub(crate) u64);

impl SurfaceId {
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceControllerEvent {
    pub surface: SurfaceId,
    pub event: ActionEvent,
}

impl Default for X11Config {
    fn default() -> Self {
        Self {
            width: 1350,
            height: 641,
            x: 0,
            y: 0,
            title: "FlameWM Render 0.0.9".to_string(),
            role: X11WindowRole::Normal,
        }
    }
}

impl From<SurfaceConfig> for X11Config {
    fn from(config: SurfaceConfig) -> Self {
        Self {
            width: config.width,
            height: config.height,
            x: config.x,
            y: config.y,
            title: config.title,
            role: config.role.into(),
        }
    }
}
