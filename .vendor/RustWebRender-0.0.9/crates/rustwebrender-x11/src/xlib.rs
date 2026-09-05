#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::os::raw::{c_char, c_int, c_long, c_short, c_uint, c_ulong, c_ushort, c_void};

#[repr(C)]
pub struct Display {
    _private: [u8; 0],
}

#[repr(C)]
pub struct Visual {
    _private: [u8; 0],
}

#[repr(C)]
pub struct _XGC {
    _private: [u8; 0],
}

pub type GC = *mut _XGC;
pub type Window = c_ulong;
pub type Drawable = c_ulong;
pub type Pixmap = c_ulong;
pub type Cursor = c_ulong;
pub type Colormap = c_ulong;
pub type Atom = c_ulong;
pub type Font = c_ulong;
pub type Time = c_ulong;
pub type Bool = c_int;
pub type Status = c_int;

#[repr(C)]
pub struct XFontStructHead {
    pub ext_data: *mut c_void,
    pub fid: Font,
}


#[repr(C)]
pub struct XImageHead {
    pub width: c_int,
    pub height: c_int,
    pub xoffset: c_int,
    pub format: c_int,
    pub data: *mut c_char,
    pub byte_order: c_int,
    pub bitmap_unit: c_int,
    pub bitmap_bit_order: c_int,
    pub bitmap_pad: c_int,
    pub depth: c_int,
    pub bytes_per_line: c_int,
    pub bits_per_pixel: c_int,
    pub red_mask: c_ulong,
    pub green_mask: c_ulong,
    pub blue_mask: c_ulong,
}

