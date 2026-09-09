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
    Overlay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceRole {
    Normal,
    Desktop,
    Dock,
    PopupMenu,
    DropdownMenu,
    Overlay,
}

impl From<SurfaceRole> for X11WindowRole {
    fn from(role: SurfaceRole) -> Self {
        match role {
            SurfaceRole::Normal => Self::Normal,
            SurfaceRole::Desktop => Self::Desktop,
            SurfaceRole::Dock => Self::Dock,
            SurfaceRole::PopupMenu => Self::PopupMenu,
            SurfaceRole::DropdownMenu => Self::DropdownMenu,
            SurfaceRole::Overlay => Self::Overlay,
        }
    }
}

/// Input participation of a surface. PassThrough surfaces never take
/// focus or pointer grabs and get an empty XShape input region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SurfaceInputMode {
    #[default]
    Interactive,
    PassThrough,
}

impl SurfaceInputMode {
    pub fn allows_grab(self) -> bool {
        matches!(self, Self::Interactive)
    }

    pub fn allows_focus(self) -> bool {
        matches!(self, Self::Interactive)
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub role: SurfaceRole,
    pub input: SurfaceInputMode,
    pub initially_visible: bool,
    pub x: i32,
    pub y: i32,
}

/// Pure policy: overlay surfaces are never WM-managed, never in the
/// workarea/taskbar, and default to pass-through input.
pub fn overlay_excluded_from_wm(role: SurfaceRole) -> bool {
    matches!(role, SurfaceRole::Overlay)
}

/// Shared popup policy: transient menus/overlays are override-redirect and
/// must map+raise atomically before any grab (XMapWindow+XRaiseWindow+flush
/// or XMapRaised). Ordinary Normal/Desktop/Dock surfaces map only.
pub fn surface_role_needs_popup_raise(role: SurfaceRole) -> bool {
    matches!(
        role,
        SurfaceRole::PopupMenu | SurfaceRole::DropdownMenu | SurfaceRole::Overlay
    )
}

/// X11-role twin of [`surface_role_needs_popup_raise`] for render-owned code.
pub fn x11_role_needs_popup_raise(role: X11WindowRole) -> bool {
    matches!(
        role,
        X11WindowRole::PopupMenu | X11WindowRole::DropdownMenu | X11WindowRole::Overlay
    )
}

pub fn overlay_window_type_name(role: SurfaceRole) -> &'static str {
    match role {
        SurfaceRole::Normal => "_NET_WM_WINDOW_TYPE_NORMAL",
        SurfaceRole::Desktop => "_NET_WM_WINDOW_TYPE_DESKTOP",
        SurfaceRole::Dock => "_NET_WM_WINDOW_TYPE_DOCK",
        SurfaceRole::PopupMenu => "_NET_WM_WINDOW_TYPE_POPUP_MENU",
        SurfaceRole::DropdownMenu => "_NET_WM_WINDOW_TYPE_DROPDOWN_MENU",
        SurfaceRole::Overlay => "_NET_WM_WINDOW_TYPE_NOTIFICATION",
    }
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            width: 1350,
            height: 641,
            title: "FlameWM Render 0.0.9".to_string(),
            role: SurfaceRole::Normal,
            input: SurfaceInputMode::Interactive,
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
