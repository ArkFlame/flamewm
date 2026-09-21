//! Safe facade over external drawable/cursor infrastructure for wm-owned
//! chrome (frames, titles, cursors). Owns the process-lifetime X connection,
//! [`ExternalDrawableSession`], retargetable [`ExternalDrawableTarget`],
//! Xcursor backend, and font/cache resources. All methods are safe to call:
//! invalid XIDs produce explicit protocol-level errors, never memory
//! unsafety. When no display is reachable the renderer reports explicit
//! errors for live paint paths and layout-compatible estimates for measure.

use std::ptr;

use flamewm_render_core::{Color, CursorKind, Rect};

use super::external_drawable::{
    ExternalDrawableSession, ExternalDrawableTarget, external_text_measure, use_cstring_title,
};
use super::native::cursor::{DeferredNativeLibraryHandles, NativeCursorSession, XcursorBackend};
use super::xlib::{
    Display, XCloseDisplay, XFlush, XOpenDisplay, install_x_protocol_error_handler, x_io_broken,
};

/// Safe renderer for wm-owned frame chrome. Created via [`Self::open`]
/// (opens its own X connection) or [`Self::unavailable`] (explicit fake
/// seam for tests/headless use; live ops error, measure stays truthful).
pub struct ExternalDecorationRenderer {
    display: *mut Display,
    session: Option<Box<ExternalDrawableSession>>,
    cursor: Option<NativeCursorSession>,
    target: Option<ExternalDrawableTarget>,
    font_family: String,
    unavailable: Option<String>,
    last_title_style: Option<(f32, i32)>,
}

impl Drop for ExternalDecorationRenderer {
    fn drop(&mut self) {
        // Release X-backed objects while the display is live, but retain every
        // dynamic backend handle until after XCloseDisplay. This includes the
        // target's Xft/XRender/XShape libraries and the cursor library.
        let mut deferred_native = DeferredNativeLibraryHandles::new();
        let broken = x_io_broken();
        let mut target = self.target.take();
        if broken {
            // A broken connection cannot safely release X resources. Preserve
            // all native owners rather than invoking their Drop paths.
            std::mem::forget(target);
            std::mem::forget(self.cursor.take());
            std::mem::forget(self.session.take());
            return;
        }
        if let Some(target) = target.as_mut() {
            // SAFETY: this renderer still owns the live display/session.
            deferred_native.extend(unsafe { target.take_deferred_native_libraries() });
        }
        drop(target);
        if let Some(cursor) = self.cursor.take() {
            // SAFETY: this renderer still owns the live display.
            deferred_native.extend(unsafe {
                DeferredNativeLibraryHandles::from_backends(Some(cursor), None, None, None)
            });
        }
        drop(self.session.take());
        if !self.display.is_null() {
            unsafe { XCloseDisplay(self.display) };
            self.display = ptr::null_mut();
        }
        drop(deferred_native);
    }
}

impl ExternalDecorationRenderer {
    /// Open the default display and bind the process-lifetime session plus
    /// the Xcursor backend. Safe: the connection is owned by `Self`.
    pub fn open(font_family: &str) -> Result<Self, String> {
        let display = unsafe { XOpenDisplay(ptr::null()) };
        if display.is_null() {
            return Err(
                "ExternalDecorationRenderer: XOpenDisplay failed; DISPLAY is unset or unreachable"
                    .to_string(),
            );
        }
        install_x_protocol_error_handler();
        let session = match unsafe { ExternalDrawableSession::new(display) } {
            Ok(session) => session,
            Err(error) => {
                unsafe { XCloseDisplay(display) };
                return Err(format!("ExternalDecorationRenderer session: {error}"));
            }
        };
        let backend = unsafe { XcursorBackend::with_theme(display, None, None) };
        Ok(Self {
            display,
            session: Some(Box::new(session)),
            cursor: Some(NativeCursorSession::wrap(backend)),
            target: None,
            font_family: font_family.to_string(),
            unavailable: None,
            last_title_style: None,
        })
    }

    /// Explicit unavailable seam for tests/headless hosts: records `reason`,
    /// touches no display, never fabricates backend success.
    pub fn unavailable(reason: &str) -> Self {
        Self {
            display: ptr::null_mut(),
            session: None,
            cursor: None,
            target: None,
            font_family: String::new(),
            unavailable: Some(reason.to_string()),
            last_title_style: None,
        }
    }

    /// False exactly when built via [`Self::unavailable`].
    pub fn is_available(&self) -> bool {
        self.unavailable.is_none()
    }

