//! Xcursor backend: semantic cursor names via libXcursor, cached,
//! with core font-cursor fallback. Logs `FLAMEWM_RENDER_CURSOR_BACKEND`
//! exactly once per process.

use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::Once;

use flamewm_render_core::CursorKind;

use crate::ffi::dynamic_library::DynamicLibrary;
use crate::xlib::{Cursor, Display, Window};

type LibraryLoadCursorFn = unsafe extern "C" fn(*mut Display, *const c_char) -> Cursor;

static LOG_ONCE: Once = Once::new();

fn log_backend(backend: &str) {
    LOG_ONCE.call_once(|| {
        eprintln!("FLAMEWM_RENDER_CURSOR_BACKEND {backend}");
    });
}

pub struct XcursorBackend {
    display: *mut Display,
    load: Option<LibraryLoadCursorFn>,
    _library: Option<DynamicLibrary>,
    cache: std::collections::HashMap<CursorKind, Cursor>,
    xcursor: bool,
}

impl XcursorBackend {
    /// # Safety
    /// `display` must be a live Xlib display outliving the backend.
    pub unsafe fn new(display: *mut Display) -> Self {
        let loaded = DynamicLibrary::open(&["libXcursor.so.1", "libXcursor.so"]).ok();
        let load = loaded.as_ref().and_then(|lib| {
            unsafe { lib.symbol::<LibraryLoadCursorFn>(b"XcursorLibraryLoadCursor\0") }.ok()
        });
        let xcursor = load.is_some();
        log_backend(if xcursor {
            "xcursor"
        } else {
            "core-font-cursor"
        });
        Self {
            display,
            load,
            _library: loaded,
            cache: std::collections::HashMap::new(),
            xcursor,
        }
    }

    pub fn uses_xcursor(&self) -> bool {
        self.xcursor
    }

    /// Resolve (and cache) a cursor for `kind`, trying Xcursor semantic
    /// names first, then the core font cursor fallback.
    ///
    /// # Safety
    /// `display` is the live display from `new`.
    pub unsafe fn cursor_for(&mut self, kind: CursorKind) -> Option<Cursor> {
        if let Some(cursor) = self.cache.get(&kind).copied() {
            return Some(cursor);
        }
        if let Some(load) = self.load {
            for name in cursor_names(kind) {
                let cname = CString::new(*name).ok()?;
                let cursor = unsafe { load(self.display, cname.as_ptr()) };
                if cursor != 0 {
                    self.cache.insert(kind, cursor);
                    return Some(cursor);
                }
            }
        }
        // Fallback: core font cursor via XCreateFontCursor.
        let cursor = unsafe { crate::xlib::XCreateFontCursor(self.display, core_shape(kind)) };
        if cursor != 0 {
            self.cache.insert(kind, cursor);
            return Some(cursor);
        }
        None
    }

    pub unsafe fn free_all(&mut self) {
        for (_, cursor) in self.cache.drain() {
            unsafe { crate::xlib::XFreeCursor(self.display, cursor) };
        }
    }

    /// Define the cursor for `kind` on `window`, falling back to Default.
    ///
    /// # Safety
    /// `window` must be live on this backend's display.
    pub unsafe fn define(&mut self, window: Window, kind: CursorKind) -> Option<Cursor> {
        let cursor = unsafe { self.cursor_for(kind) }
            .or_else(|| unsafe { self.cursor_for(CursorKind::Default) })?;
        unsafe {
            crate::xlib::XDefineCursor(self.display, window, cursor);
        }
        Some(cursor)
    }
}

fn core_shape(kind: CursorKind) -> u32 {
    match kind {
        CursorKind::Default => crate::xlib::XC_LEFT_PTR,
        CursorKind::Pointer => crate::xlib::XC_HAND2,
        CursorKind::Text => crate::xlib::XC_XTERM,
        CursorKind::Move => crate::xlib::XC_FLEUR,
        CursorKind::ResizeHorizontal => crate::xlib::XC_SB_H_DOUBLE_ARROW,
        CursorKind::ResizeVertical => crate::xlib::XC_SB_V_DOUBLE_ARROW,
        CursorKind::ResizeNorthWestSouthEast => crate::xlib::XC_BOTTOM_RIGHT_CORNER,
        CursorKind::ResizeNorthEastSouthWest => crate::xlib::XC_BOTTOM_LEFT_CORNER,
    }
}

fn cursor_names(kind: CursorKind) -> &'static [&'static str] {
    match kind {
        CursorKind::Default | CursorKind::Move => &["default", "left_ptr"],
        CursorKind::Pointer => &["pointer", "hand2", "hand"],
        CursorKind::Text => &["text", "xterm", "ibeam"],
        CursorKind::ResizeHorizontal => &["h_double_arrow", "sb_h_double_arrow", "col-resize"],
        CursorKind::ResizeVertical => &["v_double_arrow", "sb_v_double_arrow", "row-resize"],
        CursorKind::ResizeNorthWestSouthEast => &["nwse-resize", "bottom_right_corner"],
        CursorKind::ResizeNorthEastSouthWest => &["nesw-resize", "bottom_left_corner"],
    }
}
