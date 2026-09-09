use std::os::fd::{AsFd, AsRawFd, RawFd};

use super::*;
use crate::xlib::*;
use flamewm_reactor::Reactor;
use flamewm_render_core::SurfaceDamage;

/// Reserved generic action for a ButtonRelease that lands outside the
/// grabbed surface while a pointer grab is active. Feature modules share
/// this single transient-lifecycle signal; it is never Start-specific.
pub const OUTSIDE_RELEASE_ACTION: &str = "surface.outside.release";

/// Pure bounds check: window-relative press/release coords outside the
/// grabbed surface rect. With an active pointer grab X reports releases
/// to the grab window even when the pointer is outside, so coords outside
/// `[0, width) x [0, height)` mean "outside the grabbed surface".
pub fn button_release_is_outside(x: i32, y: i32, width: u32, height: u32) -> bool {
    x < 0 || y < 0 || x >= width.max(1) as i32 || y >= height.max(1) as i32
}

/// Build the generic outside-release controller event for one surface.
pub fn outside_release_event(
    surface: SurfaceId,
    button: u32,
    x: f32,
    y: f32,
    time_ms: u64,
) -> SurfaceControllerEvent {
    SurfaceControllerEvent {
        surface,
        event: ActionEvent {
            action: OUTSIDE_RELEASE_ACTION.to_string(),
            phase: ActionPhase::Release,
            button,
            x,
            y,
            inside: false,
            time_ms,
            text: None,
        },
    }
}

pub fn run_with_surface_controller_events<F>(
    mut document: RuntimeDocument,
    config: X11Config,
    mut on_event: F,
) -> Result<(), String>
where
    F: FnMut(&SurfaceControllerEvent, &mut RuntimeDocument) -> Result<(), String>,
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
        let surface = SurfaceId(1);
        let result = app.event_loop(&mut document, &mut |event, document| {
            on_event(
                &SurfaceControllerEvent {
                    surface,
                    event: event.clone(),
                },
                document,
            )
        });
        drop(app);
        XCloseDisplay(display);
        result
    }
}

struct SurfaceInstance {
    app: X11App,
    document: RuntimeDocument,
    role: SurfaceRole,
    input: SurfaceInputMode,
}

pub struct SurfaceController {
    display: *mut Display,
    screen: i32,
    next_id: u64,
    surfaces: HashMap<SurfaceId, SurfaceInstance>,
    windows: HashMap<Window, SurfaceId>,
}