    fn live(&self) -> Result<(), String> {
        if let Some(reason) = self.unavailable.as_ref() {
            return Err(format!("external decoration unavailable: {reason}"));
        }
        if self.display.is_null() || self.session.is_none() {
            return Err("external decoration has no live display".to_string());
        }
        Ok(())
    }

    fn target_mut(&mut self) -> Result<&mut ExternalDrawableTarget, String> {
        self.live()?;
        self.target
            .as_mut()
            .ok_or_else(|| "external decoration has no retargeted drawable".to_string())
    }

    /// Bind (or rebind) the paint target to `frame_xid` at `w`x`h`.
    /// A bad XID yields an explicit error from the backend, not UB.
    pub fn retarget(&mut self, frame_xid: u64, w: u32, h: u32) -> Result<(), String> {
        let drawable = frame_xid as super::xlib::Drawable;
        if drawable == 0 {
            return Err("retarget requires a nonzero frame XID".to_string());
        }
        self.live()?;
        if let Some(target) = self.target.as_mut() {
            return unsafe { target.retarget(drawable, w, h) };
        }
        let session_ptr =
            self.session.as_mut().expect("live session").as_mut() as *mut ExternalDrawableSession;
        let target = unsafe { ExternalDrawableTarget::new(session_ptr, drawable, w, h) }?;
        self.target = Some(target);
        Ok(())
    }

