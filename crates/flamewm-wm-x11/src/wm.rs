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
use crate::client::{Drag, ManagedClient};
use crate::decoration::{DecorationManager, ResizeEdges};
use crate::geometry::Rect;
use crate::snap_preview::{self, SnapPreviewSurface};

const BUTTON_PRIMARY: Button = 1;
const WM_STATE_WITHDRAWN: u32 = 0;
const WM_STATE_NORMAL: u32 = 1;
const WM_STATE_ICONIC: u32 = 3;

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

pub(crate) fn run_with_hook<F>(
    config: WmConfig,
    catalog: &std::sync::Arc<flamewm_applications::ApplicationCatalog>,
    reactor: &Rc<RefCell<flamewm_reactor::Reactor>>,
    mut hook: F,
) -> Result<(), AnyError>
where
    F: FnMut(&Rc<RustConnection>, usize) -> Result<(), AnyError>,
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
        // Buffered drain before block: consume all queued X events first.
        while let Some(event) = conn.poll_for_event()? {
            wm.handle_event(event)?;
        }
        hook(&conn, screen_num)?;
        while let Some(event) = conn.poll_for_event()? {
            wm.handle_event(event)?;
        }
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
    drag: Option<Drag>,
    current_workspace: usize,
    workspace_count: usize,
    active: Option<Window>,
    screen_rect: Rect,
    dock_struts: HashMap<Window, [u32; 12]>,
    decorations: DecorationManager,
    snap_preview: SnapPreviewSurface,
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
        let mut decorations = DecorationManager::with_catalog(std::sync::Arc::clone(&catalog));
        let _ = decorations.define_root_cursor(conn, screen.root);
        let _ = decorations.measure_decoration_text("", 12.0);
        Ok(Self {
            conn,
            screen_num,
            config,
            atoms,
            support_window,
            clients: HashMap::new(),
            managed_order: Vec::new(),
            frame_to_client: HashMap::new(),
            drag: None,
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
            decorations,
            snap_preview: SnapPreviewSurface::new(),
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
            strut[2] = u32::from(self.config.panel_reserve);
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
        self.publish_workspace_state()?;
        self.publish_client_list()?;
        self.publish_active()?;
        Ok(())
    }

    fn publish_workspace_state(&self) -> Result<(), ReplyError> {
        let root = self.screen().root;
        let work = self.work_area();
        self.conn.change_property32(
            PropMode::REPLACE,
            root,
            self.atoms.net_number_of_desktops,
            AtomEnum::CARDINAL,
            &[self.workspace_count as u32],
        )?;
        self.conn.change_property32(
            PropMode::REPLACE,
            root,
            self.atoms.net_current_desktop,
            AtomEnum::CARDINAL,
            &[self.current_workspace as u32],
        )?;
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
        self.publish_workspace_state()?;
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

        let screen = self.screen();
        let frame = self.conn.generate_id()?;
        let outer = Rect::new(
            i32::from(geometry.x),
            i32::from(geometry.y),
            u32::from(geometry.width).max(1),
            u32::from(geometry.height)
                .saturating_add(u32::from(self.config.titlebar_height))
                .max(1),
        )
        .clamp_inside(self.work_area());

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
                .background_pixel(screen.black_pixel)
                .border_pixel(screen.black_pixel)
                .event_mask(
                    EventMask::EXPOSURE
                        | EventMask::BUTTON_PRESS
                        | EventMask::BUTTON_RELEASE
                        | EventMask::POINTER_MOTION
                        | EventMask::ENTER_WINDOW
                        | EventMask::SUBSTRUCTURE_NOTIFY,
                ),
        )?;

        self.conn.grab_server()?;
        self.conn.change_save_set(SetMode::INSERT, window)?;
        self.conn
            .reparent_window(window, frame, 0, self.config.titlebar_height as i16)?;
        self.conn.change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new()
                .event_mask(EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY),
        )?;
        self.conn.configure_window(
            window,
            &ConfigureWindowAux::new()
                .x(0)
                .y(i32::from(self.config.titlebar_height))
                .width(outer.width)
                .height(
                    outer
                        .height
                        .saturating_sub(u32::from(self.config.titlebar_height))
                        .max(1),
                )
                .border_width(0),
        )?;
        self.conn.map_window(window)?;
        self.conn.map_window(frame)?;
        self.conn.ungrab_server()?;

        let title = self
            .read_title(window)
            .unwrap_or_else(|_| "Application".to_owned());
        let (icon, icon_fallback) = self.read_frame_icon(window);
        let transient_for = self.read_transient_for(window)?;
        let workspace = transient_for
            .and_then(|owner| self.clients.get(&owner).map(|state| state.workspace))
            .unwrap_or(self.current_workspace);
        let mut client = ManagedClient {
            client: window,
            frame,
            outer,
            restore: outer,
            workspace,
            title,
            title_text_width: 0,
            icon,
            icon_fallback,
            transient_for,
            minimized: false,
            maximized: false,
            fullscreen: false,
            sticky: false,
            ignore_unmap: 1,
            kind,
            close_hover: false,
        };
        self.frame_to_client.insert(frame, window);
        // Canonical title width comes from the decoration manager (live
        // `measure_decoration_text` target where safe, cached Xft-compatible
        // estimate otherwise); the retired `chrome::title_text_width`
        // heuristic duplicate is not used here.
        let title_width = self
            .decorations
            .measure_decoration_text(&client.title, 12.0)
            .0
            .round() as i32;
        client.title_text_width = title_width;
        self.clients.insert(window, client);
        self.managed_order.push(window);
        self.set_frame_extents(window)?;
        self.set_client_workspace(window, self.current_workspace)?;
        self.set_wm_state(window, WM_STATE_NORMAL)?;
        self.publish_client_list()?;
        self.focus(window)?;
        self.draw_frame(window)?;
        Ok(())
    }

    fn unmanage(&mut self, window: Window, restore_to_root: bool) -> Result<(), ReplyError> {
        self.snap_preview.hide();
        let Some(client) = self.clients.remove(&window) else {
            return Ok(());
        };
        self.managed_order.retain(|managed| *managed != window);
        self.frame_to_client.remove(&client.frame);
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
        self.publish_client_list()?;
        self.publish_active()?;
        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> Result<(), ReplyOrIdError> {
        match event {
            Event::MapRequest(event) => self.handle_map_request(event)?,
            Event::ConfigureRequest(event) => self.handle_configure_request(event)?,
            Event::DestroyNotify(event) => {
                self.dock_struts.remove(&event.window);
                let client = self.client_for(event.window);
                if let Some(client) = client {
                    self.unmanage(client, false)?;
                }
                self.publish_workspace_state()?;
            }
            Event::UnmapNotify(event) => self.handle_unmap(event.window)?,
            Event::Expose(event) => {
                if event.count == 0 {
                    if let Some(client) = self.client_for(event.window) {
                        self.draw_frame(client)?;
                    }
                }
            }
            Event::EnterNotify(event) => {
                if let Some(client) = self.client_for(event.event) {
                    self.focus(client)?;
                }
            }
            Event::ButtonPress(event) => self.handle_button_press(event)?,
            Event::ButtonRelease(event) => self.handle_button_release(event)?,
            Event::MotionNotify(event) => self.handle_motion(event)?,
            Event::PropertyNotify(event) => self.handle_property(event.window, event.atom)?,
            Event::ClientMessage(event) => self.handle_client_message(event)?,
            Event::ConfigureNotify(event) if event.window == self.screen().root => {
                self.screen_rect.width = u32::from(event.width).max(1);
                self.screen_rect.height = u32::from(event.height).max(1);
                self.publish_workspace_state()?;
            }
            _ => {}
        }
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
        let work_area = self.work_area();
        let titlebar_height = self.config.titlebar_height;
        let (frame, outer) = {
            let Some(client) = self.clients.get_mut(&client_id) else {
                return Ok(());
            };
            let mask = event.value_mask;
            if mask.contains(ConfigWindow::X) {
                client.outer.x = i32::from(event.x);
            }
            if mask.contains(ConfigWindow::Y) {
                client.outer.y = i32::from(event.y);
            }
            if mask.contains(ConfigWindow::WIDTH) {
                client.outer.width = u32::from(event.width).max(1);
            }
            if mask.contains(ConfigWindow::HEIGHT) {
                client.outer.height = u32::from(event.height)
                    .saturating_add(u32::from(titlebar_height))
                    .max(u32::from(titlebar_height) + 1);
            }
            client.outer = client.outer.clamp_inside(work_area);
            client.restore = client.outer;
            client.maximized = false;
            (client.frame, client.outer)
        };
        self.configure_frame(frame, client_id, outer)?;
        if event.value_mask.contains(ConfigWindow::STACK_MODE) {
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
        self.send_configure_notify(client_id)?;
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
            self.publish_workspace_state()?;
            return Ok(());
        }
        let Some(client_id) = self.client_for(window) else {
            return Ok(());
        };
        if atom == self.atoms.net_wm_name || atom == AtomEnum::WM_NAME.into() {
            if let Ok(title) = self.read_title(client_id) {
                let width = self
                    .decorations
                    .measure_decoration_text(&title, 12.0)
                    .0
                    .round() as i32;
                if let Some(client) = self.clients.get_mut(&client_id) {
                    client.title_text_width = width;
                    client.title = title;
                }
                self.draw_frame(client_id)?;
            }
        } else if atom == self.atoms.net_wm_icon {
            let (icon, fallback) = self.read_frame_icon(client_id);
            if let Some(client) = self.clients.get_mut(&client_id) {
                client.icon = icon;
                client.icon_fallback = fallback;
            }
            self.draw_frame(client_id)?;
        } else if atom == AtomEnum::WM_TRANSIENT_FOR.into() {
            let transient_for = self.read_transient_for(client_id)?;
            if let Some(client) = self.clients.get_mut(&client_id) {
                client.transient_for = transient_for;
            }
        }
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
        let Some(client) = self.clients.get(&client_id) else {
            return Ok(());
        };
        if event.event != client.frame {
            return Ok(());
        }
        let edges = self.resize_edges(client, event.event_x, event.event_y);
        if edges.any() {
            self.drag = Some(Drag::Resize {
                client: client_id,
                root_x: i32::from(event.root_x),
                root_y: i32::from(event.root_y),
                original: client.outer,
                edges,
            });
            return Ok(());
        }
        if event.event_y >= 0
            && event.event_y < self.config.titlebar_height as i16
            && self.control_at(client, event.event_x).is_none()
        {
            self.drag = Some(Drag::Move {
                client: client_id,
                offset_x: client.outer.x.saturating_sub(i32::from(event.root_x)),
                offset_y: client.outer.y.saturating_sub(i32::from(event.root_y)),
                original: client.outer,
            });
        }
        Ok(())
    }

    fn handle_button_release(&mut self, event: ButtonReleaseEvent) -> Result<(), ReplyError> {
        if event.detail != BUTTON_PRIMARY {
            return Ok(());
        }
        let drag = self.drag.take();
        self.snap_preview.hide();
        if let Some(Drag::Move {
            client, original, ..
        }) = drag
        {
            let target = core_snap_target(
                flamewm_api::Point::new(i32::from(event.root_x), i32::from(event.root_y)),
                to_core_rect(self.work_area()),
            );
            if target != SnapTarget::None {
                if let Some(state) = self.clients.get_mut(&client) {
                    state.restore = original;
                }
                self.apply_snap(client, target)?;
            }
            return Ok(());
        }
        if matches!(drag, Some(Drag::Resize { .. })) {
            return Ok(());
        }

        let Some(client_id) = self.client_for(event.event) else {
            return Ok(());
        };
        let Some(client) = self.clients.get(&client_id) else {
            return Ok(());
        };
        if event.event != client.frame
            || event.event_y < 0
            || event.event_y >= self.config.titlebar_height as i16
        {
            return Ok(());
        }
        match self.control_at(client, event.event_x) {
            Some(crate::decoration::FrameControl::Close) => self.close(client_id)?,
            Some(crate::decoration::FrameControl::Maximize) => self.toggle_maximize(client_id)?,
            Some(crate::decoration::FrameControl::Minimize) => self.minimize(client_id)?,
            None => {}
        }
        Ok(())
    }

    fn handle_motion(&mut self, event: MotionNotifyEvent) -> Result<(), ReplyError> {
        if let Some(client_id) = self.client_for(event.event) {
            let hover = self.clients.get(&client_id).is_some_and(|state| {
                event.event == state.frame
                    && event.event_y >= 0
                    && event.event_y < self.config.titlebar_height as i16
                    && self.control_at(state, event.event_x)
                        == Some(crate::decoration::FrameControl::Close)
            });
            let changed = if let Some(state) = self.clients.get_mut(&client_id) {
                let changed = state.close_hover != hover;
                state.close_hover = hover;
                changed
            } else {
                false
            };
            if changed {
                self.draw_frame(client_id)?;
            }
        }
        let Some(drag) = self.drag else { return Ok(()) };
        match drag {
            Drag::Move {
                client,
                offset_x,
                offset_y,
                ..
            } => {
                let new_x = i32::from(event.root_x).saturating_add(offset_x);
                let new_y = i32::from(event.root_y).saturating_add(offset_y);
                let work = self.work_area();
                if let Some(state) = self.clients.get_mut(&client) {
                    if state.maximized {
                        let restore = state.restore.clamp_inside(work);
                        state.maximized = false;
                        state.outer = restore;
                    }
                    state.outer.x = new_x;
                    state.outer.y = new_y;
                    let outer = state.outer;
                    let frame = state.frame;
                    self.conn.configure_window(
                        frame,
                        &ConfigureWindowAux::new().x(outer.x).y(outer.y),
                    )?;
                }
                self.update_snap_preview(event.root_x, event.root_y, work, client);
            }
            Drag::Resize {
                client,
                root_x,
                root_y,
                original,
                edges,
            } => {
                self.snap_preview.hide();
                let dx = i32::from(event.root_x).saturating_sub(root_x);
                let dy = i32::from(event.root_y).saturating_sub(root_y);
                let next = crate::decoration::resized_rect(original, edges, dx, dy)
                    .clamp_inside(self.work_area());
                let frame = if let Some(state) = self.clients.get_mut(&client) {
                    state.outer = next;
                    state.restore = next;
                    state.maximized = false;
                    Some(state.frame)
                } else {
                    None
                };
                if let Some(frame) = frame {
                    self.configure_frame(frame, client, next)?;
                }
            }
        }
        Ok(())
    }

    fn handle_client_message(&mut self, event: ClientMessageEvent) -> Result<(), ReplyError> {
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
                .is_some_and(|state| state.fullscreen);
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
                .is_some_and(|state| state.maximized);
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
        self.publish_window_state(client)?;
        if self.active == Some(client) {
            self.active = None;
            self.publish_active()?;
        }
        Ok(())
    }

    fn toggle_maximize(&mut self, client: Window) -> Result<(), ReplyError> {
        let Some(state) = self.clients.get(&client) else {
            return Ok(());
        };
        let maximize = !state.maximized;
        if maximize {
            let restore = state.outer;
            if let Some(state) = self.clients.get_mut(&client) {
                state.restore = restore;
            }
            self.apply_snap(client, SnapTarget::Maximize)?;
        } else {
            let restore = state.restore.clamp_inside(self.work_area());
            let frame = state.frame;
            if let Some(state) = self.clients.get_mut(&client) {
                state.outer = restore;
                state.maximized = false;
                state.fullscreen = false;
            }
            self.configure_frame(frame, client, restore)?;
            self.publish_window_state(client)?;
        }
        Ok(())
    }

    fn set_fullscreen(&mut self, client: Window, enable: bool) -> Result<(), ReplyError> {
        let Some(state) = self.clients.get(&client) else {
            return Ok(());
        };
        if enable == state.fullscreen {
            return Ok(());
        }
        let frame = state.frame;
        let next = if enable {
            state.outer
        } else {
            state.restore.clamp_inside(self.work_area())
        };
        if enable {
            let fullscreen = self.screen_rect;
            if let Some(state) = self.clients.get_mut(&client) {
                state.restore = state.outer;
                state.outer = fullscreen;
                state.fullscreen = true;
                state.maximized = false;
            }
            self.conn.configure_window(
                frame,
                &ConfigureWindowAux::new()
                    .x(fullscreen.x)
                    .y(fullscreen.y)
                    .width(fullscreen.width)
                    .height(fullscreen.height),
            )?;
            self.conn.configure_window(
                client,
                &ConfigureWindowAux::new()
                    .x(0)
                    .y(0)
                    .width(fullscreen.width)
                    .height(fullscreen.height),
            )?;
        } else {
            if let Some(state) = self.clients.get_mut(&client) {
                state.outer = next;
                state.fullscreen = false;
            }
            self.configure_frame(frame, client, next)?;
        }
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

    fn apply_snap(&mut self, client: Window, target: SnapTarget) -> Result<(), ReplyError> {
        let (work_area, current) = (
            self.work_area(),
            self.clients
                .get(&client)
                .map_or(Rect::new(0, 0, 1, 1), |state| state.outer),
        );
        let next = from_core_rect(core_snap_geometry(
            target,
            to_core_rect(work_area),
            flamewm_api::Size::new(
                i32::try_from(current.width).unwrap_or(i32::MAX),
                i32::try_from(current.height).unwrap_or(i32::MAX),
            ),
        ));
        let frame = {
            let Some(state) = self.clients.get_mut(&client) else {
                return Ok(());
            };
            state.outer = next;
            state.maximized = target == SnapTarget::Maximize;
            state.fullscreen = false;
            state.frame
        };
        self.configure_frame(frame, client, next)?;
        self.publish_window_state(client)?;
        Ok(())
    }

    fn configure_frame(
        &self,
        frame: Window,
        client: Window,
        outer: Rect,
    ) -> Result<(), ReplyError> {
        let titlebar = u32::from(self.config.titlebar_height);
        let (maximized, fullscreen) = self
            .clients
            .get(&client)
            .map_or((false, false), |state| (state.maximized, state.fullscreen));
        self.clear_frame_shape(frame, outer, maximized, fullscreen)?;
        self.conn.configure_window(
            frame,
            &ConfigureWindowAux::new()
                .x(outer.x)
                .y(outer.y)
                .width(outer.width.max(1))
                .height(outer.height.max(titlebar + 1)),
        )?;
        self.conn.configure_window(
            client,
            &ConfigureWindowAux::new()
                .x(0)
                .y(i32::from(self.config.titlebar_height))
                .width(outer.width.max(1))
                .height(outer.height.saturating_sub(titlebar).max(1)),
        )?;
        Ok(())
    }

    fn switch_workspace(&mut self, target: usize) -> Result<(), ReplyError> {
        if target >= self.workspace_count || target == self.current_workspace {
            return Ok(());
        }
        self.current_workspace = target;
        let ids = self.clients.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let Some(state) = self.clients.get_mut(&id) else {
                continue;
            };
            if state.visible_on(target) {
                self.conn.map_window(state.frame)?;
            } else {
                state.ignore_unmap = state.ignore_unmap.saturating_add(1);
                self.conn.unmap_window(state.frame)?;
            }
        }
        self.active = None;
        self.publish_workspace_state()?;
        self.publish_active()?;
        self.publish_client_list()?;
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
        self.publish_workspace_state()?;
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
        if state.maximized {
            values.push(self.atoms.net_wm_state_maximized_vert);
            values.push(self.atoms.net_wm_state_maximized_horz);
        }
        if state.fullscreen {
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
        let event = ConfigureNotifyEvent {
            response_type: CONFIGURE_NOTIFY_EVENT,
            sequence: 0,
            event: client,
            window: client,
            above_sibling: 0,
            x: clamp_i16(state.outer.x),
            y: clamp_i16(state.outer.y),
            width: clamp_u16(state.outer.width),
            height: clamp_u16(
                state
                    .outer
                    .height
                    .saturating_sub(u32::from(self.config.titlebar_height)),
            ),
            border_width: 0,
            override_redirect: false,
        };
        self.conn
            .send_event(false, client, EventMask::STRUCTURE_NOTIFY, event)?;
        Ok(())
    }

    fn draw_frame(&mut self, client: Window) -> Result<(), ReplyError> {
        // Live modular path: snapshot -> scene -> manager resolve/plan ->
        // canonical paint (bundled fallback cursor, rounded shape, PANEL
        // `#1b1e20` fill, measured title intent, XRender icon/control blits,
        // red close-hover). Title glyphs draw on the runtime-owned Xft
        // target; this crate records the canonical measure intent only
        // (`unsafe_code = "forbid"` blocks the live Xft constructors here).
        let (frame, icon, wm_class, active, titlebar) = {
            let Some(state) = self.clients.get(&client) else {
                return Ok(());
            };
            (
                state.frame,
                state.icon.clone(),
                self.wm_class(client).unwrap_or_default(),
                self.active.is_none_or(|current| current == client),
                self.config.titlebar_height,
            )
        };
        let source = self.decorations.resolve_app_icon(icon, &wm_class);
        let (snapshot, plan) = {
            let Some(state) = self.clients.get(&client) else {
                return Ok(());
            };
            let snapshot = self
                .decorations
                .snapshot_for(state, &wm_class, active, titlebar, &source);
            let scene = chrome::build_scene(
                u32::from(clamp_u16(state.outer.width)),
                &state.title,
                crate::decoration::scene_role_for(&source),
                state.close_hover.then_some(chrome::ControlRole::Close),
                active,
                state.maximized,
                state.fullscreen,
            );
            let plan = self
                .decorations
                .paint_plan_for(&snapshot, &scene.control_roles);
            (snapshot, plan)
        };
        let slot = u32::from(plan.slot.max(1));
        let icon_rgba = crate::decoration::frame_icon_rgba(&source, slot);
        let outcome = self
            .decorations
            .draw_frame(self.conn, &snapshot, &plan, icon_rgba.as_ref());
        if let Ok(outcome) = outcome.as_ref() {
            let _ = self.decorations.last_outcome().icons_blitted == outcome.icons_blitted;
        }
        let _ = (frame, snapshot.hover, outcome);
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
            crate::decoration::bounding_rectangles(outer)
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
        crate::decoration::icon_fallback_reason(&class, reason)
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
        } else {
            self.frame_to_client.get(&window).copied()
        }
    }

    fn resize_edges(&self, client: &ManagedClient, x: i16, y: i16) -> ResizeEdges {
        self.decorations.resize_edges(client, x, y)
    }

    fn control_at(
        &self,
        client: &ManagedClient,
        x: i16,
    ) -> Option<crate::decoration::FrameControl> {
        self.decorations
            .control_at(client, self.config.titlebar_height, x)
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
    use crate::decoration::interaction::frame_control_at;
    use crate::decoration::{FrameControl, resized_rect};

    #[test]
    fn resize_from_left_preserves_right_edge() {
        let original = Rect::new(100, 100, 600, 400);
        let resized = resized_rect(
            original,
            ResizeEdges {
                left: true,
                ..ResizeEdges::default()
            },
            100,
            0,
        );
        assert_eq!(resized, Rect::new(200, 100, 500, 400));
    }

    #[test]
    fn resize_enforces_minimum_size() {
        let original = Rect::new(100, 100, 600, 400);
        let resized = resized_rect(
            original,
            ResizeEdges {
                left: true,
                top: true,
                ..ResizeEdges::default()
            },
            1_000,
            1_000,
        );
        assert_eq!(resized.width, 160);
        assert_eq!(resized.height, 96);
    }

    #[test]
    fn frame_control_hit_targets_unchanged() {
        let titlebar = chrome::TITLEBAR_HEIGHT;
        // Skin chrome: button_width=38, controls tile the right edge.
        // 400-wide frame: min [286,324), max [324,362), close [362,400).
        assert_eq!(
            frame_control_at(400, titlebar, 390),
            Some(FrameControl::Close)
        );
        assert_eq!(
            frame_control_at(400, titlebar, 360),
            Some(FrameControl::Maximize)
        );
        assert_eq!(
            frame_control_at(400, titlebar, 328),
            Some(FrameControl::Maximize)
        );
        assert_eq!(frame_control_at(400, titlebar, 100), None);
        assert_eq!(
            frame_control_at(400, titlebar, 362),
            Some(FrameControl::Close)
        );
        assert_eq!(
            frame_control_at(400, titlebar, 324),
            Some(FrameControl::Maximize)
        );
        assert_eq!(
            frame_control_at(400, titlebar, 286),
            Some(FrameControl::Minimize)
        );
        assert_eq!(frame_control_at(400, titlebar, 285), None);
    }
}