impl SurfaceController {
    pub fn new() -> Result<Self, String> {
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err("XOpenDisplay failed; DISPLAY is unset or unreachable".to_string());
            }
            Ok(Self {
                screen: XDefaultScreen(display),
                display,
                next_id: 1,
                surfaces: HashMap::new(),
                windows: HashMap::new(),
            })
        }
    }

    pub fn connection_fd(&self) -> RawFd {
        unsafe { XConnectionNumber(self.display) as RawFd }
    }

    pub fn root_geometry(&self) -> (u32, u32) {
        unsafe {
            (
                XDisplayWidth(self.display, self.screen).max(0) as u32,
                XDisplayHeight(self.display, self.screen).max(0) as u32,
            )
        }
    }

    pub fn create_surface(
        &mut self,
        document: RuntimeDocument,
        config: SurfaceConfig,
    ) -> Result<SurfaceId, String> {
        let id = SurfaceId(self.next_id);
        self.next_id = self.next_id.checked_add(1).ok_or("surface id exhausted")?;
        let (role, input) = (config.role, config.input);
        let x11_config = X11Config::from(config.clone());
        let app =
            unsafe { X11App::new_surface(self.display, &x11_config, config.x, config.y, false)? };
        let window = app.window;
        let mut instance = SurfaceInstance {
            app,
            document,
            role,
            input,
        };
        // Overlay/passthrough surfaces are excluded from WM client
        // management: override-redirect + empty input region + no grab/focus.
        if overlay_excluded_from_wm(role) {
            unsafe {
                let mut attributes = XSetWindowAttributes {
                    background_pixmap: 0,
                    background_pixel: TRANSPARENT_CLEAR_PIXEL,
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
                XChangeWindowAttributes(
                    self.display,
                    window,
                    CW_OVERRIDE_REDIRECT,
                    &mut attributes,
                );
            }
        }
        if input == SurfaceInputMode::PassThrough {
            if let Some(bridge) = instance.app.xshape.as_ref() {
                // Empty input region: pointer falls through the surface.
                if let Err(error) = unsafe { bridge.apply_input_mask(window, &[]) } {
                    return Err(error);
                }
            } else {
                return Err(
                    "passthrough surface requires XShape input region (unavailable)".to_string(),
                );
            }
        }
        // Hidden create: mark Full dirty, no layout/paint/present/map/shape.
        // First show renders retained offscreen, then maps (popup map+raise).
        if !config.initially_visible {
            instance.app.mark_full();
            instance.document.mark_dirty(SurfaceDamage::Full);
            self.windows.insert(window, id);
            self.surfaces.insert(id, instance);
            return Ok(id);
        }
        // Visible create: paint retained offscreen first (redraw refreshes
        // shape once), then map (popup map+raise) + present/flush.
        unsafe {
            instance.app.redraw(&instance.document)?;
        }
        unsafe {
            if surface_role_needs_popup_raise(role) {
                XMapWindow(self.display, window);
                XRaiseWindow(self.display, window);
            } else {
                XMapWindow(self.display, window);
            }
            instance.app.present_scene()?;
            XFlush(self.display);
        }
        self.windows.insert(window, id);
        self.surfaces.insert(id, instance);
        Ok(id)
    }

    pub fn destroy_surface(&mut self, id: SurfaceId) -> Result<(), String> {
        if let Some(mut instance) = self.surfaces.remove(&id) {
            self.windows.remove(&instance.app.window);
            if instance.app.pointer_grabbed {
                unsafe {
                    XUngrabPointer(self.display, CURRENT_TIME);
                }
                instance.app.pointer_grabbed = false;
            }
            Ok(())
        } else {
            Err(format!("unknown surface {}", id.0))
        }
    }

    pub fn show(&mut self, id: SurfaceId) -> Result<(), String> {
        // First-show lifecycle: render the retained scene offscreen, refresh
        // shape once, then map (popup: map+raise atomically) + present/flush
        // so the first mapped frame is never blank. Second show sees clean
        // damage and maps/presents without repainting.
        let role = self.surface_role(id)?;
        let display = self.display;
        let instance = self.instance_mut(id)?;
        let painted = unsafe { instance.app.redraw_if_dirty(&mut instance.document)? };
        if painted {
            // SAFETY: display/window live; shape follows retained repaint.
            unsafe { instance.app.refresh_shape_mask(&instance.document) };
        }
        unsafe {
            if surface_role_needs_popup_raise(role) {
                XMapWindow(display, instance.app.window);
                XRaiseWindow(display, instance.app.window);
            } else {
                XMapWindow(display, instance.app.window);
            }
        }
        unsafe {
            instance.app.present_scene()?;
            XFlush(display);
        }
        Ok(())
    }
    pub fn hide(&mut self, id: SurfaceId) -> Result<(), String> {
        let display = self.display;
        let instance = self.instance_mut(id)?;
        if instance.app.pointer_grabbed {
            unsafe {
                XUngrabPointer(display, CURRENT_TIME);
            }
            instance.app.pointer_grabbed = false;
        }
        unsafe {
            XUnmapWindow(display, instance.app.window);
            XFlush(display);
        }
        Ok(())
    }
    pub fn raise(&self, id: SurfaceId) -> Result<(), String> {
        self.with_window(id, |display, window| unsafe {
            XRaiseWindow(display, window)
        })
    }

    pub fn move_resize(
        &self,
        id: SurfaceId,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        self.with_window(id, |display, window| unsafe {
            XMoveResizeWindow(display, window, x, y, width.max(1), height.max(1))
        })?;
        let instance = self
            .surfaces
            .get(&id)
            .ok_or_else(|| format!("unknown surface {}", id.0))?;
        // SAFETY: display/window live; mask follows move/resize.
        unsafe { instance.app.refresh_shape_mask(&instance.document) };
        Ok(())
    }

    pub fn grab_pointer(&mut self, id: SurfaceId) -> Result<(), String> {
        let instance = self
            .surfaces
            .get(&id)
            .ok_or_else(|| format!("unknown surface {}", id.0))?;
        if !instance.input.allows_grab() {
            return Err(format!(
                "surface {} is pass-through; pointer grab refused",
                id.0
            ));
        }
        let display = self.display;
        let instance = self.instance_mut(id)?;
        let result = unsafe {
            XGrabPointer(
                display,
                instance.app.window,
                1,
                (BUTTON_PRESS_MASK | BUTTON_RELEASE_MASK | POINTER_MOTION_MASK) as u32,
                GRAB_MODE_ASYNC,
                GRAB_MODE_ASYNC,
                0,
                0,
                CURRENT_TIME,
            )
        };
        if result != 0 {
            return Err(format!("XGrabPointer failed for surface {}", id.0));
        }
        instance.app.pointer_grabbed = true;
        unsafe {
            XFlush(display);
        }
        Ok(())
    }

    pub fn ungrab_pointer(&mut self, id: SurfaceId) -> Result<(), String> {
        let display = self.display;
        let instance = self.instance_mut(id)?;
        unsafe {
            XUngrabPointer(display, CURRENT_TIME);
        }
        instance.app.pointer_grabbed = false;
        unsafe {
            XFlush(display);
        }
        Ok(())
    }

    pub fn redraw(&mut self, id: SurfaceId) -> Result<(), String> {
        let _guard = flamewm_profiler::start("render.present");
        let instance = self.instance_mut(id)?;
        unsafe { instance.app.redraw(&instance.document) }
    }

    pub fn redraw_dirty(&mut self) -> Result<(), String> {
        let _guard = flamewm_profiler::start("render.present");
        let ids: Vec<SurfaceId> = self.surfaces.keys().copied().collect();
        for id in ids {
            let instance = self.instance_mut(id)?;
            // SAFETY: instance owns a live display + app resources.
            let painted = unsafe { instance.app.redraw_if_dirty(&mut instance.document)? };
            if painted {
                let instance = self.instance_mut(id)?;
                // SAFETY: display/window live; mask follows repaint.
                unsafe { instance.app.refresh_shape_mask(&instance.document) };
            }
        }
        Ok(())
    }

    /// Mark damage on one surface without painting (merged max coverage).
    pub fn mark_surface_dirty(
        &mut self,
        id: SurfaceId,
        damage: SurfaceDamage,
    ) -> Result<(), String> {
        self.instance_mut(id)?.app.mark_dirty(damage);
        self.instance_mut(id)?.document.mark_dirty(damage);
        Ok(())
    }

    /// Peek the merged damage latch for one surface (no consume).
    pub fn peek_surface_damage(&self, id: SurfaceId) -> Result<SurfaceDamage, String> {
        let instance = self
            .surfaces
            .get(&id)
            .ok_or_else(|| format!("unknown surface {}", id.0))?;
        Ok(instance
            .app
            .pending_damage
            .merge(instance.document.peek_damage()))
    }

    pub fn pump_events<F>(&mut self, mut on_action: F) -> Result<usize, String>
    where
        F: FnMut(&SurfaceControllerEvent, &mut RuntimeDocument) -> Result<(), String>,
    {
        let mut handled = 0;
        loop {
            let queued = unsafe { XEventsQueued(self.display, QUEUED_AFTER_READING) };
            if queued <= 0 {
                break;
            }
            let mut event = MaybeUninit::<XEvent>::uninit();
            unsafe {
                XNextEvent(self.display, event.as_mut_ptr());
            }
            let event = unsafe { event.assume_init() };
            let Some(id) = self.windows.get(&unsafe { event.xany.window }).copied() else {
                continue;
            };
            let event_type = unsafe { event.type_ };
            let outside_release = if event_type == BUTTON_RELEASE {
                let button = unsafe { event.xbutton };
                self.surfaces
                    .get(&id)
                    .map(|instance| {
                        instance.app.pointer_grabbed
                            && button_release_is_outside(
                                button.x,
                                button.y,
                                instance.app.width,
                                instance.app.height,
                            )
                    })
                    .unwrap_or(false)
            } else {
                false
            };
            unsafe {
                XPutBackEvent(self.display, &event as *const XEvent as *mut XEvent);
            }
            let release_info = if outside_release {
                let button = unsafe { event.xbutton };
                Some((button.button, button.x, button.y, button.time as u64))
            } else {
                None
            };
            let closed = {
                let instance = self.instance_mut(id)?;
                let mut callback = |event: &ActionEvent, document: &mut RuntimeDocument| {
                    on_action(
                        &SurfaceControllerEvent {
                            surface: id,
                            event: event.clone(),
                        },
                        document,
                    )
                };
                unsafe {
                    instance
                        .app
                        .event_loop_once(&mut instance.document, &mut callback)?;
                }
                // Drain the document-owned damage latch into the app latch so
                // `redraw_dirty` can coalesce; event_loop already merged its
                // own marks via redraw_if_dirty.
                let pending = instance.document.take_damage();
                instance.app.mark_dirty(pending);
                instance.app.close_requested
            };
            if let Some((button, x, y, time_ms)) = release_info {
                let instance = self.instance_mut(id)?;
                let scale = instance.document.ui_scale();
                let outside =
                    outside_release_event(id, button, x as f32 / scale, y as f32 / scale, time_ms);
                on_action(&outside, &mut instance.document)?;
            }
            handled += 1;
            if closed {
                self.destroy_surface(id)?;
            }
        }
        Ok(handled)
    }

    pub fn document_mut(&mut self, id: SurfaceId) -> Result<&mut RuntimeDocument, String> {
        Ok(&mut self.instance_mut(id)?.document)
    }

    pub fn controller_event(&self, id: SurfaceId, event: ActionEvent) -> SurfaceControllerEvent {
        SurfaceControllerEvent { surface: id, event }
    }

    pub fn surface_role(&self, id: SurfaceId) -> Result<SurfaceRole, String> {
        Ok(self
            .surfaces
            .get(&id)
            .ok_or_else(|| format!("unknown surface {}", id.0))?
            .role)
    }

    pub fn surface_input_mode(&self, id: SurfaceId) -> Result<SurfaceInputMode, String> {
        Ok(self
            .surfaces
            .get(&id)
            .ok_or_else(|| format!("unknown surface {}", id.0))?
            .input)
    }

    /// Overlay management policy: overlays stay raisable but are never
    /// WM-managed, never focusable, and never in the workarea/taskbar.
    pub fn is_wm_managed(&self, id: SurfaceId) -> Result<bool, String> {
        Ok(!overlay_excluded_from_wm(self.surface_role(id)?))
    }

    pub fn is_pointer_grabbed(&self, id: SurfaceId) -> Result<bool, String> {
        Ok(self
            .surfaces
            .get(&id)
            .ok_or_else(|| format!("unknown surface {}", id.0))?
            .app
            .pointer_grabbed)
    }

    pub fn run_with_reactor<F, R>(
        &mut self,
        reactor: &mut Reactor,
        mut on_action: F,
        mut on_tick: R,
    ) -> Result<(), String>
    where
        F: FnMut(&SurfaceControllerEvent, &mut RuntimeDocument) -> Result<(), String>,
        R: FnMut(&mut Self) -> Result<(), String>,
    {
        let token = reactor
            .register_fd(
                ReactorPebble(self.connection_fd()),
                calloop::Interest::READ,
                move |_, _| {
                    // X readiness only wakes dispatch; pump happens below.
                },
            )
            .map_err(|error| error.to_string())?;
        loop {
            reactor.dispatch(None).map_err(|error| error.to_string())?;
            let drained = self.pump_events(&mut on_action)?;
            if self.surfaces.is_empty() {
                break;
            }
            // Timer sources registered by the caller also wake dispatch, so
            // tick callbacks always run; redraws follow drained X events.
            on_tick(self)?;
            if drained > 0 {
                self.redraw_dirty()?;
            }
        }
        reactor.remove(token).map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Dispatch one reactor cycle and return typed events after document callbacks finish.
    ///
    /// The UI bridge uses this boundary to apply surface operations without exposing the
    /// borrowed runtime document or holding the controller borrow across those operations.
    pub fn run_with_reactor_step(
        &mut self,
        reactor: &mut Reactor,
    ) -> Result<(Vec<SurfaceControllerEvent>, usize), String> {
        let token = reactor
            .register_fd(
                ReactorPebble(self.connection_fd()),
                calloop::Interest::READ,
                move |_, _| {
                    // X readiness only wakes dispatch; pump happens below.
                },
            )
            .map_err(|error| error.to_string())?;
        let mut events = Vec::new();
        let result: Result<usize, String> = (|| {
            reactor.dispatch(None).map_err(|error| error.to_string())?;
            let drained = self.pump_events(|event, _document| {
                events.push(event.clone());
                Ok(())
            })?;
            Ok(drained)
        })();
        reactor.remove(token).map_err(|error| error.to_string())?;
        Ok((events, result?))
    }

    pub fn has_surfaces(&self) -> bool {
        !self.surfaces.is_empty()
    }

    fn instance_mut(&mut self, id: SurfaceId) -> Result<&mut SurfaceInstance, String> {
        self.surfaces
            .get_mut(&id)
            .ok_or_else(|| format!("unknown surface {}", id.0))
    }

    fn with_window<F>(&self, id: SurfaceId, operation: F) -> Result<(), String>
    where
        F: FnOnce(*mut Display, Window) -> i32,
    {
        let instance = self
            .surfaces
            .get(&id)
            .ok_or_else(|| format!("unknown surface {}", id.0))?;
        if operation(self.display, instance.app.window) == 0 {
            return Err(format!("X11 operation failed for surface {}", id.0));
        }
        unsafe {
            XFlush(self.display);
        }
        Ok(())
    }
}

/// Borrowed X connection fd wrapper for reactor registration.
/// The display outlives the registration; calloop never owns the fd.
struct ReactorPebble(RawFd);

impl AsFd for ReactorPebble {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        unsafe { std::os::fd::BorrowedFd::borrow_raw(self.0) }
    }
}

