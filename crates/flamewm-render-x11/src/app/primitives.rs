use super::*;
impl X11App {
    pub(crate) unsafe fn fill_rounded_rect(&self, rect: Rect, radius: f32) {
        let x = rect.x.round() as i32;
        let y = rect.y.round() as i32;
        let width = rect.width.round().max(0.0) as u32;
        let height = rect.height.round().max(0.0) as u32;
        if width == 0 || height == 0 {
            return;
        }
        let radius = radius.round().max(0.0).min((width.min(height) / 2) as f32) as u32;
        if radius == 0 {
            // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
            unsafe { XFillRectangle(self.display, self.backbuffer, self.gc, x, y, width, height) };
            return;
        }
        let diameter = radius * 2;
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XFillRectangle(
                self.display,
                self.backbuffer,
                self.gc,
                x + radius as i32,
                y,
                width.saturating_sub(diameter),
                height,
            );
        }
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XFillRectangle(
                self.display,
                self.backbuffer,
                self.gc,
                x,
                y + radius as i32,
                width,
                height.saturating_sub(diameter),
            );
        }
        let right = x + width as i32 - diameter as i32;
        let bottom = y + height as i32 - diameter as i32;
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XFillArc(
                self.display,
                self.backbuffer,
                self.gc,
                x,
                y,
                diameter,
                diameter,
                90 * 64,
                90 * 64,
            );
        }
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XFillArc(
                self.display,
                self.backbuffer,
                self.gc,
                right,
                y,
                diameter,
                diameter,
                0,
                90 * 64,
            );
        }
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XFillArc(
                self.display,
                self.backbuffer,
                self.gc,
                x,
                bottom,
                diameter,
                diameter,
                180 * 64,
                90 * 64,
            );
        }
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XFillArc(
                self.display,
                self.backbuffer,
                self.gc,
                right,
                bottom,
                diameter,
                diameter,
                270 * 64,
                90 * 64,
            );
        }
    }

    pub(crate) unsafe fn stroke_rounded_rect(&self, rect: Rect, radius: f32, inset: i32) {
        let x = rect.x.round() as i32 + inset;
        let y = rect.y.round() as i32 + inset;
        let width = (rect.width.round() as i32 - 1 - inset * 2).max(0) as u32;
        let height = (rect.height.round() as i32 - 1 - inset * 2).max(0) as u32;
        if width == 0 || height == 0 {
            return;
        }
        let radius = (radius.round() as i32 - inset)
            .max(0)
            .min((width.min(height) / 2) as i32) as u32;
        if radius <= 1 {
            // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
            unsafe { XDrawRectangle(self.display, self.backbuffer, self.gc, x, y, width, height) };
            return;
        }
        let diameter = radius * 2;
        let right = x + width as i32 - diameter as i32;
        let bottom = y + height as i32 - diameter as i32;
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XDrawLine(
                self.display,
                self.backbuffer,
                self.gc,
                x + radius as i32,
                y,
                x + width as i32 - radius as i32,
                y,
            );
        }
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XDrawLine(
                self.display,
                self.backbuffer,
                self.gc,
                x + radius as i32,
                y + height as i32,
                x + width as i32 - radius as i32,
                y + height as i32,
            );
        }
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XDrawLine(
                self.display,
                self.backbuffer,
                self.gc,
                x,
                y + radius as i32,
                x,
                y + height as i32 - radius as i32,
            );
        }
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XDrawLine(
                self.display,
                self.backbuffer,
                self.gc,
                x + width as i32,
                y + radius as i32,
                x + width as i32,
                y + height as i32 - radius as i32,
            );
        }
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XDrawArc(
                self.display,
                self.backbuffer,
                self.gc,
                x,
                y,
                diameter,
                diameter,
                90 * 64,
                90 * 64,
            );
        }
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XDrawArc(
                self.display,
                self.backbuffer,
                self.gc,
                right,
                y,
                diameter,
                diameter,
                0,
                90 * 64,
            );
        }
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XDrawArc(
                self.display,
                self.backbuffer,
                self.gc,
                x,
                bottom,
                diameter,
                diameter,
                180 * 64,
                90 * 64,
            );
        }
        // SAFETY: App owns live Xlib display, backbuffer, and graphics context.
        unsafe {
            XDrawArc(
                self.display,
                self.backbuffer,
                self.gc,
                right,
                bottom,
                diameter,
                diameter,
                270 * 64,
                90 * 64,
            );
        }
    }
}

pub(crate) fn scale_rect(rect: Rect, scale: f32) -> Rect {
    Rect {
        x: rect.x * scale,
        y: rect.y * scale,
        width: rect.width * scale,
        height: rect.height * scale,
    }
}
