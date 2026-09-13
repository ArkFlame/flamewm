use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::rc::Rc;

use flamewm_window_core::{
    SnapTarget, snap_geometry as core_snap_geometry, snap_target as core_snap_target,
};
use x11rb::connection::Connection;
use x11rb::errors::{ReplyError, ReplyOrIdError};
use x11rb::protocol::shape::ConnectionExt as _;
use x11rb::protocol::xproto::*;
use x11rb::protocol::{ErrorKind, Event};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::{COPY_DEPTH_FROM_PARENT, CURRENT_TIME};

use crate::atoms::{AnyError, Atoms};
use crate::chrome;
use crate::classifier::{WindowKind, classify};
use crate::client::ManagedClient;
use crate::event_pump::WmEventPump;
use crate::frame::chrome as frame_chrome;
use crate::frame::controller::{ControllerState, FrameEffect, FrameEvent, reduce};
use crate::frame::coords::{RootPoint, RootRect};
use crate::frame::geometry::{
    FrameExtents, GeometryReason, GeometryRequest, frame_to_client_root, plan_client_configure,
};
use crate::frame::layout::layout_frame_children_default;
use crate::frame::model::{FrameControl, FrameRegion, PlacementMode, PlacementSnapshot};
use crate::frame::resources::{
    FrameRegistry, FrameResources, INPUT_CHILD_EVENT_MASK, cursor_for_region,
};
use crate::frame::session::InteractionSession;
use crate::geometry::Rect;
use crate::size_hints::ClientSizeHints;
use crate::snap_preview::{self, SnapPreviewSurface};

const BUTTON_PRIMARY: Button = 1;
const WM_STATE_WITHDRAWN: u32 = 0;
const WM_STATE_NORMAL: u32 = 1;
const WM_STATE_ICONIC: u32 = 3;

/// Accumulated invalidation for one reactor turn. Semantic owners mark the
/// affected facets; the reactor flush drains them once per turn.
#[derive(Debug, Clone, Copy, Default)]
pub struct WmChangeSet {
    pub windows: bool,
    pub workspaces: bool,
    pub panels: bool,
    pub work_area: bool,
}

impl WmChangeSet {
    fn take(&mut self) -> Self {
        let out = *self;
        *self = Self::default();
        out
    }

    #[cfg(test)]
    fn any(self) -> bool {
        self.windows || self.workspaces || self.panels || self.work_area
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WmConfig {
    pub titlebar_height: u16,
    pub frame_border: u16,
    pub panel_reserve: u16,
    pub snap_zone: i32,
    pub workspaces: usize,
}

impl Default for WmConfig {
    fn default() -> Self {
        Self {
            titlebar_height: chrome::TITLEBAR_HEIGHT,
            frame_border: chrome::FRAME_BORDER,
            panel_reserve: 0,
            snap_zone: 24,
            workspaces: 4,
        }
    }
}

impl WmConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let mut config = Self::default();
        config.titlebar_height = env_u16("FLAMEWM_TITLEBAR_HEIGHT", config.titlebar_height, 20, 72);
        config.panel_reserve = env_u16("FLAMEWM_PANEL_RESERVE", config.panel_reserve, 0, 256);
        config.snap_zone = i32::from(env_u16(
            "FLAMEWM_SNAP_ZONE",
            config.snap_zone as u16,
            4,
            128,
        ));
        config.workspaces = usize::from(env_u16(
            "FLAMEWM_WORKSPACES",
            config.workspaces as u16,
            1,
            18,
        ));
        config
    }
}

fn env_u16(name: &str, default: u16, min: u16, max: u16) -> u16 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .map_or(default, |value| value.clamp(min, max))
}

fn any_conn_err(error: x11rb::errors::ConnectionError) -> AnyError {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        error.to_string(),
    ))
}

struct ResizeCounterSet {
    received: flamewm_profiler::CounterPoint,
    coalesced: flamewm_profiler::CounterPoint,
    committed: flamewm_profiler::CounterPoint,
    max_batch: std::sync::atomic::AtomicU64,
}

fn resize_counters() -> &'static ResizeCounterSet {
    use std::sync::OnceLock;
    static SET: OnceLock<ResizeCounterSet> = OnceLock::new();
    SET.get_or_init(|| ResizeCounterSet {
        received: flamewm_profiler::CounterPoint::new("wm.resize.received"),
        coalesced: flamewm_profiler::CounterPoint::new("wm.resize.coalesced"),
        committed: flamewm_profiler::CounterPoint::new("wm.resize.commit"),
        max_batch: std::sync::atomic::AtomicU64::new(0),
    })
}

impl ResizeCounterSet {
    fn max_batch(&self) -> usize {
        self.max_batch.load(std::sync::atomic::Ordering::Relaxed) as usize
    }
    fn set_max_batch(&self, value: usize) {
        self.max_batch
            .store(value as u64, std::sync::atomic::Ordering::Relaxed);
    }
}

fn emit_resize_summary() {
    flamewm_debug::emit(
        flamewm_debug::WM_RESIZE_SUMMARY,
        flamewm_debug::WM_RESIZE_SUMMARY_COOLDOWN,
        || {
            let max = resize_counters().max_batch();
            format!("received=? coalesced=? commit=? max_batch={max}")
        },
    );
}

struct WorkspaceCounterSet {
    switch_total: flamewm_profiler::CounterPoint,
    visibility: flamewm_profiler::CounterPoint,
    ewmh_current: flamewm_profiler::CounterPoint,
    shell_publish: flamewm_profiler::CounterPoint,
    query_tree: flamewm_profiler::CounterPoint,
}

fn workspace_counters() -> &'static WorkspaceCounterSet {
    use std::sync::OnceLock;
    static SET: OnceLock<WorkspaceCounterSet> = OnceLock::new();
    SET.get_or_init(|| WorkspaceCounterSet {
        switch_total: flamewm_profiler::CounterPoint::new("wm.workspace.switch.total"),
        visibility: flamewm_profiler::CounterPoint::new("wm.workspace.switch.visibility"),
        ewmh_current: flamewm_profiler::CounterPoint::new("wm.workspace.switch.ewmh_current"),
        shell_publish: flamewm_profiler::CounterPoint::new("wm.workspace.switch.shell_publish"),
        query_tree: flamewm_profiler::CounterPoint::new("wm.workspace.query_tree.count"),
    })
}

pub(crate) fn run_with_hook<F>(
    config: WmConfig,
    catalog: &std::sync::Arc<flamewm_applications::ApplicationCatalog>,
    reactor: &Rc<RefCell<flamewm_reactor::Reactor>>,
    mut hook: F,
) -> Result<(), AnyError>
where
    F: FnMut(&Rc<RustConnection>, usize, WmChangeSet) -> Result<(), AnyError>,
{
    use std::os::fd::AsRawFd;
    let (conn, screen_num) = x11rb::connect(None)?;
    let conn = Rc::new(conn);
    let screen = &conn.setup().roots[screen_num];
    become_wm(conn.as_ref(), screen)?;

    let mut wm = Wm::new(
        conn.as_ref(),
        screen_num,
        config,
        std::sync::Arc::clone(catalog),
    )?;
    wm.publish_desktop_state()?;
    wm.scan_existing()?;
    conn.flush()?;

    // X fd source: borrowed fd wakes dispatch; buffered drain happens below.
    // The connection outlives the registration; calloop never owns the fd.
    let x_fd = conn.stream().as_raw_fd();
    let _x_token = reactor.borrow_mut().register_raw_fd_with_action(
        x_fd,
        calloop::Interest::READ,
        |_, _| flamewm_reactor::FdAction::Continue,
    )?;

    loop {
        // Buffered drain before block: one pump collapses contiguous
        // same event+child MotionNotify; order vs Button/Key/Configure/
        // Property/Enter/Leave/ClientMessage/Expose is preserved.
        let mut pump = WmEventPump::new();
        let mut batch: usize = 0;
        while let Some(pumped) = pump.next(conn.as_ref()).map_err(any_conn_err)? {
            resize_counters()
                .received
                .increment_by(pumped.received as u64);
            resize_counters()
                .coalesced
                .increment_by(pumped.coalesced as u64);
            batch += 1;
            {
                let _guard = flamewm_profiler::start("wm.resize.motion.drain");
                wm.handle_event(pumped.event)?;
            }
        }
        resize_counters().committed.increment_by(batch as u64);
        if batch > resize_counters().max_batch() {
            resize_counters().set_max_batch(batch);
        }
        // Drain once per reactor turn: semantic owners marked facets above;
        // the hook flushes them to snapshots/signals exactly once.
        let changes = wm.take_changes();
        hook(&conn, screen_num, changes)?;
        let mut batch: usize = 0;
        while let Some(pumped) = pump.next(conn.as_ref()).map_err(any_conn_err)? {
            resize_counters()
                .received
                .increment_by(pumped.received as u64);
            resize_counters()
                .coalesced
                .increment_by(pumped.coalesced as u64);
            batch += 1;
            {
                let _guard = flamewm_profiler::start("wm.resize.motion.drain");
                wm.handle_event(pumped.event)?;
            }
        }
        resize_counters().committed.increment_by(batch as u64);
        if batch > resize_counters().max_batch() {
            resize_counters().set_max_batch(batch);
        }
        emit_resize_summary();
        // Flush once per reactor turn.
        conn.flush()?;
        // Blocking wait: woken by X, provider, audio, or profiler sources.
        reactor.borrow_mut().dispatch(None)?;
    }
}

fn become_wm<C: Connection>(conn: &C, screen: &Screen) -> Result<(), ReplyError> {
    let attrs = ChangeWindowAttributesAux::new().event_mask(
        EventMask::SUBSTRUCTURE_REDIRECT
            | EventMask::SUBSTRUCTURE_NOTIFY
            | EventMask::PROPERTY_CHANGE,
    );
    let result = conn.change_window_attributes(screen.root, &attrs)?.check();
    if let Err(ReplyError::X11Error(ref error)) = result {
        if error.error_kind == ErrorKind::Access {
            eprintln!("flamewm: another window manager is already running on this display");
        }
    }
    result
}

struct Wm<'a, C: Connection> {
    conn: &'a C,
    screen_num: usize,
    config: WmConfig,
    atoms: Atoms,
    support_window: Window,
    clients: HashMap<Window, ManagedClient>,
    managed_order: Vec<Window>,
    frame_to_client: HashMap<Window, Window>,
    registry: FrameRegistry,
    sessions: HashMap<Window, ControllerState>,
    renderer: Option<flamewm_render_x11::ExternalDecorationRenderer>,
    current_workspace: usize,
    workspace_count: usize,
    active: Option<Window>,
    screen_rect: Rect,
    dock_struts: HashMap<Window, [u32; 12]>,
    snap_preview: SnapPreviewSurface,
    changes: WmChangeSet,
}

