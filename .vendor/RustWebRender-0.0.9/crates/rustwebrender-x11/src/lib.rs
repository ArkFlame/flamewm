mod xft;
mod xlib;
mod xrender;

use std::collections::HashMap;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::ptr;

use rustwebrender_core::{
    build_paint_commands, Color, CursorKind, ImageAsset, InteractionState, LayoutEngine,
    LayoutResult, PaintCommand, Rect, RuntimeDocument,
};

use xft::XftBackend;
use xlib::*;
use xrender::XRenderBackend;

#[derive(Clone, Debug)]
pub struct X11Config {
    pub width: u32,
    pub height: u32,
    pub title: String,
}

impl Default for X11Config {
    fn default() -> Self {
        Self { width: 1350, height: 641, title: "RustWebRender 0.0.9".to_string() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionPhase {
    Press,
    Hover,
    Motion,
    Release,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionEvent {
    pub action: String,
    pub phase: ActionPhase,
    pub button: u32,
    pub x: f32,
    pub y: f32,
    pub inside: bool,
    pub time_ms: u64,
    pub text: Option<String>,
}

pub fn run(document: RuntimeDocument, config: X11Config) -> Result<(), String> {
    run_with_handler(document, config, |action| println!("RWR_ACTION {action}"))
}

pub fn run_with_handler<F>(document: RuntimeDocument, config: X11Config, mut on_action: F) -> Result<(), String>
where
    F: FnMut(&str),
{
    run_with_controller(document, config, move |event, _document| {
        if event.phase == ActionPhase::Release && event.inside {
            on_action(&event.action);
        }
        Ok(())
    })
}

pub fn run_with_controller<F>(mut document: RuntimeDocument, config: X11Config, mut on_action: F) -> Result<(), String>
where
    F: FnMut(&ActionEvent, &mut RuntimeDocument) -> Result<(), String>,
{
    unsafe {
        let display = XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("XOpenDisplay failed; DISPLAY is unset or unreachable".to_string());
        }
        let mut app = match X11App::new(display, &config) {
            Ok(app) => app,
            Err(error) => {
                XCloseDisplay(display);
                return Err(error);
            }
        };
        let result = app.event_loop(&mut document, &mut on_action);
        drop(app);
        XCloseDisplay(display);
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ImageCacheKey {
    asset: u16,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy)]
struct CachedImage {
    pixmap: Pixmap,
    mask: Pixmap,
}

struct X11App {
    display: *mut Display,
    screen: i32,
    depth: i32,
    window: Window,
    backbuffer: Pixmap,
    gc: GC,
    colormap: Colormap,
    colors: HashMap<Color, u64>,
    images: HashMap<ImageCacheKey, CachedImage>,
    cursors: HashMap<CursorKind, Cursor>,
    current_cursor: CursorKind,
    fonts: HashMap<u8, *mut XFontStructHead>,
    xft: Option<XftBackend>,
    xrender: Option<XRenderBackend>,
    wm_delete: Atom,
    width: u32,
    height: u32,
    interaction: InteractionState,
    layout: Option<LayoutResult>,
    pointer_button: Option<u32>,
    super_chord_used: bool,
    last_title_press: Option<(u32, u64)>,
}

impl X11App {
    unsafe fn new(display: *mut Display, config: &X11Config) -> Result<Self, String> {
        let screen = XDefaultScreen(display);
        let root = XRootWindow(display, screen);
        let black = XBlackPixel(display, screen);
        let depth = XDefaultDepth(display, screen);
        let window = XCreateSimpleWindow(
            display,
            root,
            0,
            0,
            config.width.max(1),
            config.height.max(1),
            0,
            black,
            black,
        );
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
        XSelectInput(display, window, event_mask);
        let title = CString::new(config.title.as_str()).map_err(|_| "window title contains NUL".to_string())?;
        XStoreName(display, window, title.as_ptr());
        let gc = XCreateGC(display, window, 0, ptr::null_mut());
        if gc.is_null() {
            XDestroyWindow(display, window);
            return Err("XCreateGC failed".to_string());
        }
        // Image painting uses many XCopyArea requests. The Xlib default is to
        // request GraphicsExpose/NoExpose events for each copy, which creates
        // useless event-queue traffic for a retained-mode UI renderer.
        XSetGraphicsExposures(display, gc, 0);
        let backbuffer = XCreatePixmap(display, window, config.width.max(1), config.height.max(1), depth as u32);
        if backbuffer == 0 {
            XFreeGC(display, gc);
            XDestroyWindow(display, window);
            return Err("XCreatePixmap failed for retained backbuffer".to_string());
        }
        let mut fonts = HashMap::new();
        for (bucket, name) in [(10u8, "6x10"), (13u8, "6x13"), (15u8, "9x15"), (20u8, "10x20")] {
            let name = CString::new(name).expect("static font name contains no NUL");
            let font = XLoadQueryFont(display, name.as_ptr());
            if !font.is_null() {
                fonts.insert(bucket, font);
            }
        }
        if fonts.is_empty() {
            let fixed = CString::new("fixed").expect("static font name contains no NUL");
            let font = XLoadQueryFont(display, fixed.as_ptr());
            if !font.is_null() {
                fonts.insert(13, font);
            }
        }
        if let Some(font) = fonts.values().next().copied() {
            XSetFont(display, gc, (*font).fid);
        }
        let visual = XDefaultVisual(display, screen);
        let colormap = XDefaultColormap(display, screen);
        let xft = if visual.is_null() {
            eprintln!("RWR_TEXT_BACKEND core-x11 reason=default-visual-null");
            None
        } else {
            match XftBackend::new(display, screen, backbuffer, visual, colormap) {
                Ok(backend) => {
                    eprintln!("RWR_TEXT_BACKEND xft preferred=IBM_Plex_Sans");
                    Some(backend)
                }
                Err(error) => {
                    eprintln!("RWR_TEXT_BACKEND core-x11 reason={error}");
                    None
                }
            }
        };
        let xrender = if visual.is_null() {
            None
        } else {
            match XRenderBackend::new(display, backbuffer, visual) {
                Ok(backend) => {
                    eprintln!("RWR_ALPHA_BACKEND xrender");
                    Some(backend)
                }
                Err(error) => {
                    eprintln!("RWR_ALPHA_BACKEND fallback-black reason={error}");
                    None
                }
            }
        };
        let delete_name = CString::new("WM_DELETE_WINDOW").expect("static atom contains no NUL");
        let wm_delete = XInternAtom(display, delete_name.as_ptr(), 0);
        if wm_delete != 0 {
            let mut protocol = wm_delete;
            XSetWMProtocols(display, window, &mut protocol, 1);
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
            let cursor = XCreateFontCursor(display, cursor_shape(kind));
            if cursor != 0 {
                cursors.insert(kind, cursor);
            }
        }
        if let Some(cursor) = cursors.get(&CursorKind::Default).copied() {
            XDefineCursor(display, window, cursor);
        }

        XMapWindow(display, window);
        XFlush(display);
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
            fonts,
            xft,
            xrender,
            wm_delete,
            width: config.width.max(1),
            height: config.height.max(1),
            interaction: InteractionState::default(),
            layout: None,
            pointer_button: None,
            super_chord_used: false,
            last_title_press: None,
        })
    }

    unsafe fn event_loop<F>(&mut self, document: &mut RuntimeDocument, on_action: &mut F) -> Result<(), String>
    where
        F: FnMut(&ActionEvent, &mut RuntimeDocument) -> Result<(), String>,
    {
        self.redraw(document)?;
        loop {
            let mut event = MaybeUninit::<XEvent>::uninit();
            XNextEvent(self.display, event.as_mut_ptr());
            let event = event.assume_init();
            match event.type_ {
                EXPOSE => {
                    if event.xexpose.count == 0 {
                        self.redraw(document)?;
                    }
                }
                CONFIGURE_NOTIFY => {
                    let configure = event.xconfigure;
                    let width = configure.width.max(1) as u32;
                    let height = configure.height.max(1) as u32;
                    if width != self.width || height != self.height {
                        self.width = width;
                        self.height = height;
                        self.recreate_backbuffer()?;
                        self.redraw(document)?;
                    }
                }
                MOTION_NOTIFY => {
                    // Pointer motion can arrive much faster than a full scene redraw.
                    // Always consume the newest queued motion for this window before
                    // mutating layout/painting so interaction never trails behind the
                    // physical pointer by a backlog of stale coordinates.
                    let mut motion = event.xmotion;
                    loop {
                        // Only coalesce contiguous MotionNotify events. Do not
                        // search past ButtonRelease/other events, because doing
                        // so would violate X event ordering and could extend a
                        // drag beyond its release point. QUEUED_AFTER_READING also
                        // pulls already-arrived socket input into Xlib when the
                        // in-memory queue empties, preventing a nested X server
                        // from feeding us stale motion one packet at a time.
                        if XEventsQueued(self.display, QUEUED_AFTER_READING) <= 0 {
                            break;
                        }
                        let mut peeked = MaybeUninit::<XEvent>::uninit();
                        XPeekEvent(self.display, peeked.as_mut_ptr());
                        let peeked = peeked.assume_init();
                        if peeked.type_ != MOTION_NOTIFY || peeked.xany.window != self.window {
                            break;
                        }
                        let mut queued = MaybeUninit::<XEvent>::uninit();
                        XNextEvent(self.display, queued.as_mut_ptr());
                        motion = queued.assume_init().xmotion;
                    }
                    let hover = self.hit_test(document, motion.x as f32, motion.y as f32);
                    let changed = hover != self.interaction.hover;
                    self.interaction.hover = hover;
                    self.update_cursor(document, hover);
                    if let (Some(button), Some(index)) = (self.pointer_button, self.interaction.active) {
                        let action = document.document.nodes[index as usize].action.clone();
                        if !action.is_empty() {
                            on_action(&ActionEvent {
                                action,
                                phase: ActionPhase::Motion,
                                button,
                                x: motion.x as f32 / document.ui_scale(),
                                y: motion.y as f32 / document.ui_scale(),
                                inside: hover == Some(index),
                                time_ms: motion.time as u64,
                                text: None,
                            }, document)?;
                            self.redraw(document)?;
                        } else if changed {
                            self.redraw(document)?;
                        }
                    } else if changed {
                        if let Some(index) = hover {
                            let action = document.document.nodes[index as usize].action.clone();
                            if !action.is_empty() {
                                on_action(&ActionEvent {
                                    action,
                                    phase: ActionPhase::Hover,
                                    button: 0,
                                    x: motion.x as f32 / document.ui_scale(),
                                    y: motion.y as f32 / document.ui_scale(),
                                    inside: true,
                                    time_ms: motion.time as u64,
                                    text: None,
                                }, document)?;
                            }
                        }
                        self.redraw(document)?;
                    }
                }
                LEAVE_NOTIFY => {
                    let had_hover = self.interaction.hover.take().is_some();
                    self.set_cursor(CursorKind::Default);
                    if had_hover {
                        self.redraw(document)?;
                    }
                }
                KEY_PRESS => {
                    let mut key = event.xkey;
                    let symbol = XLookupKeysym(&mut key, 0);
                    let ctrl = key.state & CONTROL_MASK != 0;
                    let alt = key.state & MOD1_MASK != 0;
                    let super_key = key.state & MOD4_MASK != 0;
                    let action = if alt && symbol == XK_TAB {
                        self.super_chord_used |= super_key;
                        Some("keyboard.alt-tab")
                    } else if ctrl && !alt && !super_key && symbol == XK_SPACE {
                        Some("keyboard.start.1")
                    } else if ctrl && super_key {
                        self.super_chord_used = true;
                        match symbol {
                            XK_LEFT => Some("keyboard.desktop.left.0"),
                            XK_RIGHT => Some("keyboard.desktop.right.0"),
                            XK_UP => Some("keyboard.desktop.up.0"),
                            XK_DOWN => Some("keyboard.desktop.down.0"),
                            _ => None,
                        }
                    } else if alt && !ctrl && !super_key {
                        match symbol {
                            XK_LEFT => Some("keyboard.desktop.left.1"),
                            XK_RIGHT => Some("keyboard.desktop.right.1"),
                            XK_UP => Some("keyboard.desktop.up.1"),
                            XK_DOWN => Some("keyboard.desktop.down.1"),
                            _ => None,
                        }
                    } else {
                        if super_key && symbol != XK_SUPER_L && symbol != XK_SUPER_R {
                            self.super_chord_used = true;
                        }
                        None
                    };
                    if let Some(action) = action {
                        on_action(&ActionEvent {
                            action: action.to_string(),
                            phase: ActionPhase::Release,
                            button: 0,
                            x: key.x as f32 / document.ui_scale(),
                            y: key.y as f32 / document.ui_scale(),
                            inside: true,
                            time_ms: key.time as u64,
                            text: None,
                        }, document)?;
                        self.redraw(document)?;
                    } else if !ctrl && !alt && !super_key {
                        let shift_index = if key.state & SHIFT_MASK != 0 { 1 } else { 0 };
                        let text_symbol = XLookupKeysym(&mut key, shift_index);
                        if let Some(text) = keyboard_text(text_symbol) {
                            on_action(&ActionEvent {
                                action: "keyboard.input".to_string(),
                                phase: ActionPhase::Release,
                                button: 0,
                                x: key.x as f32 / document.ui_scale(),
                                y: key.y as f32 / document.ui_scale(),
                                inside: true,
                                time_ms: key.time as u64,
                                text: Some(text),
                            }, document)?;
                            self.redraw(document)?;
                        }
                    }
                }
                KEY_RELEASE => {
                    let mut key = event.xkey;
                    let symbol = XLookupKeysym(&mut key, 0);
                    if symbol == XK_SUPER_L || symbol == XK_SUPER_R {
                        if !self.super_chord_used {
                            on_action(&ActionEvent {
                                action: "keyboard.start.0".to_string(),
                                phase: ActionPhase::Release,
                                button: 0,
                                x: key.x as f32 / document.ui_scale(),
                                y: key.y as f32 / document.ui_scale(),
                                inside: true,
                                time_ms: key.time as u64,
                                text: None,
                            }, document)?;
                            self.redraw(document)?;
                        }
                        self.super_chord_used = false;
                    }
                }
                BUTTON_PRESS => {
                    let button = event.xbutton;
                    let hit = self.hit_test(document, button.x as f32, button.y as f32);
                    if button.button == 1 {
                        if let Some(index) = hit {
                            let action = document.document.nodes[index as usize].action.clone();
                            let now = button.time as u64;
                            let double_title = action.starts_with("window.")
                                && action.ends_with(".move")
                                && self.last_title_press
                                    .map(|(last_index, last_time)| last_index == index && now.saturating_sub(last_time) <= 350)
                                    .unwrap_or(false);
                            if double_title {
                                self.last_title_press = None;
                                self.interaction.active = None;
                                self.pointer_button = None;
                                if let Some(prefix) = action.strip_prefix("window.").and_then(|rest| rest.strip_suffix(".move")) {
                                    on_action(&ActionEvent {
                                        action: format!("{prefix}.maximize"),
                                        phase: ActionPhase::Release,
                                        button: button.button,
                                        x: button.x as f32 / document.ui_scale(),
                                        y: button.y as f32 / document.ui_scale(),
                                        inside: true,
                                        time_ms: button.time as u64,
                                        text: None,
                                    }, document)?;
                                }
                                self.redraw(document)?;
                                continue;
                            }
                            if action.starts_with("window.") && action.ends_with(".move") {
                                self.last_title_press = Some((index, now));
                            } else {
                                self.last_title_press = None;
                            }
                        } else {
                            self.last_title_press = None;
                        }
                        self.interaction.active = hit;
                        self.pointer_button = Some(button.button);
                        if let Some(index) = hit {
                            let action = document.document.nodes[index as usize].action.clone();
                            if !action.is_empty() {
                                on_action(&ActionEvent {
                                    action,
                                    phase: ActionPhase::Press,
                                    button: button.button,
                                    x: button.x as f32 / document.ui_scale(),
                                    y: button.y as f32 / document.ui_scale(),
                                    inside: true,
                                    time_ms: button.time as u64,
                                    text: None,
                                }, document)?;
                            }
                        }
                        self.redraw(document)?;
                    }
                }
                BUTTON_RELEASE => {
                    let button = event.xbutton;
                    let hit = self.hit_test(document, button.x as f32, button.y as f32);
                    let dispatch = if button.button == 1 {
                        self.pointer_button = None;
                        self.interaction.active.take()
                    } else if button.button == 2 || button.button == 3 {
                        hit
                    } else {
                        None
                    };
                    if let Some(index) = dispatch {
                        let action = document.document.nodes[index as usize].action.clone();
                        if !action.is_empty() {
                            on_action(&ActionEvent {
                                action,
                                phase: ActionPhase::Release,
                                button: button.button,
                                x: button.x as f32 / document.ui_scale(),
                                y: button.y as f32 / document.ui_scale(),
                                inside: hit == Some(index),
                                time_ms: button.time as u64,
                                text: None,
                            }, document)?;
                        }
                    }
                    self.redraw(document)?;
                    let hover = self.hit_test(document, button.x as f32, button.y as f32);
                    self.interaction.hover = hover;
                    self.update_cursor(document, hover);
                }
                CLIENT_MESSAGE => {
                    let client = event.xclient;
                    if self.wm_delete != 0 && client.format == 32 && client.data.l[0] as u64 == self.wm_delete {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
    }

    fn hit_test(&self, document: &RuntimeDocument, x: f32, y: f32) -> Option<u32> {
        let scale = document.ui_scale();
        self.layout
            .as_ref()
            .and_then(|layout| layout.hit_test_action(document, x / scale, y / scale))
    }

    unsafe fn update_cursor(&mut self, document: &RuntimeDocument, hover: Option<u32>) {
        let kind = hover
            .and_then(|index| document.document.nodes.get(index as usize).map(|node| (index, node)))
            .map(|(index, node)| self.interaction.style_for(index, node).cursor)
            .unwrap_or(CursorKind::Default);
        self.set_cursor(kind);
    }

    unsafe fn set_cursor(&mut self, kind: CursorKind) {
        if kind == self.current_cursor {
            return;
        }
        let cursor = self
            .cursors
            .get(&kind)
            .copied()
            .or_else(|| self.cursors.get(&CursorKind::Default).copied());
        if let Some(cursor) = cursor {
            XDefineCursor(self.display, self.window, cursor);
            XFlush(self.display);
            self.current_cursor = kind;
        }
    }

    unsafe fn recreate_backbuffer(&mut self) -> Result<(), String> {
        let replacement = XCreatePixmap(self.display, self.window, self.width, self.height, self.depth as u32);
        if replacement == 0 {
            return Err("XCreatePixmap failed while resizing retained backbuffer".to_string());
        }
        if let Some(xft) = self.xft.as_mut() {
            xft.set_drawable(replacement);
        }
        if let Some(xrender) = self.xrender.as_mut() {
            if let Err(error) = xrender.set_drawable(replacement) {
                if let Some(xft) = self.xft.as_mut() {
                    xft.set_drawable(self.backbuffer);
                }
                XFreePixmap(self.display, replacement);
                return Err(error);
            }
        }
        let previous = self.backbuffer;
        self.backbuffer = replacement;
        if previous != 0 {
            XFreePixmap(self.display, previous);
        }
        Ok(())
    }

    unsafe fn redraw(&mut self, document: &RuntimeDocument) -> Result<(), String> {
        let scale = document.ui_scale();
        let layout = LayoutEngine::compute(
            document,
            self.width as f32 / scale,
            self.height as f32 / scale,
            self.interaction,
        );
        let commands = build_paint_commands(document, &layout, self.interaction);
        let clear = root_background(document, self.interaction).unwrap_or(Color::BLACK);
        let pixel = self.pixel(clear)?;
        XSetForeground(self.display, self.gc, pixel);
        XSetClipMask(self.display, self.gc, 0);
        XFillRectangle(self.display, self.backbuffer, self.gc, 0, 0, self.width, self.height);
        for command in commands {
            self.paint(command, document, scale)?;
        }
        XSetClipMask(self.display, self.gc, 0);
        XSetClipOrigin(self.display, self.gc, 0, 0);
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
        );
        XFlush(self.display);
        self.layout = Some(layout);
        Ok(())
    }

    unsafe fn paint(&mut self, command: PaintCommand, document: &RuntimeDocument, scale: f32) -> Result<(), String> {
        match command {
            PaintCommand::FillRect { rect, color, radius } => {
                let rect = scale_rect(rect, scale);
                let radius = radius * scale;
                if color.a < 255 {
                    if let Some(xrender) = self.xrender.as_mut() {
                        xrender.fill_rounded_rect(rect, radius, color)?;
                    } else {
                        let pixel = self.pixel(color)?;
                        XSetForeground(self.display, self.gc, pixel);
                        self.fill_rounded_rect(rect, radius);
                    }
                } else {
                    let pixel = self.pixel(color)?;
                    XSetForeground(self.display, self.gc, pixel);
                    self.fill_rounded_rect(rect, radius);
                }
            }
            PaintCommand::StrokeRect { rect, color, width, radius } => {
                let rect = scale_rect(rect, scale);
                let width = width * scale;
                let radius = radius * scale;
                if color.a < 255 {
                    if let Some(xrender) = self.xrender.as_mut() {
                        xrender.stroke_rounded_rect(rect, radius, width, color)?;
                    } else {
                        let pixel = self.pixel(color)?;
                        XSetForeground(self.display, self.gc, pixel);
                        let repeats = width.round().clamp(1.0, 8.0) as i32;
                        for inset in 0..repeats {
                            self.stroke_rounded_rect(rect, radius, inset);
                        }
                    }
                } else {
                    let pixel = self.pixel(color)?;
                    XSetForeground(self.display, self.gc, pixel);
                    let repeats = width.round().clamp(1.0, 8.0) as i32;
                    for inset in 0..repeats {
                        self.stroke_rounded_rect(rect, radius, inset);
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
                let cached = self.image_pixmap(asset, image, width, height)?;
                let dx = rect.x.round() as i32;
                let dy = rect.y.round() as i32;
                if cached.mask != 0 {
                    XSetClipOrigin(self.display, self.gc, dx, dy);
                    XSetClipMask(self.display, self.gc, cached.mask);
                }
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
                );
                if cached.mask != 0 {
                    XSetClipMask(self.display, self.gc, 0);
                    XSetClipOrigin(self.display, self.gc, 0, 0);
                }
            }
            PaintCommand::Text { x, y, color, size, weight, text } => {
                let x = x * scale;
                let y = y * scale;
                let size = size * scale;
                if let Some(xft) = self.xft.as_mut() {
                    xft.set_preferred_family(document.ui_font_family());
                    xft.draw_text(x, y, color, size, weight, &text)?;
                } else {
                    let pixel = self.pixel(color)?;
                    XSetForeground(self.display, self.gc, pixel);
                    if let Some(font) = self.font_for_size(size) {
                        XSetFont(self.display, self.gc, font);
                    }
                    let ascii: String = text.chars().map(|ch| if ch.is_ascii() { ch } else { '?' }).collect();
                    let string = CString::new(ascii).map_err(|_| "text contains NUL".to_string())?;
                    let len = string.as_bytes().len().min(i32::MAX as usize) as i32;
                    XDrawString(
                        self.display,
                        self.backbuffer,
                        self.gc,
                        x.round() as i32,
                        y.round() as i32,
                        string.as_ptr(),
                        len,
                    );
                }
            }
        }
        Ok(())
    }

    unsafe fn font_for_size(&self, size: f32) -> Option<Font> {
        let target = size.round().clamp(1.0, 255.0) as i32;
        self.fonts
            .iter()
            .min_by_key(|(bucket, _)| (i32::from(**bucket) - target).abs())
            .map(|(_, font)| (**font).fid)
    }

    unsafe fn image_pixmap(&mut self, asset_id: u16, asset: &ImageAsset, width: u32, height: u32) -> Result<CachedImage, String> {
        let key = ImageCacheKey { asset: asset_id, width, height };
        if let Some(cached) = self.images.get(&key).copied() {
            return Ok(cached);
        }
        let pixmap = XCreatePixmap(self.display, self.window, width, height, self.depth as u32);
        if pixmap == 0 {
            return Err(format!("XCreatePixmap failed for image asset {asset_id}"));
        }
        let visual = XDefaultVisual(self.display, self.screen);
        if visual.is_null() {
            XFreePixmap(self.display, pixmap);
            return Err("XDefaultVisual returned NULL".to_string());
        }
        let ximage = XCreateImage(
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
        );
        if ximage.is_null() {
            XFreePixmap(self.display, pixmap);
            return Err(format!("XCreateImage failed for image asset {asset_id}"));
        }
        let head = &mut *(ximage as *mut XImageHead);
        if head.bytes_per_line <= 0 {
            XDestroyImage(ximage);
            XFreePixmap(self.display, pixmap);
            return Err("XCreateImage produced invalid bytes_per_line".to_string());
        }
        let total = (head.bytes_per_line as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| "XImage buffer size overflow".to_string())?;
        let data = malloc(total);
        if data.is_null() {
            XDestroyImage(ximage);
            XFreePixmap(self.display, pixmap);
            return Err(format!("malloc failed for {total}-byte XImage"));
        }
        ptr::write_bytes(data as *mut u8, 0, total);
        head.data = data as *mut _;

        let mut transparent = vec![false; (width as usize) * (height as usize)];
        let mut has_transparency = false;
        for y in 0..height {
            let source_y = ((y as u64 * asset.height as u64) / height as u64) as u32;
            for x in 0..width {
                let source_x = ((x as u64 * asset.width as u64) / width as u64) as u32;
                let offset = ((source_y as usize * asset.width as usize) + source_x as usize) * 3;
                let r = asset.pixels[offset];
                let g = asset.pixels[offset + 1];
                let b = asset.pixels[offset + 2];
                let is_transparent = r == 255 && g == 0 && b == 255;
                if is_transparent {
                    transparent[(y as usize) * (width as usize) + x as usize] = true;
                    has_transparency = true;
                }
                let (r, g, b) = if is_transparent { (0, 0, 0) } else { (r, g, b) };
                let pixel = rgb_to_pixel(r, g, b, head.red_mask as u64, head.green_mask as u64, head.blue_mask as u64);
                XPutPixel(ximage, x as i32, y as i32, pixel as _);
            }
        }
        XPutImage(self.display, pixmap, self.gc, ximage, 0, 0, 0, 0, width, height);
        XDestroyImage(ximage);

        let mask = if has_transparency {
            let mask = XCreatePixmap(self.display, self.window, width, height, 1);
            if mask == 0 {
                XFreePixmap(self.display, pixmap);
                return Err(format!("XCreatePixmap failed for transparency mask asset {asset_id}"));
            }
            let mask_gc = XCreateGC(self.display, mask, 0, ptr::null_mut());
            if mask_gc.is_null() {
                XFreePixmap(self.display, mask);
                XFreePixmap(self.display, pixmap);
                return Err(format!("XCreateGC failed for transparency mask asset {asset_id}"));
            }
            XSetForeground(self.display, mask_gc, 0);
            XFillRectangle(self.display, mask, mask_gc, 0, 0, width, height);
            XSetForeground(self.display, mask_gc, 1);
            for y in 0..height {
                let mut x = 0u32;
                while x < width {
                    while x < width && transparent[(y as usize) * (width as usize) + x as usize] { x += 1; }
                    let start = x;
                    while x < width && !transparent[(y as usize) * (width as usize) + x as usize] { x += 1; }
                    if x > start {
                        XFillRectangle(self.display, mask, mask_gc, start as i32, y as i32, x - start, 1);
                    }
                }
            }
            XFreeGC(self.display, mask_gc);
            mask
        } else {
            0
        };
        let cached = CachedImage { pixmap, mask };
        self.images.insert(key, cached);
        Ok(cached)
    }

    unsafe fn fill_rounded_rect(&self, rect: Rect, radius: f32) {
        let x = rect.x.round() as i32;
        let y = rect.y.round() as i32;
        let width = rect.width.round().max(0.0) as u32;
        let height = rect.height.round().max(0.0) as u32;
        if width == 0 || height == 0 {
            return;
        }
        let radius = radius.round().max(0.0).min((width.min(height) / 2) as f32) as u32;
        if radius == 0 {
            XFillRectangle(self.display, self.backbuffer, self.gc, x, y, width, height);
            return;
        }
        let diameter = radius * 2;
        XFillRectangle(self.display, self.backbuffer, self.gc, x + radius as i32, y, width.saturating_sub(diameter), height);
        XFillRectangle(self.display, self.backbuffer, self.gc, x, y + radius as i32, width, height.saturating_sub(diameter));
        let right = x + width as i32 - diameter as i32;
        let bottom = y + height as i32 - diameter as i32;
        XFillArc(self.display, self.backbuffer, self.gc, x, y, diameter, diameter, 90 * 64, 90 * 64);
        XFillArc(self.display, self.backbuffer, self.gc, right, y, diameter, diameter, 0, 90 * 64);
        XFillArc(self.display, self.backbuffer, self.gc, x, bottom, diameter, diameter, 180 * 64, 90 * 64);
        XFillArc(self.display, self.backbuffer, self.gc, right, bottom, diameter, diameter, 270 * 64, 90 * 64);
    }

    unsafe fn stroke_rounded_rect(&self, rect: Rect, radius: f32, inset: i32) {
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
            XDrawRectangle(self.display, self.backbuffer, self.gc, x, y, width, height);
            return;
        }
        let diameter = radius * 2;
        let right = x + width as i32 - diameter as i32;
        let bottom = y + height as i32 - diameter as i32;
        XDrawLine(self.display, self.backbuffer, self.gc, x + radius as i32, y, x + width as i32 - radius as i32, y);
        XDrawLine(self.display, self.backbuffer, self.gc, x + radius as i32, y + height as i32, x + width as i32 - radius as i32, y + height as i32);
        XDrawLine(self.display, self.backbuffer, self.gc, x, y + radius as i32, x, y + height as i32 - radius as i32);
        XDrawLine(self.display, self.backbuffer, self.gc, x + width as i32, y + radius as i32, x + width as i32, y + height as i32 - radius as i32);
        XDrawArc(self.display, self.backbuffer, self.gc, x, y, diameter, diameter, 90 * 64, 90 * 64);
        XDrawArc(self.display, self.backbuffer, self.gc, right, y, diameter, diameter, 0, 90 * 64);
        XDrawArc(self.display, self.backbuffer, self.gc, x, bottom, diameter, diameter, 180 * 64, 90 * 64);
        XDrawArc(self.display, self.backbuffer, self.gc, right, bottom, diameter, diameter, 270 * 64, 90 * 64);
    }

    unsafe fn pixel(&mut self, color: Color) -> Result<u64, String> {
        let opaque = if color.a == 255 { color } else { blend_over_black(color) };
        if let Some(pixel) = self.colors.get(&opaque) {
            return Ok(*pixel);
        }
        let mut xcolor = XColor {
            pixel: 0,
            red: u16::from(opaque.r) * 257,
            green: u16::from(opaque.g) * 257,
            blue: u16::from(opaque.b) * 257,
            flags: DO_RED | DO_GREEN | DO_BLUE,
            pad: 0,
        };
        if XAllocColor(self.display, self.colormap, &mut xcolor) == 0 {
            return Err(format!("XAllocColor failed for #{:02x}{:02x}{:02x}", opaque.r, opaque.g, opaque.b));
        }
        self.colors.insert(opaque, xcolor.pixel as u64);
        Ok(xcolor.pixel as u64)
    }
}

impl Drop for X11App {
    fn drop(&mut self) {
        // XftDraw owns resources associated with the drawable. Destroy it before
        // the X window so its teardown never observes an invalid drawable.
        drop(self.xft.take());
        drop(self.xrender.take());
        unsafe {
            for image in self.images.values().copied() {
                XFreePixmap(self.display, image.pixmap);
                if image.mask != 0 {
                    XFreePixmap(self.display, image.mask);
                }
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

fn scale_rect(rect: Rect, scale: f32) -> Rect {
    Rect {
        x: rect.x * scale,
        y: rect.y * scale,
        width: rect.width * scale,
        height: rect.height * scale,
    }
}

fn keyboard_text(symbol: u64) -> Option<String> {
    match symbol {
        XK_BACK_SPACE => Some("\u{8}".to_string()),
        XK_RETURN => Some("\n".to_string()),
        XK_ESCAPE => Some("\u{1b}".to_string()),
        0x20..=0x7e => char::from_u32(symbol as u32).map(|ch| ch.to_string()),
        0xa0..=0xff => char::from_u32(symbol as u32).map(|ch| ch.to_string()),
        _ => None,
    }
}

fn cursor_shape(kind: CursorKind) -> u32 {
    match kind {
        CursorKind::Default => XC_LEFT_PTR,
        CursorKind::Pointer => XC_HAND2,
        CursorKind::Text => XC_XTERM,
        CursorKind::Move => XC_FLEUR,
        CursorKind::ResizeHorizontal => XC_SB_H_DOUBLE_ARROW,
        CursorKind::ResizeVertical => XC_SB_V_DOUBLE_ARROW,
        CursorKind::ResizeNorthWestSouthEast => XC_BOTTOM_RIGHT_CORNER,
        CursorKind::ResizeNorthEastSouthWest => XC_BOTTOM_LEFT_CORNER,
    }
}

fn root_background(document: &RuntimeDocument, interaction: InteractionState) -> Option<Color> {
    let index = document.document.root;
    let _ = document.document.nodes.get(index as usize)?;
    let style = document.runtime_style(index, interaction);
    let color = document.resolve_color(style.background);
    if color.a == 0 { None } else { Some(color) }
}

fn blend_over_black(color: Color) -> Color {
    let alpha = color.a as u16;
    Color::rgb(
        ((color.r as u16 * alpha) / 255) as u8,
        ((color.g as u16 * alpha) / 255) as u8,
        ((color.b as u16 * alpha) / 255) as u8,
    )
}

fn rgb_to_pixel(r: u8, g: u8, b: u8, red_mask: u64, green_mask: u64, blue_mask: u64) -> u64 {
    channel_to_mask(r, red_mask) | channel_to_mask(g, green_mask) | channel_to_mask(b, blue_mask)
}

fn channel_to_mask(value: u8, mask: u64) -> u64 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let normalized = mask >> shift;
    let max = normalized;
    let scaled = ((value as u128 * max as u128) + 127) / 255;
    ((scaled as u64) << shift) & mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_rgb_into_common_truecolor_masks() {
        assert_eq!(rgb_to_pixel(0x12, 0x34, 0x56, 0x00ff0000, 0x0000ff00, 0x000000ff), 0x00123456);
    }

    #[test]
    fn alpha_blends_over_black() {
        assert_eq!(blend_over_black(Color { r: 200, g: 100, b: 50, a: 128 }), Color::rgb(100, 50, 25));
    }
}
