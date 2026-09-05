#![allow(non_camel_case_types)]

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::{c_char, c_int, c_ulong, c_void};

use flamewm_render_core::{Color, Rect};

use crate::xlib::{Display, Drawable, Visual};

const RTLD_NOW: c_int = 2;
const PICT_OP_OVER: c_int = 3;

type Picture = c_ulong;

#[repr(C)]
struct XRenderPictFormat {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XRenderColor {
    red: u16,
    green: u16,
    blue: u16,
    alpha: u16,
}

type FindVisualFormatFn = unsafe extern "C" fn(*mut Display, *mut Visual) -> *mut XRenderPictFormat;
type CreatePictureFn = unsafe extern "C" fn(
    *mut Display,
    Drawable,
    *mut XRenderPictFormat,
    c_ulong,
    *const c_void,
) -> Picture;
type CreateSolidFillFn = unsafe extern "C" fn(*mut Display, *const XRenderColor) -> Picture;
type CompositeFn = unsafe extern "C" fn(
    *mut Display,
    c_int,
    Picture,
    Picture,
    Picture,
    c_int,
    c_int,
    c_int,
    c_int,
    c_int,
    c_int,
    u32,
    u32,
);
type FreePictureFn = unsafe extern "C" fn(*mut Display, Picture);

#[derive(Clone, Copy)]
struct XRenderApi {
    find_visual_format: FindVisualFormatFn,
    create_picture: CreatePictureFn,
    create_solid_fill: CreateSolidFillFn,
    composite: CompositeFn,
    free_picture: FreePictureFn,
}

struct DynamicLibrary {
    handle: *mut c_void,
}

impl DynamicLibrary {
    unsafe fn open(names: &[&str]) -> Result<Self, String> {
        for name in names {
            let c_name = CString::new(*name).expect("library name is static and NUL-free");
            let handle = dlopen(c_name.as_ptr(), RTLD_NOW);
            if !handle.is_null() {
                return Ok(Self { handle });
            }
        }
        Err(format!("unable to load any of: {}", names.join(", ")))
    }

    unsafe fn symbol<T: Copy>(&self, name: &'static [u8]) -> Result<T, String> {
        debug_assert_eq!(name.last().copied(), Some(0));
        dlerror();
        let raw = dlsym(self.handle, name.as_ptr() as *const c_char);
        let error = dlerror();
        if raw.is_null() || !error.is_null() {
            let message = if error.is_null() {
                "symbol resolved to NULL".to_string()
            } else {
                CStr::from_ptr(error).to_string_lossy().into_owned()
            };
            return Err(format!(
                "{}: {message}",
                String::from_utf8_lossy(&name[..name.len() - 1])
            ));
        }
        if mem::size_of::<T>() != mem::size_of::<*mut c_void>() {
            return Err("dynamic function pointer has unexpected size".to_string());
        }
        Ok(mem::transmute_copy(&raw))
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                dlclose(self.handle);
            }
        }
    }
}

pub struct XRenderBackend {
    display: *mut Display,
    destination: Picture,
    format: *mut XRenderPictFormat,
    api: XRenderApi,
    _library: DynamicLibrary,
    solids: HashMap<Color, Picture>,
}

impl XRenderBackend {
    pub unsafe fn new(
        display: *mut Display,
        drawable: Drawable,
        visual: *mut Visual,
    ) -> Result<Self, String> {
        let library = DynamicLibrary::open(&["libXrender.so.1", "libXrender.so"])?;
        let api = XRenderApi {
            find_visual_format: library.symbol(b"XRenderFindVisualFormat\0")?,
            create_picture: library.symbol(b"XRenderCreatePicture\0")?,
            create_solid_fill: library.symbol(b"XRenderCreateSolidFill\0")?,
            composite: library.symbol(b"XRenderComposite\0")?,
            free_picture: library.symbol(b"XRenderFreePicture\0")?,
        };
        let format = (api.find_visual_format)(display, visual);
        if format.is_null() {
            return Err("XRenderFindVisualFormat returned NULL".to_string());
        }
        let destination = (api.create_picture)(display, drawable, format, 0, std::ptr::null());
        if destination == 0 {
            return Err("XRenderCreatePicture returned 0".to_string());
        }
        Ok(Self {
            display,
            destination,
            format,
            api,
            _library: library,
            solids: HashMap::new(),
        })
    }

    pub unsafe fn set_drawable(&mut self, drawable: Drawable) -> Result<(), String> {
        let replacement =
            (self.api.create_picture)(self.display, drawable, self.format, 0, std::ptr::null());
        if replacement == 0 {
            return Err("XRenderCreatePicture returned 0 while changing drawable".to_string());
        }
        if self.destination != 0 {
            (self.api.free_picture)(self.display, self.destination);
        }
        self.destination = replacement;
        Ok(())
    }

