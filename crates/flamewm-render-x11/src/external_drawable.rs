//! External drawable painter: safe native methods for wm-x11.
//! No raw Xlib handles escape; all drawing goes through this renderer.

use flamewm_render_core::{Color, Rect, RuntimeDocument};

use super::app::X11App;
use super::xlib::*;

/// Safe painter over an X11App drawable. Exposes text measure/draw via
/// Xft, RGBA blit via ARGB32/XRender, shape apply, and skin color fill.
pub struct NativeDrawableRenderer<'a> {
    app: &'a mut X11App,
}

impl<'a> NativeDrawableRenderer<'a> {
    pub fn new(app: &'a mut X11App) -> Self {
        Self { app }
    }

    /// Measure text width in device pixels using the core font fallback.
    /// Returns an estimate when Xft measurement is unavailable.
    pub fn text_measure(&mut self, text: &str, size: f32) -> (f32, f32) {
        let height = (size.max(1.0) * 1.30).round();
        if unsafe { self.app.font_for_size_pub(size) }.is_some() {
            // Core-font measurement needs the raw XFontStruct pointer, which
            // stays inside X11App; use the layout-compatible estimate here.
        }
        // Estimate fallback: 0.58em per char (matches layout measure).
        ((text.chars().count() as f32 * size * 0.58).round(), height)
    }

    /// Draw text at device-pixel (x, baseline y) via Xft when available.
    pub fn draw_text(
        &mut self,
        document: &RuntimeDocument,
        x: f32,
        y: f32,
        color: Color,
        size: f32,
        weight: u16,
        text: &str,
    ) -> Result<(), String> {
        if let Some(xft) = self.app.xft.as_mut() {
            unsafe { xft.set_preferred_family(document.ui_font_family()) };
            return unsafe { xft.draw_text(x, y, color, size, weight, text) };
        }
        let pixel = unsafe { self.app.pixel_pub(color)? };
        unsafe { XSetForeground(self.app.display, self.app.gc, pixel) };
        if let Some(font) = unsafe { self.app.font_for_size_pub(size) } {
            unsafe { XSetFont(self.app.display, self.app.gc, font) };
        }
        let ascii: String = text
            .chars()
            .map(|ch| if ch.is_ascii() { ch } else { '?' })
            .collect();
        let string = std::ffi::CString::new(ascii).map_err(|_| "text contains NUL".to_string())?;
        let len = string.as_bytes().len().min(i32::MAX as usize) as i32;
        unsafe {
            XDrawString(
                self.app.display,
                self.app.backbuffer,
                self.app.gc,
                x.round() as i32,
                y.round() as i32,
                string.as_ptr(),
                len,
            )
        };
        Ok(())
    }

    /// Blit straight RGBA8 pixels at device rect via ARGB32/XRender.
    pub fn rgba_blit(
        &mut self,
        rgba8: &[u8],
        src_w: u32,
        src_h: u32,
        dest: Rect,
    ) -> Result<(), String> {
        let w = dest.width.round().max(1.0) as u32;
        let h = dest.height.round().max(1.0) as u32;
        let argb = crate::native::image::scale_and_premultiply(rgba8, src_w, src_h, w, h);
        let root = unsafe { XRootWindow(self.app.display, self.app.screen) };
        let pixmap = unsafe { XCreatePixmap(self.app.display, root, w, h, 32) };
        if pixmap == 0 {
            return Err("XCreatePixmap failed for rgba_blit".to_string());
        }
        let result = unsafe { self.app.upload_argb32_pub(pixmap, &argb, w, h) };
        if let Err(error) = result {
            unsafe { XFreePixmap(self.app.display, pixmap) };
            return Err(error);
        }
        let dx = dest.x.round() as i32;
        let dy = dest.y.round() as i32;
        let blit = if let Some(xrender) = self.app.xrender.as_mut() {
            unsafe { xrender.blit_argb32_over(pixmap, dx, dy, w, h) }
        } else {
            unsafe {
                XCopyArea(
                    self.app.display,
                    pixmap,
                    self.app.backbuffer,
                    self.app.gc,
                    0,
                    0,
                    w,
                    h,
                    dx,
                    dy,
                )
            };
            Ok(())
        };
        unsafe { XFreePixmap(self.app.display, pixmap) };
        blit
    }

    /// Fill a device-pixel rect with a skin color.
    pub fn fill_skin(&mut self, rect: Rect, color: Color) -> Result<(), String> {
        if let Some(xrender) = self.app.xrender.as_mut() {
            if color.a < 255 {
                return unsafe { xrender.fill_rounded_rect(rect, 0.0, color) };
            }
        }
        let pixel = unsafe { self.app.pixel_pub(color)? };
        unsafe { XSetForeground(self.app.display, self.app.gc, pixel) };
        unsafe {
            XFillRectangle(
                self.app.display,
                self.app.backbuffer,
                self.app.gc,
                rect.x.round() as i32,
                rect.y.round() as i32,
                rect.width.round().max(1.0) as u32,
                rect.height.round().max(1.0) as u32,
            )
        };
        Ok(())
    }

    /// Apply the rounded-corner shape mask for chrome-ready surfaces.
    pub fn apply_shape(&mut self, document: &RuntimeDocument) {
        unsafe { self.app.refresh_shape_mask(document) };
    }

    /// Flush the display connection.
    pub fn flush(&mut self) {
        unsafe { XFlush(self.app.display) };
    }
}
