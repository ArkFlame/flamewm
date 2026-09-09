//! Image cache {pixmap,picture,mask} keyed by {node,asset,revision,w,h,treatment,foreground}.
//! SymbolicForeground premul math: eff_a=round(sa*fa/255),
//! premul=round(fg*eff_a/255). No feature-specific icon branches.
//! Runtime per-node overrides are never keyed by compiled asset_id alone:
//! the node index plus the node-local revision are part of every key.

use std::sync::Once;

use super::color::rgb_to_pixel;
use super::*;
use flamewm_render_core::ImageTreatment;

static MASK_LOG_ONCE: Once = Once::new();

fn log_degraded_mask_once(asset_id: u16) {
    MASK_LOG_ONCE.call_once(|| {
        eprintln!(
            "FLAMEWM_RENDER_IMAGE_ALPHA degraded-clip-mask asset={asset_id} reason=xrender-unavailable"
        );
    });
}

pub(crate) fn image_cache_key(
    node: u32,
    asset_id: u16,
    width: u32,
    height: u32,
    revision: u64,
    treatment: ImageTreatment,
    fg: Color,
) -> ImageCacheKey {
    let treatment_byte = match treatment {
        ImageTreatment::Original => 0u8,
        ImageTreatment::SymbolicForeground => 1u8,
    };
    ImageCacheKey {
        node,
        asset: asset_id,
        width,
        height,
        revision,
        treatment: treatment_byte,
        fg_r: fg.r,
        fg_g: fg.g,
        fg_b: fg.b,
        fg_a: fg.a,
    }
}