    pub unsafe fn fill_rounded_rect(
        &mut self,
        rect: Rect,
        radius: f32,
        color: Color,
    ) -> Result<(), String> {
        let x = rect.x.round() as i32;
        let y = rect.y.round() as i32;
        let width = rect.width.round().max(0.0) as u32;
        let height = rect.height.round().max(0.0) as u32;
        if width == 0 || height == 0 || color.a == 0 {
            return Ok(());
        }
        let radius = radius.round().max(0.0).min((width.min(height) / 2) as f32) as u32;
        if radius <= 1 {
            return self.fill_rect(x, y, width, height, color);
        }

        // Paint each destination pixel at most once. With PictOpOver, overlapping
        // translucent rectangles compound alpha and create bright vertical/horizontal
        // strips. Keep the large middle band as one request, then rasterize only the
        // rounded corner rows. This also avoids one XRender request per pixel row on
        // large selection/snap rectangles during pointer motion.
        let middle_height = height.saturating_sub(radius * 2);
        if middle_height > 0 {
            self.fill_rect(x, y + radius as i32, width, middle_height, color)?;
        }
        for row in 0..radius {
            let inset = rounded_row_inset(width, height, radius, row);
            let span = width.saturating_sub(inset * 2);
            if span == 0 {
                continue;
            }
            self.fill_rect(x + inset as i32, y + row as i32, span, 1, color)?;
            let bottom_row = height - 1 - row;
            if bottom_row != row {
                self.fill_rect(x + inset as i32, y + bottom_row as i32, span, 1, color)?;
            }
        }
        Ok(())
    }

    pub unsafe fn stroke_rounded_rect(
        &mut self,
        rect: Rect,
        radius: f32,
        stroke_width: f32,
        color: Color,
    ) -> Result<(), String> {
        let x = rect.x.round() as i32;
        let y = rect.y.round() as i32;
        let width = rect.width.round().max(0.0) as u32;
        let height = rect.height.round().max(0.0) as u32;
        if width == 0 || height == 0 || color.a == 0 {
            return Ok(());
        }
        let thickness = stroke_width.round().clamp(1.0, 8.0) as u32;
        let thickness = thickness.min(width / 2).min(height / 2).max(1);
        let radius = radius.round().max(0.0).min((width.min(height) / 2) as f32) as u32;

        for row in 0..height {
            let outer_inset = rounded_row_inset(width, height, radius, row);
            let outer_left = outer_inset;
            let outer_right = width.saturating_sub(outer_inset);
            if outer_right <= outer_left {
                continue;
            }

            if row < thickness
                || row >= height.saturating_sub(thickness)
                || width <= thickness * 2
                || height <= thickness * 2
            {
                self.fill_rect(
                    x + outer_left as i32,
                    y + row as i32,
                    outer_right - outer_left,
                    1,
                    color,
                )?;
                continue;
            }

            let inner_w = width - thickness * 2;
            let inner_h = height - thickness * 2;
            let inner_row = row - thickness;
            let inner_radius = radius.saturating_sub(thickness);
            let inner_inset = rounded_row_inset(inner_w, inner_h, inner_radius, inner_row);
            let inner_left = thickness + inner_inset;
            let inner_right = width.saturating_sub(thickness + inner_inset);

            if inner_left > outer_left {
                self.fill_rect(
                    x + outer_left as i32,
                    y + row as i32,
                    inner_left - outer_left,
                    1,
                    color,
                )?;
            }
            if outer_right > inner_right {
                self.fill_rect(
                    x + inner_right as i32,
                    y + row as i32,
                    outer_right - inner_right,
                    1,
                    color,
                )?;
            }
        }
        Ok(())
    }

    unsafe fn fill_rect(
        &mut self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        color: Color,
    ) -> Result<(), String> {
        let source = self.solid(color)?;
        (self.api.composite)(
            self.display,
            PICT_OP_OVER,
            source,
            0,
            self.destination,
            0,
            0,
            0,
            0,
            x,
            y,
            width,
            height,
        );
        Ok(())
    }

    unsafe fn solid(&mut self, color: Color) -> Result<Picture, String> {
        if let Some(picture) = self.solids.get(&color).copied() {
            return Ok(picture);
        }
        // Render protocol colors are premultiplied by alpha. Passing full RGB
        // channels with a low alpha makes translucent overlays appear far too
        // bright because PictOpOver treats the source channels as already
        // premultiplied. Convert 8-bit straight-alpha Color into the 16-bit
        // premultiplied representation expected by XRender.
        let value = XRenderColor {
            red: premultiplied_u16(color.r, color.a),
            green: premultiplied_u16(color.g, color.a),
            blue: premultiplied_u16(color.b, color.a),
            alpha: u16::from(color.a) * 257,
        };
        let picture = (self.api.create_solid_fill)(self.display, &value);
        if picture == 0 {
            return Err("XRenderCreateSolidFill returned 0".to_string());
        }
        self.solids.insert(color, picture);
        Ok(picture)
    }
}

fn rounded_row_inset(width: u32, height: u32, radius: u32, row: u32) -> u32 {
    if radius <= 1 || width == 0 || height == 0 {
        return 0;
    }
    let corner_row = row.min(height.saturating_sub(1).saturating_sub(row));
    if corner_row >= radius {
        return 0;
    }
    let r = radius as f64;
    let dy = r - (corner_row as f64 + 0.5);
    let dx = (r * r - dy * dy).max(0.0).sqrt();
    (r - dx).floor().clamp(0.0, r) as u32
}

fn premultiplied_u16(channel: u8, alpha: u8) -> u16 {
    let straight = u32::from(channel) * 257;
    let a = u32::from(alpha) * 257;
    ((straight * a + 32767) / 65535) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xrender_solid_channels_are_premultiplied() {
        assert_eq!(premultiplied_u16(255, 51), 13107);
    }

    #[test]
    fn opaque_channel_is_unchanged() {
        assert_eq!(premultiplied_u16(200, 255), 51400);
    }
}

impl Drop for XRenderBackend {
    fn drop(&mut self) {
        unsafe {
            for (_, picture) in self.solids.drain() {
                (self.api.free_picture)(self.display, picture);
            }
            if self.destination != 0 {
                (self.api.free_picture)(self.display, self.destination);
                self.destination = 0;
            }
        }
    }
}

#[link(name = "dl")]
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}