#[repr(C)]
pub struct XImage {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XColor {
    pub pixel: c_ulong,
    pub red: c_ushort,
    pub green: c_ushort,
    pub blue: c_ushort,
    pub flags: c_char,
    pub pad: c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XAnyEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XExposeEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub count: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XConfigureEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub event: Window,
    pub window: Window,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub border_width: c_int,
    pub above: Window,
    pub override_redirect: Bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XMotionEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub root: Window,
    pub subwindow: Window,
    pub time: Time,
    pub x: c_int,
    pub y: c_int,
    pub x_root: c_int,
    pub y_root: c_int,
    pub state: c_uint,
    pub is_hint: c_char,
    pub same_screen: Bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XKeyEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub root: Window,
    pub subwindow: Window,
    pub time: Time,
    pub x: c_int,
    pub y: c_int,
    pub x_root: c_int,
    pub y_root: c_int,
    pub state: c_uint,
    pub keycode: c_uint,
    pub same_screen: Bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XButtonEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub root: Window,
    pub subwindow: Window,
    pub time: Time,
    pub x: c_int,
    pub y: c_int,
    pub x_root: c_int,
    pub y_root: c_int,
    pub state: c_uint,
    pub button: c_uint,
    pub same_screen: Bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union ClientMessageData {
    pub b: [c_char; 20],
    pub s: [c_short; 10],
    pub l: [c_long; 5],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XClientMessageEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub message_type: Atom,
    pub format: c_int,
    pub data: ClientMessageData,
}

#[repr(C)]
pub union XEvent {
    pub type_: c_int,
    pub xany: XAnyEvent,
    pub xexpose: XExposeEvent,
    pub xconfigure: XConfigureEvent,
    pub xmotion: XMotionEvent,
    pub xkey: XKeyEvent,
    pub xbutton: XButtonEvent,
    pub xclient: XClientMessageEvent,
    pub pad: [c_long; 24],
}

pub const KEY_PRESS_MASK: c_long = 1 << 0;
pub const KEY_RELEASE_MASK: c_long = 1 << 1;
pub const BUTTON_PRESS_MASK: c_long = 1 << 2;
pub const BUTTON_RELEASE_MASK: c_long = 1 << 3;
pub const POINTER_MOTION_MASK: c_long = 1 << 6;
pub const LEAVE_WINDOW_MASK: c_long = 1 << 5;
pub const EXPOSURE_MASK: c_long = 1 << 15;
pub const STRUCTURE_NOTIFY_MASK: c_long = 1 << 17;

pub const KEY_PRESS: c_int = 2;
pub const KEY_RELEASE: c_int = 3;
pub const BUTTON_PRESS: c_int = 4;
pub const BUTTON_RELEASE: c_int = 5;
pub const MOTION_NOTIFY: c_int = 6;
pub const LEAVE_NOTIFY: c_int = 8;
pub const EXPOSE: c_int = 12;
pub const CONFIGURE_NOTIFY: c_int = 22;
pub const CLIENT_MESSAGE: c_int = 33;

pub const DO_RED: c_char = 1;
pub const DO_GREEN: c_char = 2;
pub const DO_BLUE: c_char = 4;

pub const ZPIXMAP: c_int = 2;
pub const QUEUED_AFTER_READING: c_int = 1;

pub const SHIFT_MASK: c_uint = 1 << 0;
pub const CONTROL_MASK: c_uint = 1 << 2;
pub const MOD1_MASK: c_uint = 1 << 3;
pub const MOD4_MASK: c_uint = 1 << 6;

pub const XK_BACK_SPACE: c_ulong = 0xff08;
pub const XK_TAB: c_ulong = 0xff09;
pub const XK_SPACE: c_ulong = 0x0020;
pub const XK_RETURN: c_ulong = 0xff0d;
pub const XK_ESCAPE: c_ulong = 0xff1b;
pub const XK_LEFT: c_ulong = 0xff51;
pub const XK_UP: c_ulong = 0xff52;
pub const XK_RIGHT: c_ulong = 0xff53;
pub const XK_DOWN: c_ulong = 0xff54;
pub const XK_SUPER_L: c_ulong = 0xffeb;
pub const XK_SUPER_R: c_ulong = 0xffec;

// Standard X cursor font glyph indices from <X11/cursorfont.h>.
pub const XC_LEFT_PTR: c_uint = 68;
pub const XC_HAND2: c_uint = 60;
pub const XC_XTERM: c_uint = 152;
pub const XC_FLEUR: c_uint = 52;
pub const XC_SB_H_DOUBLE_ARROW: c_uint = 108;
pub const XC_SB_V_DOUBLE_ARROW: c_uint = 116;
pub const XC_BOTTOM_RIGHT_CORNER: c_uint = 14;
pub const XC_BOTTOM_LEFT_CORNER: c_uint = 12;


#[link(name = "X11")]
extern "C" {
    pub fn XOpenDisplay(display_name: *const c_char) -> *mut Display;
    pub fn XCloseDisplay(display: *mut Display) -> c_int;
    pub fn XDefaultScreen(display: *mut Display) -> c_int;
    pub fn XDefaultDepth(display: *mut Display, screen_number: c_int) -> c_int;
    pub fn XDefaultVisual(display: *mut Display, screen_number: c_int) -> *mut Visual;
    pub fn XRootWindow(display: *mut Display, screen_number: c_int) -> Window;
    pub fn XBlackPixel(display: *mut Display, screen_number: c_int) -> c_ulong;
    pub fn XWhitePixel(display: *mut Display, screen_number: c_int) -> c_ulong;
    pub fn XDefaultColormap(display: *mut Display, screen_number: c_int) -> Colormap;
    pub fn XCreateSimpleWindow(
        display: *mut Display,
        parent: Window,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
        border_width: c_uint,
        border: c_ulong,
        background: c_ulong,
    ) -> Window;
    pub fn XDestroyWindow(display: *mut Display, window: Window) -> c_int;
    pub fn XCreatePixmap(display: *mut Display, drawable: Drawable, width: c_uint, height: c_uint, depth: c_uint) -> Pixmap;
    pub fn XFreePixmap(display: *mut Display, pixmap: Pixmap) -> c_int;
    pub fn XCopyArea(display: *mut Display, src: Drawable, dest: Drawable, gc: GC, src_x: c_int, src_y: c_int, width: c_uint, height: c_uint, dest_x: c_int, dest_y: c_int) -> c_int;
    pub fn XSelectInput(display: *mut Display, window: Window, event_mask: c_long) -> c_int;
    pub fn XMapWindow(display: *mut Display, window: Window) -> c_int;
    pub fn XStoreName(display: *mut Display, window: Window, window_name: *const c_char) -> c_int;
    pub fn XCreateGC(display: *mut Display, drawable: Drawable, valuemask: c_ulong, values: *mut std::ffi::c_void) -> GC;
    pub fn XFreeGC(display: *mut Display, gc: GC) -> c_int;
    pub fn XSetForeground(display: *mut Display, gc: GC, foreground: c_ulong) -> c_int;
    pub fn XSetGraphicsExposures(display: *mut Display, gc: GC, graphics_exposures: Bool) -> c_int;
    pub fn XSetClipMask(display: *mut Display, gc: GC, pixmap: Pixmap) -> c_int;
    pub fn XSetClipOrigin(display: *mut Display, gc: GC, clip_x_origin: c_int, clip_y_origin: c_int) -> c_int;
    pub fn XFillRectangle(display: *mut Display, drawable: Drawable, gc: GC, x: c_int, y: c_int, width: c_uint, height: c_uint) -> c_int;
    pub fn XDrawRectangle(display: *mut Display, drawable: Drawable, gc: GC, x: c_int, y: c_int, width: c_uint, height: c_uint) -> c_int;
    pub fn XDrawLine(display: *mut Display, drawable: Drawable, gc: GC, x1: c_int, y1: c_int, x2: c_int, y2: c_int) -> c_int;
    pub fn XDrawArc(display: *mut Display, drawable: Drawable, gc: GC, x: c_int, y: c_int, width: c_uint, height: c_uint, angle1: c_int, angle2: c_int) -> c_int;
    pub fn XFillArc(display: *mut Display, drawable: Drawable, gc: GC, x: c_int, y: c_int, width: c_uint, height: c_uint, angle1: c_int, angle2: c_int) -> c_int;
    pub fn XDrawString(display: *mut Display, drawable: Drawable, gc: GC, x: c_int, y: c_int, string: *const c_char, length: c_int) -> c_int;
    pub fn XCreateImage(display: *mut Display, visual: *mut Visual, depth: c_uint, format: c_int, offset: c_int, data: *mut c_char, width: c_uint, height: c_uint, bitmap_pad: c_int, bytes_per_line: c_int) -> *mut XImage;
    pub fn XDestroyImage(image: *mut XImage) -> c_int;
    pub fn XPutImage(display: *mut Display, drawable: Drawable, gc: GC, image: *mut XImage, src_x: c_int, src_y: c_int, dest_x: c_int, dest_y: c_int, width: c_uint, height: c_uint) -> c_int;
    pub fn XPutPixel(image: *mut XImage, x: c_int, y: c_int, pixel: c_ulong) -> c_int;
    pub fn XLoadFont(display: *mut Display, name: *const c_char) -> Font;
    pub fn XLoadQueryFont(display: *mut Display, name: *const c_char) -> *mut XFontStructHead;
    pub fn XFreeFont(display: *mut Display, font_struct: *mut XFontStructHead) -> c_int;
    pub fn XUnloadFont(display: *mut Display, font: Font) -> c_int;
    pub fn XSetFont(display: *mut Display, gc: GC, font: Font) -> c_int;
    pub fn XAllocColor(display: *mut Display, colormap: Colormap, screen_in_out: *mut XColor) -> Status;
    pub fn XLookupKeysym(key_event: *mut XKeyEvent, index: c_int) -> c_ulong;
    pub fn XNextEvent(display: *mut Display, event_return: *mut XEvent) -> c_int;
    pub fn XEventsQueued(display: *mut Display, mode: c_int) -> c_int;
    pub fn XPeekEvent(display: *mut Display, event_return: *mut XEvent) -> c_int;
    pub fn XFlush(display: *mut Display) -> c_int;
    pub fn XInternAtom(display: *mut Display, atom_name: *const c_char, only_if_exists: Bool) -> Atom;
    pub fn XSetWMProtocols(display: *mut Display, window: Window, protocols: *mut Atom, count: c_int) -> Status;
    pub fn XCreateFontCursor(display: *mut Display, shape: c_uint) -> Cursor;
    pub fn XDefineCursor(display: *mut Display, window: Window, cursor: Cursor) -> c_int;
    pub fn XFreeCursor(display: *mut Display, cursor: Cursor) -> c_int;
}

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
}
