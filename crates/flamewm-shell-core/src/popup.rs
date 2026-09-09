//! Shell popup geometry. Pure rect math; no surface or runtime state here.

use flamewm_api::display::OutputSnapshot;
use flamewm_api::panels::PanelSnapshot;
use flamewm_api::{OutputId, PanelEdge, Point, Rect, Size};
use flamewm_ui_core::popover::{PopoverEdge, PopoverGeometry};
use flamewm_ui_core::style::ShellMetrics;
use flamewm_ui_core::{PopoverDirection, anchor_popover};

#[must_use]
pub fn anchor_popover_rect(anchor: Rect, size: Size) -> Rect {
    Rect::new(anchor.x, anchor.y, size.width, size.height)
}

#[must_use]
pub fn status_anchor(
    panel: &PanelSnapshot,
    output: &OutputSnapshot,
    metrics: ShellMetrics,
    slot: i32,
) -> Rect {
    panel_popup_rect(panel, output, metrics, Size::new(1, 1), slot)
}

#[must_use]
pub fn panel_anchor(
    panel: &PanelSnapshot,
    output: &OutputSnapshot,
    metrics: ShellMetrics,
    slot: i32,
) -> Rect {
    status_anchor(panel, output, metrics, slot)
}

#[must_use]
pub fn panel_popup_rect(
    panel: &PanelSnapshot,
    output: &OutputSnapshot,
    metrics: ShellMetrics,
    size: Size,
    slot: i32,
) -> Rect {
    let panel_rect = panel.geometry;
    let slot_width = i32::from(metrics.tray_button_width);
    let clock_width = i32::from(metrics.clock_min_width);
    let anchor = if panel.edge.is_horizontal() {
        let right = panel_rect.right();
        let x = right - clock_width - slot_width * (3 - slot.min(3));
        Rect::new(x, panel_rect.y, slot_width, panel_rect.height)
    } else {
        let bottom = panel_rect.bottom();
        let y = bottom - clock_width - slot_width * (3 - slot.min(3));
        Rect::new(panel_rect.x, y, panel_rect.width, slot_width)
    };
    let direction = match panel.edge {
        PanelEdge::Bottom => PopoverDirection::Above,
        PanelEdge::Top => PopoverDirection::Below,
        PanelEdge::Left => PopoverDirection::RightOf,
        PanelEdge::Right => PopoverDirection::LeftOf,
    };
    anchor_popover(
        panel.output.clone(),
        output.geometry,
        anchor,
        (size.width, size.height),
        direction,
        i32::from(metrics.popover_offset),
    )
    .rect
}

/// Native context-menu width matches `.context-menu` in flamewm.css
/// (205px) plus its 2x1px border. Height is padding (2x5px) plus one
/// 31px row per visible row.
#[must_use]
pub fn context_menu_size(visible_rows: usize) -> Size {
    let rows = visible_rows.max(1) as i32;
    Size::new(207, 10 + rows * 31)
}

/// Pointer-anchored popup rect: point-anchored at the root click point,
/// clamped inside the output work area. Falls back to output origin only
/// when no pointer data exists.
#[must_use]
pub fn context_menu_rect(
    output: OutputId,
    output_rect: Rect,
    pointer: Option<Point>,
    size: Size,
) -> Rect {
    let Some(pointer) = pointer else {
        return Rect::new(output_rect.x, output_rect.y, size.width, size.height);
    };
    let _ = output;
    Rect::new(pointer.x, pointer.y, size.width, size.height).clamp_inside(output_rect)
}

/// Context-menu placement: Below/Start preferred, Above/Start opposite.
/// Caps height to the work area and reports scrolling when capped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextMenuPlacement {
    pub rect: Rect,
    pub requires_scroll: bool,
}

#[must_use]
pub fn context_menu_placement(
    anchor: Rect,
    size: Size,
    work_area: Rect,
    offset: i32,
) -> ContextMenuPlacement {
    let width = size.width.max(0).min(work_area.width.max(0));
    let below = PopoverGeometry::place(
        anchor,
        Size::new(width, size.height.max(0)),
        PopoverEdge::Below,
        work_area,
        offset,
    );
    let above = PopoverGeometry::place(
        anchor,
        Size::new(width, size.height.max(0)),
        PopoverEdge::Above,
        work_area,
        offset,
    );
    // Preferred: Below/Start. Opposite: Above/Start. Start (x) is clamped
    // to the work area in both cases.
    let below_fits_height =
        size.height.max(0) <= work_area.height.max(0) && below.edge == PopoverEdge::Below;
    let above_fits_height =
        size.height.max(0) <= work_area.height.max(0) && above.edge == PopoverEdge::Above;
    let geometry = if below_fits_height {
        below
    } else if above_fits_height {
        above
    } else {
        below
    };
    let available = work_area.height.max(0);
    let requires_scroll = size.height.max(0) > available;
    let height = size.height.max(0).min(available);
    let rect = Rect::from_parts(geometry.origin, Size::new(width, height)).clamp_inside(work_area);
    ContextMenuPlacement {
        rect,
        requires_scroll,
    }
}