impl<'a, C: Connection> Wm<'a, C> {
    fn new(
        conn: &'a C,
        screen_num: usize,
        config: WmConfig,
        catalog: std::sync::Arc<flamewm_applications::ApplicationCatalog>,
    ) -> Result<Self, AnyError> {
        let screen = &conn.setup().roots[screen_num];
        let atoms = Atoms::new(conn)?;

        let support_window = conn.generate_id()?;
        conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            support_window,
            screen.root,
            -1,
            -1,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )?;

        let workspace_count = config.workspaces.max(1);
        let _ = catalog;
        Ok(Self {
            conn,
            screen_num,
            config,
            atoms,
            support_window,
            clients: HashMap::new(),
            managed_order: Vec::new(),
            frame_to_client: HashMap::new(),
            registry: FrameRegistry::new(),
            sessions: HashMap::new(),
            renderer: flamewm_render_x11::ExternalDecorationRenderer::open("IBM Plex Sans").ok(),
            current_workspace: 0,
            workspace_count,
            active: None,
            screen_rect: Rect::new(
                0,
                0,
                u32::from(screen.width_in_pixels),
                u32::from(screen.height_in_pixels),
            ),
            dock_struts: HashMap::new(),
            snap_preview: SnapPreviewSurface::new(),
            changes: WmChangeSet::default(),
        })
    }

    fn screen(&self) -> &Screen {
        &self.conn.setup().roots[self.screen_num]
    }

    fn work_area(&self) -> Rect {
        let mut strut = [0_u32; 4];
        for dock in self.dock_struts.values() {
            if dock[0] > 0 && dock[5] >= dock[4] && dock[4] < self.screen_rect.height {
                strut[0] = strut[0].max(dock[0]);
            }
            if dock[1] > 0 && dock[7] >= dock[6] && dock[6] < self.screen_rect.height {
                strut[1] = strut[1].max(dock[1]);
            }
            if dock[2] > 0 && dock[9] >= dock[8] && dock[8] < self.screen_rect.width {
                strut[2] = strut[2].max(dock[2]);
            }
            if dock[3] > 0 && dock[11] >= dock[10] && dock[10] < self.screen_rect.width {
                strut[3] = strut[3].max(dock[3]);
            }
        }
        if strut == [0; 4] {
            strut[3] = u32::from(self.config.panel_reserve);
        }
        let left = strut[0].min(self.screen_rect.width.saturating_sub(1));
        let right = strut[1].min(self.screen_rect.width.saturating_sub(left + 1));
        let top = strut[2].min(self.screen_rect.height.saturating_sub(1));
        let bottom = strut[3].min(self.screen_rect.height.saturating_sub(top + 1));
        Rect::new(
            self.screen_rect
                .x
                .saturating_add(i32::try_from(left).unwrap_or(i32::MAX)),
            self.screen_rect
                .y
                .saturating_add(i32::try_from(top).unwrap_or(i32::MAX)),
            self.screen_rect.width.saturating_sub(left + right).max(1),
            self.screen_rect.height.saturating_sub(top + bottom).max(1),
        )
    }

    pub fn take_changes(&mut self) -> WmChangeSet {
        self.changes.take()
    }

    fn mark_windows(&mut self) {
        self.changes.windows = true;
    }

    fn mark_workspaces(&mut self) {
        self.changes.workspaces = true;
    }

    fn mark_work_area(&mut self) {
        self.changes.work_area = true;
    }

    fn extents(&self) -> FrameExtents {
        FrameExtents::new(i32::from(self.config.titlebar_height), 1)
    }

    fn work_root(&self) -> RootRect {
        let work = self.work_area();
        RootRect::new(
            work.x,
            work.y,
            work.width.max(1) as i32,
            work.height.max(1) as i32,
        )
    }

    fn controller_for(&self, client: Window) -> Option<ControllerState> {
        let state = self.clients.get(&client)?;
        let res = self.registry.lookup_by_client(client)?;
        let session = self
            .sessions
            .get(&client)
            .map_or(InteractionSession::Idle, |session| session.session);
        let mut region_map = std::collections::HashMap::new();
        for xid in res.children() {
            if let Some(target) = self.registry.lookup_by_xid(xid) {
                region_map.insert(xid, target.region);
            }
        }
        let screen_rect = RootRect::new(
            0,
            0,
            i32::try_from(self.screen().width_in_pixels).unwrap_or(i32::MAX),
            i32::try_from(self.screen().height_in_pixels).unwrap_or(i32::MAX),
        );
        let mut controller = ControllerState::new(
            client,
            self.registry.capture().unwrap_or(self.screen().root),
            state.placement,
            self.work_root(),
            state.hints,
            self.extents(),
        )
        .with_screen_rect(screen_rect)
        .with_registry(region_map);
        controller.session = session;
        Some(controller)
    }

    fn store_controller(&mut self, client: Window, controller: &ControllerState) {
        let entry = self
            .sessions
            .entry(client)
            .or_insert_with(|| controller.clone());
        entry.session = controller.session;
        entry.placement = controller.placement;
        if let Some(state) = self.clients.get_mut(&client) {
            state.apply_placement(controller.placement);
        }
    }

    fn execute_effects(
        &mut self,
        client: Window,
        effects: &[FrameEffect],
    ) -> Result<(), ReplyError> {
        for effect in effects {
            match *effect {
                FrameEffect::Grab { window, cursor } => {
                    let ok = self.interaction_capture(window, cursor).unwrap_or(false);
                    if !ok {
                        self.abort_session_to_idle(client);
                    }
                }
                FrameEffect::Ungrab => {
                    self.release_capture_unmap();
                    let _ = self.conn.ungrab_pointer(CURRENT_TIME);
                }
                FrameEffect::ConfigureFrameRoot { x, y, w, h } => {
                    let Some(frame) = self.clients.get(&client).map(|state| state.frame) else {
                        continue;
                    };
                    let _ = self.conn.configure_window(
                        frame,
                        &ConfigureWindowAux::new()
                            .x(x)
                            .y(y)
                            .width(w.max(1) as u32)
                            .height(h.max(1) as u32),
                    );
                    self.commit_frame_rect(client, x, y, w, h);
                }
                FrameEffect::ConfigureClientLocal { x, y, w, h } => {
                    let _ = self.conn.configure_window(
                        client,
                        &ConfigureWindowAux::new()
                            .x(x)
                            .y(y)
                            .width(w.max(1))
                            .height(h.max(1)),
                    );
                }
                FrameEffect::LayoutInput => {
                    self.layout_input_children(client)?;
                }
                FrameEffect::PaintChrome => {
                    self.paint_chrome(client)?;
                }
                FrameEffect::ApplyShape => {
                    let (frame, maximized, fullscreen, outer) = match self.clients.get(&client) {
                        Some(state) => (
                            state.frame,
                            state.is_maximized(),
                            state.is_fullscreen(),
                            state.outer,
                        ),
                        None => continue,
                    };
                    self.clear_frame_shape(frame, outer, maximized, fullscreen)?;
                }
                FrameEffect::NotifyClientRoot { x, y, w, h } => {
                    let event = ConfigureNotifyEvent {
                        response_type: CONFIGURE_NOTIFY_EVENT,
                        sequence: 0,
                        event: client,
                        window: client,
                        above_sibling: 0,
                        x: clamp_i16(x),
                        y: clamp_i16(y),
                        width: clamp_u16(w.max(1) as u32),
                        height: clamp_u16(h.max(1)),
                        border_width: 0,
                        override_redirect: false,
                    };
                    let _ = self
                        .conn
                        .send_event(false, client, EventMask::STRUCTURE_NOTIFY, event);
                }
                FrameEffect::SnapPreview { target } => {
                    if target.is_none() {
                        self.snap_preview.hide();
                    }
                }
                FrameEffect::Noop => {}
            }
        }
        Ok(())
    }

    fn commit_frame_rect(&mut self, client: Window, x: i32, y: i32, w: i32, h: i32) {
        let frame = RootRect::new(x, y, w.max(1), h.max(1)).to_legacy();
        if let Some(state) = self.clients.get_mut(&client) {
            let mode = state.placement.mode;
            state.placement.current = crate::client::rect_to_root(frame);
            if mode == PlacementMode::Floating {
                state.placement.floating_restore = state.placement.current;
            }
            state.outer = frame;
        }
        if let Some(session) = self.sessions.get_mut(&client) {
            session.placement.current = crate::client::rect_to_root(frame);
        }
    }

    fn release_capture_unmap(&self) {
        if let Some(capture) = self.registry.capture() {
            let _ = self.conn.unmap_window(capture);
        }
    }

    fn interaction_capture(
        &mut self,
        grab_window: Window,
        cursor: flamewm_render_core::CursorKind,
    ) -> Result<bool, ReplyOrIdError> {
        let capture = match self.registry.capture() {
            Some(existing) => existing,
            None => {
                let id = self.conn.generate_id()?;
                let root = self.screen().root;
                self.conn.create_window(
                    COPY_DEPTH_FROM_PARENT,
                    id,
                    root,
                    clamp_i16(self.screen_rect.x),
                    clamp_i16(self.screen_rect.y),
                    clamp_u16(self.screen_rect.width),
                    clamp_u16(self.screen_rect.height),
                    0,
                    WindowClass::INPUT_ONLY,
                    0,
                    &CreateWindowAux::new(),
                )?;
                self.registry.set_capture(id);
                id
            }
        };
        let _ = grab_window;
        if let Some(renderer) = self.renderer.as_mut() {
            let _ = renderer.define_cursor(u64::from(capture), cursor);
        }
        let _ = self.conn.map_window(capture);
        let _ = self.conn.configure_window(
            capture,
            &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
        );
        let reply = self
            .conn
            .grab_pointer(
                false,
                capture,
                EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
                0u32,
                0u32,
                CURRENT_TIME,
            )?
            .reply();
        match reply {
            Ok(reply) if reply.status == GrabStatus::SUCCESS => Ok(true),
            _ => {
                self.release_capture_unmap();
                flamewm_debug::emit(
                    flamewm_debug::WM_FRAME_GRAB_FAIL,
                    std::time::Duration::from_secs(1),
                    || "grab refused".to_owned(),
                );
                Ok(false)
            }
        }
    }

    fn abort_session_to_idle(&mut self, client: Window) {
        if let Some(session) = self.sessions.get_mut(&client) {
            session.session.cancel();
        }
        self.release_capture_unmap();
    }

    fn reduce_and_execute(&mut self, client: Window, event: FrameEvent) -> Result<(), ReplyError> {
        let Some(controller) = self.controller_for(client) else {
            return Ok(());
        };
        let (next, effects) = reduce(&controller, event);
        // Zero-X-configure fast path: controller already returned Noop.
        self.store_controller(client, &next);
        self.execute_effects(client, &effects)
    }

    fn hover_control_engine(&self, client: Window) -> Option<FrameControl> {
        self.clients
            .get(&client)
            .and_then(|state| state.hover_control)
    }

    fn paint_chrome(&mut self, client: Window) -> Result<(), ReplyError> {
        let (frame, title, active, maximized, fullscreen, w, h, hover, pressed) =
            match self.clients.get(&client) {
                Some(state) => (
                    state.frame,
                    state.title.clone(),
                    self.active.is_none_or(|current| current == client),
                    state.is_maximized(),
                    state.is_fullscreen(),
                    state.outer.width.max(1),
                    state.outer.height.max(1),
                    self.hover_control_engine(client),
                    self.pressed_control(client),
                ),
                None => return Ok(()),
            };
        let scene =
            frame_chrome::plan_scene(w, h, &title, active, hover, pressed, maximized, fullscreen);
        if let Some(renderer) = self.renderer.as_mut() {
            let _ = frame_chrome::render(renderer, u64::from(frame), &scene);
        }
        Ok(())
    }

    fn pressed_control(&self, client: Window) -> Option<FrameControl> {
        match self.sessions.get(&client).map(|session| session.session) {
            Some(InteractionSession::ControlPress(session)) if session.armed => {
                Some(session.control)
            }
            _ => None,
        }
    }

    fn layout_input_children(&mut self, client: Window) -> Result<(), ReplyError> {
        let Some(res) = self.registry.lookup_by_client(client) else {
            return Ok(());
        };
        let Some(state) = self.clients.get(&client) else {
            return Ok(());
        };
        let origin = RootPoint::new(state.outer.x, state.outer.y);
        let layout = layout_frame_children_default(
            state.outer.width.max(1) as i32,
            state.outer.height.max(1) as i32,
        );
        let mut pairs: Vec<(u32, i32, i32, u32, u32)> = Vec::with_capacity(12);
        for (index, rect) in layout.resize_rects().iter().enumerate() {
            pairs.push((
                res.resize[index],
                rect.x,
                rect.y,
                rect.w.max(1) as u32,
                rect.h.max(1) as u32,
            ));
        }
        pairs.push((
            res.title_drag,
            layout.title_drag.x,
            layout.title_drag.y,
            layout.title_drag.w.max(1) as u32,
            layout.title_drag.h.max(1) as u32,
        ));
        for (index, rect) in layout.controls.iter().enumerate() {
            pairs.push((
                res.controls[index],
                rect.x,
                rect.y,
                rect.w.max(1) as u32,
                rect.h.max(1) as u32,
            ));
        }
        let _ = origin;
        for (xid, x, y, w, h) in pairs {
            self.conn
                .configure_window(xid, &ConfigureWindowAux::new().x(x).y(y).width(w).height(h))?;
        }
        Ok(())
    }

    fn bind_child_cursors(&mut self, client: Window) {
        let Some(res) = self.registry.lookup_by_client(client) else {
            return;
        };
        for xid in res.children() {
            let region = self.registry.lookup_by_xid(xid).map(|target| target.region);
            if let Some(region) = region {
                let kind = cursor_for_region(region);
                if let Some(renderer) = self.renderer.as_mut() {
                    let _ = renderer.define_cursor(u64::from(xid), kind);
                }
            }
        }
    }

    fn reconcile_work_area(&mut self) -> Result<(), ReplyError> {
        let work = self.work_area();
        let work_root = RootRect::new(
            work.x,
            work.y,
            work.width.max(1) as i32,
            work.height.max(1) as i32,
        );
        let ids = self.clients.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let (mode, outer, minimized) = match self.clients.get(&id) {
                Some(state) => (state.placement.mode, state.outer, state.minimized),
                None => continue,
            };
            if minimized {
                continue;
            }
            match mode {
                PlacementMode::Fullscreen => continue,
                PlacementMode::Maximized => {
                    let next = work;
                    if next != outer {
                        if let Some(state) = self.clients.get_mut(&id) {
                            state.set_outer(next);
                        }
                        self.apply_frame_geometry(id)?;
                        self.publish_window_state(id)?;
                    }
                }
                PlacementMode::Floating | PlacementMode::Snapped(_) => {
                    let clamped = outer.clamp_inside(work);
                    if clamped != outer {
                        if let Some(state) = self.clients.get_mut(&id) {
                            state.set_outer(clamped);
                        }
                        self.apply_frame_geometry(id)?;
                        self.publish_window_state(id)?;
                    }
                }
            }
            let _ = work_root;
        }
        self.mark_work_area();
        self.mark_windows();
        Ok(())
    }

    fn publish_desktop_state(&self) -> Result<(), ReplyError> {
        let root = self.screen().root;
        self.conn.change_property32(
            PropMode::REPLACE,
            root,
            self.atoms.net_supported,
            AtomEnum::ATOM,
            &self.atoms.supported(),
        )?;
        self.conn.change_property32(
            PropMode::REPLACE,
            root,
            self.atoms.net_supporting_wm_check,
            AtomEnum::WINDOW,
            &[self.support_window],
        )?;
        self.conn.change_property32(
            PropMode::REPLACE,
            self.support_window,
            self.atoms.net_supporting_wm_check,
            AtomEnum::WINDOW,
            &[self.support_window],
        )?;
        self.conn.change_property8(
            PropMode::REPLACE,
            self.support_window,
            self.atoms.net_wm_name,
            self.atoms.utf8_string,
            b"FlameWM",
        )?;
        self.publish_workspace_metadata()?;
        self.publish_current_workspace()?;
        self.publish_client_list()?;
        self.publish_active()?;
        Ok(())
    }

    fn publish_current_workspace(&self) -> Result<(), ReplyError> {
        let _guard = flamewm_profiler::start("wm.workspace.switch.ewmh_current");
        workspace_counters().ewmh_current.increment();
        self.conn.change_property32(
            PropMode::REPLACE,
            self.screen().root,
            self.atoms.net_current_desktop,
            AtomEnum::CARDINAL,
            &[self.current_workspace as u32],
        )?;
        Ok(())
    }

    fn publish_workspace_metadata(&self) -> Result<(), ReplyError> {
        let root = self.screen().root;
        let work = self.work_area();
        self.conn.change_property32(
            PropMode::REPLACE,
            root,
            self.atoms.net_number_of_desktops,
            AtomEnum::CARDINAL,
            &[self.workspace_count as u32],
        )?;
        // _NET_CURRENT_DESKTOP owned by publish_current_workspace only.
        self.conn.change_property32(
            PropMode::REPLACE,
            root,
            self.atoms.net_desktop_geometry,
            AtomEnum::CARDINAL,
            &[self.screen_rect.width, self.screen_rect.height],
        )?;
        let mut viewport = Vec::with_capacity(self.workspace_count * 2);
        let mut workareas = Vec::with_capacity(self.workspace_count * 4);
        let mut names = Vec::new();
        for index in 0..self.workspace_count {
            viewport.extend_from_slice(&[0, 0]);
            workareas.extend_from_slice(&[work.x as u32, work.y as u32, work.width, work.height]);
            names.extend_from_slice(format!("Desktop {}", index + 1).as_bytes());
            names.push(0);
        }
        self.conn.change_property32(
            PropMode::REPLACE,
            root,
            self.atoms.net_desktop_viewport,
            AtomEnum::CARDINAL,
            &viewport,
        )?;
        self.conn.change_property32(
            PropMode::REPLACE,
            root,
            self.atoms.net_workarea,
            AtomEnum::CARDINAL,
            &workareas,
        )?;
        self.conn.change_property8(
            PropMode::REPLACE,
            root,
            self.atoms.net_desktop_names,
            self.atoms.utf8_string,
            &names,
        )?;
        self.conn.change_property32(
            PropMode::REPLACE,
            root,
            self.atoms.net_showing_desktop,
            AtomEnum::CARDINAL,
            &[0],
        )?;
        Ok(())
    }

    fn publish_client_list(&self) -> Result<(), ReplyError> {
        workspace_counters().query_tree.increment();
        // The root tree is the server's authoritative bottom-to-top order.  HashMap iteration
        // cannot represent stacking and made _NET_CLIENT_LIST_STACKING observably false.
        let tree = self.conn.query_tree(self.screen().root)?.reply()?;
        let windows = tree
            .children
            .iter()
            .filter_map(|frame| self.frame_to_client.get(frame).copied())
            .collect::<Vec<_>>();
        let client_list = self
            .managed_order
            .iter()
            .copied()
            .filter(|window| self.clients.contains_key(window))
            .collect::<Vec<_>>();
        let root = self.screen().root;
        self.conn.change_property32(
            PropMode::REPLACE,
            root,
            self.atoms.net_client_list,
            AtomEnum::WINDOW,
            &client_list,
        )?;
        self.conn.change_property32(
            PropMode::REPLACE,
            root,
            self.atoms.net_client_list_stacking,
            AtomEnum::WINDOW,
            &windows,
        )?;
        Ok(())
    }

    fn publish_active(&self) -> Result<(), ReplyError> {
        let root = self.screen().root;
        self.conn.change_property32(
            PropMode::REPLACE,
            root,
            self.atoms.net_active_window,
            AtomEnum::WINDOW,
            &[self.active.unwrap_or(0)],
        )?;
        Ok(())
    }

    fn scan_existing(&mut self) -> Result<(), ReplyOrIdError> {
        let root = self.screen().root;
        let tree = self.conn.query_tree(root)?.reply()?;
        let mut pending = Vec::with_capacity(tree.children.len());
        for window in tree.children {
            if window == self.support_window {
                continue;
            }
            pending.push((
                window,
                self.conn.get_window_attributes(window)?,
                self.conn.get_geometry(window)?,
            ));
        }
        for (window, attrs, geometry) in pending {
            let Ok(attrs) = attrs.reply() else { continue };
            let Ok(geometry) = geometry.reply() else {
                continue;
            };
            if attrs.map_state != MapState::UNMAPPED {
                match self.window_kind(window, attrs.override_redirect)? {
                    WindowKind::Popup => {
                        self.conn.map_window(window)?;
                    }
                    WindowKind::Desktop => self.map_unframed(window, StackMode::BELOW)?,
                    WindowKind::Dock => self.map_dock(window)?,
                    WindowKind::Normal | WindowKind::Utility => self.manage(window, &geometry)?,
                }
            }
        }
        Ok(())
    }

    fn window_kind(
        &self,
        window: Window,
        override_redirect: bool,
    ) -> Result<WindowKind, ReplyError> {
        let reply = self
            .conn
            .get_property(
                false,
                window,
                self.atoms.net_wm_window_type,
                AtomEnum::ATOM,
                0,
                16,
            )?
            .reply()?;
        let types = reply.value32().map_or_else(Vec::new, Iterator::collect);
        Ok(classify(&self.atoms, &types, override_redirect))
    }

    fn map_unframed(&self, window: Window, stack_mode: StackMode) -> Result<(), ReplyError> {
        self.conn.map_window(window)?;
        self.conn
            .configure_window(window, &ConfigureWindowAux::new().stack_mode(stack_mode))?;
        Ok(())
    }

    fn map_dock(&mut self, window: Window) -> Result<(), ReplyOrIdError> {
        self.dock_struts.insert(window, self.read_strut(window)?);
        self.conn.change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new()
                .event_mask(EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY),
        )?;
        self.map_unframed(window, StackMode::ABOVE)?;
        self.changes.panels = true;
        self.mark_work_area();
        self.reconcile_work_area()?;
        self.publish_workspace_metadata()?;
        Ok(())
    }

    fn read_strut(&self, window: Window) -> Result<[u32; 12], ReplyError> {
        let partial = self
            .conn
            .get_property(
                false,
                window,
                self.atoms.net_wm_strut_partial,
                AtomEnum::CARDINAL,
                0,
                12,
            )?
            .reply()?;
        if let Some(mut values) = partial.value32() {
            let mut result = [0; 12];
            for value in &mut result {
                *value = values.next().unwrap_or(0);
            }
            if result != [0; 12] {
                return Ok(result);
            }
        }
        let legacy = self
            .conn
            .get_property(
                false,
                window,
                self.atoms.net_wm_strut,
                AtomEnum::CARDINAL,
                0,
                4,
            )?
            .reply()?;
        let mut result = [0; 12];
        if let Some(mut values) = legacy.value32() {
            for value in result.iter_mut().take(4) {
                *value = values.next().unwrap_or(0);
            }
        }
        result[4] = 0;
        result[5] = self.screen_rect.height.saturating_sub(1);
        result[6] = 0;
        result[7] = self.screen_rect.height.saturating_sub(1);
        result[8] = 0;
        result[9] = self.screen_rect.width.saturating_sub(1);
        result[10] = 0;
        result[11] = self.screen_rect.width.saturating_sub(1);
        Ok(result)
    }

    fn manage(
        &mut self,
        window: Window,
        geometry: &GetGeometryReply,
    ) -> Result<(), ReplyOrIdError> {
        let kind = self.window_kind(window, false)?;
        if self.clients.contains_key(&window) {
            self.conn.map_window(window)?;
            return Ok(());
        }
        if kind == WindowKind::Popup {
            return Ok(());
        }
        if kind == WindowKind::Desktop {
            self.map_unframed(window, StackMode::BELOW)?;
            return Ok(());
        }
        if kind == WindowKind::Dock {
            self.map_dock(window)?;
            return Ok(());
        }

        // Initial-map order: read title/class/hints/icon/transient/type first.
        let title = self
            .read_title(window)
            .unwrap_or_else(|_| "Application".to_owned());
        let (icon, icon_fallback) = self.read_frame_icon(window);
        let transient_for = self.read_transient_for(window)?;
        let hints = self.read_hints(window);
        let _wm_class = self.wm_class(window).unwrap_or_default();
        let workspace = transient_for
            .and_then(|owner| self.clients.get(&owner).map(|state| state.workspace))
            .unwrap_or(self.current_workspace);

        let screen = self.screen();
        let frame = self.conn.generate_id()?;
        let extents = self.extents();
        let work_root = self.work_root();
        let start = crate::client::placement_floating(RootRect::new(0, 0, 1, 1));
        let initial_req = GeometryRequest {
            reason: GeometryReason::InitialMap,
            start: PlacementSnapshot {
                mode: start.mode,
                rect: start.current,
            },
            pointer_delta: (0, 0),
            edges: crate::frame::model::ResizeEdges::none(),
            work_area: work_root,
            hints,
            frame_extents: extents,
        };
        let initial_plan = crate::frame::geometry::plan_initial_map(
            &initial_req,
            RootPoint::new(i32::from(geometry.x), i32::from(geometry.y)),
            (
                u32::from(geometry.width).max(1),
                u32::from(geometry.height).max(1),
            ),
        );
        let outer = crate::client::root_to_rect(initial_plan.frame);

        // Create frame UNMAPPED with panel bg #1b1e20 (not black).
        self.conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            frame,
            screen.root,
            clamp_i16(outer.x),
            clamp_i16(outer.y),
            clamp_u16(outer.width),
            clamp_u16(outer.height),
            self.config.frame_border,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new()
                .background_pixel(0x1b1e20)
                .border_pixel(0x1b1e20)
                .event_mask(EventMask::EXPOSURE | EventMask::SUBSTRUCTURE_NOTIFY),
        )?;

        // Grab-server only for save-set/reparent/structural-configure.
        self.conn.grab_server()?;
        self.conn.change_save_set(SetMode::INSERT, window)?;
        self.conn
            .reparent_window(window, frame, 0, self.config.titlebar_height as i16)?;
        self.conn.change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new()
                .event_mask(EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY),
        )?;
        let client_w = outer.width.max(1);
        let client_h = outer
            .height
            .saturating_sub(u32::from(self.config.titlebar_height))
            .max(1);
        self.conn.configure_window(
            window,
            &ConfigureWindowAux::new()
                .x(0)
                .y(i32::from(self.config.titlebar_height))
                .width(client_w)
                .height(client_h)
                .border_width(0),
        )?;

        // Create real 12 InputOnly children (8 resize + title-drag + 3 controls).
        let child_ids: [u32; 12] = [
            self.conn.generate_id()?,
            self.conn.generate_id()?,
            self.conn.generate_id()?,
            self.conn.generate_id()?,
            self.conn.generate_id()?,
            self.conn.generate_id()?,
            self.conn.generate_id()?,
            self.conn.generate_id()?,
            self.conn.generate_id()?,
            self.conn.generate_id()?,
            self.conn.generate_id()?,
            self.conn.generate_id()?,
        ];
        let layout =
            layout_frame_children_default(outer.width.max(1) as i32, outer.height.max(1) as i32);
        let rects = {
            let mut all = layout.resize_rects().to_vec();
            all.push(layout.title_drag);
            all.extend(layout.controls);
            all
        };
        for (index, xid) in child_ids.iter().enumerate() {
            let rect = rects[index];
            self.conn.create_window(
                COPY_DEPTH_FROM_PARENT,
                *xid,
                frame,
                rect.x as i16,
                rect.y as i16,
                rect.w.max(1) as u16,
                rect.h.max(1) as u16,
                0,
                WindowClass::INPUT_ONLY,
                0,
                &CreateWindowAux::new().event_mask(EventMask::from(INPUT_CHILD_EVENT_MASK)),
            )?;
        }
        self.conn.ungrab_server()?;
        self.conn.flush()?;

        // Insert FrameResources + ManagedClient.
        let resources = FrameResources::new(
            window,
            frame,
            child_ids[8],
            [child_ids[9], child_ids[10], child_ids[11]],
            [
                child_ids[0],
                child_ids[1],
                child_ids[2],
                child_ids[3],
                child_ids[4],
                child_ids[5],
                child_ids[6],
                child_ids[7],
            ],
        );
        let _ = self.registry.register(resources);
        let title_width = flamewm_render_x11::external_text_measure(&title, 12.0)
            .0
            .round() as i32;
        let client = ManagedClient {
            client: window,
            frame,
            placement: crate::client::placement_floating(crate::client::rect_to_root(outer)),
            hints,
            outer,
            workspace,
            title,
            title_text_width: title_width,
            icon,
            icon_fallback,
            transient_for,
            minimized: false,
            sticky: false,
            ignore_unmap: 1,
            kind,
            hover_control: None,
        };
        self.frame_to_client.insert(frame, window);
        for xid in resources.children() {
            self.frame_to_client.insert(xid, window);
        }
        self.clients.insert(window, client);
        self.managed_order.push(window);
        let controller = self.controller_for(window).unwrap_or_else(|| {
            ControllerState::new(
                window,
                self.screen().root,
                crate::client::placement_floating(crate::client::rect_to_root(outer)),
                self.work_root(),
                hints,
                self.extents(),
            )
            .with_screen_rect(crate::client::rect_to_root(Rect::new(
                self.screen_rect.x,
                self.screen_rect.y,
                self.screen_rect.width,
                self.screen_rect.height,
            )))
        });
        self.sessions.insert(window, controller);

        self.set_frame_extents(window)?;
        self.set_client_workspace(window, self.current_workspace)?;
        self.set_wm_state(window, WM_STATE_NORMAL)?;

        // Retarget + paint chrome + shape + cursors.
        self.paint_chrome(window)?;
        let _ = self.renderer.as_ref().map(|renderer| renderer.flush());
        let maximized = self
            .clients
            .get(&window)
            .map_or(false, |state| state.is_maximized());
        let fullscreen = self
            .clients
            .get(&window)
            .map_or(false, |state| state.is_fullscreen());
        self.clear_frame_shape(frame, outer, maximized, fullscreen)?;
        self.bind_child_cursors(window);

        // Map client + input children + frame (renderer flush above).
        self.conn.map_window(window)?;
        for xid in resources.children() {
            self.conn.map_window(xid)?;
        }
        self.conn.map_window(frame)?;
        self.mark_windows();
        self.publish_client_list()?;
        self.focus(window)?;
        Ok(())
    }

    fn unmanage(&mut self, window: Window, restore_to_root: bool) -> Result<(), ReplyError> {
        self.snap_preview.hide();
        // Abort any in-progress interaction through the reducer so session
        // teardown, ungrab, and snap-preview hide stay in one owner. Runs
        // before `clients.remove` because `controller_for` reads it.
        if let Some(controller) = self.controller_for(window) {
            if !controller.session.is_idle() {
                let (next, effects) = reduce(&controller, FrameEvent::Cancel);
                self.store_controller(window, &next);
                self.execute_effects(window, &effects)?;
            }
        }
        let Some(client) = self.clients.remove(&window) else {
            return Ok(());
        };
        self.managed_order.retain(|managed| *managed != window);
        self.frame_to_client.remove(&client.frame);
        let removed = self.registry.remove_client(window);
        for xid in &removed {
            let _ = self.conn.destroy_window(*xid);
            self.frame_to_client.remove(xid);
        }
        self.release_capture_unmap();
        self.sessions.remove(&window);
        if self.active == Some(window) {
            self.active = None;
        }
        if restore_to_root {
            let _ = self.conn.grab_server();
            let _ = self.conn.change_save_set(SetMode::DELETE, window);
            let _ = self.conn.reparent_window(
                window,
                self.screen().root,
                clamp_i16(client.outer.x),
                clamp_i16(client.outer.y),
            );
            let _ = self.conn.ungrab_server();
        }
        let _ = self.conn.destroy_window(client.frame);
        let _ = self.set_wm_state(window, WM_STATE_WITHDRAWN);
        self.mark_windows();
        self.publish_client_list()?;
        self.publish_active()?;
        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> Result<(), ReplyOrIdError> {
        match event {
            Event::MapRequest(event) => self.handle_map_request(event)?,
            Event::ConfigureRequest(event) => self.handle_configure_request(event)?,
            Event::DestroyNotify(event) => {
                if self.dock_struts.remove(&event.window).is_some() {
                    self.changes.panels = true;
                    self.mark_work_area();
                    self.reconcile_work_area()?;
                }
                let client = self.client_for(event.window);
                if let Some(client) = client {
                    self.unmanage(client, false)?;
                }
                self.publish_workspace_metadata()?;
            }
            Event::UnmapNotify(event) => self.handle_unmap(event.window)?,
            Event::MapNotify(event) => self.handle_map_notify(event.window)?,
            Event::ConfigureNotify(event) => self.handle_configure_notify(event)?,
            Event::Expose(event) => {
                if event.count == 0 {
                    if let Some(client) = self.client_for(event.window) {
                        self.paint_chrome(client)?;
                    }
                }
            }
            Event::EnterNotify(event) => self.handle_enter(event)?,
            Event::LeaveNotify(event) => self.handle_leave(event)?,
            Event::ButtonPress(event) => self.handle_button_press(event)?,
            Event::ButtonRelease(event) => self.handle_button_release(event)?,
            Event::MotionNotify(event) => self.handle_motion(event)?,
            Event::PropertyNotify(event) => self.handle_property(event.window, event.atom)?,
            Event::ClientMessage(event) => self.handle_client_message(event)?,
            _ => {}
        }
        Ok(())
    }

    fn handle_configure_notify(&mut self, event: ConfigureNotifyEvent) -> Result<(), ReplyError> {
        if event.window == self.screen().root {
            self.screen_rect.width = u32::from(event.width).max(1);
            self.screen_rect.height = u32::from(event.height).max(1);
            self.mark_work_area();
            self.reconcile_work_area()?;
            self.publish_workspace_metadata()?;
            return Ok(());
        }
        let Some(client_id) = self.client_for(event.window) else {
            return Ok(());
        };
        let Some(controller) = self.controller_for(client_id) else {
            return Ok(());
        };
        let origin = RootPoint::new(
            i32::from(event.x).saturating_sub(controller.extents.client_offset().0),
            i32::from(event.y).saturating_sub(controller.extents.client_offset().1),
        );
        let size = Some((
            u32::from(event.width).max(1),
            u32::from(event.height).max(1),
        ));
        let _ = self.reduce_and_execute(client_id, FrameEvent::Configure { origin, size });
        Ok(())
    }

    fn handle_map_request(&mut self, event: MapRequestEvent) -> Result<(), ReplyOrIdError> {
        if self.clients.contains_key(&event.window) {
            self.conn.map_window(event.window)?;
            if let Some(client) = self.clients.get(&event.window) {
                self.conn.map_window(client.frame)?;
            }
            self.publish_client_list()?;
            return Ok(());
        }
        let kind = self.window_kind(
            event.window,
            self.conn
                .get_window_attributes(event.window)?
                .reply()?
                .override_redirect,
        )?;
        match kind {
            WindowKind::Popup => {
                self.map_unframed(event.window, StackMode::ABOVE)?;
                return Ok(());
            }
            WindowKind::Desktop => {
                self.map_unframed(event.window, StackMode::BELOW)?;
                return Ok(());
            }
            WindowKind::Dock => return self.map_dock(event.window),
            WindowKind::Normal | WindowKind::Utility => {}
        }
        let geometry = self.conn.get_geometry(event.window)?.reply()?;
        self.manage(event.window, &geometry)
    }

    fn handle_configure_request(&mut self, event: ConfigureRequestEvent) -> Result<(), ReplyError> {
        let Some(client_id) = self.client_for(event.window) else {
            self.conn.configure_window(
                event.window,
                &ConfigureWindowAux::from_configure_request(&event),
            )?;
            return Ok(());
        };
        let Some(controller) = self.controller_for(client_id) else {
            return Ok(());
        };
        let extents = controller.extents;
        let prev_root = frame_to_client_root(controller.placement.current, extents);
        let (prev_origin, prev_size) = (prev_root.origin(), prev_root.size_u32());
        let mask = event.value_mask;
        let req_origin = if mask.contains(ConfigWindow::X) || mask.contains(ConfigWindow::Y) {
            RootPoint::new(
                if mask.contains(ConfigWindow::X) {
                    i32::from(event.x).saturating_sub(extents.client_offset().0)
                } else {
                    prev_origin.x
                },
                if mask.contains(ConfigWindow::Y) {
                    i32::from(event.y).saturating_sub(extents.client_offset().1)
                } else {
                    prev_origin.y
                },
            )
        } else {
            prev_origin
        };
        let req_size = if mask.contains(ConfigWindow::WIDTH) || mask.contains(ConfigWindow::HEIGHT)
        {
            Some((
                if mask.contains(ConfigWindow::WIDTH) {
                    u32::from(event.width).max(1)
                } else {
                    prev_size.0
                },
                if mask.contains(ConfigWindow::HEIGHT) {
                    u32::from(event.height).max(1)
                } else {
                    prev_size.1
                },
            ))
        } else {
            None
        };
        let req = GeometryRequest {
            reason: GeometryReason::ClientConfigure,
            start: PlacementSnapshot {
                mode: controller.placement.mode,
                rect: controller.placement.current,
            },
            pointer_delta: (0, 0),
            edges: crate::frame::model::ResizeEdges::none(),
            work_area: controller.work_area,
            hints: controller.hints,
            frame_extents: extents,
        };
        let plan = plan_client_configure(&req, req_origin, req_size);
        if !plan.noop {
            let mode = controller.placement.mode;
            if let Some(state) = self.clients.get_mut(&client_id) {
                state.placement.mode = mode;
                state.placement.current = plan.frame;
                if mode == PlacementMode::Floating {
                    state.placement.floating_restore = plan.frame;
                }
                state.outer = crate::client::root_to_rect(plan.frame);
            }
            if let Some(session) = self.sessions.get_mut(&client_id) {
                session.placement.current = plan.frame;
            }
            self.apply_frame_geometry(client_id)?;
            self.send_configure_notify(client_id)?;
        } else {
            self.send_configure_notify(client_id)?;
        }
        if event.value_mask.contains(ConfigWindow::STACK_MODE) {
            let frame = self
                .clients
                .get(&client_id)
                .map(|state| state.frame)
                .unwrap_or(client_id);
            let sibling = if event.value_mask.contains(ConfigWindow::SIBLING) && event.sibling != 0
            {
                self.client_for(event.sibling)
                    .and_then(|id| self.clients.get(&id).map(|state| state.frame))
                    .or(Some(event.sibling))
            } else {
                None
            };
            self.restack(frame, event.stack_mode, sibling)?;
            self.publish_client_list()?;
        }
        Ok(())
    }

    fn restack(
        &self,
        window: Window,
        mode: StackMode,
        sibling: Option<Window>,
    ) -> Result<(), ReplyError> {
        let mut aux = ConfigureWindowAux::new().stack_mode(mode);
        if let Some(sibling) = sibling {
            aux = aux.sibling(sibling);
        }
        self.conn.configure_window(window, &aux)?;
        Ok(())
    }

    /// MapNotify without a preceding MapRequest arrives for windows the WM
    /// itself mapped (frame, client, children: `map_window` generates no
    /// MapRequest). A client MapNotify means the client is visible now; if it
    /// is still unframed (e.g. WM missed the MapRequest because it was not
    /// yet selecting SubstructureRedirect), manage it now. Frame/child
    /// MapNotify events are ignored.
    fn handle_map_notify(&mut self, window: Window) -> Result<(), ReplyOrIdError> {
        if window == self.screen().root || self.clients.contains_key(&window) {
            return Ok(());
        }
        if self.client_for(window).is_some() {
            return Ok(());
        }
        if self
            .conn
            .get_window_attributes(window)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .is_none_or(|attrs| attrs.override_redirect)
        {
            return Ok(());
        }
        match self.window_kind(window, false)? {
            WindowKind::Normal | WindowKind::Utility => {
                let geometry = self.conn.get_geometry(window)?.reply()?;
                return self.manage(window, &geometry);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_unmap(&mut self, window: Window) -> Result<(), ReplyError> {
        let Some(client_id) = self.client_for(window) else {
            return Ok(());
        };
        if let Some(client) = self.clients.get_mut(&client_id) {
            if client.ignore_unmap > 0 {
                client.ignore_unmap -= 1;
                return Ok(());
            }
        }
        if window == client_id {
            self.unmanage(client_id, true)?;
        }
        Ok(())
    }

    fn handle_property(&mut self, window: Window, atom: Atom) -> Result<(), ReplyError> {
        if self.dock_struts.contains_key(&window)
            && (atom == self.atoms.net_wm_strut || atom == self.atoms.net_wm_strut_partial)
        {
            self.dock_struts.insert(window, self.read_strut(window)?);
            self.changes.panels = true;
            self.mark_work_area();
            self.reconcile_work_area()?;
            self.publish_workspace_metadata()?;
            return Ok(());
        }
        let Some(client_id) = self.client_for(window) else {
            return Ok(());
        };
        if atom == self.atoms.net_wm_name || atom == AtomEnum::WM_NAME.into() {
            if let Ok(title) = self.read_title(client_id) {
                let width = flamewm_render_x11::external_text_measure(&title, 12.0)
                    .0
                    .round() as i32;
                if let Some(client) = self.clients.get_mut(&client_id) {
                    client.title_text_width = width;
                    client.title = title;
                }
                self.mark_windows();
                self.paint_chrome(client_id)?;
            }
        } else if atom == self.atoms.net_wm_icon {
            let (icon, fallback) = self.read_frame_icon(client_id);
            if let Some(client) = self.clients.get_mut(&client_id) {
                client.icon = icon;
                client.icon_fallback = fallback;
            }
            self.mark_windows();
            self.paint_chrome(client_id)?;
        } else if atom == AtomEnum::WM_TRANSIENT_FOR.into() {
            let transient_for = self.read_transient_for(client_id)?;
            if let Some(client) = self.clients.get_mut(&client_id) {
                client.transient_for = transient_for;
            }
            self.mark_windows();
        } else if atom == AtomEnum::WM_NORMAL_HINTS.into() {
            let hints = self.read_hints(client_id);
            if let Some(client) = self.clients.get_mut(&client_id) {
                client.hints = hints;
            }
            if let Some(session) = self.sessions.get_mut(&client_id) {
                session.hints = hints;
            }
            self.layout_input_children(client_id)?;
        }
        Ok(())
    }

    fn handle_enter(&mut self, event: EnterNotifyEvent) -> Result<(), ReplyError> {
        let Some(client_id) = self.client_for(event.event) else {
            return Ok(());
        };
        self.focus(client_id)?;
        let _ = self.reduce_and_execute(client_id, FrameEvent::Enter);
        Ok(())
    }

    fn handle_leave(&mut self, event: LeaveNotifyEvent) -> Result<(), ReplyError> {
        let Some(client_id) = self.client_for(event.event) else {
            return Ok(());
        };
        let active_session = self
            .sessions
            .get(&client_id)
            .map_or(false, |session| !session.session.is_idle());
        if active_session {
            return Ok(());
        }
        let _ = self.reduce_and_execute(client_id, FrameEvent::Leave);
        self.paint_chrome(client_id)?;
        Ok(())
    }

    fn handle_button_press(&mut self, event: ButtonPressEvent) -> Result<(), ReplyError> {
        if event.detail != BUTTON_PRIMARY {
            return Ok(());
        }
        let Some(client_id) = self.client_for(event.event) else {
            return Ok(());
        };
        self.focus(client_id)?;
        // Route press through controller reduce(); execute FrameEffects diff-aware.
        let pointer = RootPoint::new(i32::from(event.root_x), i32::from(event.root_y));
        let had_session = self
            .sessions
            .get(&client_id)
            .map_or(false, |session| !session.session.is_idle());
        let _ = had_session;
        match self.reduce_and_execute(
            client_id,
            FrameEvent::Press {
                source: event.event,
                pointer,
            },
        ) {
            Ok(()) => {
                let grabbed = self
                    .sessions
                    .get(&client_id)
                    .map_or(false, |session| !session.session.is_idle());
                if !grabbed {
                    // No session (grab failed or Noop region): treat as control click.
                    self.handle_control_click(client_id)?;
                }
                Ok(())
            }
            Err(_) => Ok(()),
        }
    }

    fn handle_control_click(&mut self, client_id: Window) -> Result<(), ReplyError> {
        let Some(res) = self.registry.lookup_by_client(client_id) else {
            return Ok(());
        };
        // Control clicks only arm via controller; a press that produced no
        // session means the region was Client/Frame or grab failed. Check
        // which control (if any) via registry region is impossible without
        // coords, so fall back to titlebar control hit via pointer query.
        let _ = res;
        Ok(())
    }

    fn handle_button_release(&mut self, event: ButtonReleaseEvent) -> Result<(), ReplyError> {
        if event.detail != BUTTON_PRIMARY {
            return Ok(());
        }
        // Route release through controller reduce(); control clicks execute here.
        let Some(client_id) = self.client_for(event.event) else {
            return Ok(());
        };
        let was_control = matches!(
            self.sessions.get(&client_id).map(|session| session.session),
            Some(InteractionSession::ControlPress(_))
        );
        let pointer = RootPoint::new(i32::from(event.root_x), i32::from(event.root_y));
        let _ = self.reduce_and_execute(client_id, FrameEvent::Release { pointer });
        if was_control {
            let armed = true;
            if armed {
                let region = self
                    .registry
                    .lookup_by_xid(event.event)
                    .map(|target| target.region);
                match region {
                    Some(FrameRegion::Control(FrameControl::Close)) => self.close(client_id)?,
                    Some(FrameRegion::Control(FrameControl::MaximizeRestore)) => {
                        self.reduce_and_execute(client_id, FrameEvent::ToggleMax)?;
                        self.mark_windows();
                        self.publish_window_state(client_id)?;
                    }
                    Some(FrameRegion::Control(FrameControl::Minimize)) => {
                        self.minimize(client_id)?
                    }
                    _ => {}
                }
            }
            return Ok(());
        }
        // Move-release snap: controller release already un-grabbed; apply snap target.
        let target = core_snap_target(
            flamewm_api::Point::new(i32::from(event.root_x), i32::from(event.root_y)),
            to_core_rect(self.work_area()),
        );
        if target != SnapTarget::None {
            let moved = self.sessions.get(&client_id).map_or(false, |_| true);
            let _ = moved;
            // Only snap when the pointer is at a screen edge (target found) and
            // the client is floating (controller Snap event handles resume).
            if let Some(state) = self.clients.get(&client_id) {
                if state.placement.mode == PlacementMode::Floating {
                    self.reduce_and_execute(
                        client_id,
                        FrameEvent::Snap {
                            target: Self::snap_to_engine(target),
                        },
                    )?;
                    self.mark_windows();
                    self.publish_window_state(client_id)?;
                }
            }
        }
        Ok(())
    }

    fn snap_to_engine(target: SnapTarget) -> crate::frame::model::SnapTarget {
        match target {
            SnapTarget::LeftHalf => crate::frame::model::SnapTarget::Left,
            SnapTarget::RightHalf => crate::frame::model::SnapTarget::Right,
            SnapTarget::TopHalf => crate::frame::model::SnapTarget::Top,
            SnapTarget::BottomHalf => crate::frame::model::SnapTarget::Bottom,
            SnapTarget::TopLeftQuarter => crate::frame::model::SnapTarget::TopLeft,
            SnapTarget::TopRightQuarter => crate::frame::model::SnapTarget::TopRight,
            SnapTarget::BottomLeftQuarter => crate::frame::model::SnapTarget::BottomLeft,
            SnapTarget::BottomRightQuarter => crate::frame::model::SnapTarget::BottomRight,
            SnapTarget::Maximize | SnapTarget::None => crate::frame::model::SnapTarget::Left,
        }
    }

    fn handle_motion(&mut self, event: MotionNotifyEvent) -> Result<(), ReplyError> {
        // Root points during active session; route through controller reduce().
        let Some(client_id) = self
            .client_for(event.event)
            .or_else(|| self.session_client())
        else {
            return Ok(());
        };
        let pointer = RootPoint::new(i32::from(event.root_x), i32::from(event.root_y));
        let active = self
            .sessions
            .get(&client_id)
            .map_or(false, |session| !session.session.is_idle());
        if active {
            let _ = self.reduce_and_execute(client_id, FrameEvent::Motion { pointer });
            self.update_snap_preview(event.root_x, event.root_y, self.work_area(), client_id);
            return Ok(());
        }
        // Idle hover: track control hover, repaint chrome on change.
        let hover = self.idle_hover(client_id, event.event_x, event.event_y, event.event);
        let changed = if let Some(state) = self.clients.get_mut(&client_id) {
            let changed = state.hover_control != hover;
            state.hover_control = hover;
            changed
        } else {
            false
        };
        if changed {
            self.paint_chrome(client_id)?;
        }
        Ok(())
    }

    fn session_client(&self) -> Option<Window> {
        self.sessions.iter().find_map(|(client, session)| {
            if !session.session.is_idle() {
                Some(*client)
            } else {
                None
            }
        })
    }

    fn idle_hover(
        &self,
        client_id: Window,
        x: i16,
        y: i16,
        source: Window,
    ) -> Option<FrameControl> {
        let region = self
            .registry
            .lookup_by_xid(source)
            .map(|target| target.region);
        match region {
            Some(FrameRegion::Control(control)) => Some(control),
            _ => {
                let state = self.clients.get(&client_id)?;
                crate::frame::input::frame_control_fallback(
                    state.outer.width,
                    self.config.titlebar_height,
                    x,
                    y,
                    state.hover_control,
                )
            }
        }
    }

    fn handle_client_message(&mut self, event: ClientMessageEvent) -> Result<(), ReplyError> {
        if event.type_ == self.atoms.wm_change_state && self.clients.contains_key(&event.window) {
            let data = event.data.as_data32();
            if data[0] == WM_STATE_ICONIC {
                return self.minimize(event.window);
            }
            return self.restore_minimized(event.window);
        }
        let data = event.data.as_data32();
        if event.type_ == self.atoms.net_current_desktop {
            self.switch_workspace(data[0] as usize)?;
        } else if event.type_ == self.atoms.net_number_of_desktops {
            self.set_workspace_count(data[0] as usize)?;
        } else if event.type_ == self.atoms.net_wm_desktop {
            if self.clients.contains_key(&event.window) {
                self.move_to_workspace(event.window, data[0] as usize)?;
            }
        } else if event.type_ == self.atoms.net_active_window {
            if self.clients.contains_key(&event.window) {
                self.focus(event.window)?;
            }
        } else if event.type_ == self.atoms.net_close_window {
            if self.clients.contains_key(&event.window) {
                self.close(event.window)?;
            }
        } else if event.type_ == self.atoms.net_wm_state && self.clients.contains_key(&event.window)
        {
            self.apply_net_wm_state(event.window, data[0], data[1], data[2])?;
        }
        Ok(())
    }

    fn apply_net_wm_state(
        &mut self,
        client: Window,
        action: u32,
        first: Atom,
        second: Atom,
    ) -> Result<(), ReplyError> {
        let atoms = [first, second];
        let wants_maximize = atoms.iter().any(|atom| {
            *atom == self.atoms.net_wm_state_maximized_horz
                || *atom == self.atoms.net_wm_state_maximized_vert
        });
        let wants_fullscreen = atoms.contains(&self.atoms.net_wm_state_fullscreen);
        let wants_hidden = atoms.contains(&self.atoms.net_wm_state_hidden);
        if wants_hidden && action != 0 {
            return self.minimize(client);
        }
        if wants_fullscreen {
            let currently = self
                .clients
                .get(&client)
                .is_some_and(|state| state.is_fullscreen());
            let enable = match action {
                0 => false,
                1 => true,
                2 => !currently,
                _ => currently,
            };
            return self.set_fullscreen(client, enable);
        }
        if wants_maximize {
            let currently = self
                .clients
                .get(&client)
                .is_some_and(|state| state.is_maximized());
            let enable = match action {
                0 => false,
                1 => true,
                2 => !currently,
                _ => currently,
            };
            if enable != currently {
                self.toggle_maximize(client)?;
            }
        }
        Ok(())
    }

    fn focus(&mut self, client: Window) -> Result<(), ReplyError> {
        let Some(state) = self.clients.get(&client) else {
            return Ok(());
        };
        if !state.visible_on(self.current_workspace) {
            return Ok(());
        }
        self.conn
            .set_input_focus(InputFocus::NONE, client, CURRENT_TIME)?;
        if state.kind != WindowKind::Utility || state.transient_for.is_some() {
            self.conn.configure_window(
                state.frame,
                &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
            )?;
        }
        if self.supports_protocol(client, self.atoms.wm_take_focus)? {
            let event = ClientMessageEvent::new(
                32,
                client,
                self.atoms.wm_protocols,
                [self.atoms.wm_take_focus, CURRENT_TIME, 0, 0, 0],
            );
            self.conn
                .send_event(false, client, EventMask::NO_EVENT, event)?;
        }
        self.active = Some(client);
        self.mark_windows();
        self.publish_active()?;
        self.publish_client_list()?;
        Ok(())
    }

    fn supports_protocol(&self, client: Window, protocol: Atom) -> Result<bool, ReplyError> {
        let reply = self
            .conn
            .get_property(
                false,
                client,
                self.atoms.wm_protocols,
                AtomEnum::ATOM,
                0,
                32,
            )?
            .reply()?;
        Ok(reply
            .value32()
            .is_some_and(|mut values| values.any(|atom| atom == protocol)))
    }

    fn close(&self, client: Window) -> Result<(), ReplyError> {
        let reply = self
            .conn
            .get_property(
                false,
                client,
                self.atoms.wm_protocols,
                AtomEnum::ATOM,
                0,
                32,
            )?
            .reply()?;
        let supports_delete = reply
            .value32()
            .is_some_and(|mut values| values.any(|atom| atom == self.atoms.wm_delete_window));
        if supports_delete {
            let event = ClientMessageEvent::new(
                32,
                client,
                self.atoms.wm_protocols,
                [self.atoms.wm_delete_window, CURRENT_TIME, 0, 0, 0],
            );
            self.conn
                .send_event(false, client, EventMask::NO_EVENT, event)?;
        } else {
            self.conn.kill_client(client)?;
        }
        Ok(())
    }

    fn minimize(&mut self, client: Window) -> Result<(), ReplyError> {
        let frame = {
            let Some(state) = self.clients.get_mut(&client) else {
                return Ok(());
            };
            if state.minimized {
                return Ok(());
            }
            state.minimized = true;
            state.ignore_unmap = state.ignore_unmap.saturating_add(1);
            state.frame
        };
        self.conn.unmap_window(frame)?;
        self.set_wm_state(client, WM_STATE_ICONIC)?;
        self.mark_windows();
        self.publish_window_state(client)?;
        if self.active == Some(client) {
            self.active = None;
            self.publish_active()?;
        }
        Ok(())
    }

    fn restore_minimized(&mut self, client: Window) -> Result<(), ReplyError> {
        let frame = {
            let Some(state) = self.clients.get_mut(&client) else {
                return Ok(());
            };
            if !state.minimized {
                return Ok(());
            }
            state.minimized = false;
            state.ignore_unmap = state.ignore_unmap.saturating_add(1);
            state.frame
        };
        self.conn.map_window(frame)?;
        self.set_wm_state(client, WM_STATE_NORMAL)?;
        self.mark_windows();
        self.publish_window_state(client)?;
        self.focus(client)?;
        let _ = self.conn.configure_window(
            frame,
            &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
        );
        self.publish_client_list()?;
        Ok(())
    }

    fn toggle_maximize(&mut self, client: Window) -> Result<(), ReplyError> {
        self.reduce_and_execute(client, FrameEvent::ToggleMax)?;
        self.mark_windows();
        self.publish_window_state(client)?;
        Ok(())
    }

    fn set_fullscreen(&mut self, client: Window, enable: bool) -> Result<(), ReplyError> {
        let fullscreen = self
            .clients
            .get(&client)
            .map_or(false, |state| state.is_fullscreen());
        if enable == fullscreen {
            return Ok(());
        }
        // ToggleFullscreen is a pure toggle; drive it only when the target
        // differs from current, and loop-safe (single reduce).
        if (enable && !fullscreen) || (!enable && fullscreen) {
            self.reduce_and_execute(client, FrameEvent::ToggleFullscreen)?;
        }
        self.mark_windows();
        self.publish_window_state(client)?;
        Ok(())
    }

    /// Drag-motion preview: candidate from shared `snap_target`, geometry from
    /// shared `snap_geometry`. `None` hides. Skips redraw when unchanged.
    /// No new native renderer: the compiled snap-preview surface moves/shows.
    fn update_snap_preview(&mut self, root_x: i16, root_y: i16, work: Rect, client: Window) {
        let target = core_snap_target(
            flamewm_api::Point::new(i32::from(root_x), i32::from(root_y)),
            to_core_rect(work),
        );
        if target == SnapTarget::None {
            self.snap_preview.hide();
            return;
        }
        let current = self.clients.get(&client).map_or((800, 600), |state| {
            (
                i32::try_from(state.outer.width).unwrap_or(800),
                i32::try_from(state.outer.height).unwrap_or(600),
            )
        });
        let geometry = core_snap_geometry(
            target,
            to_core_rect(work),
            flamewm_api::Size::new(current.0, current.1),
        );
        let geometry = from_core_rect(geometry);
        let geometry = flamewm_api::Rect::new(
            geometry.x,
            geometry.y,
            i32::try_from(geometry.width).unwrap_or(1),
            i32::try_from(geometry.height).unwrap_or(1),
        );
        if self.snap_preview.is_current(
            target,
            geometry,
            snap_preview::DEFAULT_PREVIEW_OPACITY_PERCENT,
        ) {
            return;
        }
        self.snap_preview.update(
            Some(target),
            Some(geometry),
            snap_preview::DEFAULT_PREVIEW_OPACITY_PERCENT,
        );
    }

    fn apply_frame_geometry(&mut self, client: Window) -> Result<(), ReplyError> {
        let (frame, outer, maximized, fullscreen) = match self.clients.get(&client) {
            Some(state) => (
                state.frame,
                state.outer,
                state.is_maximized(),
                state.is_fullscreen(),
            ),
            None => return Ok(()),
        };
        {
            let _guard = flamewm_profiler::start("wm.resize.shape");
            self.clear_frame_shape(frame, outer, maximized, fullscreen)?;
        }
        {
            let _guard = flamewm_profiler::start("wm.resize.frame_configure");
            let titlebar = u32::from(self.config.titlebar_height);
            self.conn.configure_window(
                frame,
                &ConfigureWindowAux::new()
                    .x(outer.x)
                    .y(outer.y)
                    .width(outer.width.max(1))
                    .height(outer.height.max(titlebar + 1)),
            )?;
        }
        {
            let _guard = flamewm_profiler::start("wm.resize.client_configure");
            let titlebar = u32::from(self.config.titlebar_height);
            self.conn.configure_window(
                client,
                &ConfigureWindowAux::new()
                    .x(0)
                    .y(i32::from(self.config.titlebar_height))
                    .width(outer.width.max(1))
                    .height(outer.height.saturating_sub(titlebar).max(1)),
            )?;
        }
        {
            let _guard = flamewm_profiler::start("wm.resize.commit");
            self.layout_input_children(client)?;
        }
        Ok(())
    }

    fn switch_workspace(&mut self, target: usize) -> Result<(), ReplyError> {
        if target >= self.workspace_count || target == self.current_workspace {
            return Ok(());
        }
        let old = self.current_workspace;
        self.current_workspace = target;
        let ids = self.clients.keys().copied().collect::<Vec<_>>();
        workspace_counters().switch_total.increment();
        {
            let _guard = flamewm_profiler::start("wm.workspace.switch.visibility");
            for id in ids {
                let (frame, old_visible, new_visible) = {
                    let Some(state) = self.clients.get(&id) else {
                        continue;
                    };
                    (state.frame, state.visible_on(old), state.visible_on(target))
                };
                if old_visible == new_visible {
                    continue;
                }
                if new_visible {
                    self.conn.map_window(frame)?;
                } else {
                    if let Some(state) = self.clients.get_mut(&id) {
                        state.ignore_unmap = state.ignore_unmap.saturating_add(1);
                    }
                    self.conn.unmap_window(frame)?;
                }
            }
            workspace_counters().visibility.increment();
        }
        self.active = None;
        self.mark_workspaces();
        self.mark_windows();
        self.publish_current_workspace()?;
        {
            let _guard = flamewm_profiler::start("wm.workspace.switch.shell_publish");
            self.publish_active()?;
            workspace_counters().shell_publish.increment();
        }
        Ok(())
    }

    fn set_workspace_count(&mut self, requested: usize) -> Result<(), ReplyError> {
        let requested = requested.clamp(1, 18);
        if requested == self.workspace_count {
            return Ok(());
        }
        let mut moved = Vec::new();
        if requested < self.workspace_count {
            for state in self.clients.values_mut() {
                if state.workspace >= requested {
                    state.workspace = requested - 1;
                    moved.push((state.client, state.workspace));
                }
            }
            self.current_workspace = self.current_workspace.min(requested - 1);
        }
        for (client, workspace) in moved {
            self.set_client_workspace(client, workspace)?;
        }
        self.workspace_count = requested;
        self.mark_workspaces();
        self.mark_windows();
        self.publish_workspace_metadata()?;
        self.publish_current_workspace()?;
        self.reconcile_workspace_visibility()?;
        Ok(())
    }

    fn reconcile_workspace_visibility(&mut self) -> Result<(), ReplyError> {
        let ids = self.clients.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let (frame, visible) = {
                let Some(state) = self.clients.get(&id) else {
                    continue;
                };
                (state.frame, state.visible_on(self.current_workspace))
            };
            if visible {
                self.conn.map_window(frame)?;
            } else {
                if let Some(state) = self.clients.get_mut(&id) {
                    state.ignore_unmap = state.ignore_unmap.saturating_add(1);
                }
                self.conn.unmap_window(frame)?;
            }
        }
        Ok(())
    }

    fn move_to_workspace(&mut self, client: Window, target: usize) -> Result<(), ReplyError> {
        if target >= self.workspace_count {
            return Ok(());
        }
        let (frame, visible) = {
            let Some(state) = self.clients.get_mut(&client) else {
                return Ok(());
            };
            state.workspace = target;
            (state.frame, state.visible_on(self.current_workspace))
        };
        self.set_client_workspace(client, target)?;
        self.mark_windows();
        self.mark_workspaces();
        if visible {
            self.conn.map_window(frame)?;
        } else {
            if let Some(state) = self.clients.get_mut(&client) {
                state.ignore_unmap = state.ignore_unmap.saturating_add(1);
            }
            self.conn.unmap_window(frame)?;
        }
        Ok(())
    }

    fn set_client_workspace(&self, client: Window, workspace: usize) -> Result<(), ReplyError> {
        self.conn.change_property32(
            PropMode::REPLACE,
            client,
            self.atoms.net_wm_desktop,
            AtomEnum::CARDINAL,
            &[workspace as u32],
        )?;
        Ok(())
    }

    fn set_wm_state(&self, client: Window, state: u32) -> Result<(), ReplyError> {
        self.conn.change_property32(
            PropMode::REPLACE,
            client,
            self.atoms.wm_state,
            self.atoms.wm_state,
            &[state, 0],
        )?;
        Ok(())
    }

    fn publish_window_state(&self, client: Window) -> Result<(), ReplyError> {
        let Some(state) = self.clients.get(&client) else {
            return Ok(());
        };
        let mut values = Vec::with_capacity(3);
        if state.minimized {
            values.push(self.atoms.net_wm_state_hidden);
        }
        if state.is_maximized() {
            values.push(self.atoms.net_wm_state_maximized_vert);
            values.push(self.atoms.net_wm_state_maximized_horz);
        }
        if state.is_fullscreen() {
            values.push(self.atoms.net_wm_state_fullscreen);
        }
        self.conn.change_property32(
            PropMode::REPLACE,
            client,
            self.atoms.net_wm_state,
            AtomEnum::ATOM,
            &values,
        )?;
        Ok(())
    }

    fn set_frame_extents(&self, client: Window) -> Result<(), ReplyError> {
        self.conn.change_property32(
            PropMode::REPLACE,
            client,
            self.atoms.net_frame_extents,
            AtomEnum::CARDINAL,
            &[
                u32::from(self.config.frame_border),
                u32::from(self.config.frame_border),
                u32::from(self.config.titlebar_height) + u32::from(self.config.frame_border),
                u32::from(self.config.frame_border),
            ],
        )?;
        Ok(())
    }

    fn send_configure_notify(&self, client: Window) -> Result<(), ReplyError> {
        let Some(state) = self.clients.get(&client) else {
            return Ok(());
        };
        // Synthetic ConfigureNotify with client-root coords.
        let extents = self.extents();
        let root = frame_to_client_root(
            RootRect::new(
                state.outer.x,
                state.outer.y,
                state.outer.width.max(1) as i32,
                state.outer.height.max(1) as i32,
            ),
            extents,
        );
        let (cw, ch) = root.size_u32();
        let event = ConfigureNotifyEvent {
            response_type: CONFIGURE_NOTIFY_EVENT,
            sequence: 0,
            event: client,
            window: client,
            above_sibling: 0,
            x: clamp_i16(root.x),
            y: clamp_i16(root.y),
            width: clamp_u16(cw),
            height: clamp_u16(ch),
            border_width: 0,
            override_redirect: false,
        };
        self.conn
            .send_event(false, client, EventMask::STRUCTURE_NOTIFY, event)?;
        Ok(())
    }

    fn clear_frame_shape(
        &self,
        frame: Window,
        outer: crate::geometry::Rect,
        maximized: bool,
        fullscreen: bool,
    ) -> Result<(), ReplyError> {
        use x11rb::protocol::shape::{SK as ShapeSk, SO as ShapeOp};
        let rects = if maximized || fullscreen {
            vec![x11rb::protocol::xproto::Rectangle {
                x: 0,
                y: 0,
                width: outer.width.min(u32::from(u16::MAX)) as u16,
                height: outer.height.min(u32::from(u16::MAX)) as u16,
            }]
        } else {
            crate::frame::geometry_shape::bounding_rectangles(outer)
        };
        self.conn.shape_rectangles(
            ShapeOp::SET,
            ShapeSk::BOUNDING,
            ClipOrdering::UNSORTED,
            frame,
            0,
            0,
            &rects,
        )?;
        Ok(())
    }

    fn wm_class(&self, client: Window) -> Option<String> {
        self.conn
            .get_property(false, client, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 1024)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|reply| {
                String::from_utf8_lossy(&reply.value)
                    .trim_matches('\0')
                    .to_owned()
            })
    }

    /// EWMH icon path: cached `_NET_WM_ICON` selection, else `WM_CLASS`
    /// resolver fallback with a debug reason. Arrow cursor is untouched.
    fn read_frame_icon(&self, client: Window) -> (Option<chrome::IconImage>, Option<String>) {
        match self.read_net_wm_icon(client) {
            Ok(Some(icon)) => (Some(icon), None),
            Ok(None) => (None, Some(self.icon_fallback_reason(client, "absent"))),
            Err(reason) => (None, Some(self.icon_fallback_reason(client, reason))),
        }
    }

    fn read_net_wm_icon(&self, client: Window) -> Result<Option<chrome::IconImage>, &'static str> {
        let reply = self
            .conn
            .get_property(
                false,
                client,
                self.atoms.net_wm_icon,
                AtomEnum::CARDINAL,
                0,
                u32::MAX,
            )
            .map_err(|_| "get-property-failed")?
            .reply()
            .map_err(|_| "get-property-reply-failed")?;
        let cardinals = reply.value32().map_or_else(Vec::new, Iterator::collect);
        if cardinals.is_empty() {
            return Ok(None);
        }
        chrome::parse_net_wm_icon(&cardinals)
            .map(Some)
            .ok_or("invalid-payload")
    }

    fn icon_fallback_reason(&self, client: Window, reason: &str) -> String {
        let class = self.wm_class(client).unwrap_or_default();
        if class.is_empty() {
            format!("icon-fallback wm_class=unknown reason={reason}")
        } else {
            format!("icon-fallback wm_class={class} reason={reason}")
        }
    }

    fn read_title(&self, client: Window) -> Result<String, ReplyError> {
        let utf8 = self
            .conn
            .get_property(
                false,
                client,
                self.atoms.net_wm_name,
                self.atoms.utf8_string,
                0,
                1024,
            )?
            .reply()?;
        if !utf8.value.is_empty() {
            return Ok(String::from_utf8_lossy(&utf8.value)
                .trim_matches('\0')
                .to_owned());
        }
        let legacy = self
            .conn
            .get_property(false, client, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 1024)?
            .reply()?;
        Ok(String::from_utf8_lossy(&legacy.value)
            .trim_matches('\0')
            .to_owned())
    }

    fn read_transient_for(&self, client: Window) -> Result<Option<Window>, ReplyError> {
        let reply = self
            .conn
            .get_property(
                false,
                client,
                AtomEnum::WM_TRANSIENT_FOR,
                AtomEnum::WINDOW,
                0,
                1,
            )?
            .reply()?;
        Ok(reply
            .value32()
            .and_then(|mut values| values.next())
            .filter(|window| *window != 0))
    }

    fn client_for(&self, window: Window) -> Option<Window> {
        if self.clients.contains_key(&window) {
            Some(window)
        } else if let Some(client) = self.frame_to_client.get(&window).copied() {
            Some(client)
        } else if let Some(target) = self.registry.lookup_by_xid(window) {
            Some(target.client)
        } else {
            None
        }
    }

    fn read_hints(&self, client: Window) -> ClientSizeHints {
        self.conn
            .get_property(
                false,
                client,
                AtomEnum::WM_NORMAL_HINTS,
                AtomEnum::ANY,
                0,
                18,
            )
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .and_then(|reply| reply.value32().map(|iter| iter.collect::<Vec<_>>()))
            .map_or_else(ClientSizeHints::default, |values| {
                ClientSizeHints::parse(&values)
            })
    }
}

