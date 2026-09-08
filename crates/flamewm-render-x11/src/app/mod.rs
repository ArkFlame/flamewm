use std::collections::HashMap;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::ptr;

use flamewm_render_core::{
    ActionEvent, ActionPhase, Color, CursorKind, ImageAsset, InteractionState, LayoutEngine,
    LayoutResult, PaintCommand, PointerButton, Rect, RuntimeDocument, build_paint_commands,
};

use super::config::{X11Config, X11WindowRole};
use super::native::cursor::XcursorBackend;
use super::native::target::{GeometryCommit, PresentationState, presenter_error};
use super::xft::XftBackend;
use super::xlib::*;
use super::xrender::XRenderBackend;
use super::xshape::{XShapeBridge, rounded_mask_spans};
use super::*;
use cursor::cursor_shape;

mod color;
mod cursor;
mod events;
mod geometry;
mod images;
mod input;
mod layout_helpers;
mod primitives;
mod render;

pub(crate) use input::{
    pointer_button_from_raw, slider_thumb_rect, slider_value_from_pointer, wheel_scroll_delta,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ImageCacheKey {
    asset: u16,
    width: u32,
    height: u32,
    revision: u64,
}

#[derive(Clone, Copy)]
struct CachedImage {
    pixmap: Pixmap,
    mask: Pixmap,
}

unsafe fn free_cached_image(display: *mut Display, image: CachedImage) {
    unsafe {
        XFreePixmap(display, image.pixmap);
        if image.mask != 0 {
            XFreePixmap(display, image.mask);
        }
    }
}

pub(crate) struct X11App {
    pub(crate) display: *mut Display,
    pub(crate) screen: i32,
    pub(crate) depth: i32,
    pub(crate) window: Window,
    pub(crate) backbuffer: Pixmap,
    pub(crate) gc: GC,
    pub(crate) colormap: Colormap,
    pub(crate) colors: HashMap<Color, u64>,
    images: HashMap<ImageCacheKey, CachedImage>,
    pub(crate) cursors: HashMap<CursorKind, Cursor>,
    pub(crate) current_cursor: CursorKind,
    pub(crate) xcursor: Option<XcursorBackend>,
    pub(crate) fonts: HashMap<u8, *mut XFontStructHead>,
    pub(crate) xft: Option<XftBackend>,
    pub(crate) xrender: Option<XRenderBackend>,
    pub(crate) xshape: Option<XShapeBridge>,
    pub(crate) wm_delete: Atom,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) presentation: PresentationState,
    pub(crate) clip_stack: Vec<Rect>,
    pub(crate) scroll_offsets: HashMap<u32, (f32, f32)>,
    pub(crate) range_drag: Option<(u32, f32, f32, f32, f32)>,
    pub(crate) interaction: InteractionState,
    pub(crate) layout: Option<LayoutResult>,
    pub(crate) pointer_button: Option<u32>,
    pub(crate) pointer_grabbed: bool,
    pub(crate) super_chord_used: bool,
    pub(crate) last_title_press: Option<(u32, u64)>,
    pub(crate) single_event: bool,
    pub(crate) close_requested: bool,
}
impl X11App {
    /// # Safety
    ///
    /// `display` must be a live Xlib display owned by the caller and remain
    /// open until the returned app has been dropped.
    pub(crate) unsafe fn new(display: *mut Display, config: &X11Config) -> Result<Self, String> {
        unsafe { Self::new_surface(display, config, config.x, config.y, true) }
    }

