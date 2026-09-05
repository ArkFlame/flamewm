#![allow(non_camel_case_types)]

use std::collections::HashMap;
use std::env;
use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::{c_char, c_int, c_uchar, c_ulong, c_void};
use std::path::Path;
use std::ptr;

use rustwebrender_core::Color;

use crate::xlib::{Colormap, Display, Drawable, Visual};

const RTLD_NOW: c_int = 2;

#[repr(C)]
struct XftDraw {
    _private: [u8; 0],
}

#[repr(C)]
struct XftFont {
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

#[repr(C)]
#[derive(Clone, Copy)]
struct XftColor {
    pixel: c_ulong,
    color: XRenderColor,
}

#[repr(C)]
struct FcConfig {
    _private: [u8; 0],
}

type XftDrawCreateFn = unsafe extern "C" fn(*mut Display, Drawable, *mut Visual, Colormap) -> *mut XftDraw;
type XftDrawDestroyFn = unsafe extern "C" fn(*mut XftDraw);
type XftDrawChangeFn = unsafe extern "C" fn(*mut XftDraw, Drawable);
type XftDrawStringUtf8Fn = unsafe extern "C" fn(*mut XftDraw, *const XftColor, *mut XftFont, c_int, c_int, *const c_uchar, c_int);
type XftFontOpenNameFn = unsafe extern "C" fn(*mut Display, c_int, *const c_char) -> *mut XftFont;
type XftFontCloseFn = unsafe extern "C" fn(*mut Display, *mut XftFont);
type XftColorAllocValueFn = unsafe extern "C" fn(*mut Display, *mut Visual, Colormap, *const XRenderColor, *mut XftColor) -> c_int;
type XftColorFreeFn = unsafe extern "C" fn(*mut Display, *mut Visual, Colormap, *mut XftColor);

type FcConfigGetCurrentFn = unsafe extern "C" fn() -> *mut FcConfig;
type FcConfigAppFontAddFileFn = unsafe extern "C" fn(*mut FcConfig, *const c_uchar) -> c_int;
type FcConfigBuildFontsFn = unsafe extern "C" fn(*mut FcConfig) -> c_int;

#[derive(Clone, Copy)]
struct XftApi {
    draw_create: XftDrawCreateFn,
    draw_destroy: XftDrawDestroyFn,
    draw_change: XftDrawChangeFn,
    draw_string_utf8: XftDrawStringUtf8Fn,
    font_open_name: XftFontOpenNameFn,
    font_close: XftFontCloseFn,
    color_alloc_value: XftColorAllocValueFn,
    color_free: XftColorFreeFn,
}

#[derive(Clone, Copy)]
struct FontconfigApi {
    config_get_current: FcConfigGetCurrentFn,
    config_app_font_add_file: FcConfigAppFontAddFileFn,
    config_build_fonts: FcConfigBuildFontsFn,
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
            return Err(format!("{}: {message}", String::from_utf8_lossy(&name[..name.len() - 1])));
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct FontKey {
    pixels: u16,
    weight: u16,
}

pub struct XftBackend {
    display: *mut Display,
    screen: c_int,
    visual: *mut Visual,
    colormap: Colormap,
    draw: *mut XftDraw,
    api: XftApi,
    _xft_library: DynamicLibrary,
    _fontconfig_library: Option<DynamicLibrary>,
    fonts: HashMap<FontKey, *mut XftFont>,
    colors: HashMap<Color, XftColor>,
    preferred_family: String,
}

impl XftBackend {
    pub unsafe fn new(
        display: *mut Display,
        screen: c_int,
        drawable: Drawable,
        visual: *mut Visual,
        colormap: Colormap,
    ) -> Result<Self, String> {
        let xft_library = DynamicLibrary::open(&["libXft.so.2", "libXft.so"])?;
        let api = XftApi {
            draw_create: xft_library.symbol(b"XftDrawCreate\0")?,
            draw_destroy: xft_library.symbol(b"XftDrawDestroy\0")?,
            draw_change: xft_library.symbol(b"XftDrawChange\0")?,
            draw_string_utf8: xft_library.symbol(b"XftDrawStringUtf8\0")?,
            font_open_name: xft_library.symbol(b"XftFontOpenName\0")?,
            font_close: xft_library.symbol(b"XftFontClose\0")?,
            color_alloc_value: xft_library.symbol(b"XftColorAllocValue\0")?,
            color_free: xft_library.symbol(b"XftColorFree\0")?,
        };

        let fontconfig_library = match DynamicLibrary::open(&["libfontconfig.so.1", "libfontconfig.so"]) {
            Ok(library) => {
                let fc = FontconfigApi {
                    config_get_current: library.symbol(b"FcConfigGetCurrent\0")?,
                    config_app_font_add_file: library.symbol(b"FcConfigAppFontAddFile\0")?,
                    config_build_fonts: library.symbol(b"FcConfigBuildFonts\0")?,
                };
                register_application_fonts(fc)?;
                Some(library)
            }
            Err(error) => {
                eprintln!("RWR_FONTCONFIG_WARNING {error}; only system-visible fonts can be used");
                None
            }
        };

        let draw = (api.draw_create)(display, drawable, visual, colormap);
        if draw.is_null() {
            return Err("XftDrawCreate returned NULL".to_string());
        }

        let preferred_family = env::var("RWR_UI_FONT")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "IBM Plex Sans".to_string());

