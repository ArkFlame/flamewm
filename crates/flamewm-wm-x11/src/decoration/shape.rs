//! Decoration shape: canonical rounded-mask spans for frame windows.
//!
//! Single formula owner is render-x11 (`external_rounded_row_inset`, float
//! rounded `floor(r - dx)` at pixel-row centers): full-width rows when the
//! radius is 0/1, when maximized/fullscreen, or mid-span; corner insets
//! otherwise. Bounding-region assembly lives here over safe `x11rb`
//! rectangles; the live mask apply lives in `render-x11`'s XShape bridge.

use flamewm_render_x11::external_rounded_row_inset;

use x11rb::protocol::xproto::Rectangle;

use crate::chrome;
use crate::geometry::Rect;

/// Rounded radius normally, rectangular when maximized or fullscreen.
#[must_use]
pub fn effective_radius(maximized: bool, fullscreen: bool) -> u16 {
    chrome::effective_radius(maximized, fullscreen)
}

/// Canonical corner inset for one row (float rounded, shared with
/// render-x11 mask spans and the XRender fill path).
#[must_use]
pub fn rounded_row_inset(width: u32, height: u32, radius: u32, row: u32) -> u32 {
    external_rounded_row_inset(width, height, radius, row)
}

/// Canonical row spans (y, x_start, x_end_exclusive) for a rounded rect:
/// float rounded mask, single full-width rows when rectangular.
#[must_use]
pub fn rounded_mask_spans(width: u32, height: u32, radius: u32) -> Vec<(u32, u32, u32)> {
    let _guard = flamewm_profiler::start("wm.decoration.shape");
    if width == 0 || height == 0 {
        return Vec::new();
    }
    if radius <= 1 {
        return (0..height).map(|row| (row, 0, width)).collect();
    }
    let mut spans = Vec::with_capacity(height as usize);
    for row in 0..height {
        let inset = external_rounded_row_inset(width, height, radius, row);
        let start = inset.min(width);
        let end = width.saturating_sub(inset).max(start);
        spans.push((row, start, end));
    }
    spans
}

/// Bounding rectangles for a frame outer: canonical row spans (float
/// rounded normally, single rect when maximized/fullscreen).
#[must_use]
pub fn bounding_rectangles(outer: Rect) -> Vec<Rectangle> {
    rounded_mask_spans(
        outer.width.min(u32::from(u16::MAX)),
        outer.height.min(u32::from(u16::MAX)),
        u32::from(effective_radius(false, false)),
    )
    .into_iter()
    .map(|(y, start, end)| Rectangle {
        x: i16::try_from(start.min(u32::from(u16::MAX))).unwrap_or(i16::MAX),
        y: i16::try_from(y.min(u32::from(u16::MAX))).unwrap_or(i16::MAX),
        width: end.saturating_sub(start).min(u32::from(u16::MAX)) as u16,
        height: 1,
    })
    .collect()
}

/// Bounding rectangles for an explicit maximized/fullscreen state: single
/// rect when rectangular, canonical spans otherwise.
#[must_use]
pub fn bounding_rectangles_for(outer: Rect, maximized: bool, fullscreen: bool) -> Vec<Rectangle> {
    let radius = u32::from(effective_radius(maximized, fullscreen));
    if radius == 0 {
        return vec![Rectangle {
            x: 0,
            y: 0,
            width: outer.width.min(u32::from(u16::MAX)) as u16,
            height: outer.height.min(u32::from(u16::MAX)) as u16,
        }];
    }
    bounding_rectangles(outer)
}

/// True when the frame keeps its rectangular bounding shape and skips the
/// rounded paint mask (maximized or fullscreen).
#[must_use]
pub fn is_rectangular(maximized: bool, fullscreen: bool) -> bool {
    effective_radius(maximized, fullscreen) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t26_shape_spans_match_canonical_mask() {
        assert_eq!(
            rounded_mask_spans(8, 8, 4),
            vec![
                (0, 2, 6),
                (1, 0, 8),
                (2, 0, 8),
                (3, 0, 8),
                (4, 0, 8),
                (5, 0, 8),
                (6, 0, 8),
                (7, 2, 6),
            ]
        );
        assert_eq!(rounded_row_inset(8, 8, 4, 0), 2);
        assert_eq!(rounded_mask_spans(4, 2, 0), vec![(0, 0, 4), (1, 0, 4)]);
        assert!(rounded_mask_spans(0, 8, 4).is_empty());
    }

    #[test]
    fn t26_max_full_is_single_rect() {
        let outer = Rect::new(0, 0, 400, 300);
        let rects = bounding_rectangles_for(outer, true, false);
        assert_eq!(rects.len(), 1);
        assert_eq!((rects[0].width, rects[0].height), (400, 300));
        let full = bounding_rectangles_for(outer, false, true);
        assert_eq!(full.len(), 1);
        assert!(is_rectangular(true, false));
        assert!(!is_rectangular(false, false));
    }
}