    /// # Safety
    ///
    /// `display` must be a live Xlib display owned by the caller and remain
    /// open until the returned app has been dropped.
    pub(crate) unsafe fn new_surface(
        display: *mut Display,
        config: &X11Config,
        x: i32,
        y: i32,
        initially_visible: bool,
    ) -> Result<Self, String> {
        let screen = unsafe { XDefaultScreen(display) };
        let root = unsafe { XRootWindow(display, screen) };
        let black = unsafe { XBlackPixel(display, screen) };
        let depth = unsafe { XDefaultDepth(display, screen) };
        let window = unsafe {
            XCreateSimpleWindow(
                display,
                root,
                x,
                y,
                config.width.max(1),
                config.height.max(1),
                0,
                black,
                black,
            )
        };
        if window == 0 {
            return Err("XCreateSimpleWindow failed".to_string());
        }
        let event_mask = EXPOSURE_MASK
            | STRUCTURE_NOTIFY_MASK
            | POINTER_MOTION_MASK
            | LEAVE_WINDOW_MASK
            | BUTTON_PRESS_MASK
            | BUTTON_RELEASE_MASK
            | KEY_PRESS_MASK
            | KEY_RELEASE_MASK;
        unsafe {
            XSelectInput(display, window, event_mask);
        }
        let title = CString::new(config.title.as_str())
            .map_err(|_| "window title contains NUL".to_string())?;
        unsafe {
            XStoreName(display, window, title.as_ptr());
        }
        set_window_role(display, window, config.role);
        if matches!(
            config.role,
            X11WindowRole::PopupMenu | X11WindowRole::DropdownMenu
        ) {
            let mut attributes = XSetWindowAttributes {
                background_pixmap: 0,
                background_pixel: 0,
                border_pixmap: 0,
                border_pixel: 0,
                bit_gravity: 0,
                win_gravity: 0,
                backing_store: 0,
                backing_planes: 0,
                backing_pixel: 0,
                save_under: 0,
                event_mask: 0,
                do_not_propagate_mask: 0,
                override_redirect: 1,
                colormap: 0,
                cursor: 0,
            };
            unsafe {
                XChangeWindowAttributes(display, window, CW_OVERRIDE_REDIRECT, &mut attributes);
            }
        }
        let gc = unsafe { XCreateGC(display, window, 0, ptr::null_mut()) };
        if gc.is_null() {
            unsafe {
                XDestroyWindow(display, window);
            }
            return Err("XCreateGC failed".to_string());
        }
        // Image painting uses many XCopyArea requests. The Xlib default is to
        // request GraphicsExpose/NoExpose events for each copy, which creates
        // useless event-queue traffic for a retained-mode UI renderer.
        unsafe {
            XSetGraphicsExposures(display, gc, 0);
        }
        let backbuffer = unsafe {
            XCreatePixmap(
                display,
                window,
                config.width.max(1),
                config.height.max(1),
                depth as u32,
            )
        };
        if backbuffer == 0 {
            unsafe {
                XFreeGC(display, gc);
                XDestroyWindow(display, window);
            }
            return Err("XCreatePixmap failed for retained backbuffer".to_string());
        }
        let mut fonts = HashMap::new();
        for (bucket, name) in [
            (10u8, "6x10"),
            (13u8, "6x13"),
            (15u8, "9x15"),
            (20u8, "10x20"),
        ] {
            let name = CString::new(name).expect("static font name contains no NUL");
            let font = unsafe { XLoadQueryFont(display, name.as_ptr()) };
            if !font.is_null() {
                fonts.insert(bucket, font);
            }
        }
        if fonts.is_empty() {
            let fixed = CString::new("fixed").expect("static font name contains no NUL");
            let font = unsafe { XLoadQueryFont(display, fixed.as_ptr()) };
            if !font.is_null() {
                fonts.insert(13, font);
            }
        }
        if let Some(font) = fonts.values().next().copied() {
            let fid = unsafe { (*font).fid };
            unsafe {
                XSetFont(display, gc, fid);
            }
        }
        let visual = unsafe { XDefaultVisual(display, screen) };
        let colormap = unsafe { XDefaultColormap(display, screen) };
        let xft = if visual.is_null() {
            eprintln!("FLAMEWM_RENDER_TEXT_BACKEND core-x11 reason=default-visual-null");
            None
        } else {
            match unsafe { XftBackend::new(display, screen, backbuffer, visual, colormap) } {
                Ok(backend) => {
                    eprintln!("FLAMEWM_RENDER_TEXT_BACKEND xft preferred=IBM_Plex_Sans");
                    Some(backend)
                }
                Err(error) => {
                    eprintln!("FLAMEWM_RENDER_TEXT_BACKEND core-x11 reason={error}");
                    None
                }
            }
        };
        let xrender = if visual.is_null() {
            None
        } else {
            match unsafe { XRenderBackend::new(display, backbuffer, visual) } {
                Ok(backend) => {
                    eprintln!("FLAMEWM_RENDER_ALPHA_BACKEND xrender");
                    Some(backend)
                }
                Err(error) => {
                    eprintln!("FLAMEWM_RENDER_ALPHA_BACKEND fallback-black reason={error}");
                    None
                }
            }
        };
        // Shape bridge is render-owned: popup/menu rounded corners. Failure is
        // explicit degraded-rectangle mode, never a silent transparency claim.
        let xshape = match unsafe { XShapeBridge::new(display) } {
            Ok(bridge) => Some(bridge),
            Err(error) => {
                eprintln!("FLAMEWM_RENDER_SHAPE_BACKEND fallback-rectangle reason={error}");
                None
            }
        };
        let delete_name = CString::new("WM_DELETE_WINDOW").expect("static atom contains no NUL");
        let wm_delete = unsafe { XInternAtom(display, delete_name.as_ptr(), 0) };
        if wm_delete != 0 {
            let mut protocol = wm_delete;
            unsafe {
                XSetWMProtocols(display, window, &mut protocol, 1);
            }
        }

        let mut cursors = HashMap::new();
        for kind in [
            CursorKind::Default,
            CursorKind::Pointer,
            CursorKind::Text,
            CursorKind::Move,
            CursorKind::ResizeHorizontal,
            CursorKind::ResizeVertical,
            CursorKind::ResizeNorthWestSouthEast,
            CursorKind::ResizeNorthEastSouthWest,
        ] {
            let cursor = unsafe { XCreateFontCursor(display, cursor_shape(kind)) };
            if cursor != 0 {
                cursors.insert(kind, cursor);
            }
        }
        if let Some(cursor) = cursors.get(&CursorKind::Default).copied() {
            unsafe {
                XDefineCursor(display, window, cursor);
            }
        }
        let xcursor = unsafe { XcursorBackend::new(display) };
        let presentation = if initially_visible {
            PresentationState::Projected
        } else {
            PresentationState::CreatedHidden
        };

        if initially_visible {
            unsafe {
                XMapWindow(display, window);
            }
        }
        unsafe {
            XFlush(display);
        }
        Ok(Self {
            display,
            screen,
            depth,
            window,
            backbuffer,
            gc,
            colormap,
            colors: HashMap::new(),
            images: HashMap::new(),
            cursors,
            current_cursor: CursorKind::Default,
            xcursor: Some(xcursor),
            fonts,
            xft,
            xrender,
            xshape,
            wm_delete,
            width: config.width.max(1),
            height: config.height.max(1),
            presentation,
            clip_stack: Vec::new(),
            scroll_offsets: HashMap::new(),
            range_drag: None,
            interaction: InteractionState::default(),
            layout: None,
            pointer_button: None,
            pointer_grabbed: false,
            super_chord_used: false,
            last_title_press: None,
            single_event: false,
            close_requested: false,
        })
    }
}

