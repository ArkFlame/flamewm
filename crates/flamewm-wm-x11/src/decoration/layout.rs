//! Decoration layout: title measurement anchors, paint-plan derivation,
//! and resize geometry. Pure helpers, no display access.
//!
//! Title widths anchor on the canonical Xft-compatible measure
//! (`external_text_measure`, IBM Plex Sans 12px semibold): layout uses the
//! measured width, never a fixed advance.
//!
//! Geometry anchors on skin named metrics (`WINDOW_CHROME.metrics`):
//! 38x31 control buttons tiled from the right frame edge, the 16px control
//! glyph centered in its button, the 20px app-icon slot at the left pad.

use flamewm_render_x11::external_text_measure;
use flamewm_skin::recipes::window_chrome::{self as window_recipe, ICON_PAD_LEFT, WINDOW_CHROME};

use crate::chrome::{self, ControlRole};
use crate::geometry::Rect;

pub use crate::client::ResizeEdges;

use super::model::{FramePaintPlan, FrameSnapshot};

/// Canonical title measure width (device px, rounded) for layout anchoring.
#[must_use]
pub fn measured_title_width(title: &str, size: f32) -> i32 {
    let _guard = flamewm_profiler::start("wm.decoration.measure");
    external_text_measure(title, size).0.round() as i32
}

/// Derive a [`FramePaintPlan`] from a snapshot and its resolved glyph roles.
/// Title centering uses the measured width (`snapshot.title_text_width`);
/// callers must fill it via [`measured_title_width`] first. Right-edge
/// buttons tile skin 38x31 slots from the frame edge; the 16px glyph is
/// centered inside its 38px slot for blit placement.
#[must_use]
pub fn paint_plan_for(snapshot: &FrameSnapshot, glyph_roles: Vec<ControlRole>) -> FramePaintPlan {
    let _guard = flamewm_profiler::start("wm.decoration.layout");
    let titlebar = snapshot.titlebar_height;
    let slot = chrome::icon_slot_for(titlebar);
    let titlebar_rect = window_recipe::SceneRect::new(
        0,
        0,
        i32::try_from(snapshot.outer.width).unwrap_or(i32::MAX),
        i32::from(titlebar),
    );
    let geometries = window_recipe::control_button_geometries(titlebar_rect);
    let right_occupied = geometries[0].bounds.x;
    let left_occupied = ICON_PAD_LEFT + i32::from(WINDOW_CHROME.metrics.app_icon_visual_edge);
    let paint_x = chrome::center_title_x(
        i32::try_from(snapshot.outer.width).unwrap_or(i32::MAX),
        snapshot.title_text_width,
        left_occupied,
        right_occupied,
        window_recipe::TITLE_PAD,
    );
    FramePaintPlan {
        slot,
        paint_x,
        baseline: chrome::title_baseline(titlebar),
        glyph_roles,
        close_hover_bg: snapshot.close_hover,
        radius: u32::from(chrome::effective_radius(
            snapshot.maximized,
            snapshot.fullscreen,
        )),
        has_native_icon: snapshot.has_native_icon,
        has_catalog_icon: snapshot.has_catalog_icon,
    }
}

/// Titlebar border hit-test edges for resize drags live in
/// [`crate::client::ResizeEdges`]; layout geometry and drag state share that
/// one type.

#[must_use]
pub fn resized_rect(original: Rect, edges: ResizeEdges, dx: i32, dy: i32) -> Rect {
    const MIN_WIDTH: i32 = 160;
    const MIN_HEIGHT: i32 = 96;
    let mut left = original.x;
    let mut top = original.y;
    let mut right = original.right();
    let mut bottom = original.bottom();
    if edges.left {
        left = left.saturating_add(dx).min(right.saturating_sub(MIN_WIDTH));
    }
    if edges.right {
        right = right.saturating_add(dx).max(left.saturating_add(MIN_WIDTH));
    }
    if edges.top {
        top = top
            .saturating_add(dy)
            .min(bottom.saturating_sub(MIN_HEIGHT));
    }
    if edges.bottom {
        bottom = bottom
            .saturating_add(dy)
            .max(top.saturating_add(MIN_HEIGHT));
    }
    Rect::new(
        left,
        top,
        u32::try_from(right.saturating_sub(left)).unwrap_or(MIN_WIDTH as u32),
        u32::try_from(bottom.saturating_sub(top)).unwrap_or(MIN_HEIGHT as u32),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoration::paint::TITLE_FONT_SIZE;

    #[test]
    fn t25_layout_uses_measured_title_width() {
        let width = measured_title_width("hello", TITLE_FONT_SIZE);
        let (expected, _) = external_text_measure("hello", TITLE_FONT_SIZE);
        assert_eq!(width, expected.round() as i32);
        let snapshot = FrameSnapshot {
            frame: 1,
            outer: Rect::new(0, 0, 400, 300),
            title: "hello".to_owned(),
            title_text_width: width,
            has_native_icon: false,
            has_catalog_icon: false,
            hover: None,
            active: true,
            maximized: false,
            fullscreen: false,
            close_hover: false,
            titlebar_height: 31,
        };
        let plan = paint_plan_for(&snapshot, Vec::new());
        assert_eq!(
            plan.paint_x,
            super::chrome::center_title_x(400, width, 26, 286, 6)
        );
    }
}
