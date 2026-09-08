use super::*;

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

impl X11App {
    pub(crate) unsafe fn event_loop_once<F>(
        &mut self,
        document: &mut RuntimeDocument,
        on_action: &mut F,
    ) -> Result<(), String>
    where
        F: FnMut(&ActionEvent, &mut RuntimeDocument) -> Result<(), String>,
    {
        self.single_event = true;
        // SAFETY: This wrapper is itself called under the same live-display invariant.
        let result = unsafe { self.event_loop(document, on_action) };
        self.single_event = false;
        result
    }

    pub(crate) unsafe fn event_loop<F>(
        &mut self,
        document: &mut RuntimeDocument,
        on_action: &mut F,
    ) -> Result<(), String>
    where
        F: FnMut(&ActionEvent, &mut RuntimeDocument) -> Result<(), String>,
    {
        // SAFETY: The caller owns a live X11 display and app resources for the loop.
        unsafe { self.redraw(document)? };
        loop {
            let mut event = MaybeUninit::<XEvent>::uninit();
            // SAFETY: XNextEvent initializes the supplied XEvent storage before returning.
            unsafe { XNextEvent(self.display, event.as_mut_ptr()) };
            // SAFETY: XNextEvent wrote a complete XEvent into the MaybeUninit storage.
            let event = unsafe { event.assume_init() };
            // SAFETY: xany is the common XEvent prefix and is valid for every event type.
            if self.single_event && unsafe { event.xany.window } != self.window {
                // SAFETY: The event was returned by XNextEvent and remains initialized.
                unsafe { XPutBackEvent(self.display, &event as *const XEvent as *mut XEvent) };
                return Ok(());
            }
            // SAFETY: type_ is the common discriminator at the start of every XEvent variant.
            match unsafe { event.type_ } {
                EXPOSE => {
                    // SAFETY: The discriminator identifies XExposeEvent in this match arm.
                    if unsafe { event.xexpose.count } == 0 {
                        // SAFETY: The caller owns a live X11 display and app resources.
                        unsafe { self.redraw(document)? };
                    }
                }
                MAP_NOTIFY => {
                    // Native presenter: MapNotify advances Painted -> Mapped.
                    // Stray MapNotify in any other state is ignored, never an error.
                    let _ = self.note_mapped();
                }
                CONFIGURE_NOTIFY => {
                    // SAFETY: The discriminator identifies XConfigureEvent in this match arm.
                    let configure = unsafe { event.xconfigure };
                    let width = configure.width.max(1) as u32;
                    let height = configure.height.max(1) as u32;
                    if width != self.width || height != self.height {
                        self.width = width;
                        self.height = height;
                        // SAFETY: The display, window, and retained backbuffer belong to this app.
                        unsafe { self.recreate_backbuffer()? };
                        // SAFETY: The caller owns a live X11 display and app resources.
                        unsafe { self.redraw(document)? };
                        // SAFETY: window and display are live; mask follows resize.
                        unsafe { self.refresh_shape_mask(document) };
                    }
                }
                MOTION_NOTIFY => {
                    // Pointer motion can arrive much faster than a full scene redraw.
                    // Always consume the newest queued motion for this window before
                    // mutating layout/painting so interaction never trails behind the
                    // physical pointer by a backlog of stale coordinates.
                    // SAFETY: The discriminator identifies XMotionEvent in this match arm.
                    let mut motion = unsafe { event.xmotion };
                    loop {
                        // Only coalesce contiguous MotionNotify events. Do not
                        // search past ButtonRelease/other events, because doing
                        // so would violate X event ordering and could extend a
                        // drag beyond its release point. QUEUED_AFTER_READING also
                        // pulls already-arrived socket input into Xlib when the
                        // in-memory queue empties, preventing a nested X server
                        // from feeding us stale motion one packet at a time.
                        // SAFETY: self.display is the live display used by this event loop.
                        if unsafe { XEventsQueued(self.display, QUEUED_AFTER_READING) } <= 0 {
                            break;
                        }
                        let mut peeked = MaybeUninit::<XEvent>::uninit();
                        // SAFETY: XPeekEvent initializes the supplied XEvent storage.
                        unsafe { XPeekEvent(self.display, peeked.as_mut_ptr()) };
                        // SAFETY: XPeekEvent wrote a complete XEvent into the storage.
                        let peeked = unsafe { peeked.assume_init() };
                        // SAFETY: type_ and xany are the common XEvent prefix.
                        if unsafe { peeked.type_ } != MOTION_NOTIFY
                            // SAFETY: xany is valid for the event after checking its common prefix.
                            || unsafe { peeked.xany.window } != self.window
                        {
                            break;
                        }
                        let mut queued = MaybeUninit::<XEvent>::uninit();
                        // SAFETY: XNextEvent initializes the supplied XEvent storage.
                        unsafe { XNextEvent(self.display, queued.as_mut_ptr()) };
                        // SAFETY: The peeked event established a contiguous same-window motion
                        // event; XNextEvent removes that exact event from the queue.
                        let queued = unsafe { queued.assume_init() };
                        // SAFETY: The contiguous event was checked as MOTION_NOTIFY above.
                        motion = unsafe { queued.xmotion };
                    }
                    let hover = self.hit_test(document, motion.x as f32, motion.y as f32);
                    let changed = hover != self.interaction.hover;
                    self.interaction.hover = hover;
                    self.update_cursor(document, hover);
                    // Range drag: active node is a slider track; emit semantic
                    // delta via existing ActionEvent path (Motion, text=value).
                    if let Some((drag_index, track_x, track_w, min, max)) = self.range_drag {
                        let scale = document.ui_scale();
                        let track = Rect {
                            x: track_x,
                            y: 0.0,
                            width: track_w,
                            height: 8.0,
                        };
                        let value = slider_value_from_pointer(
                            track,
                            min,
                            max,
                            0.0,
                            motion.x as f32 / scale,
                        );
                        let action = document.document.nodes[drag_index as usize].action.clone();
                        if !action.is_empty() {
                            on_action(
                                &ActionEvent {
                                    action,
                                    phase: ActionPhase::Motion,
                                    button: self.pointer_button.unwrap_or(1),
                                    x: motion.x as f32 / scale,
                                    y: motion.y as f32 / scale,
                                    inside: hover == Some(drag_index),
                                    time_ms: motion.time as u64,
                                    text: Some(value.to_string()),
                                },
                                document,
                            )?;
                            unsafe { self.redraw(document)? };
                            if self.single_event {
                                return Ok(());
                            }
                            continue;
                        }
                    }
                    if let (Some(button), Some(index)) =
                        (self.pointer_button, self.interaction.active)
                    {
                        let action = document.document.nodes[index as usize].action.clone();
                        if !action.is_empty() {
                            on_action(
                                &ActionEvent {
                                    action,
                                    phase: ActionPhase::Motion,
                                    button,
                                    x: motion.x as f32 / document.ui_scale(),
                                    y: motion.y as f32 / document.ui_scale(),
                                    inside: hover == Some(index),
                                    time_ms: motion.time as u64,
                                    text: None,
                                },
                                document,
                            )?;
                            // SAFETY: The caller owns a live X11 display and app resources.
                            unsafe { self.redraw(document)? };
                        } else if changed {
                            // SAFETY: The caller owns a live X11 display and app resources.
                            unsafe { self.redraw(document)? };
                        }
                    } else if changed {
                        if let Some(index) = hover {
                            let action = document.document.nodes[index as usize].action.clone();
                            if !action.is_empty() {
                                on_action(
                                    &ActionEvent {
                                        action,
                                        phase: ActionPhase::Hover,
                                        button: 0,
                                        x: motion.x as f32 / document.ui_scale(),
                                        y: motion.y as f32 / document.ui_scale(),
                                        inside: true,
                                        time_ms: motion.time as u64,
                                        text: None,
                                    },
                                    document,
                                )?;
                            }
                        }
                        // SAFETY: The caller owns a live X11 display and app resources.
                        unsafe { self.redraw(document)? };
                    }
                }
                LEAVE_NOTIFY => {
                    let had_hover = self.interaction.hover.take().is_some();
                    self.set_cursor(CursorKind::Default);
                    if had_hover {
                        // SAFETY: The caller owns a live X11 display and app resources.
                        unsafe { self.redraw(document)? };
                    }
                }
                KEY_PRESS => {
                    // SAFETY: The discriminator identifies XKeyEvent in this match arm.
                    let mut key = unsafe { event.xkey };
                    // SAFETY: key points to the initialized XKeyEvent copied from XEvent.
                    let symbol = unsafe { XLookupKeysym(&mut key, 0) };
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
                        on_action(
                            &ActionEvent {
                                action: action.to_string(),
                                phase: ActionPhase::Release,
                                button: 0,
                                x: key.x as f32 / document.ui_scale(),
                                y: key.y as f32 / document.ui_scale(),
                                inside: true,
                                time_ms: key.time as u64,
                                text: None,
                            },
                            document,
                        )?;
                        // SAFETY: The caller owns a live X11 display and app resources.
                        unsafe { self.redraw(document)? };
                    } else if !ctrl && !alt && !super_key {
                        let shift_index = if key.state & SHIFT_MASK != 0 { 1 } else { 0 };
                        // SAFETY: key points to the initialized XKeyEvent copied from XEvent.
                        let text_symbol = unsafe { XLookupKeysym(&mut key, shift_index) };
                        if let Some(text) = keyboard_text(text_symbol) {
                            on_action(
                                &ActionEvent {
                                    action: "keyboard.input".to_string(),
                                    phase: ActionPhase::Release,
                                    button: 0,
                                    x: key.x as f32 / document.ui_scale(),
                                    y: key.y as f32 / document.ui_scale(),
                                    inside: true,
                                    time_ms: key.time as u64,
                                    text: Some(text),
                                },
                                document,
                            )?;
                            // SAFETY: The caller owns a live X11 display and app resources.
                            unsafe { self.redraw(document)? };
                        }
                    }
                }
                KEY_RELEASE => {
                    // SAFETY: The discriminator identifies XKeyEvent in this match arm.
                    let mut key = unsafe { event.xkey };
                    // SAFETY: key points to the initialized XKeyEvent copied from XEvent.
                    let symbol = unsafe { XLookupKeysym(&mut key, 0) };
                    if symbol == XK_SUPER_L || symbol == XK_SUPER_R {
                        if !self.super_chord_used {
                            on_action(
                                &ActionEvent {
                                    action: "keyboard.start.0".to_string(),
                                    phase: ActionPhase::Release,
                                    button: 0,
                                    x: key.x as f32 / document.ui_scale(),
                                    y: key.y as f32 / document.ui_scale(),
                                    inside: true,
                                    time_ms: key.time as u64,
                                    text: None,
                                },
                                document,
                            )?;
                            // SAFETY: The caller owns a live X11 display and app resources.
                            unsafe { self.redraw(document)? };
                        }
                        self.super_chord_used = false;
                    }
                }
                BUTTON_PRESS => {
                    // SAFETY: The discriminator identifies XButtonEvent in this match arm.
                    let button = unsafe { event.xbutton };
                    let raw = pointer_button_from_raw(button.button);
                    // Wheel Button4/5: scroll delta via existing ActionEvent path
                    // (semantic "scroll" action, Motion phase), no press state.
                    if let Some((dx, dy)) = wheel_scroll_delta(raw) {
                        let scale = document.ui_scale();
                        on_action(
                            &ActionEvent {
                                action: "scroll".to_string(),
                                phase: ActionPhase::Motion,
                                button: button.button,
                                x: button.x as f32 / scale + dx * 0.0,
                                y: button.y as f32 / scale,
                                inside: true,
                                time_ms: button.time as u64,
                                text: Some(format!("{dx},{dy}")),
                            },
                            document,
                        )?;
                        // SAFETY: The caller owns a live X11 display and app resources.
                        unsafe { self.redraw(document)? };
                        if self.single_event {
                            return Ok(());
                        }
                        continue;
                    }
                    let hit = self.hit_test(document, button.x as f32, button.y as f32);
                    if button.button == 1 {
                        if let Some(index) = hit {
                            let action = document.document.nodes[index as usize].action.clone();
                            let now = button.time as u64;
                            let double_title = action.starts_with("window.")
                                && action.ends_with(".move")
                                && self
                                    .last_title_press
                                    .map(|(last_index, last_time)| {
                                        last_index == index && now.saturating_sub(last_time) <= 350
                                    })
                                    .unwrap_or(false);
                            if double_title {
                                self.last_title_press = None;
                                self.interaction.active = None;
                                self.pointer_button = None;
                                if let Some(prefix) = action
                                    .strip_prefix("window.")
                                    .and_then(|rest| rest.strip_suffix(".move"))
                                {
                                    on_action(
                                        &ActionEvent {
                                            action: format!("{prefix}.maximize"),
                                            phase: ActionPhase::Release,
                                            button: button.button,
                                            x: button.x as f32 / document.ui_scale(),
                                            y: button.y as f32 / document.ui_scale(),
                                            inside: true,
                                            time_ms: button.time as u64,
                                            text: None,
                                        },
                                        document,
                                    )?;
                                }
                                // SAFETY: The caller owns a live X11 display and app resources.
                                unsafe { self.redraw(document)? };
                                return Ok(());
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
                                on_action(
                                    &ActionEvent {
                                        action,
                                        phase: ActionPhase::Press,
                                        button: button.button,
                                        x: button.x as f32 / document.ui_scale(),
                                        y: button.y as f32 / document.ui_scale(),
                                        inside: true,
                                        time_ms: button.time as u64,
                                        text: None,
                                    },
                                    document,
                                )?;
                            }
                        }
                        // Range drag arm: action names starting with "range." or
                        // "slider." capture the press as a slider drag; pointer
                        // geometry comes from the retained layout box.
                        if let Some(index) = hit {
                            let press_action =
                                document.document.nodes[index as usize].action.clone();
                            if press_action.starts_with("range.")
                                || press_action.starts_with("slider.")
                            {
                                if let Some(rect) = self.node_global_rect(document, index) {
                                    self.range_drag = Some((
                                        index,
                                        rect.x / document.ui_scale(),
                                        rect.width / document.ui_scale(),
                                        0.0,
                                        100.0,
                                    ));
                                }
                            }
                        }
                        // SAFETY: The caller owns a live X11 display and app resources.
                        unsafe { self.redraw(document)? };
                    }
                }
                BUTTON_RELEASE => {
                    // SAFETY: The discriminator identifies XButtonEvent in this match arm.
                    let button = unsafe { event.xbutton };
                    // End any active range drag; emit final value via Release.
                    if button.button == 1 {
                        if let Some((drag_index, track_x, track_w, min, max)) =
                            self.range_drag.take()
                        {
                            let scale = document.ui_scale();
                            let value = slider_value_from_pointer(
                                Rect {
                                    x: track_x,
                                    y: 0.0,
                                    width: track_w,
                                    height: 8.0,
                                },
                                min,
                                max,
                                0.0,
                                button.x as f32 / scale,
                            );
                            let action =
                                document.document.nodes[drag_index as usize].action.clone();
                            if !action.is_empty() {
                                on_action(
                                    &ActionEvent {
                                        action,
                                        phase: ActionPhase::Release,
                                        button: button.button,
                                        x: button.x as f32 / scale,
                                        y: button.y as f32 / scale,
                                        inside: true,
                                        time_ms: button.time as u64,
                                        text: Some(value.to_string()),
                                    },
                                    document,
                                )?;
                            }
                        }
                    }
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
                            on_action(
                                &ActionEvent {
                                    action,
                                    phase: ActionPhase::Release,
                                    button: button.button,
                                    x: button.x as f32 / document.ui_scale(),
                                    y: button.y as f32 / document.ui_scale(),
                                    inside: hit == Some(index),
                                    time_ms: button.time as u64,
                                    text: None,
                                },
                                document,
                            )?;
                        }
                    }
                    // SAFETY: The caller owns a live X11 display and app resources.
                    unsafe { self.redraw(document)? };
                    let hover = self.hit_test(document, button.x as f32, button.y as f32);
                    self.interaction.hover = hover;
                    self.update_cursor(document, hover);
                }
                CLIENT_MESSAGE => {
                    // SAFETY: The discriminator identifies XClientMessageEvent in this arm.
                    let client = unsafe { event.xclient };
                    if self.wm_delete != 0
                        && client.format == 32
                        // SAFETY: XClientMessageEvent format 32 selects the initialized long data
                        // representation used by the WM_DELETE protocol.
                        && unsafe { client.data.l[0] } as u64 == self.wm_delete
                    {
                        self.close_requested = true;
                    }
                }
                _ => {}
            }
            if self.single_event {
                return Ok(());
            }
        }
    }

    pub(crate) unsafe fn event_loop_with_reactor<F, R>(
        &mut self,
        document: &mut RuntimeDocument,
        on_action: &mut F,
        reactor: &mut Reactor,
        on_reactor: &mut R,
    ) -> Result<(), String>
    where
        F: FnMut(&ActionEvent, &mut RuntimeDocument) -> Result<(), String>,
        R: FnMut(&mut RuntimeDocument) -> Result<(), String>,
    {
        // SAFETY: The caller owns a live X11 display and app resources.
        unsafe { self.redraw(document)? };
        loop {
            reactor.dispatch(None).map_err(|error| error.to_string())?;
            on_reactor(document)?;
            // SAFETY: self.display is the live display owned by this app.
            if unsafe { XEventsQueued(self.display, QUEUED_AFTER_READING) } <= 0 {
                continue;
            }
            self.single_event = true;
            // SAFETY: The caller owns the live X11 display and app resources.
            unsafe { self.event_loop(document, on_action)? };
            self.single_event = false;
            if self.close_requested {
                return Ok(());
            }
        }
    }

    pub(crate) fn hit_test(&self, document: &RuntimeDocument, x: f32, y: f32) -> Option<u32> {
        let scale = document.ui_scale();
        self.layout
            .as_ref()
            .and_then(|layout| layout.hit_test_action(document, x / scale, y / scale))
    }

    pub(crate) unsafe fn refresh_shape_mask(&self, document: &RuntimeDocument) {
        // Top-level rounded popup/menu/dropdown visible shape: root radius
        // plus surface bounds. Square surfaces skip the bridge entirely.
        let radius = flamewm_render_core::paint::root_corner_radius(document, self.interaction);
        let shape =
            flamewm_render_core::paint::surface_shape_pixels(self.width, self.height, radius);
        if shape.2 == 0 {
            return;
        }
        let Some(bridge) = self.xshape.as_ref() else {
            eprintln!(
                "FLAMEWM_RENDER_SHAPE_BACKEND degraded-square-corners reason=xshape-unavailable"
            );
            return;
        };
        let spans = rounded_mask_spans(shape.0, shape.1, shape.2);
        // SAFETY: window is a live X window on the bridge's display.
        if let Err(error) = unsafe { bridge.apply_rounded_mask(self.window, &spans) } {
            eprintln!("FLAMEWM_RENDER_SHAPE_BACKEND degraded-square-corners reason={error}");
        }
    }

    pub(crate) unsafe fn recreate_backbuffer(&mut self) -> Result<(), String> {
        // SAFETY: self.display and self.window are live handles owned by this app.
        let replacement = unsafe {
            XCreatePixmap(
                self.display,
                self.window,
                self.width,
                self.height,
                self.depth as u32,
            )
        };
        if replacement == 0 {
            return Err("XCreatePixmap failed while resizing retained backbuffer".to_string());
        }
        if let Some(xft) = self.xft.as_mut() {
            // SAFETY: `replacement` is a fresh pixmap on the live display.
            unsafe { xft.set_drawable(replacement) };
        }
        if let Some(xrender) = self.xrender.as_mut() {
            // SAFETY: `replacement` is a fresh pixmap on the live display.
            if let Err(error) = unsafe { xrender.set_drawable(replacement) } {
                if let Some(xft) = self.xft.as_mut() {
                    // SAFETY: restore the previous live backbuffer on the same display.
                    unsafe { xft.set_drawable(self.backbuffer) };
                }
                // SAFETY: replacement was created by XCreatePixmap on self.display.
                unsafe { XFreePixmap(self.display, replacement) };
                return Err(error);
            }
        }
        let previous = self.backbuffer;
        self.backbuffer = replacement;
        if previous != 0 {
            // SAFETY: previous is the app-owned pixmap replaced above.
            unsafe { XFreePixmap(self.display, previous) };
        }
        Ok(())
    }
}