        let mut backend = Self {
            display,
            screen,
            visual,
            colormap,
            draw,
            api,
            _xft_library: xft_library,
            _fontconfig_library: fontconfig_library,
            fonts: HashMap::new(),
            colors: HashMap::new(),
            preferred_family,
        };

        // Fail early only if *no* usable Xft font exists. IBM Plex Sans remains
        // the preferred face, with conservative system fallbacks for portability.
        backend.font_for(13.0, 400)?;
        Ok(backend)
    }

    pub unsafe fn set_drawable(&mut self, drawable: Drawable) {
        (self.api.draw_change)(self.draw, drawable);
    }

    pub unsafe fn set_preferred_family(&mut self, family: &str) {
        let family = family.trim();
        if family.is_empty() || family == self.preferred_family {
            return;
        }
        for (_, font) in self.fonts.drain() {
            (self.api.font_close)(self.display, font);
        }
        self.preferred_family = family.to_string();
    }

    pub unsafe fn draw_text(
        &mut self,
        x: f32,
        baseline_y: f32,
        color: Color,
        size: f32,
        weight: u16,
        text: &str,
    ) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }
        let font = self.font_for(size, weight)?;
        let xft_color = self.color_for(color)?;
        let bytes = text.as_bytes();
        let len = bytes.len().min(i32::MAX as usize) as c_int;
        (self.api.draw_string_utf8)(
            self.draw,
            &xft_color,
            font,
            x.round() as c_int,
            baseline_y.round() as c_int,
            bytes.as_ptr(),
            len,
        );
        Ok(())
    }

    unsafe fn font_for(&mut self, size: f32, weight: u16) -> Result<*mut XftFont, String> {
        let key = FontKey {
            pixels: size.round().clamp(6.0, 96.0) as u16,
            weight: normalize_weight(weight),
        };
        if let Some(font) = self.fonts.get(&key).copied() {
            return Ok(font);
        }

        let weight_name = match key.weight {
            0..=400 => "regular",
            401..=500 => "medium",
            501..=600 => "semibold",
            _ => "bold",
        };
        let mut families = Vec::with_capacity(4);
        families.push(self.preferred_family.as_str());
        if self.preferred_family != "IBM Plex Sans" {
            families.push("IBM Plex Sans");
        }
        families.extend(["Noto Sans", "DejaVu Sans", "sans-serif"]);

        for family in families {
            let pattern = format!(
                "{family}:pixelsize={}:weight={weight_name}:antialias=true:hinting=true:hintstyle=hintslight:rgba=none",
                key.pixels
            );
            let c_pattern = CString::new(pattern).map_err(|_| "font pattern contains NUL".to_string())?;
            let font = (self.api.font_open_name)(self.display, self.screen, c_pattern.as_ptr());
            if !font.is_null() {
                self.fonts.insert(key, font);
                return Ok(font);
            }
        }
        Err(format!("Xft could not open a UI font at {} px", key.pixels))
    }

    unsafe fn color_for(&mut self, color: Color) -> Result<XftColor, String> {
        if let Some(value) = self.colors.get(&color).copied() {
            return Ok(value);
        }
        let render = XRenderColor {
            red: u16::from(color.r) * 257,
            green: u16::from(color.g) * 257,
            blue: u16::from(color.b) * 257,
            alpha: u16::from(color.a) * 257,
        };
        let mut allocated = XftColor {
            pixel: 0,
            color: render,
        };
        if (self.api.color_alloc_value)(
            self.display,
            self.visual,
            self.colormap,
            &render,
            &mut allocated,
        ) == 0
        {
            return Err(format!(
                "XftColorAllocValue failed for rgba({},{},{},{})",
                color.r, color.g, color.b, color.a
            ));
        }
        self.colors.insert(color, allocated);
        Ok(allocated)
    }
}

