use std::collections::VecDeque;

use crate::start::{self, StartCategory};
use flamewm_api::applications::{ApplicationLaunchOptions, DesktopApplication};
use flamewm_api::display::DisplaySnapshot;
use flamewm_api::panels::PanelsSnapshot;
use flamewm_api::session::{SessionAction, SessionCapabilities};
use flamewm_api::system::{SystemAction, SystemSnapshot};
use flamewm_api::window::WindowSnapshot;
use flamewm_api::workspace::WorkspaceSnapshot;
use flamewm_control_core::{ControlRequest, ControlResponse};
use flamewm_control_dbus::ControlClient;
use flamewm_shell_core::popup::{
    anchor_popover_rect, context_menu_rect, context_menu_size, panel_anchor,
};
use flamewm_shell_core::start_surface_layout;
use flamewm_ui_core::style::ShellMetrics;
use flamewm_ui_x11::{
    decode_document, SurfaceConfig, SurfaceHandle, SurfaceInputMode, SurfaceRole, SurfaceRuntime,
    UiBackendError, UiTemplate,
};

const PANEL_ARTIFACT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-panel.rwr"));
const START_ARTIFACT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-start.rwr"));
const START_SUBMENU_ARTIFACT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-start-submenu.rwr"));
const TASK_MENU_ARTIFACT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-task-menu.rwr"));
const MEDIA_ARTIFACT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-media.rwr"));
const AUDIO_ARTIFACT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-audio.rwr"));
const NETWORK_ARTIFACT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-network.rwr"));
const CALENDAR_ARTIFACT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-calendar.rwr"));

/// Complete renderer input. No UI component owns a competing copy of this state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSnapshot {
    pub displays: DisplaySnapshot,
    pub panels: PanelsSnapshot,
    pub windows: Vec<WindowSnapshot>,
    pub applications: Vec<DesktopApplication>,
    pub workspaces: Option<WorkspaceSnapshot>,
    pub session: SessionCapabilities,
    pub system: SystemSnapshot,
}

#[cfg(test)]
impl Default for ShellSnapshot {
    fn default() -> Self {
        Self {
            displays: DisplaySnapshot {
                generation: 0,
                outputs: Vec::new(),
                pending: None,
            },
            panels: PanelsSnapshot {
                revision: 0,
                panels: Vec::new(),
                tasks: Vec::new(),
                pinned_apps: Vec::new(),
            },
            windows: Vec::new(),
            applications: Vec::new(),
            workspaces: None,
            session: SessionCapabilities::default(),
            system: SystemSnapshot {
                revision: 0,
                network: Default::default(),
                media: Default::default(),
                audio: Default::default(),
            },
        }
    }
}