impl AsRawFd for ReactorPebble {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outside_release_bounds_are_window_relative() {
        assert!(!button_release_is_outside(10, 10, 800, 600));
        assert!(!button_release_is_outside(0, 0, 800, 600));
        assert!(!button_release_is_outside(799, 599, 800, 600));
        assert!(button_release_is_outside(-1, 10, 800, 600));
        assert!(button_release_is_outside(10, -1, 800, 600));
        assert!(button_release_is_outside(800, 10, 800, 600));
        assert!(button_release_is_outside(10, 600, 800, 600));
    }

    #[test]
    fn outside_event_uses_reserved_generic_action() {
        let event = outside_release_event(SurfaceId(3), 1, 850.0, 10.0, 42);
        assert_eq!(event.surface, SurfaceId(3));
        assert_eq!(event.event.action, OUTSIDE_RELEASE_ACTION);
        assert_eq!(event.event.phase, ActionPhase::Release);
        assert_eq!(event.event.button, 1);
        assert!(!event.event.inside);
        assert_eq!(event.event.time_ms, 42);
        assert!(event.event.text.is_none());
    }

    #[test]
    fn surface_id_round_trips() {
        let id = SurfaceId(7);
        assert_eq!(id.get(), 7);
    }

    #[test]
    fn overlay_maps_to_notification_type_and_is_unmanaged() {
        assert_eq!(
            overlay_window_type_name(SurfaceRole::Overlay),
            "_NET_WM_WINDOW_TYPE_NOTIFICATION"
        );
        assert!(overlay_excluded_from_wm(SurfaceRole::Overlay));
        assert!(!overlay_excluded_from_wm(SurfaceRole::Normal));
        assert!(!overlay_excluded_from_wm(SurfaceRole::Dock));
    }

