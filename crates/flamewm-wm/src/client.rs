use x11rb::protocol::xproto::Window;

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
    pub class: String,
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