impl ShellSnapshot {
    pub fn load(client: &ControlClient) -> Result<Self, String> {
        let panels = match client.call(&ControlRequest::GetPanels) {
            Ok(ControlResponse::Panels(snapshot)) => snapshot,
            Ok(response) => return Err(format!("GetPanels returned {response:?}")),
            Err(error) => return Err(format!("GetPanels failed: {}", error.message)),
        };
        let windows = match client.call(&ControlRequest::GetWindows) {
            Ok(ControlResponse::Windows(snapshot)) => snapshot,
            Ok(response) => return Err(format!("GetWindows returned {response:?}")),
            Err(error) => return Err(format!("GetWindows failed: {}", error.message)),
        };
        let applications = match client.call(&ControlRequest::GetApplications) {
            Ok(ControlResponse::Applications(snapshot)) => snapshot,
            Ok(response) => return Err(format!("GetApplications returned {response:?}")),
            Err(error) => return Err(format!("GetApplications failed: {}", error.message)),
        };
        let workspaces = match client.call(&ControlRequest::GetWorkspaces) {
            Ok(ControlResponse::Workspaces(snapshot)) => snapshot,
            Ok(response) => return Err(format!("GetWorkspaces returned {response:?}")),
            Err(error) => return Err(format!("GetWorkspaces failed: {}", error.message)),
        };
        let session = match client.call(&ControlRequest::GetSessionCapabilities) {
            Ok(ControlResponse::SessionCapabilities(capabilities)) => capabilities,
            Ok(response) => return Err(format!("GetSessionCapabilities returned {response:?}")),
            Err(error) => return Err(format!("GetSessionCapabilities failed: {}", error.message)),
        };
        let displays = match client.call(&ControlRequest::GetDisplays) {
            Ok(ControlResponse::Displays(snapshot)) => snapshot,
            Ok(response) => return Err(format!("GetDisplays returned {response:?}")),
            Err(error) => return Err(format!("GetDisplays failed: {}", error.message)),
        };
        let system = match client.call(&ControlRequest::GetSystem) {
            Ok(ControlResponse::System(snapshot)) => snapshot,
            Ok(response) => return Err(format!("GetSystem returned {response:?}")),
            Err(error) => return Err(format!("GetSystem failed: {}", error.message)),
        };

        Ok(Self {
            displays,
            panels,
            windows,
            applications,
            workspaces: Some(workspaces),
            session,
            system,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellControl {
    ToggleStart,
    CloseStart,
    SelectStartCategory(StartCategory),
    Launch(String),
    ActivateWindow(flamewm_api::WindowRef),
    MinimizeWindow(flamewm_api::WindowRef),
    RestoreWindow(flamewm_api::WindowRef),
    SystemAction {
        action: SystemAction,
        expected_revision: u64,
    },
    SessionAction(SessionAction),
    OpenPopover(String),
    ClosePopovers,
    OpenContextMenu(ContextMenuState),
    CloseContextMenu,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextMenuKind {
    Task(flamewm_api::TaskEntryId),
    Workspace { index: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMenuState {
    pub kind: ContextMenuKind,
    pub revision: u64,
    /// Root-coordinate pointer anchor (ActionEvent local x/y resolved
    /// against the source surface global rect). `None` only when pointer
    /// data was absent; that is the sole output-origin fallback case.
    pub anchor: Option<flamewm_api::Point>,
}

#[derive(Debug, Clone)]
pub struct ShellRuntime {
    snapshot: ShellSnapshot,
    controls: VecDeque<ShellControl>,
}

#[cfg(test)]
impl Default for ShellRuntime {
    fn default() -> Self {
        Self::new(ShellSnapshot::default())
    }
}

impl ShellRuntime {
    #[must_use]
    pub fn new(snapshot: ShellSnapshot) -> Self {
        Self {
            snapshot,
            controls: VecDeque::new(),
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> &ShellSnapshot {
        &self.snapshot
    }

    pub fn replace_snapshot(&mut self, snapshot: ShellSnapshot) {
        self.snapshot = snapshot;
    }

    /// Revision-gated dynamic resnapshot driven only by reactor wakeups.
    ///
    /// Fetches `GetSystem` plus the live panel/task/workspace domains and
    /// applies them when any revision or content differs. Returns `true` when
    /// the caller must reproject. No polling loop is created here; the shell
    /// loop calls this from existing surface-event and clock-tick paths.
    pub fn resnapshot_dynamic(&mut self, client: &ControlClient) -> Result<bool, String> {
        let system = match client.call(&ControlRequest::GetSystem) {
            Ok(ControlResponse::System(snapshot)) => snapshot,
            Ok(response) => return Err(format!("GetSystem returned {response:?}")),
            Err(error) => return Err(format!("GetSystem failed: {}", error.message)),
        };
        let workspaces = match client.call(&ControlRequest::GetWorkspaces) {
            Ok(ControlResponse::Workspaces(snapshot)) => snapshot,
            Ok(response) => return Err(format!("GetWorkspaces returned {response:?}")),
            Err(error) => return Err(format!("GetWorkspaces failed: {}", error.message)),
        };
        let panels = match client.call(&ControlRequest::GetPanels) {
            Ok(ControlResponse::Panels(snapshot)) => snapshot,
            Ok(response) => return Err(format!("GetPanels returned {response:?}")),
            Err(error) => return Err(format!("GetPanels failed: {}", error.message)),
        };
        let windows = match client.call(&ControlRequest::GetWindows) {
            Ok(ControlResponse::Windows(snapshot)) => snapshot,
            Ok(response) => return Err(format!("GetWindows returned {response:?}")),
            Err(error) => return Err(format!("GetWindows failed: {}", error.message)),
        };
        let changed = system.revision != self.snapshot.system.revision
            || self.snapshot.workspaces.as_ref() != Some(&workspaces)
            || panels != self.snapshot.panels
            || windows != self.snapshot.windows;
        if changed {
            self.snapshot.system = system;
            self.snapshot.workspaces = Some(workspaces);
            self.snapshot.panels = panels;
            self.snapshot.windows = windows;
        }
        Ok(changed)
    }

    #[must_use]
    pub fn control_request_for_action(&self, action: &str) -> Option<ControlRequest> {
        let slot = action
            .strip_prefix("workspace-")
            .or_else(|| action.strip_prefix("workspace."))
            .and_then(|value| value.parse::<usize>().ok())?
            .checked_sub(1)?;
        let workspaces = crate::taskbar::workspaces::project(self.snapshot.workspaces.as_ref());
        let page = crate::taskbar::workspaces::visible_page(&workspaces);
        let workspace = page.get(slot)?;
        crate::taskbar::workspaces::control_request_for_click(
            self.snapshot.workspaces.as_ref(),
            workspace.index,
        )
    }

    /// Map a stable context-menu row action (`task.menu.<row>` /
    /// `workspace.menu.<row>`) plus the open menu state to a typed control.
    /// Row identity is the row id, never the label. Pinned task entries
    /// resolve `unpin` only; running non-pinned entries resolve
    /// `activate`/`close`. Workspace rows are revision-fenced against the
    /// open menu.
    #[must_use]
    pub fn context_menu_request_for_action(
        &self,
        action: &str,
        menu: Option<&ContextMenuState>,
    ) -> Option<ControlRequest> {
        let menu = menu?;
        match &menu.kind {
            ContextMenuKind::Task(id) => {
                if self.snapshot.panels.revision != menu.revision {
                    return None;
                }
                let row = action.strip_prefix("task.menu.")?;
                let entry = self
                    .snapshot
                    .panels
                    .tasks
                    .iter()
                    .find(|entry| &entry.id == id)?;
                if entry.id.is_pinned() {
                    if row != crate::taskbar::context_menu::TASK_ROW_UNPIN {
                        return None;
                    }
                    return Some(ControlRequest::UnpinApp {
                        app: entry.app_id.clone(),
                        expected_revision: self.snapshot.panels.revision,
                    });
                }
                let window = match entry.kind {
                    flamewm_api::panels::TaskEntryKind::PinnedSlot { window } => window,
                    flamewm_api::panels::TaskEntryKind::Window { window } => Some(window),
                }?;
                if !self
                    .snapshot
                    .windows
                    .iter()
                    .any(|snapshot| snapshot.reference == window)
                {
                    return None;
                }
                match row {
                    crate::taskbar::context_menu::TASK_ROW_ACTIVATE => {
                        Some(ControlRequest::ActivateWindow(window))
                    }
                    crate::taskbar::context_menu::TASK_ROW_CLOSE => {
                        Some(ControlRequest::CloseWindow(window))
                    }
                    _ => None,
                }
            }
            ContextMenuKind::Workspace { index } => {
                let row = action.strip_prefix("workspace.menu.")?;
                let snapshot = self.snapshot.workspaces.as_ref()?;
                if snapshot.revision != menu.revision {
                    return None;
                }
                crate::taskbar::context_menu::workspace_action(snapshot, *index, row)
                    .map(|intent| intent.into_control_request())
            }
        }
    }

    /// Typed SystemAction control for audio rows from a provider snapshot.
    /// The provider owns volumes/mutes; the shell only forwards the id-keyed
    /// action. Rejects out-of-range percents the same way the provider does.
    #[must_use]
    pub fn audio_system_action_for_row(
        &self,
        row_id: &str,
        percent: Option<u8>,
        muted: Option<bool>,
    ) -> Option<ControlRequest> {
        crate::taskbar::status::audio::system_action_for_row(
            &self.snapshot.system,
            row_id,
            percent,
            muted,
        )
    }

    /// Typed SystemAction control for network rows from a provider snapshot.
    /// Secrets travel via the SecretAgent snapshot only; this never carries
    /// a secret value.
    #[must_use]
    pub fn network_system_action_for_path(
        &self,
        path: &str,
        scan: bool,
        disconnect: bool,
    ) -> Option<ControlRequest> {
        crate::taskbar::status::network::system_action_for_path(
            &self.snapshot.system,
            path,
            scan,
            disconnect,
        )
    }

    #[must_use]
    pub fn task_click_requests_for_slot(&self, slot: usize) -> Option<Vec<ControlRequest>> {
        let task_visual_states = flamewm_shell_core::task_visual_states(
            &self.snapshot.panels,
            &self.snapshot.windows,
            &self.snapshot.applications,
        );
        let state = task_visual_states.get(slot)?;
        let entry = self
            .snapshot
            .panels
            .tasks
            .iter()
            .find(|entry| entry.id == state.id)?;
        Some(
            match flamewm_shell_core::task_click_action(entry, &self.snapshot.windows) {
                flamewm_shell_core::TaskClickAction::Launch(app) => {
                    vec![ControlRequest::LaunchApplication {
                        app,
                        options: ApplicationLaunchOptions::default(),
                    }]
                }
                flamewm_shell_core::TaskClickAction::RestoreAndActivate(window) => vec![
                    ControlRequest::RestoreWindow(window),
                    ControlRequest::ActivateWindow(window),
                ],
                flamewm_shell_core::TaskClickAction::Minimize(window) => {
                    vec![ControlRequest::MinimizeWindow(window)]
                }
                flamewm_shell_core::TaskClickAction::Activate(window) => {
                    vec![ControlRequest::ActivateWindow(window)]
                }
            },
        )
    }

    pub fn open_task_context_for_slot(&mut self, slot: usize) {
        self.open_task_context_at_slot(slot, None);
    }

    /// Pointer-anchored variant: `pointer` is the root-coordinate click
    /// point (local x/y + source surface global rect).
    pub fn open_task_context_at_slot(&mut self, slot: usize, pointer: Option<flamewm_api::Point>) {
        let states = flamewm_shell_core::task_visual_states(
            &self.snapshot.panels,
            &self.snapshot.windows,
            &self.snapshot.applications,
        );
        if let Some(state) = states.get(slot) {
            self.controls
                .push_back(ShellControl::OpenContextMenu(ContextMenuState {
                    kind: ContextMenuKind::Task(state.id.clone()),
                    revision: self.snapshot.panels.revision,
                    anchor: pointer,
                }));
        }
    }

    pub fn open_workspace_context_for_slot(&mut self, slot: usize) {
        self.open_workspace_context_at_slot(slot, None);
    }

    /// Pointer-anchored variant: `pointer` is the root-coordinate click
    /// point (local x/y + source surface global rect).
    pub fn open_workspace_context_at_slot(
        &mut self,
        slot: usize,
        pointer: Option<flamewm_api::Point>,
    ) {
        let views = crate::taskbar::workspaces::project(self.snapshot.workspaces.as_ref());
        if let Some(workspace) = crate::taskbar::workspaces::visible_page(&views).get(slot) {
            if let Some(snapshot) = self.snapshot.workspaces.as_ref() {
                self.controls
                    .push_back(ShellControl::OpenContextMenu(ContextMenuState {
                        kind: ContextMenuKind::Workspace {
                            index: workspace.index,
                        },
                        revision: snapshot.revision,
                        anchor: pointer,
                    }));
            }
        }
    }

    pub fn dispatch(&mut self, action: &str) {
        let control = match action {
            "start.toggle" => Some(ShellControl::ToggleStart),
            "start.close" => Some(ShellControl::CloseStart),
            value if value.starts_with("start.category.") => {
                start::category_for_action(value).map(ShellControl::SelectStartCategory)
            }
            "media.open" | "media.toggle" | "volume.open" | "network.open" | "clock.open" => Some(
                ShellControl::OpenPopover(action.split('.').next().unwrap_or(action).to_owned()),
            ),
            "popover.close" => Some(ShellControl::ClosePopovers),
            "session.lock" => Some(ShellControl::SessionAction(SessionAction::Lock)),
            "session.logout" => Some(ShellControl::SessionAction(SessionAction::Logout)),
            "session.suspend" => Some(ShellControl::SessionAction(SessionAction::Suspend)),
            "session.reboot" => Some(ShellControl::SessionAction(SessionAction::Reboot)),
            "session.shutdown" => Some(ShellControl::SessionAction(SessionAction::Shutdown)),
            value if value.starts_with("start.") => {
                Some(ShellControl::Launch(value[6..].to_owned()))
            }
            _ => None,
        };
        if let Some(control) = control {
            self.controls.push_back(control);
        }
    }

    pub fn drain_controls(&mut self) -> impl Iterator<Item = ShellControl> + '_ {
        self.controls.drain(..)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Transient {
    StartGroup,
    Media,
    Audio,
    Network,
    Calendar,
    ContextMenu,
}

pub struct ShellSurfaces {
    pub panel: SurfaceHandle,
    pub start: SurfaceHandle,
    pub start_submenu: SurfaceHandle,
    pub task_menu: SurfaceHandle,
    pub media: SurfaceHandle,
    pub audio: SurfaceHandle,
    pub network: SurfaceHandle,
    pub calendar: SurfaceHandle,
    output: flamewm_api::display::OutputSnapshot,
    panel_snapshot: flamewm_api::panels::PanelSnapshot,
    start_layout: flamewm_shell_core::StartSurfaceLayout,
    transient: Option<Transient>,
}

impl ShellSurfaces {
    pub fn create(runtime: &mut SurfaceRuntime, snapshot: &ShellSnapshot) -> Result<Self, String> {
        let panel_snapshot = snapshot
            .panels
            .panels
            .first()
            .cloned()
            .ok_or_else(|| "GetPanels returned no panel".to_owned())?;
        let output = snapshot
            .displays
            .outputs
            .iter()
            .find(|output| output.id == panel_snapshot.output && output.connected)
            .cloned()
            .ok_or_else(|| "GetDisplays returned no connected panel output".to_owned())?;
        let metrics = ShellMetrics::default();
        let start_layout = start_surface_layout(
            panel_snapshot.output.clone(),
            output.geometry,
            panel_snapshot.edge,
            metrics,
        );
        let panel = create_surface(
            runtime,
            PANEL_ARTIFACT,
            SurfaceConfig {
                width: checked_size(panel_snapshot.geometry.width)?,
                height: checked_size(panel_snapshot.geometry.height)?,
                title: "FlameWM panel".to_owned(),
                role: SurfaceRole::Dock,
                input: SurfaceInputMode::Interactive,
                initially_visible: panel_snapshot.visible,
                x: panel_snapshot.geometry.x,
                y: panel_snapshot.geometry.y,
            },
        )?;
        let start = create_surface(
            runtime,
            START_ARTIFACT,
            popup_config(
                "FlameWM start",
                SurfaceRole::PopupMenu,
                start_layout.popover,
            ),
        )?;
        let start_submenu = create_surface(
            runtime,
            START_SUBMENU_ARTIFACT,
            popup_config(
                "FlameWM start submenu",
                SurfaceRole::DropdownMenu,
                flamewm_api::Rect::new(
                    start_layout.popover.right(),
                    start_layout.popover.y,
                    i32::from(metrics.start_submenu_width),
                    i32::from(metrics.start_menu_min_height),
                ),
            ),
        )?;
        let task_menu = create_surface(
            runtime,
            TASK_MENU_ARTIFACT,
            popup_config(
                "FlameWM task menu",
                SurfaceRole::PopupMenu,
                flamewm_api::Rect::new(output.geometry.x, output.geometry.y, 217, 180),
            ),
        )?;
        let media = create_surface(
            runtime,
            MEDIA_ARTIFACT,
            status_popup_config(
                "FlameWM media",
                flamewm_shell_core::MEDIA_POPOVER_SIZE,
                panel_anchor(&panel_snapshot, &output, metrics, 1),
            ),
        )?;
        let audio = create_surface(
            runtime,
            AUDIO_ARTIFACT,
            status_popup_config(
                "FlameWM audio",
                flamewm_shell_core::AUDIO_POPOVER_SIZE,
                panel_anchor(&panel_snapshot, &output, metrics, 0),
            ),
        )?;
        let network = create_surface(
            runtime,
            NETWORK_ARTIFACT,
            status_popup_config(
                "FlameWM network",
                flamewm_shell_core::NETWORK_POPOVER_SIZE,
                panel_anchor(&panel_snapshot, &output, metrics, 2),
            ),
        )?;
        let calendar = create_surface(
            runtime,
            CALENDAR_ARTIFACT,
            status_popup_config(
                "FlameWM calendar",
                flamewm_shell_core::calendar_popover_size(metrics),
                panel_anchor(&panel_snapshot, &output, metrics, 3),
            ),
        )?;
        Ok(Self {
            panel,
            start,
            start_submenu,
            task_menu,
            media,
            audio,
            network,
            calendar,
            output,
            panel_snapshot,
            start_layout,
            transient: None,
        })
    }

    pub fn close_transients(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        for surface in [
            self.start,
            self.start_submenu,
            self.task_menu,
            self.media,
            self.audio,
            self.network,
            self.calendar,
        ] {
            if runtime.is_pointer_grabbed(surface).map_err(ui_error)? {
                runtime.ungrab_pointer(surface).map_err(ui_error)?;
            }
            runtime.hide(surface).map_err(ui_error)?;
        }
        self.transient = None;
        Ok(())
    }

    pub fn toggle_start(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        if self.transient == Some(Transient::StartGroup) {
            return self.close_transients(runtime);
        }
        self.close_transients(runtime)?;
        // Panel toggle opens the same single-grab root+submenu group so
        // root and submenu never hold competing grabs.
        self.show(runtime, self.start, self.start_layout.popover)?;
        let rect = flamewm_api::Rect::new(
            self.start_layout.popover.right(),
            self.start_layout.popover.y,
            i32::from(ShellMetrics::default().start_submenu_width),
            i32::from(ShellMetrics::default().start_menu_min_height),
        );
        self.show_grabbed(runtime, self.start_submenu, rect, Transient::StartGroup)
    }

    fn open_start_submenu_tail(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        let size = runtime
            .document_intrinsic_size(self.start_submenu)
            .map(|(width, height)| {
                flamewm_api::Size::new(width.max(1.0) as i32, height.max(1.0) as i32)
            })
            .unwrap_or(flamewm_api::Size::new(280, 320));
        let clamped = flamewm_api::Size::new(
            size.width.min(self.output.geometry.width.max(1)),
            size.height.min(self.output.geometry.height.max(1)),
        );
        let geometry = flamewm_ui_core::popover::PopoverGeometry::place_aligned(
            self.start_layout.popover,
            clamped,
            match self.panel_snapshot.edge {
                flamewm_api::PanelEdge::Right => flamewm_ui_core::popover::PopoverEdge::Left,
                _ => flamewm_ui_core::popover::PopoverEdge::Right,
            },
            flamewm_ui_core::popover::PopoverAlign::Start,
            self.output.geometry,
            0,
        );
        let rect = flamewm_api::Rect::from_parts(geometry.origin, clamped);
        self.show_grabbed(runtime, self.start_submenu, rect, Transient::StartGroup)
    }
    /// outside release, Escape, or launch closes both surfaces, and the grab
    /// is released exactly once through `close_transients`.
    pub fn open_start_group(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        if self.transient == Some(Transient::StartGroup) {
            return Ok(());
        }
        self.close_transients(runtime)?;
        self.show(runtime, self.start, self.start_layout.popover)?;
        return self.open_start_submenu_tail(runtime);
    }

    #[must_use]
    pub fn is_start_group_open(&self) -> bool {
        self.transient == Some(Transient::StartGroup)
    }

    /// Closes the Start group (root plus submenu) with a single ungrab,
    /// tolerating any legacy stray state through `close_transients`.
    pub fn close_start_group(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        if self.transient == Some(Transient::StartGroup) {
            return self.close_transients(runtime);
        }
        Ok(())
    }

    pub fn open_start_submenu(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        self.open_start_group(runtime)
    }

    pub fn open_status(&mut self, runtime: &mut SurfaceRuntime, name: &str) -> Result<(), String> {
        // Live path: stable panel node ids, edge from the panel edge, Align
        // End; no fixed popup_rect. Re-measure happens via intrinsic size
        // after each content projection.
        self.open_status_anchored_id(runtime, name, status_source_id(name))
    }

    /// Anchored open from a stable panel node id (`tray-media`,
    /// `tray-volume`, `tray-network`, `clock-button`).
    pub fn open_status_anchored_id(
        &mut self,
        runtime: &mut SurfaceRuntime,
        name: &str,
        source_id: &str,
    ) -> Result<(), String> {
        let (surface, transient) = match name {
            "media" => (self.media, Transient::Media),
            "volume" | "audio" => (self.audio, Transient::Audio),
            "network" => (self.network, Transient::Network),
            "clock" => (self.calendar, Transient::Calendar),
            _ => return Err(format!("unsupported shell popup {name}")),
        };
        self.close_transients(runtime)?;
        let rect = anchored_rect_by_id(
            runtime,
            self.output.geometry,
            self.panel,
            source_id,
            surface,
            panel_edge_to_popover(self.panel_snapshot.edge),
        );
        self.show_grabbed(runtime, surface, rect, transient)
    }

    /// Re-measure the open popup after content changed: intrinsic size +
    /// clamp + Align End placement. No-op when nothing is open.
    pub fn remeasure_open_popup(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        let (surface, name) = match self.transient {
            Some(Transient::Media) => (self.media, "media"),
            Some(Transient::Audio) => (self.audio, "audio"),
            Some(Transient::Network) => (self.network, "network"),
            Some(Transient::Calendar) => (self.calendar, "clock"),
            _ => return Ok(()),
        };
        let rect = anchored_rect_by_id(
            runtime,
            self.output.geometry,
            self.panel,
            status_source_id(name),
            surface,
            panel_edge_to_popover(self.panel_snapshot.edge),
        );
        self.show(runtime, surface, rect)
    }

    pub fn open_status_anchored(
        &mut self,
        runtime: &mut SurfaceRuntime,
        name: &str,
        source_surface: SurfaceHandle,
        source_node: u32,
    ) -> Result<(), String> {
        let (surface, transient, edge) = match name {
            "media" => (
                self.media,
                Transient::Media,
                flamewm_ui_core::popover::PopoverEdge::Above,
            ),
            "volume" | "audio" => (
                self.audio,
                Transient::Audio,
                flamewm_ui_core::popover::PopoverEdge::Above,
            ),
            "network" => (
                self.network,
                Transient::Network,
                flamewm_ui_core::popover::PopoverEdge::Above,
            ),
            "clock" => (
                self.calendar,
                Transient::Calendar,
                flamewm_ui_core::popover::PopoverEdge::Above,
            ),
            _ => return Err(format!("unsupported shell popup {name}")),
        };
        self.close_transients(runtime)?;
        let rect = anchored_rect(
            runtime,
            self.output.geometry,
            source_surface,
            source_node,
            surface,
            edge,
        );
        self.show_grabbed(runtime, surface, rect, transient)
    }

    pub fn open_context_menu(
        &mut self,
        runtime: &mut SurfaceRuntime,
        state: &ContextMenuState,
        snapshot: &ShellSnapshot,
    ) -> Result<(), String> {
        self.close_transients(runtime)?;
        // Shrink the native surface to its visible content rows so no
        // spare (black) area remains for the shape mask to cover.
        let rows = crate::taskbar::context_menu::visible_row_count_for_kind(
            &snapshot.panels,
            &snapshot.windows,
            snapshot.workspaces.as_ref(),
            &state.kind,
        );
        let size = context_menu_size(rows);
        let rect = context_menu_rect(
            self.panel_snapshot.output.clone(),
            self.output.geometry,
            state.anchor,
            size,
        );
        self.show_grabbed(runtime, self.task_menu, rect, Transient::ContextMenu)
    }

    /// Resolve a surface-local pointer position (ActionEvent x/y from the
    /// panel surface) to root coordinates via the panel global rect.
    #[must_use]
    pub fn root_pointer(&self, x: f32, y: f32) -> flamewm_api::Point {
        flamewm_api::Point::new(
            self.panel_snapshot.geometry.x + x as i32,
            self.panel_snapshot.geometry.y + y as i32,
        )
    }

    fn show_grabbed(
        &mut self,
        runtime: &mut SurfaceRuntime,
        surface: SurfaceHandle,
        rect: flamewm_api::Rect,
        transient: Transient,
    ) -> Result<(), String> {
        self.show(runtime, surface, rect)?;
        runtime.raise(surface).map_err(ui_error)?;
        let _ = runtime.redraw(surface).map_err(ui_error)?;
        runtime.grab_pointer(surface).map_err(ui_error)?;
        self.transient = Some(transient);
        Ok(())
    }

    fn show(
        &self,
        runtime: &mut SurfaceRuntime,
        surface: SurfaceHandle,
        rect: flamewm_api::Rect,
    ) -> Result<(), String> {
        runtime
            .move_resize(
                surface,
                rect.x,
                rect.y,
                checked_size(rect.width)?,
                checked_size(rect.height)?,
            )
            .map_err(ui_error)?;
        runtime.show(surface).map_err(ui_error)
    }
}

fn create_surface(
    runtime: &mut SurfaceRuntime,
    bytes: &[u8],
    config: SurfaceConfig,
) -> Result<SurfaceHandle, String> {
    let document = decode_document(bytes)?;
    runtime
        .create_surface(UiTemplate::new(document), config)
        .map_err(ui_error)
}

fn popup_config(title: &str, role: SurfaceRole, rect: flamewm_api::Rect) -> SurfaceConfig {
    SurfaceConfig {
        width: checked_size(rect.width).unwrap_or(1),
        height: checked_size(rect.height).unwrap_or(1),
        title: title.to_owned(),
        role,
        input: SurfaceInputMode::Interactive,
        initially_visible: false,
        x: rect.x,
        y: rect.y,
    }
}

fn status_popup_config(
    title: &str,
    size: flamewm_api::Size,
    anchor: flamewm_api::Rect,
) -> SurfaceConfig {
    let rect = anchor_popover_rect(anchor, size);
    popup_config(title, SurfaceRole::PopupMenu, rect)
}

fn checked_size(value: i32) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("invalid surface size {value}"))
}

fn status_source_id(name: &str) -> &'static str {
    match name {
        "media" => "tray-media",
        "volume" | "audio" => "tray-volume",
        "network" => "tray-network",
        "clock" => "clock-button",
        _ => "tray-network",
    }
}

fn panel_edge_to_popover(edge: flamewm_api::PanelEdge) -> flamewm_ui_core::popover::PopoverEdge {
    match edge {
        flamewm_api::PanelEdge::Top => flamewm_ui_core::popover::PopoverEdge::Below,
        _ => flamewm_ui_core::popover::PopoverEdge::Above,
    }
}

/// Measured anchoring from a stable panel node id: source rect from
/// `node_global_rect_by_id`, popup size from the intrinsic document size,
/// placed with `PopoverGeometry::place_aligned` + Align End, clamped to the
/// work area.
fn anchored_rect_by_id(
    runtime: &mut SurfaceRuntime,
    work_area: flamewm_api::Rect,
    panel: SurfaceHandle,
    source_id: &str,
    popup_surface: SurfaceHandle,
    edge: flamewm_ui_core::popover::PopoverEdge,
) -> flamewm_api::Rect {
    let source = runtime
        .node_global_rect_by_id(panel, source_id)
        .map(|rect| {
            flamewm_api::Rect::new(
                rect.x as i32,
                rect.y as i32,
                rect.width.max(1.0) as i32,
                rect.height.max(1.0) as i32,
            )
        })
        .unwrap_or(work_area);
    let size = runtime
        .document_intrinsic_size(popup_surface)
        .map(|(width, height)| {
            flamewm_api::Size::new(width.max(1.0) as i32, height.max(1.0) as i32)
        })
        .unwrap_or(flamewm_api::Size::new(310, 250));
    let geometry = flamewm_ui_core::popover::PopoverGeometry::place_aligned(
        source,
        size,
        edge,
        flamewm_ui_core::popover::PopoverAlign::End,
        work_area,
        8,
    );
    flamewm_api::Rect::from_parts(geometry.origin, size)
}

/// Measured anchoring: source rect from `node_global_rect`, popup size from
/// the intrinsic document size, placed with `PopoverGeometry::place`.
fn anchored_rect(
    runtime: &mut SurfaceRuntime,
    work_area: flamewm_api::Rect,
    source_surface: SurfaceHandle,
    source_node: u32,
    popup_surface: SurfaceHandle,
    edge: flamewm_ui_core::popover::PopoverEdge,
) -> flamewm_api::Rect {
    let source = runtime
        .node_global_rect(source_surface, source_node)
        .map(|rect| {
            flamewm_api::Rect::new(
                rect.x as i32,
                rect.y as i32,
                rect.width.max(1.0) as i32,
                rect.height.max(1.0) as i32,
            )
        })
        .unwrap_or(work_area);
    let size = runtime
        .document_intrinsic_size(popup_surface)
        .map(|(width, height)| {
            flamewm_api::Size::new(width.max(1.0) as i32, height.max(1.0) as i32)
        })
        .unwrap_or(flamewm_api::Size::new(310, 250));
    let geometry =
        flamewm_ui_core::popover::PopoverGeometry::place(source, size, edge, work_area, 8);
    flamewm_api::Rect::from_parts(geometry.origin, size)
}

fn ui_error(error: UiBackendError) -> String {
    format!("{error:?}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_output_rect() -> flamewm_api::Rect {
        flamewm_api::Rect::new(0, 0, 1920, 1080)
    }

    fn test_output_id() -> flamewm_api::OutputId {
        flamewm_api::OutputId::new("HDMI-1")
    }

    #[test]
    fn workspace_right_click_rect_is_near_pointer_not_origin() {
        let size = context_menu_size(3);
        let pointer = Some(flamewm_api::Point::new(400, 900));
        let rect = context_menu_rect(test_output_id(), test_output_rect(), pointer, size);
        assert_eq!((rect.x, rect.y), (400, 900));
        assert_eq!((rect.width, rect.height), (size.width, size.height));
    }

    #[test]
    fn context_menu_is_clamped_inside_output() {
        let size = context_menu_size(3);
        let pointer = Some(flamewm_api::Point::new(1900, 1060));
        let rect = context_menu_rect(test_output_id(), test_output_rect(), pointer, size);
        assert_eq!(rect.width, size.width);
        assert_eq!(rect.height, size.height);
        assert!(rect.right() <= 1920);
        assert!(rect.bottom() <= 1080);
        assert!(rect.x >= 0 && rect.y >= 0);
    }

    #[test]
    fn context_menu_size_shrinks_to_visible_rows() {
        assert_eq!(context_menu_size(1).height, 41);
        assert_eq!(context_menu_size(2).height, 72);
        assert_eq!(context_menu_size(3).height, 103);
        assert!(context_menu_size(1).height < 180);
        assert!(context_menu_size(3).height < 180);
    }

    #[test]
    fn absent_pointer_falls_back_to_output_origin() {
        let size = context_menu_size(2);
        let rect = context_menu_rect(test_output_id(), test_output_rect(), None, size);
        assert_eq!((rect.x, rect.y), (0, 0));
    }
}
