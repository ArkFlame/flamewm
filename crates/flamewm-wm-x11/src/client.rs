use x11rb::protocol::xproto::Window;

use crate::chrome::IconImage;
use crate::classifier::WindowKind;
use crate::geometry::Rect;

#[derive(Debug, Clone)]
pub struct ManagedClient {
    pub client: Window,
    pub frame: Window,
    pub outer: Rect,
    pub restore: Rect,
    pub workspace: usize,
    pub title: String,
    pub title_text_width: i32,
    /// Cached `_NET_WM_ICON` selection (largest valid image), alpha preserved.
    pub icon: Option<IconImage>,
    /// Debug reason when the icon slot falls back to `WM_CLASS` identity.
    pub icon_fallback: Option<String>,
    pub transient_for: Option<Window>,
    pub minimized: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub sticky: bool,
    pub ignore_unmap: u8,
    pub kind: WindowKind,
    pub close_hover: bool,
}

impl ManagedClient {
    pub fn visible_on(&self, workspace: usize) -> bool {
        !self.minimized && (self.sticky || self.workspace == workspace)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResizeEdges {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

impl ResizeEdges {
    pub const fn any(self) -> bool {
        self.left || self.right || self.top || self.bottom
    }

    #[must_use]
    pub fn at(outer_width: u32, outer_height: u32, x: i16, y: i16) -> Self {
        let threshold = 6_i16;
        let width = outer_width.clamp(1, u32::from(u16::MAX)) as i16;
        let height = outer_height.clamp(1, u32::from(u16::MAX)) as i16;
        Self {
            left: x <= threshold,
            right: x >= width.saturating_sub(threshold),
            top: y <= threshold,
            bottom: y >= height.saturating_sub(threshold),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Drag {
    Move {
        client: Window,
        offset_x: i32,
        offset_y: i32,
        original: Rect,
    },
    Resize {
        client: Window,
        root_x: i32,
        root_y: i32,
        original: Rect,
        edges: ResizeEdges,
    },
}
