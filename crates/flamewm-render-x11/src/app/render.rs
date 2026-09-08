use super::color::root_background;
use super::primitives::scale_rect;
use super::*;

impl X11App {
    /// Intersect the clip stack and apply via XSetClipRectangles on the GC.
    /// Empty stack clears the clip.
    unsafe fn apply_clip_stack(&mut self) {
        if self.clip_stack.is_empty() {
            unsafe { XSetClipMask(self.display, self.gc, 0) };
            unsafe { XSetClipOrigin(self.display, self.gc, 0, 0) };
            return;
        }
        let mut region = self.clip_stack[0];
        for rect in self.clip_stack.iter().skip(1) {
            let x1 = region.x.max(rect.x);
            let y1 = region.y.max(rect.y);
            let x2 = (region.x + region.width).min(rect.x + rect.width);
            let y2 = (region.y + region.height).min(rect.y + rect.height);
            region = Rect {
                x: x1,
                y: y1,
                width: (x2 - x1).max(0.0),
                height: (y2 - y1).max(0.0),
            };
        }
        if region.width <= 0.0 || region.height <= 0.0 {
            region = Rect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            };
        }
        let rect = XRectangle {
            x: region.x.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16,
            y: region.y.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16,
            width: region.width.round().max(0.0).min(u16::MAX as f32) as u16,
            height: region.height.round().max(0.0).min(u16::MAX as f32) as u16,
        };
        // Unsorted single rectangle; ordering constant 0 (Unsorted).
        unsafe { XSetClipRectangles(self.display, self.gc, 0, 0, &rect, 1, 0) };
    }

    pub(crate) unsafe fn font_for_size_pub(&self, size: f32) -> Option<Font> {
        unsafe { self.font_for_size(size) }
    }

    pub(crate) unsafe fn pixel_pub(&mut self, color: Color) -> Result<u64, String> {
        unsafe { self.pixel(color) }
    }

    pub(crate) unsafe fn match_argb32_visual(&mut self) -> Option<*mut Visual> {
        let mut vinfo: XVisualInfo = unsafe { std::mem::zeroed() };
        let matched = unsafe {
            XMatchVisualInfo(self.display, self.screen, 32, TRUE_COLOR_CLASS, &mut vinfo)
        };
        if matched != 0 && !vinfo.visual.is_null() {
            Some(vinfo.visual)
        } else {
            None
        }
    }

    pub(crate) unsafe fn upload_argb32_pub(
        &mut self,
        pixmap: Pixmap,
        argb: &[u32],
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let Some(visual) = (unsafe { self.match_argb32_visual() }) else {
            return Err("no 32-bit TrueColor visual for ARGB32 upload".to_string());
        };
        unsafe { self.upload_argb32_words(pixmap, visual, argb, width, height) }
    }

    unsafe fn font_for_size(&self, size: f32) -> Option<Font> {
        let target = size.round().clamp(1.0, 255.0) as i32;
        self.fonts
            .iter()
            .min_by_key(|(bucket, _)| (i32::from(**bucket) - target).abs())
            .map(|(_, font)| {
                // SAFETY: font pointers are loaded and retained by X11App for its lifetime.
                unsafe { (**font).fid }
            })
    }

    pub(crate) unsafe fn redraw(&mut self, document: &RuntimeDocument) -> Result<(), String> {
        let scale = document.ui_scale();
        let layout = LayoutEngine::compute(
            document,
            self.width as f32 / scale,
            self.height as f32 / scale,
            self.interaction,
        );
        let commands = build_paint_commands(document, &layout, self.interaction);
        let clear = root_background(document, self.interaction).unwrap_or(Color::BLACK);
        let pixel = unsafe { self.pixel(clear)? };
        // SAFETY: display, GC, and backbuffer are valid X11 resources owned by this app.
        unsafe { XSetForeground(self.display, self.gc, pixel) };
        // SAFETY: display and GC are valid X11 resources owned by this app.
        unsafe { XSetClipMask(self.display, self.gc, 0) };
        // SAFETY: display, GC, and backbuffer are valid X11 resources owned by this app.
        unsafe {
            XFillRectangle(
                self.display,
                self.backbuffer,
                self.gc,
                0,
                0,
                self.width,
                self.height,
            )
        };
        for command in commands {
            // SAFETY: redraw is entered only with the app's initialized X11 resources.
            unsafe { self.paint(command, document, scale)? };
        }
        // SAFETY: display and GC are valid X11 resources owned by this app.
        unsafe { XSetClipMask(self.display, self.gc, 0) };
        // SAFETY: display and GC are valid X11 resources owned by this app.
        unsafe { XSetClipOrigin(self.display, self.gc, 0, 0) };
        // SAFETY: display, window, GC, and backbuffer are valid X11 resources owned by this app.
        unsafe {
            XCopyArea(
                self.display,
                self.backbuffer,
                self.window,
                self.gc,
                0,
                0,
                self.width,
                self.height,
                0,
                0,
            )
        };
        // SAFETY: display is the valid connection owned by this app.
        unsafe { XFlush(self.display) };
        self.layout = Some(layout);
        // SAFETY: window and display are live; mask follows content/redraw.
        unsafe { self.refresh_shape_mask(document) };
        Ok(())
    }

    pub(crate) unsafe fn paint(
        &mut self,
        command: PaintCommand,
        document: &RuntimeDocument,
        scale: f32,
    ) -> Result<(), String> {
        match command {
            PaintCommand::FillRect {
                rect,
                color,
                radius,
            } => {
                let rect = scale_rect(rect, scale);
                let radius = radius * scale;
                if color.a < 255 {
                    if let Some(xrender) = self.xrender.as_mut() {
                        // SAFETY: same initialized backbuffer/display contract as `paint`.
                        unsafe { xrender.fill_rounded_rect(rect, radius, color)? };
                    } else {
                        // SAFETY: the app's display and colormap are initialized for this draw.
                        let pixel = unsafe { self.pixel(color)? };
                        // SAFETY: display and GC are valid X11 resources owned by this app.
                        unsafe { XSetForeground(self.display, self.gc, pixel) };
                        // SAFETY: the app's display, GC, and backbuffer are valid for this draw.
                        unsafe { self.fill_rounded_rect(rect, radius) };
                    }
                } else {
                    // SAFETY: the app's display and colormap are initialized for this draw.
                    let pixel = unsafe { self.pixel(color)? };
                    // SAFETY: display and GC are valid X11 resources owned by this app.
                    unsafe { XSetForeground(self.display, self.gc, pixel) };
                    // SAFETY: the app's display, GC, and backbuffer are valid for this draw.
                    unsafe { self.fill_rounded_rect(rect, radius) };
                }
            }
            PaintCommand::StrokeRect {
                rect,
                color,
                width,
                radius,
            } => {
                let rect = scale_rect(rect, scale);
                let width = width * scale;
                let radius = radius * scale;
                if color.a < 255 {
                    if let Some(xrender) = self.xrender.as_mut() {
                        // SAFETY: same initialized backbuffer/display contract as `paint`.
                        unsafe { xrender.stroke_rounded_rect(rect, radius, width, color)? };
                    } else {
                        // SAFETY: the app's display and colormap are initialized for this draw.
                        let pixel = unsafe { self.pixel(color)? };
                        // SAFETY: display and GC are valid X11 resources owned by this app.
                        unsafe { XSetForeground(self.display, self.gc, pixel) };
                        let repeats = width.round().clamp(1.0, 8.0) as i32;
                        for inset in 0..repeats {
                            // SAFETY: the app's display, GC, and backbuffer are valid for this draw.
                            unsafe { self.stroke_rounded_rect(rect, radius, inset) };
                        }
                    }
                } else {
                    // SAFETY: the app's display and colormap are initialized for this draw.
                    let pixel = unsafe { self.pixel(color)? };
                    // SAFETY: display and GC are valid X11 resources owned by this app.
                    unsafe { XSetForeground(self.display, self.gc, pixel) };
                    let repeats = width.round().clamp(1.0, 8.0) as i32;
                    for inset in 0..repeats {
                        // SAFETY: the app's display, GC, and backbuffer are valid for this draw.
                        unsafe { self.stroke_rounded_rect(rect, radius, inset) };
                    }
                }
            }
            PaintCommand::Image { rect, asset } => {
                let rect = scale_rect(rect, scale);
                let Some(image) = document.document.assets.get(asset as usize) else {
                    return Err(format!("paint references missing image asset {asset}"));
                };
                let width = rect.width.round().max(1.0) as u32;
                let height = rect.height.round().max(1.0) as u32;
                // SAFETY: image_pixmap uses the app's initialized X11 resources and cache.
                let cached = unsafe {
                    self.image_pixmap(asset, image, document.image_revision(asset), width, height)?
                };
                let dx = rect.x.round() as i32;
                let dy = rect.y.round() as i32;
                // True-alpha path: depth-32 premultiplied pixmap via XRender
                // PictOpOver (mask stays 0). Degraded 1-bit clip mask only
                // when XRender is unavailable (logged once at upload).
                if self.xrender.is_some() {
                    if let Some(xrender) = self.xrender.as_mut() {
                        // SAFETY: same initialized backbuffer/display contract as `paint`.
                        unsafe { xrender.blit_argb32_over(cached.pixmap, dx, dy, width, height)? };
                        return Ok(());
                    }
                }
                if cached.mask != 0 {
                    // SAFETY: display and GC are valid X11 resources owned by this app.
                    unsafe { XSetClipOrigin(self.display, self.gc, dx, dy) };
                    // SAFETY: display and GC are valid X11 resources owned by this app.
                    unsafe { XSetClipMask(self.display, self.gc, cached.mask) };
                }
                // SAFETY: cached pixmap, backbuffer, display, and GC are valid X11 resources.
                unsafe {
                    XCopyArea(
                        self.display,
                        cached.pixmap,
                        self.backbuffer,
                        self.gc,
                        0,
                        0,
                        width,
                        height,
                        dx,
                        dy,
                    )
                };
                if cached.mask != 0 {
                    // SAFETY: display and GC are valid X11 resources owned by this app.
                    unsafe { XSetClipMask(self.display, self.gc, 0) };
                    // SAFETY: display and GC are valid X11 resources owned by this app.
                    unsafe { XSetClipOrigin(self.display, self.gc, 0, 0) };
                }
            }
            PaintCommand::PushClip { rect } => {
                let rect = scale_rect(rect, scale);
                self.clip_stack.push(rect);
                unsafe { self.apply_clip_stack() };
            }
            PaintCommand::PopClip => {
                self.clip_stack.pop();
                unsafe { self.apply_clip_stack() };
            }
            PaintCommand::ScrollbarTrack { rect, .. } => {
                let rect = scale_rect(rect, scale);
                let pixel = unsafe { self.pixel(Color::rgb(48, 49, 52))? };
                unsafe { XSetForeground(self.display, self.gc, pixel) };
                unsafe {
                    XFillRectangle(
                        self.display,
                        self.backbuffer,
                        self.gc,
                        rect.x.round() as i32,
                        rect.y.round() as i32,
                        rect.width.round().max(1.0) as u32,
                        rect.height.round().max(1.0) as u32,
                    )
                };
            }
            PaintCommand::ScrollbarThumb { rect, .. } => {
                let rect = scale_rect(rect, scale);
                let pixel = unsafe { self.pixel(Color::rgb(120, 123, 128))? };
                unsafe { XSetForeground(self.display, self.gc, pixel) };
                unsafe {
                    XFillRectangle(
                        self.display,
                        self.backbuffer,
                        self.gc,
                        rect.x.round() as i32,
                        rect.y.round() as i32,
                        rect.width.round().max(1.0) as u32,
                        rect.height.round().max(1.0) as u32,
                    )
                };
            }
            PaintCommand::Text {
                x,
                y,
                color,
                size,
                weight,
                text,
            } => {
                let x = x * scale;
                let y = y * scale;
                let size = size * scale;
                if let Some(xft) = self.xft.as_mut() {
                    // SAFETY: same initialized backbuffer/display contract as `paint`.
                    unsafe { xft.set_preferred_family(document.ui_font_family()) };
                    // SAFETY: same initialized backbuffer/display contract as `paint`.
                    unsafe { xft.draw_text(x, y, color, size, weight, &text)? };
                } else {
                    // SAFETY: the app's display and colormap are initialized for this draw.
                    let pixel = unsafe { self.pixel(color)? };
                    // SAFETY: display and GC are valid X11 resources owned by this app.
                    unsafe { XSetForeground(self.display, self.gc, pixel) };
                    // SAFETY: font pointers are loaded and retained by X11App for its lifetime.
                    if let Some(font) = unsafe { self.font_for_size(size) } {
                        // SAFETY: display and GC are valid X11 resources owned by this app.
                        unsafe { XSetFont(self.display, self.gc, font) };
                    }
                    let ascii: String = text
                        .chars()
                        .map(|ch| if ch.is_ascii() { ch } else { '?' })
                        .collect();
                    let string =
                        CString::new(ascii).map_err(|_| "text contains NUL".to_string())?;
                    let len = string.as_bytes().len().min(i32::MAX as usize) as i32;
                    // SAFETY: display, backbuffer, and GC are valid; string remains alive for the call.
                    unsafe {
                        XDrawString(
                            self.display,
                            self.backbuffer,
                            self.gc,
                            x.round() as i32,
                            y.round() as i32,
                            string.as_ptr(),
                            len,
                        )
                    };
                }
            }
        }
        Ok(())
    }
}
