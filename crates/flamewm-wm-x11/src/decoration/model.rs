//! Decoration model: WM-owned frame state snapshots and paint plans.
//!
//! Pure data, no display access. The live paint executor in [`super::paint`]
//! consumes [`FrameSnapshot`] plus [`FramePaintPlan`] through safe `x11rb`
//! calls; headless tests pin the plan derivation without a display.

use crate::chrome::ControlRole;
use crate::geometry::Rect;

/// Point-in-time frame inputs for one paint pass.
#[derive(Debug, Clone)]
pub struct FrameSnapshot {
    pub frame: u32,
    pub outer: Rect,
    pub title: String,
    pub title_text_width: i32,
    pub has_native_icon: bool,
    pub has_catalog_icon: bool,
    pub hover: Option<ControlRole>,
    pub active: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub close_hover: bool,
    pub titlebar_height: u16,
}

/// Headless-testable paint plan for one frame: background fill, app-icon
/// blit, title draw at `paint_x`/`baseline`, control blits for
/// `glyph_roles` (close-hover background when `close_hover_bg`), rounded
/// shape of `radius`, flush.
#[derive(Debug, Clone, PartialEq)]
pub struct FramePaintPlan {
    pub slot: u16,
    pub paint_x: i32,
    pub baseline: f32,
    pub glyph_roles: Vec<ControlRole>,
    pub close_hover_bg: bool,
    pub radius: u32,
    pub has_native_icon: bool,
    pub has_catalog_icon: bool,
}
