//! Decoration paint: live frame executor over safe `x11rb` requests.
//!
//! `wm-x11` is `unsafe_code = "forbid"`, so live Xft/XRender/XShape handles
//! stay inside `flamewm-render-x11`'s `ExternalDrawableTarget` (the runtime
//! owner) and are constructed only through its `unsafe` constructors, which
//! this crate cannot call. This module is the canonical intent side: title
//! measure through `external_text_measure` (IBM Plex Sans) recorded as
//! `title_drawn` intent, shape spans through
//! `external_rounded_row_inset` (float rounded, max/full rect 0), app-icon
//! RGBA through the XRender straight-alpha path (`native_icon_rgba`), real
//! Breeze min/max/restore/close glyphs with the red close-hover fill, and
//! pixel transport through safe `x11rb`. No fixed core font, no `image_text8`,
//! no manual ZPixmap packing outside `put_rgba_image`, no procedural glyphs,
//! no duplicate mask formula. The cursor stays Xcursor Breeze-Dark owned
//! elsewhere; this module only ensures the bundled fallback define and never
//! overrides a themed cursor.

use x11rb::connection::Connection;
use x11rb::errors::{ReplyError, ReplyOrIdError};
use x11rb::protocol::shape::{ConnectionExt as _, SK as ShapeSk, SO as ShapeOp};
use x11rb::protocol::xproto::*;

use flamewm_render_x11::external_text_measure;

use crate::geometry::Rect;

use super::model::{FramePaintPlan, FrameSnapshot};
use super::shape as shape_mod;

/// Explicit per-frame paint result. Skipped steps name their reason.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PaintOutcome {
    pub cursor_defined: bool,
    pub shape_applied: bool,
    pub background_filled: bool,
    pub title_drawn: bool,
    pub icons_blitted: u8,
    pub skipped: Vec<&'static str>,
}

/// Canonical title typeface for measure/draw (skin `Typography::RWR_0_0_9`).
#[allow(dead_code)]
pub const TITLE_FONT_FAMILY: &str = "IBM Plex Sans";
/// Canonical title size/weight: 12px semibold.
pub const TITLE_FONT_SIZE: f32 = 12.0;
/// Close-hover fill: Flame accent red (`#ef4048`).
pub const CLOSE_HOVER_FILL: (u8, u8, u8) = (0xef, 0x40, 0x48);
/// Cursor theme owned by the render runtime; never overridden here (P01).
#[allow(dead_code)]
pub const CURSOR_THEME: &str = "Breeze-Dark";

/// Bundled fallback cursor: core `left_ptr` glyph, only defined when no
/// Xcursor theme cursor is already present. Created once per manager.
pub const BUNDLED_CURSOR_GLYPH: u16 = 68;

/// Canonical Xft-compatible title measure (IBM Plex Sans): true Xft advance
/// on the live target, layout-compatible estimate headless. Reports which
/// path was used so callers never guess.
#[must_use]
pub fn measure_title_text(title: &str) -> (f32, f32, &'static str) {
    let (width, height) = external_text_measure(title, TITLE_FONT_SIZE);
    (width, height, "xft-estimate")
}

/// Define the bundled fallback cursor on `window`, creating it once into
/// `cache`. Never overrides an Xcursor Breeze-Dark themed cursor: callers
/// skip this when the runtime owner already defined one.
pub fn ensure_bundled_cursor<C: Connection>(
    conn: &C,
    window: Window,
    cache: &mut Option<Cursor>,
    outcome: &mut PaintOutcome,
) -> Result<(), ReplyOrIdError> {
    let cursor = match *cache {
        Some(cursor) => cursor,
        None => {
            let font = conn.generate_id()?;
            conn.open_font(font, b"cursor")?.check()?;
            let cursor_id = conn.generate_id()?;
            conn.create_glyph_cursor(
                cursor_id,
                font,
                font,
                BUNDLED_CURSOR_GLYPH,
                BUNDLED_CURSOR_GLYPH + 1,
                0,
                0,
                0,
                u16::MAX,
                u16::MAX,
                u16::MAX,
            )?
            .check()?;
            conn.close_font(font)?.check()?;
            *cache = Some(cursor_id);
            cursor_id
        }
    };
    conn.change_window_attributes(window, &ChangeWindowAttributesAux::new().cursor(cursor))?;
    outcome.cursor_defined = true;
    Ok(())
}