    /// Fill `rect` with `color` on the current target.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) -> Result<(), String> {
        self.target_mut()?.fill_rect(rect, color)
    }

    /// Draw `text` with `font` family at (`x`, `baseline`) in `color`.
    /// NUL bytes are rejected explicitly; a missing Xft backend errors
    /// (no fallback paint). Caller supplies pixel size and weight.
    pub fn draw_title(
        &mut self,
        text: &str,
        font: &str,
        size_px: f32,
        weight: i32,
        x: f32,
        baseline: f32,
        color: Color,
    ) -> Result<(), String> {
        if !size_px.is_finite() || size_px <= 0.0 {
            return Err("draw_title requires a positive finite size_px".to_string());
        }
        if !(100..=900).contains(&weight) {
            return Err("draw_title requires weight in 100..=900".to_string());
        }
        let _ = use_cstring_title(text)?;
        let family = if font.is_empty() {
            self.font_family.clone()
        } else {
            font.to_string()
        };
        self.last_title_style = Some((size_px, weight));
        self.target_mut()?
            .draw_text(&family, x, baseline, color, size_px, weight as u16, text)
    }

    /// Last (size_px, weight) accepted by [`Self::draw_title`], including
    /// style validation before any display touch. `None` until first call.
    pub fn last_title_style(&self) -> Option<(f32, i32)> {
        self.last_title_style
    }

    /// Straight RGBA8 blit scaled to `dest` on the current target.
    /// Length is validated before touching the display.
    pub fn blit_rgba(
        &mut self,
        rgba8: &[u8],
        src_w: u32,
        src_h: u32,
        dest: Rect,
    ) -> Result<(), String> {
        let expected = (src_w.max(1) as usize)
            .checked_mul(src_h.max(1) as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| "rgba size overflow".to_string())?;
        if rgba8.len() != expected {
            return Err(format!(
                "rgba length {} does not match {src_w}x{src_h} (expected {expected})",
                rgba8.len()
            ));
        }
        self.target_mut()?.blit_rgba(rgba8, src_w, src_h, dest)
    }

    /// Apply the rounded-corner bounding mask of `radius` to the current
    /// target drawable. Errors explicitly when XShape is unavailable.
    pub fn apply_shape(&self, radius: u32) -> Result<(), String> {
        self.live()?;
        self.target
            .as_ref()
            .ok_or_else(|| "external decoration has no retargeted drawable".to_string())?
            .apply_rounded_shape(radius)
    }

    /// Define the semantic cursor `kind` on `frame_xid`.
    pub fn define_cursor(&mut self, frame_xid: u64, kind: CursorKind) -> Result<(), String> {
        let window = frame_xid as super::xlib::Window;
        if window == 0 {
            return Err("define_cursor requires a nonzero frame XID".to_string());
        }
        self.live()?;
        let cursor = self
            .cursor
            .as_mut()
            .ok_or_else(|| "external decoration has no cursor backend".to_string())?;
        match unsafe { cursor.define(window, kind) } {
            Some(_) => Ok(()),
            None => Err("define_cursor failed: no cursor resolved for kind".to_string()),
        }
    }

    /// Measure `text` at `size`: live Xft advance when the backend is up,
    /// else the layout-compatible estimate. Never fabricates Xft success.
    pub fn measure_title(&mut self, text: &str, font: &str, size: f32) -> (f32, f32) {
        let estimate = external_text_measure(text, size);
        if self.unavailable.is_some() {
            return estimate;
        }
        let family = if font.is_empty() {
            self.font_family.clone()
        } else {
            font.to_string()
        };
        match self.target.as_mut() {
            Some(target) => target.measure_decoration_text(&family, size, 400, text),
            None => estimate,
        }
    }

    /// Flush the owned display connection. No-op when unavailable.
    pub fn flush(&self) {
        if self.display.is_null() {
            return;
        }
        unsafe { XFlush(self.display) };
    }

    /// Native image-cache stats: (entries, bytes). (0, 0) when no target.
    pub fn cache_stats(&self) -> (usize, usize) {
        match self.target.as_ref() {
            Some(target) => (target.image_cache_len(), target.image_cache_bytes()),
            None => (0, 0),
        }
    }

    /// Preferred font family bound at [`Self::open`].
    pub fn font_family(&self) -> &str {
        &self.font_family
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_reports_explicit_errors() {
        let mut renderer = ExternalDecorationRenderer::unavailable("no DISPLAY in test");
        assert!(!renderer.is_available());
        assert!(renderer.retarget(42, 8, 8).is_err());
        assert!(
            renderer
                .fill_rect(
                    Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 4.0,
                        height: 4.0,
                    },
                    Color {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 255,
                    }
                )
                .is_err()
        );
        assert!(
            renderer
                .draw_title(
                    "hi",
                    "sans",
                    12.0,
                    600,
                    0.0,
                    12.0,
                    Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 255,
                    }
                )
                .is_err()
        );
        assert!(
            renderer
                .blit_rgba(
                    &[0u8; 16],
                    2,
                    2,
                    Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 2.0,
                        height: 2.0,
                    }
                )
                .is_err()
        );
        assert!(renderer.apply_shape(4).is_err());
        assert!(renderer.define_cursor(42, CursorKind::Default).is_err());
        assert_eq!(renderer.cache_stats(), (0, 0));
        // Flush on an unavailable renderer must not crash.
        renderer.flush();
    }

    #[test]
    fn measure_without_display_matches_layout_estimate() {
        let mut renderer = ExternalDecorationRenderer::unavailable("no DISPLAY in test");
        assert_eq!(renderer.measure_title("hello", "sans", 13.0), (38.0, 17.0));
        assert_eq!(renderer.measure_title("", "sans", 13.0), (0.0, 17.0));
    }

    #[test]
    fn rejects_zero_xid_and_bad_rgba_before_display() {
        let mut renderer = ExternalDecorationRenderer::unavailable("no DISPLAY in test");
        // Zero XIDs are rejected even before liveness is consulted.
        assert!(renderer.retarget(0, 8, 8).is_err());
        assert!(renderer.define_cursor(0, CursorKind::Pointer).is_err());
        // RGBA length mismatch is validated before touching any display.
        let mut live_check = ExternalDecorationRenderer::unavailable("no DISPLAY in test");
        let bad = live_check.blit_rgba(
            &[0u8; 15],
            2,
            2,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 2.0,
                height: 2.0,
            },
        );
        assert!(bad.is_err());
    }

    #[test]
    fn rejects_nul_title_without_display() {
        let mut renderer = ExternalDecorationRenderer::unavailable("no DISPLAY in test");
        let result = renderer.draw_title(
            "a\0b",
            "sans",
            12.0,
            600,
            0.0,
            12.0,
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn title_style_params_reach_draw_target_contract() {
        let mut renderer = ExternalDecorationRenderer::unavailable("no DISPLAY in test");
        assert_eq!(renderer.last_title_style(), None);
        // 12px/600 must be accepted into the draw-target contract (forwarded
        // as style params, not hardcoded 13px/400); the call then fails only
        // on the unavailable display, but the recorded style proves the
        // params reached the contract.
        let result = renderer.draw_title(
            "title",
            "sans",
            12.0,
            600,
            0.0,
            12.0,
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
        );
        assert!(result.is_err());
        assert_eq!(renderer.last_title_style(), Some((12.0, 600)));
        // A second distinct style overwrites: no hardcoded fallback remains.
        let _ = renderer.draw_title(
            "title",
            "sans",
            13.0,
            400,
            0.0,
            12.0,
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
        );
        assert_eq!(renderer.last_title_style(), Some((13.0, 400)));
        // Invalid style never reaches the contract.
        let _ = renderer.draw_title(
            "title",
            "sans",
            0.0,
            600,
            0.0,
            12.0,
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
        );
        assert_eq!(renderer.last_title_style(), Some((13.0, 400)));
    }
}
