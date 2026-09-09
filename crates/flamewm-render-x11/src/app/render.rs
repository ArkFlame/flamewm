use super::primitives::scale_rect;
use super::*;
use crate::native::coverage::build_coverage;

/// Fingerprint of retained scroll state for the paint cache key.
/// Revision alone is not enough: scroll writes are light-touch.
fn scroll_fingerprint(document: &RuntimeDocument) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let mut mix = |word: u64| {
        hash ^= word;
        hash = hash.wrapping_mul(0x100000001b3);
    };
    mix(document.revision());
    // Revision covers content edits; scroll offsets are hashed directly by
    // sampling node-local state through the public scroll API is not
    // possible without an index list, so the paint path passes offsets via
    // `build_paint_commands_with_scroll` and the cache key includes the
    // interaction state plus revision here. Full scroll-list hashing lives
    // in the document-owned mark path (scroll writes mark Full).
    hash
}

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

    #[allow(dead_code)]
    pub(crate) unsafe fn font_for_size_pub(&self, size: f32) -> Option<Font> {
        unsafe { self.font_for_size(size) }
    }

    #[allow(dead_code)]
    pub(crate) unsafe fn pixel_pub(&mut self, color: Color) -> Result<u64, String> {
        unsafe { self.pixel(color) }
    }
    #[allow(dead_code)]
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

    #[allow(dead_code)]
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
        let layout_guard = flamewm_profiler::start("render.layout");
        let scale = document.ui_scale();
        let viewport = (self.width, self.height);
        let reuse_layout = self.layout.as_ref().is_some_and(|cached| {
            cached.revision == document.revision() && self.cached_viewport == viewport
        });
        if !reuse_layout {
            let layout = LayoutEngine::compute(
                document,
                self.width as f32 / scale,
                self.height as f32 / scale,
                self.interaction,
            );
            self.layout = Some(layout);
            self.cached_viewport = viewport;
        }
        drop(layout_guard);
        let paint_guard = flamewm_profiler::start("render.paint");
        let scroll_fp = scroll_fingerprint(document);
        let cache_valid = self.cached_paint.as_ref().is_some_and(|cached| {
            cached.revision == document.revision()
                && cached.interaction == self.interaction
                && cached.scroll_fingerprint == scroll_fp
                && reuse_layout
        });
        if !cache_valid {
            let layout = self.layout.as_ref().expect("layout retained above");
            let fresh =
                build_paint_commands_with_scroll(document, layout, self.interaction, &|index| {
                    document.scroll_offset(index)
                });
            self.cached_paint = Some(CachedPaint {
                commands: fresh,
                revision: document.revision(),
                interaction: self.interaction,
                scroll_fingerprint: scroll_fp,
            });
        }
        drop(paint_guard);
        let _present_guard = flamewm_profiler::start("render.present");
        self.clear_scene()?;
        // Borrow retained commands by index: build fresh only when cache
        // invalid; single coverage/shape per frame; no clone path.
        let count = self
            .cached_paint
            .as_ref()
            .expect("paint cached above")
            .commands
            .len();
        for index in 0..count {
            // Borrow retained command without cloning the vector or the
            // item: raw pointer scoped to one iteration; paint takes &.
            let command: &PaintCommand = unsafe {
                let base = self
                    .cached_paint
                    .as_ref()
                    .expect("paint cached above")
                    .commands
                    .as_ptr();
                &*base.add(index)
            };
            // SAFETY: redraw is entered only with the app's initialized X11 resources.
            unsafe { self.paint(command, document, scale)? };
        }
        unsafe { self.present_scene()? };
        // Single shape per changed frame, from retained commands.
        let cached = self.cached_paint.as_ref().expect("paint cached above");
        unsafe { self.refresh_shape_mask_from_coverage(&cached.commands) };
        Ok(())
    }

    pub(crate) unsafe fn present_retained(
        &mut self,
        document: &RuntimeDocument,
    ) -> Result<(), String> {
        let _guard = flamewm_profiler::start("render.present");
        if self.layout.is_some() {
            unsafe { self.present_scene()? };
            unsafe { XFlush(self.display) };
            return Ok(());
        }
        unsafe { self.redraw(document) }
    }

    /// Mark damage without painting: merged (max coverage) latch consumed
    /// by `redraw_if_dirty`. Returns false when damage is empty.
    pub(crate) fn mark_dirty(&mut self, damage: SurfaceDamage) {
        self.pending_damage = self.pending_damage.merge(damage);
    }

    pub(crate) fn mark_full(&mut self) {
        self.pending_damage = SurfaceDamage::Full;
    }

    /// Paint only when damage is latched or the document revision moved
    /// past the retained cache; otherwise skip the frame.
    pub(crate) unsafe fn redraw_if_dirty(
        &mut self,
        document: &mut RuntimeDocument,
    ) -> Result<bool, String> {
        let doc_damage = document.take_damage();
        self.pending_damage = self.pending_damage.merge(doc_damage);
        let stale = self
            .layout
            .as_ref()
            .is_none_or(|cached| cached.revision != document.revision());
        if self.pending_damage.is_empty() && !stale {
            return Ok(false);
        }
        self.pending_damage = SurfaceDamage::None;
        unsafe { self.redraw(document)? };
        Ok(true)
    }

    /// Transparent ARGB scene clear: 0x00000000, never root_background/BLACK.
    pub(crate) fn clear_scene(&mut self) -> Result<(), String> {
        // Depth-32 ARGB scene: pixel 0 == 0x00000000 transparent clear.
        // XRender fills composite over this; the GC path fills pixel 0 too.
        unsafe { XSetForeground(self.display, self.gc, TRANSPARENT_CLEAR_PIXEL) };
        unsafe { XSetClipMask(self.display, self.gc, 0) };
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
        Ok(())
    }

    /// Present per SurfaceAlphaMode: composited/shape via backbuffer copy,
    /// opaque fallback paints flattened pixels (already flattened in pixel()).
    pub(crate) unsafe fn present_scene(&mut self) -> Result<(), String> {
        unsafe { XSetClipMask(self.display, self.gc, 0) };
        unsafe { XSetClipOrigin(self.display, self.gc, 0, 0) };
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
        unsafe { XFlush(self.display) };
        Ok(())
    }

    /// Visual-coverage shape refresh from the coverage builder.
    pub(crate) unsafe fn refresh_shape_mask_from_coverage(&self, commands: &[PaintCommand]) {
        let Some(bridge) = self.xshape.as_ref() else {
            return;
        };
        let spans = build_coverage(commands, self.width, self.height, 0.0);
        let rects: Vec<(u32, u32, u32)> = spans;
        if rects.is_empty() {
            return;
        }
        if let Err(error) = unsafe { bridge.apply_rounded_mask(self.window, &rects) } {
            eprintln!("FLAMEWM_RENDER_SHAPE_BACKEND degraded reason={error}");
        }
    }

    pub(crate) unsafe fn paint(
        &mut self,
        command: &PaintCommand,
        document: &RuntimeDocument,
        scale: f32,
    ) -> Result<(), String> {
        match command {
            PaintCommand::FillRect {
                rect,
                color,
                radius,
            } => {
                let rect = scale_rect(*rect, scale);
                let radius = *radius * scale;
                // Canonical scene target: XRender handles all alpha; the GC
                // path only runs in explicit OpaqueFallback (flattened pixel).
                if self.xrender.is_some() {
                    if let Some(xrender) = self.xrender.as_mut() {
                        // SAFETY: same initialized backbuffer/display contract as `paint`.
                        unsafe { xrender.fill_rounded_rect(rect, radius, *color)? };
                        return Ok(());
                    }
                }
                if self.alpha_mode != SurfaceAlphaMode::OpaqueFallback && color.a < 255 {
                    return Err(
                        "translucent fill without XRender outside OpaqueFallback".to_string()
                    );
                }
                {
                    // SAFETY: the app's display and colormap are initialized for this draw.
                    let pixel = unsafe { self.pixel(*color)? };
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
                let rect = scale_rect(*rect, scale);
                let width = *width * scale;
                let radius = *radius * scale;
                if self.xrender.is_some() {
                    if let Some(xrender) = self.xrender.as_mut() {
                        // SAFETY: same initialized backbuffer/display contract as `paint`.
                        unsafe { xrender.stroke_rounded_rect(rect, radius, width, *color)? };
                        return Ok(());
                    }
                }
                if self.alpha_mode != SurfaceAlphaMode::OpaqueFallback && color.a < 255 {
                    return Err(
                        "translucent stroke without XRender outside OpaqueFallback".to_string()
                    );
                }
                {
                    // SAFETY: the app's display and colormap are initialized for this draw.
                    let pixel = unsafe { self.pixel(*color)? };
                    // SAFETY: display and GC are valid X11 resources owned by this app.
                    unsafe { XSetForeground(self.display, self.gc, pixel) };
                    let repeats = width.round().clamp(1.0, 8.0) as i32;
                    for inset in 0..repeats {
                        // SAFETY: the app's display, GC, and backbuffer are valid for this draw.
                        unsafe { self.stroke_rounded_rect(rect, radius, inset) };
                    }
                }
            }
            PaintCommand::Image {
                rect,
                asset,
                node,
                revision,
                treatment,
                tint,
            } => {
                let rect = scale_rect(*rect, scale);
                let Some((compiled_asset, src_w, src_h, _, pixels)) =
                    document.image_for_node(*node)
                else {
                    return Err(format!("paint references missing image for node {node}"));
                };
                let asset_id = compiled_asset.unwrap_or(*asset);
                let expected_revision = document.image_revision_for_node(*node);
                debug_assert_eq!(
                    *revision, expected_revision,
                    "stale image paint command for node {node}"
                );
                let width = rect.width.round().max(1.0) as u32;
                let height = rect.height.round().max(1.0) as u32;
                // SAFETY: image_pixmap uses the app's initialized X11 resources and cache.
                let cached = unsafe {
                    self.image_pixmap(
                        *node,
                        asset_id,
                        pixels,
                        src_w,
                        src_h,
                        expected_revision,
                        width,
                        height,
                        *treatment,
                        *tint,
                    )?
                };
                let dx = rect.x.round() as i32;
                let dy = rect.y.round() as i32;
                // Canonical scene target: XRender/Xft paint onto the scene
                // backbuffer; composited path uses PictOpOver with true alpha.
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
                let rect = scale_rect(*rect, scale);
                self.clip_stack.push(rect);
                unsafe { self.apply_clip_stack() };
            }
            PaintCommand::PopClip => {
                self.clip_stack.pop();
                unsafe { self.apply_clip_stack() };
            }
            PaintCommand::ScrollbarTrack { rect, .. } => {
                let rect = scale_rect(*rect, scale);
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
                let rect = scale_rect(*rect, scale);
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
                let x = *x * scale;
                let y = *y * scale;
                let size = *size * scale;
                if let Some(xft) = self.xft.as_mut() {
                    // SAFETY: same initialized backbuffer/display contract as `paint`.
                    unsafe { xft.set_preferred_family(document.ui_font_family()) };
                    // SAFETY: same initialized backbuffer/display contract as `paint`.
                    unsafe { xft.draw_text(x, y, *color, size, *weight, text)? };
                } else {
                    // SAFETY: the app's display and colormap are initialized for this draw.
                    let pixel = unsafe { self.pixel(*color)? };
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