fn set_window_role(display: *mut Display, window: Window, role: X11WindowRole) {
    let property_name = CString::new("_NET_WM_WINDOW_TYPE").expect("static atom contains no NUL");
    let role_name = CString::new(match role {
        X11WindowRole::Normal => "_NET_WM_WINDOW_TYPE_NORMAL",
        X11WindowRole::Desktop => "_NET_WM_WINDOW_TYPE_DESKTOP",
        X11WindowRole::Dock => "_NET_WM_WINDOW_TYPE_DOCK",
        X11WindowRole::PopupMenu => "_NET_WM_WINDOW_TYPE_POPUP_MENU",
        X11WindowRole::DropdownMenu => "_NET_WM_WINDOW_TYPE_DROPDOWN_MENU",
    })
    .expect("static atom contains no NUL");
    let property = unsafe { XInternAtom(display, property_name.as_ptr(), 0) };
    let role_atom = unsafe { XInternAtom(display, role_name.as_ptr(), 0) };
    if property != 0 && role_atom != 0 {
        unsafe {
            XChangeProperty(
                display,
                window,
                property,
                XA_ATOM,
                32,
                PROP_MODE_REPLACE,
                &role_atom as *const Atom as *const u8,
                1,
            );
        }
    }
}

impl Drop for X11App {
    fn drop(&mut self) {
        // XftDraw owns resources associated with the drawable. Destroy it before
        // the X window so its teardown never observes an invalid drawable.
        drop(self.xft.take());
        drop(self.xrender.take());
        drop(self.xshape.take());
        if let Some(mut xcursor) = self.xcursor.take() {
            unsafe { xcursor.free_all() };
        }
        unsafe {
            if self.pointer_grabbed {
                XUngrabPointer(self.display, CURRENT_TIME);
                self.pointer_grabbed = false;
            }
            for image in self.images.values().copied() {
                free_cached_image(self.display, image);
            }
            for cursor in self.cursors.values().copied() {
                XFreeCursor(self.display, cursor);
            }
            for font in self.fonts.values().copied() {
                XFreeFont(self.display, font);
            }
            if self.backbuffer != 0 {
                XFreePixmap(self.display, self.backbuffer);
                self.backbuffer = 0;
            }
            if !self.gc.is_null() {
                XFreeGC(self.display, self.gc);
            }
            if self.window != 0 {
                XDestroyWindow(self.display, self.window);
            }
        }
    }
}