impl Drop for XftBackend {
    fn drop(&mut self) {
        unsafe {
            for (_, mut color) in self.colors.drain() {
                (self.api.color_free)(self.display, self.visual, self.colormap, &mut color);
            }
            for (_, font) in self.fonts.drain() {
                (self.api.font_close)(self.display, font);
            }
            if !self.draw.is_null() {
                (self.api.draw_destroy)(self.draw);
                self.draw = ptr::null_mut();
            }
        }
    }
}

unsafe fn register_application_fonts(api: FontconfigApi) -> Result<(), String> {
    let Some(value) = env::var_os("RWR_FONT_FILES") else {
        return Ok(());
    };
    let config = (api.config_get_current)();
    if config.is_null() {
        return Err("FcConfigGetCurrent returned NULL".to_string());
    }
    let mut added = 0usize;
    for path in env::split_paths(&value) {
        if !path.is_file() {
            return Err(format!("RWR_FONT_FILES entry does not exist: {}", path.display()));
        }
        let bytes = path_bytes(&path)?;
        let c_path = CString::new(bytes).map_err(|_| format!("font path contains NUL: {}", path.display()))?;
        if (api.config_app_font_add_file)(config, c_path.as_ptr() as *const c_uchar) == 0 {
            return Err(format!("FcConfigAppFontAddFile rejected {}", path.display()));
        }
        added += 1;
    }
    if added > 0 && (api.config_build_fonts)(config) == 0 {
        return Err("FcConfigBuildFonts failed after adding application fonts".to_string());
    }
    Ok(())
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Result<Vec<u8>, String> {
    use std::os::unix::ffi::OsStrExt;
    Ok(path.as_os_str().as_bytes().to_vec())
}

#[cfg(not(unix))]
fn path_bytes(path: &Path) -> Result<Vec<u8>, String> {
    path.to_str()
        .map(|value| value.as_bytes().to_vec())
        .ok_or_else(|| format!("font path is not UTF-8: {}", path.display()))
}

fn normalize_weight(weight: u16) -> u16 {
    match weight {
        0..=450 => 400,
        451..=550 => 500,
        551..=650 => 600,
        _ => 700,
    }
}

#[link(name = "dl")]
extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_weights_map_to_available_plex_faces() {
        assert_eq!(normalize_weight(400), 400);
        assert_eq!(normalize_weight(500), 500);
        assert_eq!(normalize_weight(600), 600);
        assert_eq!(normalize_weight(700), 700);
    }
}