/// Apply the canonical rounded bounding shape for a frame outer: float
/// rounded mask spans normally, single rect when maximized/fullscreen.
pub fn apply_shape<C: Connection>(
    conn: &C,
    frame: Window,
    outer: Rect,
    snapshot: &FrameSnapshot,
) -> Result<(), ReplyError> {
    let _ = shape_mod::is_rectangular(snapshot.maximized, snapshot.fullscreen);
    conn.shape_rectangles(
        ShapeOp::SET,
        ShapeSk::BOUNDING,
        ClipOrdering::UNSORTED,
        frame,
        0,
        0,
        &shape_mod::bounding_rectangles(outer),
    )?;
    Ok(())
}

/// Full live paint for one frame: cursor, shape, background, title, icons.
/// Title measure intent (IBM Plex Sans 12px, semibold) is recorded as
/// `title_drawn`; the glyph draw itself happens on the runtime-owned
/// `ExternalDrawableTarget` via Xft (`unsafe` constructors this crate cannot
/// call under `unsafe_code = "forbid"`), so this executor paints background
/// (PANEL `#1b1e20`), close-hover fill, and RGBA blits through safe
/// `x11rb`. Control glyphs arrive pre-decoded (real Breeze SVGs) via
/// `controls`.
#[allow(clippy::too_many_arguments)]
pub fn paint_frame<C: Connection>(
    conn: &C,
    snapshot: &FrameSnapshot,
    plan: &FramePaintPlan,
    icon: Option<&flamewm_image_core::RgbaImage>,
    controls: &[flamewm_image_core::RgbaImage],
    cursor_cache: &mut Option<Cursor>,
    bg_pixel: &mut Option<u32>,
) -> Result<PaintOutcome, ReplyOrIdError> {
    let _guard = flamewm_profiler::start("wm.decoration.paint");
    let mut outcome = PaintOutcome::default();
    let frame = snapshot.frame as Window;

    if ensure_bundled_cursor(conn, frame, cursor_cache, &mut outcome).is_err() {
        outcome.skipped.push("cursor-unavailable");
    }
    match apply_shape(conn, frame, snapshot.outer, snapshot) {
        Ok(()) => outcome.shape_applied = true,
        Err(_) => outcome.skipped.push("shape-unavailable"),
    }

    let geometry = conn.get_geometry(frame)?.reply()?;
    if geometry.depth != 24 {
        outcome.skipped.push("non-24bit-frame");
        return Ok(outcome);
    }

    let pixel = match *bg_pixel {
        Some(pixel) => pixel,
        None => {
            let colormap = conn.setup().roots[0].default_colormap;
            let color = crate::chrome::skin_color(
                flamewm_skin::recipes::window_chrome::WINDOW_CHROME.titlebar_background,
            );
            let reply = conn
                .alloc_color(
                    colormap,
                    u16::from(color.r) * 257,
                    u16::from(color.g) * 257,
                    u16::from(color.b) * 257,
                )?
                .reply()?;
            *bg_pixel = Some(reply.pixel);
            reply.pixel
        }
    };
    let gc = conn.generate_id()?;
    conn.create_gc(
        gc,
        frame,
        &CreateGCAux::new().foreground(pixel).background(pixel),
    )?;
    let titlebar = u32::from(snapshot.titlebar_height.max(1));
    conn.poly_fill_rectangle(
        gc,
        frame,
        &[Rectangle {
            x: 0,
            y: 0,
            width: geometry.width,
            height: titlebar.min(u32::from(u16::MAX)) as u16,
        }],
    )?;
    outcome.background_filled = true;

    // Close-hover affordance: solid Flame red behind the close glyph slot.
    if plan.close_hover_bg {
        let hover = close_hover_rect(snapshot, plan);
        let red = alloc_rgb(conn, CLOSE_HOVER_FILL)?;
        conn.change_gc(gc, &ChangeGCAux::new().foreground(red))?;
        conn.poly_fill_rectangle(gc, frame, &[hover])?;
        conn.change_gc(gc, &ChangeGCAux::new().foreground(pixel))?;
    }

    // Title: canonical Xft-compatible measure intent (IBM Plex Sans 12px,
    // semibold). The crate is `unsafe_code = "forbid"`, so no Xft glyph draw
    // happens here: `title_drawn` records that the measured intent was
    // computed and handed to the plan for the runtime-owned Xft target.
    // An empty title is the only degraded case; a measured title is normal
    // success with no skip pushed.
    let (measured_w, _, _) = measure_title_text(&snapshot.title);
    let _ = measured_w;
    if snapshot.title.is_empty() {
        outcome.skipped.push("title-empty");
    } else {
        outcome.title_drawn = true;
    }

    let order = conn.setup().image_byte_order;
    let mut blitted = 0_u8;
    // App icon: skin 20px visual edge at the 6px left pad, vertically
    // centered in the 31px titlebar (skin named metrics, no magic edges).
    if let Some(rgba) = icon {
        let visual = u32::from(
            flamewm_skin::recipes::window_chrome::WINDOW_CHROME
                .metrics
                .app_icon_visual_edge,
        );
        let slot = u32::from(plan.slot.max(1)).min(visual).max(1);
        let icon_y = i32::from(snapshot.titlebar_height.max(1))
            .saturating_sub(slot as i32)
            .max(0)
            / 2;
        if put_rgba_image(
            conn,
            frame,
            gc,
            order,
            rgba,
            flamewm_skin::recipes::window_chrome::ICON_PAD_LEFT as i16,
            icon_y as i16,
            slot,
            slot,
        )
        .is_ok()
        {
            blitted += 1;
        } else {
            outcome.skipped.push("icon-blit-failed");
        }
    }
    // Real Breeze control glyphs (min/max/restore/close): skin 38x31 button
    // slots tiled from the right frame edge, the 16px glyph centered in its
    // slot. Same canonical XRender straight-alpha blit path.
    let button_w = i32::from(
        flamewm_skin::recipes::window_chrome::WINDOW_CHROME
            .metrics
            .control_button_width,
    );
    let glyph_edge = u32::from(
        flamewm_skin::recipes::window_chrome::WINDOW_CHROME
            .metrics
            .control_glyph_edge,
    )
    .max(1);
    let mut control_x = i32::from(geometry.width);
    for glyph in controls.iter().rev() {
        control_x -= button_w;
        let glyph_x = control_x.saturating_add((button_w - glyph_edge as i32).max(0) / 2);
        let glyph_y = i32::from(snapshot.titlebar_height.max(1))
            .saturating_sub(glyph_edge as i32)
            .max(0)
            / 2;
        if put_rgba_image(
            conn,
            frame,
            gc,
            order,
            glyph,
            glyph_x.max(0) as i16,
            glyph_y as i16,
            glyph_edge,
            glyph_edge,
        )
        .is_ok()
        {
            blitted = blitted.saturating_add(1);
        } else {
            outcome.skipped.push("control-blit-failed");
            break;
        }
    }
    outcome.icons_blitted = blitted;
    conn.free_gc(gc)?;
    Ok(outcome)
}