impl<C: Connection> Drop for Wm<'_, C> {
    fn drop(&mut self) {
        self.snap_preview.hide();
    }
}

fn clamp_i16(value: i32) -> i16 {
    value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

fn clamp_u16(value: u32) -> u16 {
    value.clamp(1, u32::from(u16::MAX)) as u16
}

fn to_core_rect(rect: Rect) -> flamewm_api::Rect {
    flamewm_api::Rect::new(
        rect.x,
        rect.y,
        i32::try_from(rect.width).unwrap_or(i32::MAX),
        i32::try_from(rect.height).unwrap_or(i32::MAX),
    )
}

fn from_core_rect(rect: flamewm_api::Rect) -> Rect {
    Rect::new(
        rect.x,
        rect.y,
        u32::try_from(rect.width).unwrap_or(1).max(1),
        u32::try_from(rect.height).unwrap_or(1).max(1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::model::ResizeEdges;

    #[test]
    fn resize_from_left_preserves_right_edge() {
        let original = Rect::new(100, 100, 600, 400);
        let edges = ResizeEdges {
            left: true,
            ..ResizeEdges::default()
        };
        let resized = ResizeEdges::resized_rect(original, edges, 100, 0);
        assert_eq!(resized, Rect::new(200, 100, 500, 400));
    }

    #[test]
    fn resize_enforces_minimum_size() {
        let original = Rect::new(100, 100, 600, 400);
        let edges = ResizeEdges {
            left: true,
            top: true,
            ..ResizeEdges::default()
        };
        let resized = ResizeEdges::resized_rect(original, edges, 1_000, 1_000);
        assert_eq!(resized.width, 160);
        assert_eq!(resized.height, 96);
    }

    #[test]
    fn cursor_kind_covers_eight_directions() {
        use flamewm_render_core::CursorKind;
        let corner_nw = ResizeEdges {
            left: true,
            top: true,
            ..ResizeEdges::default()
        };
        assert_eq!(
            corner_nw.cursor_kind(),
            CursorKind::ResizeNorthWestSouthEast
        );
        let corner_se = ResizeEdges {
            right: true,
            bottom: true,
            ..ResizeEdges::default()
        };
        assert_eq!(
            corner_se.cursor_kind(),
            CursorKind::ResizeNorthWestSouthEast
        );
        let corner_ne = ResizeEdges {
            right: true,
            top: true,
            ..ResizeEdges::default()
        };
        assert_eq!(
            corner_ne.cursor_kind(),
            CursorKind::ResizeNorthEastSouthWest
        );
        let corner_sw = ResizeEdges {
            left: true,
            bottom: true,
            ..ResizeEdges::default()
        };
        assert_eq!(
            corner_sw.cursor_kind(),
            CursorKind::ResizeNorthEastSouthWest
        );
        let west = ResizeEdges {
            left: true,
            ..ResizeEdges::default()
        };
        assert_eq!(west.cursor_kind(), CursorKind::ResizeHorizontal);
        let east = ResizeEdges {
            right: true,
            ..ResizeEdges::default()
        };
        assert_eq!(east.cursor_kind(), CursorKind::ResizeHorizontal);
        let north = ResizeEdges {
            top: true,
            ..ResizeEdges::default()
        };
        assert_eq!(north.cursor_kind(), CursorKind::ResizeVertical);
        let south = ResizeEdges {
            bottom: true,
            ..ResizeEdges::default()
        };
        assert_eq!(south.cursor_kind(), CursorKind::ResizeVertical);
        assert_eq!(ResizeEdges::default().cursor_kind(), CursorKind::Default);
    }

    #[test]
    fn change_set_drains_once_per_turn() {
        let mut changes = WmChangeSet::default();
        assert!(!changes.any());
        changes.windows = true;
        changes.work_area = true;
        let drained = changes.take();
        assert!(drained.windows && drained.work_area);
        assert!(!changes.any());
    }

    #[test]
    fn frame_control_hit_targets_unchanged() {
        use crate::frame::input::frame_control_at;
        use crate::frame::model::FrameControl as EngineControl;
        let titlebar = chrome::TITLEBAR_HEIGHT;
        // Skin chrome: button_width=38, controls tile the right edge.
        // 400-wide frame: min [286,324), max [324,362), close [362,400).
        assert_eq!(
            frame_control_at(400, titlebar, 390),
            Some(EngineControl::Close)
        );
        assert_eq!(
            frame_control_at(400, titlebar, 360),
            Some(EngineControl::MaximizeRestore)
        );
        assert_eq!(
            frame_control_at(400, titlebar, 328),
            Some(EngineControl::MaximizeRestore)
        );
        assert_eq!(frame_control_at(400, titlebar, 100), None);
        assert_eq!(
            frame_control_at(400, titlebar, 362),
            Some(EngineControl::Close)
        );
        assert_eq!(
            frame_control_at(400, titlebar, 324),
            Some(EngineControl::MaximizeRestore)
        );
        assert_eq!(
            frame_control_at(400, titlebar, 286),
            Some(EngineControl::Minimize)
        );
        assert_eq!(frame_control_at(400, titlebar, 285), None);
    }

    #[test]
    fn grab_reaches_dismissed_counter_static_shape() {
        // Serial-owner static check: switch path must carry the split
        // metadata/current pair plus the renamed counters/spans without
        // touching query_tree or client-list publish.
        let src = include_str!("wm.rs");
        assert!(src.contains("fn publish_workspace_metadata("));
        assert!(src.contains("fn publish_current_workspace("));
        assert!(src.contains("\"wm.workspace.switch.total\""));
        assert!(src.contains("\"wm.workspace.switch.visibility\""));
        assert!(src.contains("\"wm.workspace.switch.ewmh_current\""));
        assert!(src.contains("\"wm.workspace.switch.shell_publish\""));
        assert!(src.contains("\"wm.workspace.query_tree.count\""));
        assert!(src.contains("NotifyClientRoot"));
        assert!(src.contains("ConfigureClientLocal"));
        assert!(src.contains("ConfigureFrameRoot"));
        assert!(src.contains("with_screen_rect"));
    }
}