    #[test]
    fn passthrough_refuses_grab_and_focus() {
        assert!(!SurfaceInputMode::PassThrough.allows_grab());
        assert!(!SurfaceInputMode::PassThrough.allows_focus());
        assert!(SurfaceInputMode::Interactive.allows_grab());
        assert!(SurfaceInputMode::Interactive.allows_focus());
    }

    #[test]
    fn config_maps_overlay_role() {
        let config = SurfaceConfig {
            role: SurfaceRole::Overlay,
            ..SurfaceConfig::default()
        };
        let x11: X11Config = config.into();
        assert_eq!(x11.role, X11WindowRole::Overlay);
    }

    #[test]
    fn popup_policy_maps_raise_for_transients_only() {
        assert!(!surface_role_needs_popup_raise(SurfaceRole::Normal));
        assert!(!surface_role_needs_popup_raise(SurfaceRole::Desktop));
        assert!(!surface_role_needs_popup_raise(SurfaceRole::Dock));
        assert!(surface_role_needs_popup_raise(SurfaceRole::PopupMenu));
        assert!(surface_role_needs_popup_raise(SurfaceRole::DropdownMenu));
        assert!(surface_role_needs_popup_raise(SurfaceRole::Overlay));
        assert!(!x11_role_needs_popup_raise(X11WindowRole::Normal));
        assert!(x11_role_needs_popup_raise(X11WindowRole::PopupMenu));
        assert!(x11_role_needs_popup_raise(X11WindowRole::DropdownMenu));
        assert!(x11_role_needs_popup_raise(X11WindowRole::Overlay));
    }

    #[test]
    fn damage_latch_merges_without_paint() {
        use flamewm_render_core::Rect;
        assert!(SurfaceDamage::None.is_empty());
        assert!(!SurfaceDamage::Full.is_empty());
        assert!(!SurfaceDamage::Region(Rect::default()).is_empty());
        assert_eq!(
            SurfaceDamage::None.merge(SurfaceDamage::Full),
            SurfaceDamage::Full
        );
        assert_eq!(
            SurfaceDamage::Full.merge(SurfaceDamage::None),
            SurfaceDamage::Full
        );
        assert_eq!(
            SurfaceDamage::None.merge(SurfaceDamage::None),
            SurfaceDamage::None
        );
    }
}

impl Drop for SurfaceController {
    fn drop(&mut self) {
        self.surfaces.clear();
        self.windows.clear();
        unsafe {
            XCloseDisplay(self.display);
        }
    }
}