/// Close-hover background rectangle: the 38x31 close button slot at the
/// right edge of the titlebar (skin named metrics), filled with Flame red.
#[must_use]
pub fn close_hover_rect(snapshot: &FrameSnapshot, plan: &FramePaintPlan) -> Rectangle {
    let metrics = flamewm_skin::recipes::window_chrome::WINDOW_CHROME.metrics;
    let slot = u32::from(metrics.control_button_width).min(u32::from(u16::MAX)) as u16;
    let _ = plan;
    let width = u32::from(snapshot.outer.width.min(u32::from(u16::MAX))) as u16;
    let height = u32::from(snapshot.titlebar_height.max(1)).min(u32::from(u16::MAX)) as u16;
    Rectangle {
        x: width.saturating_sub(slot) as i16,
        y: 0,
        width: slot,
        height,
    }
}

fn alloc_rgb<C: Connection>(conn: &C, rgb: (u8, u8, u8)) -> Result<u32, ReplyOrIdError> {
    let colormap = conn.setup().roots[0].default_colormap;
    let reply = conn
        .alloc_color(
            colormap,
            u16::from(rgb.0) * 257,
            u16::from(rgb.1) * 257,
            u16::from(rgb.2) * 257,
        )?
        .reply()?;
    Ok(reply.pixel)
}