impl X11App {
    pub(super) unsafe fn image_pixmap(
        &mut self,
        node: u32,
        asset_id: u16,
        pixels: &[u8],
        src_width: u32,
        src_height: u32,
        revision: u64,
        width: u32,
        height: u32,
        treatment: ImageTreatment,
        tint: Option<Color>,
    ) -> Result<CachedImage, String> {
        let fg = tint.unwrap_or(Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        });
        let key = image_cache_key(node, asset_id, width, height, revision, treatment, fg);
        if let Some(cached) = self.images.get(&key).copied() {
            return Ok(cached);
        }
        let stale_keys: Vec<ImageCacheKey> = self
            .images
            .keys()
            .filter(|cached| {
                cached.node == node && cached.asset == asset_id && cached.revision != revision
            })
            .copied()
            .collect();
        for stale_key in stale_keys {
            if let Some(stale) = self.images.remove(&stale_key) {
                unsafe { free_cached_image(self.display, stale) };
            }
        }
        let argb = match treatment {
            ImageTreatment::Original => crate::native::image::scale_and_premultiply(
                pixels, src_width, src_height, width, height,
            ),
            ImageTreatment::SymbolicForeground => {
                super::symbolic_foreground_argb32(pixels, src_width, src_height, width, height, fg)
            }
        };
        if self.xrender.is_some() {
            // True-alpha path: 32-bit TrueColor visual pixmap + XPutImage of
            // ARGB32 words. XCreatePixmap against the (24-bit default) window
            // with depth 32 has no matching visual and XPutImage then dies
            // with BadMatch, so match a 32-bit TrueColor visual explicitly.
            let root = unsafe { XRootWindow(self.display, self.screen) };
            let mut vinfo: XVisualInfo = unsafe { std::mem::zeroed() };
            let matched = unsafe {
                XMatchVisualInfo(self.display, self.screen, 32, TRUE_COLOR_CLASS, &mut vinfo)
            };
            if matched != 0 && !vinfo.visual.is_null() {
                let pixmap = unsafe { XCreatePixmap(self.display, root, width, height, 32) };
                if pixmap == 0 {
                    return Err(format!("XCreatePixmap failed for image asset {asset_id}"));
                }
                if let Err(error) =
                    unsafe { self.upload_argb32_words(pixmap, vinfo.visual, &argb, width, height) }
                {
                    unsafe { XFreePixmap(self.display, pixmap) };
                    return Err(error);
                }
                let cached = CachedImage {
                    pixmap,
                    picture: 1,
                    mask: 0,
                };
                self.images.insert(key, cached);
                return Ok(cached);
            }
            eprintln!(
                "FLAMEWM_RENDER_IMAGE_ALPHA degraded-clip-mask asset={asset_id} reason=no-32bit-visual"
            );
        }
        // Degraded path (no XRender): depth pixmap + 1-bit threshold mask.
        log_degraded_mask_once(asset_id);
        let pixmap =
            unsafe { XCreatePixmap(self.display, self.window, width, height, self.depth as u32) };
        if pixmap == 0 {
            return Err(format!("XCreatePixmap failed for image asset {asset_id}"));
        }
        if let Err(error) = unsafe {
            self.upload_opaque_fallback(pixmap, pixels, src_width, src_height, width, height)
        } {
            unsafe { XFreePixmap(self.display, pixmap) };
            return Err(error);
        }
        let mask = unsafe {
            self.build_threshold_mask(pixels, src_width, src_height, width, height, asset_id)?
        };
        let cached = CachedImage {
            pixmap,
            picture: 0,
            mask,
        };
        self.images.insert(key, cached);
        Ok(cached)
    }

    pub(crate) unsafe fn upload_argb32_words(
        &mut self,
        pixmap: Pixmap,
        visual: *mut Visual,
        argb: &[u32],
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        if visual.is_null() {
            return Err("ARGB32 visual is NULL".to_string());
        }
        let ximage = unsafe {
            XCreateImage(
                self.display,
                visual,
                32,
                ZPIXMAP,
                0,
                ptr::null_mut(),
                width,
                height,
                32,
                0,
            )
        };
        if ximage.is_null() {
            return Err("XCreateImage failed for ARGB32 upload".to_string());
        }
        let head = unsafe { &mut *(ximage as *mut XImageHead) };
        if head.bytes_per_line <= 0 {
            unsafe { XDestroyImage(ximage) };
            return Err("XCreateImage produced invalid bytes_per_line".to_string());
        }
        let total = (head.bytes_per_line as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| "XImage buffer size overflow".to_string())?;
        let data = unsafe { malloc(total) };
        if data.is_null() {
            unsafe { XDestroyImage(ximage) };
            return Err(format!("malloc failed for {total}-byte XImage"));
        }
        unsafe { ptr::write_bytes(data as *mut u8, 0, total) };
        head.data = data as *mut _;
        let bpl_words = head.bytes_per_line as usize / 4;
        let raw = unsafe { std::slice::from_raw_parts_mut(data as *mut u32, total / 4) };
        for y in 0..height as usize {
            let dst_off = y * bpl_words;
            let src_off = y * width as usize;
            raw[dst_off..dst_off + width as usize]
                .copy_from_slice(&argb[src_off..src_off + width as usize]);
        }
        unsafe {
            // The app GC is bound to the (24-bit default) window drawable and
            // cannot target a depth-32 pixmap (BadMatch). Use a transient GC
            // created against the pixmap itself.
            let pixmap_gc = XCreateGC(self.display, pixmap, 0, ptr::null_mut());
            if pixmap_gc.is_null() {
                XDestroyImage(ximage);
                return Err("XCreateGC failed for ARGB32 upload".to_string());
            }
            XPutImage(
                self.display,
                pixmap,
                pixmap_gc,
                ximage,
                0,
                0,
                0,
                0,
                width,
                height,
            );
            XFreeGC(self.display, pixmap_gc);
        };
        unsafe { XDestroyImage(ximage) };
        Ok(())
    }

    unsafe fn upload_opaque_fallback(
        &mut self,
        pixmap: Pixmap,
        pixels: &[u8],
        src_width: u32,
        src_height: u32,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        let visual = unsafe { XDefaultVisual(self.display, self.screen) };
        if visual.is_null() {
            return Err("XDefaultVisual returned NULL".to_string());
        }
        let ximage = unsafe {
            XCreateImage(
                self.display,
                visual,
                self.depth as u32,
                ZPIXMAP,
                0,
                ptr::null_mut(),
                width,
                height,
                32,
                0,
            )
        };
        if ximage.is_null() {
            return Err("XCreateImage failed for image fallback".to_string());
        }
        let head = unsafe { &mut *(ximage as *mut XImageHead) };
        if head.bytes_per_line <= 0 {
            unsafe { XDestroyImage(ximage) };
            return Err("XCreateImage produced invalid bytes_per_line".to_string());
        }
        let total = (head.bytes_per_line as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| "XImage buffer size overflow".to_string())?;
        let data = unsafe { malloc(total) };
        if data.is_null() {
            unsafe { XDestroyImage(ximage) };
            return Err(format!("malloc failed for {total}-byte XImage"));
        }
        unsafe { ptr::write_bytes(data as *mut u8, 0, total) };
        head.data = data as *mut _;
        for y in 0..height {
            let sy = ((y as u64 * src_height as u64) / height as u64) as u32;
            for x in 0..width {
                let sx = ((x as u64 * src_width as u64) / width as u64) as u32;
                let offset = ((sy as usize * src_width as usize) + sx as usize) * 4;
                let pixel = rgb_to_pixel(
                    pixels[offset],
                    pixels[offset + 1],
                    pixels[offset + 2],
                    head.red_mask as u64,
                    head.green_mask as u64,
                    head.blue_mask as u64,
                );
                unsafe { XPutPixel(ximage, x as i32, y as i32, pixel as _) };
            }
        }
        unsafe {
            XPutImage(
                self.display,
                pixmap,
                self.gc,
                ximage,
                0,
                0,
                0,
                0,
                width,
                height,
            )
        };
        unsafe { XDestroyImage(ximage) };
        Ok(())
    }

    unsafe fn build_threshold_mask(
        &mut self,
        pixels: &[u8],
        src_width: u32,
        src_height: u32,
        width: u32,
        height: u32,
        asset_id: u16,
    ) -> Result<Pixmap, String> {
        let mut transparent = vec![false; (width as usize) * (height as usize)];
        for y in 0..height {
            let sy = ((y as u64 * src_height as u64) / height as u64) as u32;
            for x in 0..width {
                let sx = ((x as u64 * src_width as u64) / width as u64) as u32;
                let a = pixels[((sy as usize * src_width as usize) + sx as usize) * 4 + 3];
                transparent[(y as usize) * (width as usize) + x as usize] = a < 128;
            }
        }
        if !transparent.iter().any(|t| *t) {
            return Ok(0);
        }
        let mask = unsafe { XCreatePixmap(self.display, self.window, width, height, 1) };
        if mask == 0 {
            return Err(format!(
                "XCreatePixmap failed for transparency mask asset {asset_id}"
            ));
        }
        let mask_gc = unsafe { XCreateGC(self.display, mask, 0, ptr::null_mut()) };
        if mask_gc.is_null() {
            unsafe { XFreePixmap(self.display, mask) };
            return Err(format!(
                "XCreateGC failed for transparency mask asset {asset_id}"
            ));
        }
        unsafe { XSetForeground(self.display, mask_gc, 0) };
        unsafe { XFillRectangle(self.display, mask, mask_gc, 0, 0, width, height) };
        unsafe { XSetForeground(self.display, mask_gc, 1) };
        for y in 0..height {
            let mut x = 0u32;
            while x < width {
                while x < width && transparent[(y as usize) * (width as usize) + x as usize] {
                    x += 1;
                }
                let start = x;
                while x < width && !transparent[(y as usize) * (width as usize) + x as usize] {
                    x += 1;
                }
                if x > start {
                    unsafe {
                        XFillRectangle(
                            self.display,
                            mask,
                            mask_gc,
                            start as i32,
                            y as i32,
                            x - start,
                            1,
                        )
                    };
                }
            }
        }
        unsafe { XFreeGC(self.display, mask_gc) };
        Ok(mask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_node_overrides_never_collide_on_asset_alone() {
        let fg = Color::rgb(255, 255, 255);
        let a = image_cache_key(0, 0, 10, 10, 1, ImageTreatment::Original, fg);
        let b = image_cache_key(1, 0, 10, 10, 1, ImageTreatment::Original, fg);
        assert_ne!(a, b, "sibling nodes sharing asset must key separately");
        let a2 = image_cache_key(0, 0, 10, 10, 2, ImageTreatment::Original, fg);
        assert_ne!(a, a2, "revision advance must change the key");
        let a_resized = image_cache_key(0, 0, 12, 10, 2, ImageTreatment::Original, fg);
        assert_ne!(a2, a_resized, "size is part of the key");
        let a_tinted = image_cache_key(
            0,
            0,
            12,
            10,
            2,
            ImageTreatment::SymbolicForeground,
            Color::rgb(10, 20, 30),
        );
        assert_ne!(a_resized, a_tinted, "treatment+palette are part of the key");
    }
}