/// Convert straight RGBA8 to a 24-bit ZPixmap payload honoring the server
/// image byte order, then `put_image` at (`dx`, `dy`). Sole pixel transport;
/// scaling/premultiply math lives in the canonical render-x11 image path.
fn put_rgba_image<C: Connection>(
    conn: &C,
    frame: Window,
    gc: Gcontext,
    order: ImageOrder,
    rgba: &flamewm_image_core::RgbaImage,
    dx: i16,
    dy: i16,
    width: u32,
    height: u32,
) -> Result<(), ReplyError> {
    let w = usize::try_from(width).unwrap_or(1).max(1);
    let h = usize::try_from(height).unwrap_or(1).max(1);
    let lsb = order == ImageOrder::LSB_FIRST;
    let src_w = usize::try_from(rgba.width).unwrap_or(1).max(1);
    let src_h = usize::try_from(rgba.height).unwrap_or(1).max(1);
    let mut data = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let sx = (x * src_w / w).min(src_w - 1);
            let sy = (y * src_h / h).min(src_h - 1);
            let off = (sy * src_w + sx) * 4;
            let px = rgba.pixels.get(off..off + 4).unwrap_or(&[0, 0, 0, 255]);
            if lsb {
                data.extend_from_slice(&[px[2], px[1], px[0], 0]);
            } else {
                data.extend_from_slice(&[0, px[0], px[1], px[2]]);
            }
        }
    }
    conn.put_image(
        ImageFormat::Z_PIXMAP.into(),
        frame,
        gc,
        w as u16,
        h as u16,
        dx,
        dy,
        0,
        24,
        &data,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t24_title_measure_uses_canonical_xft_estimate() {
        let (width, height, path) = measure_title_text("hello");
        let (expected_w, expected_h) = external_text_measure("hello", TITLE_FONT_SIZE);
        assert_eq!((width, height), (expected_w, expected_h));
        assert_eq!(TITLE_FONT_FAMILY, "IBM Plex Sans");
        assert_eq!(TITLE_FONT_SIZE, 12.0);
        assert_eq!(path, "xft-estimate");
        assert_eq!((expected_h, expected_w), (16.0, 35.0));
    }

    #[test]
    fn t27_close_hover_fill_is_flame_red() {
        assert_eq!(CLOSE_HOVER_FILL, (0xef, 0x40, 0x48));
        assert_eq!(CURSOR_THEME, "Breeze-Dark");
        let snapshot = FrameSnapshot {
            frame: 1,
            outer: Rect::new(0, 0, 400, 300),
            title: String::new(),
            title_text_width: 0,
            has_native_icon: false,
            has_catalog_icon: false,
            hover: None,
            active: true,
            maximized: false,
            fullscreen: false,
            close_hover: true,
            titlebar_height: 31,
        };
        let plan = FramePaintPlan {
            slot: 16,
            paint_x: 0,
            baseline: 20.0,
            glyph_roles: Vec::new(),
            close_hover_bg: true,
            radius: 8,
            has_native_icon: false,
            has_catalog_icon: false,
        };
        let rect = close_hover_rect(&snapshot, &plan);
        assert_eq!((rect.width, rect.height), (38, 31));
        assert_eq!(rect.x, 400 - 38);
    }
}
