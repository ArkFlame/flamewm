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
use flamewm_control_wire::ControlSignal;
use flamewm_shell_core::popup::{
    anchor_popover_rect, context_menu_rect, context_menu_size, fitted_start_placement,
    measured_popup_rect, panel_anchor, work_area_for_panel, PopupRefusal,
};
use flamewm_shell_core::start_surface_layout;
use flamewm_ui_core::style::ShellMetrics;
use flamewm_ui_x11::{
    decode_document, GeometryTrace, SurfaceConfig, SurfaceHandle, SurfaceInputMode, SurfaceRole,
    SurfaceRuntime, UiBackendError, UiTemplate,
};

const PANEL_ARTIFACT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-panel.rwr"));
const START_ARTIFACT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-start.rwr"));
const TASK_MENU_ARTIFACT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-task-menu.rwr"));
const MEDIA_ARTIFACT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-media.rwr"));
// J07: audio/network/calendar artifacts live in the helper role
// (`quick_controls::host`), not in the parent runtime.

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
    /// Single-roundtrip startup: exactly one `GetShellBootstrap` call. The
    /// aggregate carries every domain the panel/start surfaces need;
    /// steady-state updates arrive via `ControlSignal` delivery, never by
    /// re-calling this loader.
    pub fn load(client: &ControlClient) -> Result<Self, String> {
        let bootstrap = match client.call(&ControlRequest::GetShellBootstrap) {
            Ok(ControlResponse::ShellBootstrap(snapshot)) => snapshot,
            Ok(response) => return Err(format!("GetShellBootstrap returned {response:?}")),
            Err(error) => return Err(format!("GetShellBootstrap failed: {}", error.message)),
        };

        Ok(Self {
            displays: bootstrap.displays,
            panels: bootstrap.panels,
            windows: bootstrap.windows,
            applications: bootstrap.applications,
            workspaces: Some(bootstrap.workspaces),
            session: bootstrap.session,
            system: bootstrap.system,
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
    dirty: ShellDirty,
}

/// Domain dirty flags set by `ControlSignal` delivery and reconciled at most
/// once per turn. Each flag preserves single ownership: windows, workspaces,
/// panels, system, and applications segments map to one fetch each.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShellDirty {
    pub windows: bool,
    pub workspaces: bool,
    pub panels: bool,
    pub system: bool,
    pub applications: bool,
}

impl ShellDirty {
    #[must_use]
    pub fn any(self) -> bool {
        self.windows || self.workspaces || self.panels || self.system || self.applications
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Per-domain change set from `reconcile_dirty`. The shell loop projects
/// only the surfaces each changed domain owns; unchanged domains (and
/// hidden surfaces) are never reprojected.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChangedDomains {
    pub windows: bool,
    pub workspaces: bool,
    pub panels: bool,
    pub system: bool,
    pub applications: bool,
}

impl ChangedDomains {
    #[must_use]
    pub fn any(self) -> bool {
        self.windows || self.workspaces || self.panels || self.system || self.applications
    }
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
            dirty: ShellDirty::default(),
        }
    }

    #[must_use]
    pub fn dirty(&self) -> ShellDirty {
        self.dirty
    }

    /// Record a delivered `ControlSignal` by applying snapshot payloads
    /// directly. No fetch happens here and legacy revision-only signals
    /// (`WindowsChanged` / `WorkspacesChanged` / `PanelsChanged`) are
    /// ignored for normal refresh; the shell loop projects from the
    /// already-applied snapshot once per turn.
    pub fn mark_signal(&mut self, signal: &ControlSignal) {
        match signal {
            ControlSignal::WindowsSnapshotChanged { windows, .. } => {
                let _s = crate::runtime::shell_span("shell.signal_to_visual.windows").start();
                if *windows != self.snapshot.windows {
                    self.snapshot.windows = windows.clone();
                    self.dirty.windows = true;
                }
            }
            ControlSignal::WorkspacesSnapshotChanged { snapshot } => {
                let _s = crate::runtime::shell_span("shell.signal_to_visual.workspaces").start();
                if self.snapshot.workspaces.as_ref() != Some(snapshot) {
                    self.snapshot.workspaces = Some(snapshot.clone());
                    self.dirty.workspaces = true;
                }
            }
            ControlSignal::PanelsSnapshotChanged { snapshot } => {
                if *snapshot != self.snapshot.panels {
                    self.snapshot.panels = snapshot.clone();
                    self.dirty.panels = true;
                }
            }
            ControlSignal::SystemChanged { .. } => self.dirty.system = true,
            _ => {}
        }
    }

    /// Drain already-applied snapshot domains (windows/workspaces/panels)
    /// into a change set without any fetch. Proves the zero-fetch contract:
    /// snapshot signals need no `GetWindows` / `GetWorkspaces` / `GetPanels`
    /// roundtrip.
    pub fn reconcile_applied(&mut self) -> ChangedDomains {
        let mut changed = ChangedDomains::default();
        if self.dirty.windows {
            changed.windows = true;
            self.dirty.windows = false;
        }
        if self.dirty.workspaces {
            changed.workspaces = true;
            self.dirty.workspaces = false;
        }
        if self.dirty.panels {
            changed.panels = true;
            self.dirty.panels = false;
        }
        changed
    }

    /// Reconcile remaining fetch-owned dirty domains (currently system
    /// only). Windows/workspaces/panels arrive as snapshot payloads via
    /// `mark_signal` + `reconcile_applied` and are never fetched here;
    /// legacy revision-only signals set no flags for normal refresh.
    /// Returns the set of domains that actually changed so the caller can
    /// project only the affected surfaces (§6: Windows->task slots only,
    /// Workspaces->pager only, no broad refresh of hidden surfaces).
    pub fn reconcile_dirty(&mut self, client: &ControlClient) -> Result<ChangedDomains, String> {
        let mut changed = self.reconcile_applied();
        if !self.dirty.any() {
            return Ok(changed);
        }
        if self.dirty.system {
            let system = match client.call(&ControlRequest::GetSystem) {
                Ok(ControlResponse::System(snapshot)) => snapshot,
                Ok(response) => return Err(format!("GetSystem returned {response:?}")),
                Err(error) => return Err(format!("GetSystem failed: {}", error.message)),
            };
            if system.revision != self.snapshot.system.revision {
                self.snapshot.system = system;
                changed.system = true;
                domain_reconcile_counter(DomainKind::System).increment();
            }
        }
        // Applications are owned by `refresh_applications` (gated on the
        // applications dirty flag by the caller); never fetched here so one
        // flag maps to exactly one fetch.
        self.dirty.clear();
        Ok(changed)
    }

    /// Fetch the live applications list and replace the start-set domain.
    /// Returns `true` when applications content changed. Always consumes
    /// the applications dirty flag so one flag maps to exactly one fetch.
    pub fn refresh_applications(&mut self, client: &ControlClient) -> Result<bool, String> {
        self.dirty.applications = false;
        let applications = match client.call(&ControlRequest::GetApplications) {
            Ok(ControlResponse::Applications(snapshot)) => snapshot,
            Ok(response) => return Err(format!("GetApplications returned {response:?}")),
            Err(error) => return Err(format!("GetApplications failed: {}", error.message)),
        };
        if applications == self.snapshot.applications {
            return Ok(false);
        }
        self.snapshot.applications = applications;
        domain_reconcile_counter(DomainKind::Applications).increment();
        Ok(true)
    }

    #[must_use]
    pub fn snapshot(&self) -> &ShellSnapshot {
        &self.snapshot
    }

    pub fn replace_snapshot(&mut self, snapshot: ShellSnapshot) {
        self.snapshot = snapshot;
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
    ContextMenu,
}

/// Resolved panel placement inputs: snapshot rects plus derived work
/// area and Start layout. Stored in `ShellSurfaces`; recomputed by
/// `sync_panel_context` on panel/display snapshot change (pure/state,
/// main.rs wiring reserved for J07).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelPlacementContext {
    pub output: flamewm_api::display::OutputSnapshot,
    pub panel: flamewm_api::panels::PanelSnapshot,
    pub work_area: flamewm_api::Rect,
    pub start_layout: flamewm_shell_core::StartSurfaceLayout,
    pub display_generation: u64,
    pub panels_revision: u64,
}

impl PanelPlacementContext {
    #[must_use]
    pub fn new(
        output: flamewm_api::display::OutputSnapshot,
        panel: flamewm_api::panels::PanelSnapshot,
        display_generation: u64,
        panels_revision: u64,
    ) -> Self {
        let work_area = work_area_for_panel(output.geometry, panel.geometry, panel.edge);
        let start_layout = start_surface_layout(
            panel.output.clone(),
            output.geometry,
            panel.edge,
            ShellMetrics::default(),
        );
        Self {
            output,
            panel,
            work_area,
            start_layout,
            display_generation,
            panels_revision,
        }
    }
}

/// Resolve a `PanelPlacementContext` from a snapshot without touching
/// live surfaces. Returns `None` when the panel or its connected output
/// is missing, or when the snapshot edge differs from `expected_edge`
/// (mixed-edge snapshots are rejected; the stored context is kept).
#[must_use]
pub fn panel_context_for_snapshot(
    snapshot: &ShellSnapshot,
    expected_edge: flamewm_api::PanelEdge,
) -> Option<PanelPlacementContext> {
    let panel = snapshot.panels.panels.first().cloned()?;
    if panel.edge != expected_edge {
        return None;
    }
    let output = snapshot
        .displays
        .outputs
        .iter()
        .find(|output| output.id == panel.output && output.connected)
        .cloned()?;
    Some(PanelPlacementContext::new(
        output,
        panel,
        snapshot.displays.generation,
        snapshot.panels.revision,
    ))
}

/// F10: pure edge/geometry consistency predicate shared by
/// `check_edge_consistent` and unit tests.
#[must_use]
pub fn edge_geometry_consistent(
    edge: flamewm_api::PanelEdge,
    panel: flamewm_api::Rect,
    output: flamewm_api::Rect,
) -> bool {
    match edge {
        flamewm_api::PanelEdge::Bottom => panel.bottom() == output.bottom(),
        flamewm_api::PanelEdge::Top => panel.y == output.y,
        flamewm_api::PanelEdge::Left => panel.x == output.x,
        flamewm_api::PanelEdge::Right => panel.right() == output.right(),
    }
}

/// F10: pure resync resolver. Prefer the stored edge (rejects mixed-edge
/// snapshots); on an edge switch fall back to the snapshot's own edge so
/// a genuine Top -> Bottom move still resyncs (geometry-driven).
#[must_use]
pub fn synced_placement_for_snapshot(
    snapshot: &ShellSnapshot,
    stored_edge: flamewm_api::PanelEdge,
) -> Option<PanelPlacementContext> {
    if let Some(placement) = panel_context_for_snapshot(snapshot, stored_edge) {
        return Some(placement);
    }
    let first = snapshot.panels.panels.first()?;
    if first.edge == stored_edge {
        return None;
    }
    panel_context_for_snapshot(snapshot, first.edge)
}

/// J07 parent role: panel + Start + task menu + media only. The parent no
/// longer creates or owns audio/network/calendar native surfaces; the
/// quick-control helper owns those surfaces and the parent reaches them
/// through [`crate::quick_controls::QuickControlSupervisor`].
pub struct ShellSurfaces {
    pub panel: SurfaceHandle,
    pub start: SurfaceHandle,
    pub task_menu: SurfaceHandle,
    pub media: SurfaceHandle,
    output: flamewm_api::display::OutputSnapshot,
    panel_snapshot: flamewm_api::panels::PanelSnapshot,
    start_layout: flamewm_shell_core::StartSurfaceLayout,
    placement: PanelPlacementContext,
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
        let task_menu = create_surface(
            runtime,
            TASK_MENU_ARTIFACT,
            popup_config(
                "FlameWM task menu",
                SurfaceRole::DropdownMenu,
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
        let placement = PanelPlacementContext::new(
            output.clone(),
            panel_snapshot.clone(),
            snapshot.displays.generation,
            snapshot.panels.revision,
        );
        let start_layout = placement.start_layout.clone();
        Ok(Self {
            panel,
            start,
            task_menu,
            media,
            output,
            panel_snapshot,
            start_layout,
            placement,
            transient: None,
        })
    }

    /// Proven close branch: single-turn close per surface (ungrab-once
    /// when grabbed + unmap + Expose-present, same turn, no sleeps, no
    /// compositor), then clear the open marker. Never touches panel or
    /// mapped popup geometry.
    pub fn close_transients(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        for surface in [self.start, self.task_menu, self.media] {
            runtime.close_surface(surface).map_err(ui_error)?;
        }
        self.transient = None;
        Ok(())
    }

    pub fn toggle_start(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        if self.transient == Some(Transient::StartGroup) {
            return self.close_transients(runtime);
        }
        self.close_transients(runtime)?;
        self.open_start_group_measured(runtime)
    }

    /// Single measured unified Start show: the already-projected Start
    /// document is measured once (intrinsic union), anchored to the
    /// retained panel device rect, then prepare (move_resize) ->
    /// present (show) -> grab. Never mapped at 0,0: missing
    /// layout/anchor refuses via `PopupRefusal`.
    fn open_start_group_measured(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        // C04 correlated trace: one txn threads anchor -> intrinsic ->
        // fitted -> ui-request -> native-request -> retained. Debug-only
        // observed stage is one probe after present (0 extra X sync in
        // normal mode).
        let trace = GeometryTrace::begin("start-menu");
        let anchor = start_device_anchor(runtime, self.panel, &self.start_layout);
        trace.anchor((anchor.x, anchor.y, anchor.width, anchor.height));
        let root_size = measured_start_root_size_traced(runtime, self.start, &self.output, &trace)?;
        let placement = fitted_start_placement(
            anchor,
            root_size,
            self.panel_snapshot.edge,
            self.output.geometry,
            8,
        )
        .map_err(|refusal| refusal.to_string())?;
        trace.fitted((
            placement.rect.x,
            placement.rect.y,
            placement.rect.width,
            placement.rect.height,
        ));
        // Canonical present: `show` owns the single ordered present cycle;
        // no separate raise/redraw chains. The unified surface takes the
        // single group grab.
        let panel_revision = self.placement.panels_revision;
        let edge = self.panel_snapshot.edge;
        let panel_rect = self.panel_snapshot.geometry;
        let output_rect = self.output.geometry;
        let work_area = self.placement.work_area;
        let anchor_t = (anchor.x, anchor.y, anchor.width, anchor.height);
        let intrinsic_t = (root_size.width, root_size.height);
        let fitted_t = (
            placement.rect.x,
            placement.rect.y,
            placement.rect.width,
            placement.rect.height,
        );
        let native_request_t = fitted_t;
        self.map_raise_grab_traced(runtime, self.start, placement.rect, &trace)?;
        trace_observed_probe(runtime, self.start, &trace);
        let native_observed = runtime
            .surface_device_rect(self.start)
            .ok()
            .map(|r| (r.x as i32, r.y as i32, r.width as i32, r.height as i32));
        debug_popup_record(
            "start",
            panel_revision,
            edge,
            panel_rect,
            output_rect,
            work_area,
            anchor_t,
            intrinsic_t,
            fitted_t,
            native_request_t,
            native_observed,
            trace.txn,
        );
        debug_stack_canary_check(runtime, self.start, "start");
        let _ = placement.content_was_capped;
        self.transient = Some(Transient::StartGroup);
        Ok(())
    }

    /// outside release, Escape, or launch closes the unified surface, and
    /// the grab is released exactly once through `close_transients`.
    /// Already-open category switches resize transactionally: the surface
    /// is re-measured and re-prepared before re-map, so no 0,0 or
    /// half-resized frame is ever visible.
    pub fn open_start_group(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        if self.transient == Some(Transient::StartGroup) {
            return self.resize_start_group_measured(runtime);
        }
        self.close_transients(runtime)?;
        self.open_start_group_measured(runtime)
    }

    /// Transactional resize of the already-open unified Start surface:
    /// re-measure, prepare geometry, then map/present without dropping
    /// the single group grab.
    fn resize_start_group_measured(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        let trace = GeometryTrace::begin("start-menu");
        let anchor = start_device_anchor(runtime, self.panel, &self.start_layout);
        trace.anchor((anchor.x, anchor.y, anchor.width, anchor.height));
        let root_size = measured_start_root_size_traced(runtime, self.start, &self.output, &trace)?;
        let placement = fitted_start_placement(
            anchor,
            root_size,
            self.panel_snapshot.edge,
            self.output.geometry,
            8,
        )
        .map_err(|refusal| refusal.to_string())?;
        trace.fitted((
            placement.rect.x,
            placement.rect.y,
            placement.rect.width,
            placement.rect.height,
        ));
        let panel_revision = self.placement.panels_revision;
        let edge = self.panel_snapshot.edge;
        let panel_rect = self.panel_snapshot.geometry;
        let output_rect = self.output.geometry;
        let work_area = self.placement.work_area;
        let anchor_t = (anchor.x, anchor.y, anchor.width, anchor.height);
        let intrinsic_t = (root_size.width, root_size.height);
        let fitted_t = (
            placement.rect.x,
            placement.rect.y,
            placement.rect.width,
            placement.rect.height,
        );
        self.map_raise_traced(runtime, self.start, placement.rect, &trace)?;
        trace_observed_probe(runtime, self.start, &trace);
        let native_observed = runtime
            .surface_device_rect(self.start)
            .ok()
            .map(|r| (r.x as i32, r.y as i32, r.width as i32, r.height as i32));
        debug_popup_record(
            "start",
            panel_revision,
            edge,
            panel_rect,
            output_rect,
            work_area,
            anchor_t,
            intrinsic_t,
            fitted_t,
            fitted_t,
            native_observed,
            trace.txn,
        );
        debug_stack_canary_check(runtime, self.start, "start");
        Ok(())
    }

    #[must_use]
    pub fn is_start_group_open(&self) -> bool {
        self.transient == Some(Transient::StartGroup)
    }

    #[must_use]
    pub fn is_media_open(&self) -> bool {
        self.transient == Some(Transient::Media)
    }

    #[must_use]
    pub fn is_audio_open(&self) -> bool {
        false
    }

    #[must_use]
    pub fn is_network_open(&self) -> bool {
        false
    }

    /// Closes the unified Start surface with a single ungrab,
    /// tolerating any legacy stray state through `close_transients`.
    pub fn close_start_group(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        if self.transient == Some(Transient::StartGroup) {
            return self.close_transients(runtime);
        }
        Ok(())
    }

    pub fn open_status(&mut self, runtime: &mut SurfaceRuntime, name: &str) -> Result<(), String> {
        // J07: media stays in-process; audio/network/clock delegate to the
        // helper via the supervisor (caller path in main.rs). Direct
        // in-process opens of helper-owned popups are refused here so the
        // parent never regrows native ownership.
        if matches!(name, "volume" | "audio" | "network" | "clock") {
            return Err(format!("quick-control {name} owned by helper process"));
        }
        // Live path: stable panel node ids, edge from the panel edge, Align
        // End; no fixed popup_rect. Single measured show: intrinsic size is
        // resolved after content projection, then geometry -> prepare ->
        // map/raise/grab -> present in one pass (see open_status_anchored_id).
        self.open_status_anchored_id(runtime, name, status_source_id(name))
    }

    /// Anchored open from a stable panel node id (`tray-media`,
    /// `tray-volume`, `tray-network`, `clock-button`).
    /// Single measured show: measure (intrinsic) -> anchor (node rect) ->
    /// prepare geometry/backbuffer/shape -> map/raise/grab -> present.
    /// Refuses `PendingLayout`/`PendingAnchor`; never maps at 0,0.
    pub fn open_status_anchored_id(
        &mut self,
        runtime: &mut SurfaceRuntime,
        name: &str,
        source_id: &str,
    ) -> Result<(), String> {
        let (surface, transient) = match name {
            "media" => (self.media, Transient::Media),
            _ => return Err(format!("unsupported shell popup {name}")),
        };
        self.close_transients(runtime)?;
        let rect = measured_status_rect_by_id(
            runtime,
            self.output.geometry,
            self.panel,
            source_id,
            surface,
            panel_edge_to_popover(self.panel_snapshot.edge),
        )?;
        // Geometry -> prepare -> map/raise/grab -> present in one pass.
        self.map_raise_grab(runtime, surface, rect)?;
        self.transient = Some(transient);
        Ok(())
    }

    /// Re-measure the open popup after content changed: intrinsic size +
    /// clamp + Align End placement. No-op when nothing is open.
    /// Transactional: the new rect is prepared before re-mapping, and a
    /// refused measurement leaves the currently mapped rect untouched.
    pub fn remeasure_open_popup(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), String> {
        let (surface, name) = match self.transient {
            Some(Transient::Media) => (self.media, "media"),
            _ => return Ok(()),
        };
        let rect = measured_status_rect_by_id(
            runtime,
            self.output.geometry,
            self.panel,
            status_source_id(name),
            surface,
            panel_edge_to_popover(self.panel_snapshot.edge),
        )?;
        self.map_raise(runtime, surface, rect)
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
            _ => return Err(format!("unsupported shell popup {name}")),
        };
        self.close_transients(runtime)?;
        let rect = measured_status_rect_by_node(
            runtime,
            self.output.geometry,
            source_surface,
            source_node,
            surface,
            edge,
        )?;
        self.map_raise_grab(runtime, surface, rect)?;
        self.transient = Some(transient);
        Ok(())
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

    /// Retained-device anchor for a quick-control source id, with the
    /// snapshot-geometry fallback used only pre-map. Mirrors the
    /// in-process contract without regrowing native ownership.
    pub fn quick_anchor(
        &self,
        runtime: &SurfaceRuntime,
        fallback: &flamewm_api::Rect,
        source_id: &str,
    ) -> flamewm_api::Rect {
        crate::runtime::retained_source_rect(runtime, self.panel, source_id).unwrap_or(*fallback)
    }

    /// Recompute the stored placement context from a fresh snapshot.
    /// Pure/state only: resolve the panel's output, validate that the
    /// snapshot edge matches the stored panel edge (reject mixed-edge
    /// snapshots), recompute work area + Start layout, then update all
    /// placement fields atomically. Returns `false` when the output or
    /// panel is missing (state untouched). Wired to main.rs (F10):
    /// `resync_panel_geometry` calls this BEFORE next open/projection.
    /// F10: on an edge switch (stored Top, snapshot Bottom) the gated
    /// resolve misses; fall back to the snapshot's own edge so a
    /// genuine Top -> Bottom move still resyncs (geometry-driven).
    #[must_use]
    pub fn sync_panel_context(&mut self, snapshot: &ShellSnapshot) -> bool {
        if let Some(placement) = panel_context_for_snapshot(snapshot, self.panel_snapshot.edge) {
            self.output = placement.output.clone();
            self.panel_snapshot = placement.panel.clone();
            self.start_layout = placement.start_layout.clone();
            self.placement = placement;
            return true;
        }
        let Some(first) = snapshot.panels.panels.first() else {
            return false;
        };
        let edge = first.edge;
        if edge == self.panel_snapshot.edge {
            return false;
        }
        let Some(placement) = panel_context_for_snapshot(snapshot, edge) else {
            return false;
        };
        self.output = placement.output.clone();
        self.panel_snapshot = placement.panel.clone();
        self.start_layout = placement.start_layout.clone();
        self.placement = placement;
        true
    }

    /// Parent-side anchor inputs for a helper open: retained anchor +
    /// work area + panel edge. The parent computes, the helper places.
    #[must_use]
    pub fn work_area(&self) -> flamewm_api::Rect {
        self.placement.work_area
    }

    /// F10: current context generation read by every Start/Quick open
    /// path in main.rs before projection. Proves the open uses the
    /// post-sync context, never a stale cached rect.
    #[must_use]
    pub fn placement_generation(&self) -> (u64, u64) {
        (
            self.placement.display_generation,
            self.placement.panels_revision,
        )
    }

    /// F10: native panel surface handle for geometry updates.
    #[allow(dead_code)]
    #[must_use]
    pub fn panel_handle(&self) -> SurfaceHandle {
        self.panel
    }

    /// F10: last-synced panel snapshot geometry.
    #[must_use]
    pub fn panel_geometry(&self) -> flamewm_api::Rect {
        self.panel_snapshot.geometry
    }

    /// F10: edge/geometry consistency gate. `Bottom` requires
    /// `panel.bottom == output.bottom`, `Top` requires `panel.y ==
    /// output.y`, `Left` requires `panel.x == output.x`, `Right`
    /// requires `panel.right == output.right`. Returns a diagnostic
    /// refusal string; callers refuse popup open on `Err`.
    pub fn check_edge_consistent(&self) -> Result<(), String> {
        let panel = self.panel_snapshot.geometry;
        let output = self.output.geometry;
        let edge = self.panel_snapshot.edge;
        let consistent = match edge {
            flamewm_api::PanelEdge::Bottom => panel.bottom() == output.bottom(),
            flamewm_api::PanelEdge::Top => panel.y == output.y,
            flamewm_api::PanelEdge::Left => panel.x == output.x,
            flamewm_api::PanelEdge::Right => panel.right() == output.right(),
        };
        debug_assert!(
            consistent,
            "panel edge {edge:?} inconsistent: panel={panel:?} output={output:?}"
        );
        if consistent {
            Ok(())
        } else {
            Err(format!(
                "panel edge {edge:?} inconsistent: panel={panel:?} output={output:?}"
            ))
        }
    }

    /// F10: move_resize the panel native surface to the synced snapshot
    /// geometry. Returns `true` when a move_resize was issued.
    pub fn move_panel_to_synced(&self, runtime: &mut SurfaceRuntime) -> Result<bool, String> {
        let target = self.panel_snapshot.geometry;
        let current = runtime
            .surface_device_rect(self.panel)
            .map(|rect| {
                flamewm_api::Rect::new(
                    rect.x as i32,
                    rect.y as i32,
                    rect.width as i32,
                    rect.height as i32,
                )
            })
            .unwrap_or(self.panel_snapshot.geometry);
        if current == target {
            return Ok(false);
        }
        let width = checked_size(target.width)?;
        let height = checked_size(target.height)?;
        runtime
            .move_resize(self.panel, target.x, target.y, width, height)
            .map_err(ui_error)?;
        Ok(true)
    }

    #[must_use]
    pub fn placement(&self) -> &PanelPlacementContext {
        &self.placement
    }

    #[must_use]
    pub fn panel_edge(&self) -> flamewm_api::PanelEdge {
        self.panel_snapshot.edge
    }

    /// Resolve a surface-local pointer position (ActionEvent x/y from the
    /// panel surface) to root coordinates via the retained panel device
    /// rect. Falls back to the snapshot geometry only when no retained
    /// layout exists (pre-map).
    #[must_use]
    pub fn root_pointer(&self, runtime: &SurfaceRuntime, x: f32, y: f32) -> flamewm_api::Point {
        let origin = runtime
            .surface_device_rect(self.panel)
            .map(|rect| (rect.x as i32, rect.y as i32))
            .unwrap_or((
                self.panel_snapshot.geometry.x,
                self.panel_snapshot.geometry.y,
            ));
        flamewm_api::Point::new(origin.0 + x as i32, origin.1 + y as i32)
    }

    fn show_grabbed(
        &mut self,
        runtime: &mut SurfaceRuntime,
        surface: SurfaceHandle,
        rect: flamewm_api::Rect,
        transient: Transient,
    ) -> Result<(), String> {
        self.map_raise_grab(runtime, surface, rect)?;
        self.transient = Some(transient);
        Ok(())
    }

    fn prepare_traced(
        &self,
        runtime: &mut SurfaceRuntime,
        surface: SurfaceHandle,
        rect: flamewm_api::Rect,
        trace: &GeometryTrace,
    ) -> Result<(), String> {
        if rect.width <= 0 || rect.height <= 0 {
            return Err(PopupRefusal::PendingLayout.to_string());
        }
        runtime
            .move_resize_traced(
                surface,
                rect.x,
                rect.y,
                checked_size(rect.width)?,
                checked_size(rect.height)?,
                Some(*trace),
            )
            .map_err(ui_error)
    }

    /// Present step for the grabbed tail: canonical present_mapped path
    /// (show = geometry/move_resize origin-tracked + map/raise/present).
    /// No separate raise or redraw chains: `show` owns the single ordered
    /// present cycle, then the single group grab.
    fn map_raise_grab(
        &mut self,
        runtime: &mut SurfaceRuntime,
        surface: SurfaceHandle,
        rect: flamewm_api::Rect,
    ) -> Result<(), String> {
        self.map_raise_grab_traced(runtime, surface, rect, &GeometryTrace::begin("shell-popup"))
    }

    fn map_raise_grab_traced(
        &mut self,
        runtime: &mut SurfaceRuntime,
        surface: SurfaceHandle,
        rect: flamewm_api::Rect,
        trace: &GeometryTrace,
    ) -> Result<(), String> {
        self.prepare_traced(runtime, surface, rect, trace)?;
        runtime.show(surface).map_err(ui_error)?;
        runtime
            .grab_pointer(surface)
            .map_err(|_| PopupRefusal::PointerGrabRefused.to_string())
    }

    /// Present step without grab (Start root / remeasure tail): canonical
    /// present via `show` only.
    fn map_raise(
        &self,
        runtime: &mut SurfaceRuntime,
        surface: SurfaceHandle,
        rect: flamewm_api::Rect,
    ) -> Result<(), String> {
        self.map_raise_traced(runtime, surface, rect, &GeometryTrace::begin("shell-popup"))
    }

    fn map_raise_traced(
        &self,
        runtime: &mut SurfaceRuntime,
        surface: SurfaceHandle,
        rect: flamewm_api::Rect,
        trace: &GeometryTrace,
    ) -> Result<(), String> {
        self.prepare_traced(runtime, surface, rect, trace)?;
        runtime.show(surface).map_err(ui_error)
    }

    /// Retained-device anchor for a status source id. Reads the live panel
    /// node rect from retained layout (`node_device_rect_by_id`, root-space
    /// device rect, no recompute). `None` when no retained layout exists.
    /// Legacy helper retained until the helper role fully owns status
    /// popups; J07 parent delegates quick-control opens to the supervisor.
    #[allow(dead_code)]
    fn retained_source_by_id(
        runtime: &SurfaceRuntime,
        panel: SurfaceHandle,
        source_id: &str,
    ) -> Option<flamewm_api::Rect> {
        runtime
            .node_device_rect_by_id(panel, source_id)
            .map(|rect| to_api_rect(rect.x, rect.y, rect.width, rect.height))
            .filter(|rect| rect.width > 0 && rect.height > 0)
    }
}

/// Retained-device node rect by id (root-space device rect, no
/// recompute). Shared by the in-process media path and the parent-side
/// quick-control anchor computation.
#[must_use]
pub fn retained_source_rect(
    runtime: &SurfaceRuntime,
    panel: SurfaceHandle,
    source_id: &str,
) -> Option<flamewm_api::Rect> {
    retained_source_by_id(runtime, panel, source_id)
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

/// Retained-device anchor: live panel node rect (`node_device_rect_by_id`,
/// root-space device rect, no recompute) when retained layout exists.
fn retained_source_by_id(
    runtime: &SurfaceRuntime,
    panel: SurfaceHandle,
    source_id: &str,
) -> Option<flamewm_api::Rect> {
    runtime
        .node_device_rect_by_id(panel, source_id)
        .map(|rect| to_api_rect(rect.x, rect.y, rect.width, rect.height))
        .filter(|rect| rect.width > 0 && rect.height > 0)
}

/// Start root anchor from retained panel `task-start` node rect, else the
/// static layout anchor. Never 0,0 unless the layout resolves there.
fn start_device_anchor(
    runtime: &SurfaceRuntime,
    panel: SurfaceHandle,
    layout: &flamewm_shell_core::StartSurfaceLayout,
) -> flamewm_api::Rect {
    retained_source_by_id(runtime, panel, "task-start").unwrap_or_else(|| start_anchor_rect(layout))
}

/// Render-space device rect (f32 root space) -> API integer rect.
/// Inferred render rect type; this crate names no render-core type.
fn to_api_rect(x: f32, y: f32, width: f32, height: f32) -> flamewm_api::Rect {
    flamewm_api::Rect::new(
        x as i32,
        y as i32,
        width.max(1.0) as i32,
        height.max(1.0) as i32,
    )
}

/// Measured anchoring from a stable panel node id. Uses retained-live node
/// rects (`node_device_rect_by_id`) first, then the legacy layout-recompute
/// path only when no retained layout exists.
fn measured_status_rect_by_id(
    runtime: &mut SurfaceRuntime,
    work_area: flamewm_api::Rect,
    panel: SurfaceHandle,
    source_id: &str,
    popup_surface: SurfaceHandle,
    edge: flamewm_ui_core::popover::PopoverEdge,
) -> Result<flamewm_api::Rect, String> {
    let source = retained_source_by_id(runtime, panel, source_id).or_else(|| {
        runtime
            .node_global_rect_by_id(panel, source_id)
            .map(|rect| {
                flamewm_api::Rect::new(
                    rect.x as i32,
                    rect.y as i32,
                    rect.width.max(1.0) as i32,
                    rect.height.max(1.0) as i32,
                )
            })
    });
    let source = source.ok_or_else(|| PopupRefusal::PendingAnchor.to_string())?;
    let intrinsic = measure_outer_intrinsic(
        runtime,
        popup_surface,
        "media",
        Some(source),
        edge,
        flamewm_ui_core::popover::PopoverAlign::End,
        work_area,
    )?;
    measured_popup_rect(
        Some(source),
        intrinsic,
        edge,
        flamewm_ui_core::popover::PopoverAlign::End,
        work_area,
        8,
    )
    .map_err(|refusal| refusal.to_string())
}

/// Measured anchoring from a node handle. Same refusal contract as
/// `measured_status_rect_by_id`; never falls back to 0,0 or a fixed size.
fn measured_status_rect_by_node(
    runtime: &mut SurfaceRuntime,
    work_area: flamewm_api::Rect,
    source_surface: SurfaceHandle,
    source_node: u32,
    popup_surface: SurfaceHandle,
    edge: flamewm_ui_core::popover::PopoverEdge,
) -> Result<flamewm_api::Rect, String> {
    let source = runtime
        .node_device_rect(source_surface, source_node)
        .ok()
        .map(|rect| to_api_rect(rect.x, rect.y, rect.width, rect.height))
        .filter(|rect| rect.width > 0 && rect.height > 0)
        .or_else(|| {
            runtime
                .node_global_rect(source_surface, source_node)
                .map(|rect| {
                    flamewm_api::Rect::new(
                        rect.x as i32,
                        rect.y as i32,
                        rect.width.max(1.0) as i32,
                        rect.height.max(1.0) as i32,
                    )
                })
                .ok()
        });
    let intrinsic = measure_outer_intrinsic(
        runtime,
        popup_surface,
        "media",
        source,
        edge,
        flamewm_ui_core::popover::PopoverAlign::End,
        work_area,
    )?;
    measured_popup_rect(
        source,
        intrinsic,
        edge,
        flamewm_ui_core::popover::PopoverAlign::End,
        work_area,
        8,
    )
    .map_err(|refusal| refusal.to_string())
}

fn ui_error(error: UiBackendError) -> String {
    format!("{error:?}")
}

/// Start root measure: intrinsic size of the already-projected Start
/// document. Refuses `PendingLayout` instead of using a fixed size.
/// The document root box is `#start-menu` itself (unified root width from
/// `ShellMetrics::start_menu_width`), so the intrinsic width is used
/// directly; only the output/work area clamps the size. Height beyond the
/// space above the panel is capped by `fitted_start_placement` (bottom
/// adjacency kept, content scrolls internally).
#[allow(dead_code)]
fn measured_start_root_size(
    runtime: &mut SurfaceRuntime,
    surface: SurfaceHandle,
    output: &flamewm_api::display::OutputSnapshot,
) -> Result<flamewm_api::Size, String> {
    let max = outer_measure_constraint(output.geometry);
    let logical = logical_constraint(max)?;
    let measured = runtime
        .measure_outer_intrinsic_device_size(surface, max.0, max.1)
        .map_err(|_| PopupRefusal::PendingLayout.to_string())?;
    emit_popup_measure(
        "start",
        "",
        None,
        output.geometry,
        Some(max),
        Some(logical),
        Some(measured),
        None,
        None,
    );
    let size = device_to_popup_size(measured)
        .filter(|size| size.width > 0 && size.height > 0)
        .ok_or_else(|| PopupRefusal::PendingLayout.to_string())?;
    let size = flamewm_api::Size::new(
        size.width.min(output.geometry.width.max(1)),
        size.height.min(output.geometry.height.max(1)),
    );
    Ok(size)
}

/// Traced Start root measure: same contract, plus the intrinsic stage on
/// the caller's txn. Does not move START Y.
#[allow(dead_code)]
fn measured_start_root_size_traced(
    runtime: &mut SurfaceRuntime,
    surface: SurfaceHandle,
    output: &flamewm_api::display::OutputSnapshot,
    trace: &GeometryTrace,
) -> Result<flamewm_api::Size, String> {
    let size = measured_start_root_size(runtime, surface, output)?;
    trace.intrinsic((size.width, size.height));
    Ok(size)
}

/// Debug-only observed stage from retained state (no X sync roundtrip).
/// Fills the observed leg of the correlated trace off the hot path.
#[allow(dead_code)]
fn trace_observed_retained(
    runtime: &SurfaceRuntime,
    surface: SurfaceHandle,
    trace: &GeometryTrace,
) {
    if let Ok(rect) = runtime.surface_device_rect(surface) {
        trace.observed((
            rect.x as i32,
            rect.y as i32,
            rect.width as i32,
            rect.height as i32,
        ));
    }
}

/// DEBUG-ONLY gate: true only when `FLAMEWM_DEBUG=1`. Normal mode must
/// issue zero extra X syncs; callers check this before any probe.
/// Local copy (no new cross-crate dep); mirrors the render owner.
#[allow(dead_code)]
fn shell_debug_enabled() -> bool {
    std::env::var_os("FLAMEWM_DEBUG").is_some_and(|v| v == "1")
}

/// DEBUG-ONLY observed root-rect probe after present: normal mode is zero
/// extra X sync; debug mode (`FLAMEWM_DEBUG=1`) issues one observed root
/// rect probe (retained read via the X native owner, no layout recompute).
#[allow(dead_code)]
fn trace_observed_probe(runtime: &SurfaceRuntime, surface: SurfaceHandle, trace: &GeometryTrace) {
    if shell_debug_enabled() {
        let _ = runtime.debug_probe_root_rect(surface, *trace);
    } else {
        trace_observed_retained(runtime, surface, trace);
    }
}

/// DEBUG-ONLY correlated popup record: panel_revision, edge, panel_rect,
/// output_rect, work_area, anchor, intrinsic, fitted, native_request,
/// native_observed. Emitted only under `FLAMEWM_DEBUG=1`; one record per
/// popup show (Start in-process, Audio/Network/Calendar helper).
#[allow(dead_code)]
fn debug_popup_record(
    kind: &str,
    panel_revision: u64,
    edge: flamewm_api::PanelEdge,
    panel_rect: flamewm_api::Rect,
    output_rect: flamewm_api::Rect,
    work_area: flamewm_api::Rect,
    anchor: (i32, i32, i32, i32),
    intrinsic: (i32, i32),
    fitted: (i32, i32, i32, i32),
    native_request: (i32, i32, i32, i32),
    native_observed: Option<(i32, i32, i32, i32)>,
    txn: u64,
) {
    if !shell_debug_enabled() {
        return;
    }
    eprintln!(
        "popup-record txn={txn} kind={kind} panel_revision={panel_revision} edge={edge:?} panel_rect={panel_rect:?} output_rect={output_rect:?} work_area={work_area:?} anchor={anchor:?} intrinsic={intrinsic:?} fitted={fitted:?} native_request={native_request:?} native_observed={native_observed:?}",
    );
}

/// DEBUG-ONLY stack canary check: desktop < normal < dock < popup. Runs
/// only under `FLAMEWM_DEBUG=1` (or test). Pass = leave stacking alone.
/// Fail = repair through the existing map/raise owner and report; no
/// redesign of stacking order.
#[allow(dead_code)]
fn debug_stack_canary_check(runtime: &mut SurfaceRuntime, surface: SurfaceHandle, kind: &str) {
    if !shell_debug_enabled() {
        return;
    }
    match runtime.debug_stack_canary() {
        Ok(true) => {}
        Ok(false) => {
            // Repair path: re-use the existing map/raise owner only.
            let _ = runtime.raise(surface);
            eprintln!("flamewm-shell: debug stack canary FAILED kind={kind}; re-raised via existing owner");
        }
        Err(error) => {
            eprintln!("flamewm-shell: debug stack canary error kind={kind} error={error:?}");
        }
    }
}

/// Start root anchor: the resolved popover anchor from surface layout.
/// Never 0,0 unless the layout itself resolves there.
fn start_anchor_rect(layout: &flamewm_shell_core::StartSurfaceLayout) -> flamewm_api::Rect {
    layout.anchor
}

/// Panel-edge -> Start root placement edge. Legacy helper for the generic
/// measured path; fitted Start placement (J07) uses the panel edge directly.
#[allow(dead_code)]
fn start_root_edge(edge: flamewm_api::PanelEdge) -> flamewm_ui_core::popover::PopoverEdge {
    match edge {
        flamewm_api::PanelEdge::Top => flamewm_ui_core::popover::PopoverEdge::Below,
        flamewm_api::PanelEdge::Left => flamewm_ui_core::popover::PopoverEdge::Right,
        flamewm_api::PanelEdge::Right => flamewm_ui_core::popover::PopoverEdge::Left,
        _ => flamewm_ui_core::popover::PopoverEdge::Above,
    }
}

/// C03 outer-intrinsic popup measure helpers. Every popup consumer
/// resolves through `measure_outer_intrinsic` (work-area device
/// constraint -> `SurfaceRuntime::measure_outer_intrinsic_device_size`).
/// `IntrinsicMeasureError`/backend failure refuses via `PendingLayout`;
/// the caller keeps the alive surface untouched (no map at 0,0).
fn outer_measure_constraint(work_area: flamewm_api::Rect) -> (f32, f32) {
    let width = work_area.width.max(1).min(i32::MAX);
    let height = work_area.height.max(1).min(i32::MAX);
    (width as f32, height as f32)
}

fn logical_constraint(max_device: (f32, f32)) -> Result<(f32, f32), String> {
    // Shared-surface ui_scale reads 1.0 for the headless/pure-math probe;
    // live surfaces read their own scale through the runtime measure path.
    let scale = 1.0f32;
    if max_device.0 <= 0.0 || max_device.1 <= 0.0 {
        return Err(PopupRefusal::PendingLayout.to_string());
    }
    Ok((max_device.0 / scale, max_device.1 / scale))
}

fn device_to_popup_size(measured: (f32, f32)) -> Option<flamewm_api::Size> {
    if !measured.0.is_finite() || !measured.1.is_finite() {
        return None;
    }
    let width = measured.0.max(1.0) as i32;
    let height = measured.1.max(1.0) as i32;
    if !width.is_positive() || !height.is_positive() {
        return None;
    }
    Some(flamewm_api::Size::new(width, height))
}

fn emit_popup_measure(
    role: &str,
    kind: &str,
    anchor: Option<flamewm_api::Rect>,
    work_area: flamewm_api::Rect,
    max_device: Option<(f32, f32)>,
    logical: Option<(f32, f32)>,
    measured: Option<(f32, f32)>,
    fitted: Option<flamewm_api::Rect>,
    refusal: Option<&str>,
) {
    flamewm_debug::emit(
        flamewm_debug::SHELL_POPUP_MEASURE,
        flamewm_debug::SHELL_POPUP_MEASURE_COOLDOWN,
        || {
            format!(
                "role={role} kind={kind} anchor={anchor:?} work_area={work_area:?} max_device={max_device:?} logical={logical:?} measured={measured:?} fitted={fitted:?} refusal={refusal:?}"
            )
        },
    );
}

fn measure_outer_intrinsic(
    runtime: &mut SurfaceRuntime,
    surface: SurfaceHandle,
    role: &'static str,
    anchor: Option<flamewm_api::Rect>,
    edge: flamewm_ui_core::popover::PopoverEdge,
    align: flamewm_ui_core::popover::PopoverAlign,
    work_area: flamewm_api::Rect,
) -> Result<Option<flamewm_api::Size>, String> {
    let max = outer_measure_constraint(work_area);
    let logical = logical_constraint(max)?;
    let measured = runtime
        .measure_outer_intrinsic_device_size(surface, max.0, max.1)
        .map_err(|_| PopupRefusal::PendingLayout.to_string())?;
    let kind = format!("{edge:?}/{align:?}");
    emit_popup_measure(
        role,
        &kind,
        anchor,
        work_area,
        Some(max),
        Some(logical),
        Some(measured),
        None,
        None,
    );
    Ok(device_to_popup_size(measured))
}

/// Shell spans (static only): shell.windows.reconcile, shell.workspaces.reconcile,
/// shell.tasks.project, shell.pager.project, shell.popup.request,
/// shell.quick_control.open_request, shell.quick_control.restart,
/// shell.signal_to_visual.windows, shell.signal_to_visual.workspaces.
#[allow(dead_code)]
pub fn shell_span(label: &'static str) -> flamewm_profiler::ProfilePoint {
    debug_assert!(
        matches!(
            label,
            "shell.windows.reconcile"
                | "shell.workspaces.reconcile"
                | "shell.tasks.project"
                | "shell.pager.project"
                | "shell.popup.request"
                | "shell.quick_control.open_request"
                | "shell.quick_control.restart"
                | "shell.quick.snapshot"
                | "shell.quick.project"
                | "shell.quick.measure"
                | "shell.quick.place"
                | "shell.quick.prepare"
                | "shell.quick.present"
                | "shell.quick.grab"
                | "shell.quick.close"
                | "shell.quick.open.total"
                | "shell.signal_to_visual.windows"
                | "shell.signal_to_visual.workspaces"
        ),
        "shell span label must be a static J07 span, got {label}"
    );
    flamewm_profiler::ProfilePoint::new(label)
}

/// J07 reconcile counters, one per domain. `OnceLock` statics keep labels
/// stable across turns; the profiler registry dedups by label.
/// J07 reconcile domain tag for per-domain counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainKind {
    Windows,
    Workspaces,
    Panels,
    System,
    Applications,
}

pub fn domain_reconcile_counter(kind: DomainKind) -> &'static flamewm_profiler::CounterPoint {
    use std::sync::OnceLock;
    match kind {
        DomainKind::Windows => {
            static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
            C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.domain.windows.reconcile"))
        }
        DomainKind::Workspaces => {
            static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
            C.get_or_init(|| {
                flamewm_profiler::CounterPoint::new("shell.domain.workspaces.reconcile")
            })
        }
        DomainKind::Panels => {
            static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
            C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.domain.panels.reconcile"))
        }
        DomainKind::System => {
            static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
            C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.domain.system.reconcile"))
        }
        DomainKind::Applications => {
            static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
            C.get_or_init(|| {
                flamewm_profiler::CounterPoint::new("shell.domain.applications.reconcile")
            })
        }
    }
}

/// J07 projection scope tag: partial panel vs start vs popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionKind {
    Partial,
    Start,
    Popup,
}

#[allow(dead_code)]
pub fn projection_counter(kind: ProjectionKind) -> &'static flamewm_profiler::CounterPoint {
    use std::sync::OnceLock;
    match kind {
        ProjectionKind::Partial => {
            static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
            C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.projection.panel.partial"))
        }
        ProjectionKind::Start => {
            static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
            C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.projection.panel.start"))
        }
        ProjectionKind::Popup => {
            static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
            C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.projection.panel.popup"))
        }
    }
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

    fn test_window(reference: flamewm_api::WindowRef) -> flamewm_api::window::WindowSnapshot {
        flamewm_api::window::WindowSnapshot {
            reference,
            title: "term".to_owned(),
            app_id: flamewm_api::DesktopAppId::new("term"),
            outer_geometry: flamewm_api::Rect::new(0, 0, 100, 100),
            restore_geometry: flamewm_api::Rect::new(0, 0, 100, 100),
            state: flamewm_api::window::WindowState::Normal,
            sticky: false,
            focused: true,
            workspace: flamewm_api::WorkspaceRef::new(0, 1),
            output: test_output_id(),
            state_generation: 1,
        }
    }

    #[test]
    fn window_snapshot_signal_applies_without_fetch() {
        let mut runtime = ShellRuntime::default();
        let windows = vec![test_window(flamewm_api::WindowRef::new(1, 1))];
        runtime.mark_signal(&ControlSignal::WindowsSnapshotChanged {
            revision: 7,
            windows: windows.clone(),
        });
        assert_eq!(runtime.snapshot().windows, windows);
        let changed = runtime.reconcile_applied();
        assert!(changed.windows);
        assert!(!runtime.dirty().any());
    }

    #[test]
    fn legacy_window_revision_signal_sets_no_flag() {
        let mut runtime = ShellRuntime::default();
        runtime.mark_signal(&ControlSignal::WindowsChanged { revision: 9 });
        runtime.mark_signal(&ControlSignal::WorkspacesChanged { revision: 3 });
        runtime.mark_signal(&ControlSignal::PanelsChanged { revision: 4 });
        assert!(!runtime.dirty().any());
    }

    #[test]
    fn workspace_snapshot_signal_updates_model_without_fetch() {
        let mut runtime = ShellRuntime::default();
        let snapshot = flamewm_api::workspace::WorkspaceSnapshot {
            revision: 5,
            count: 2,
            active_index: 1,
            last_index: Some(0),
            names: vec!["one".to_owned(), "two".to_owned()],
        };
        runtime.mark_signal(&ControlSignal::WorkspacesSnapshotChanged {
            snapshot: snapshot.clone(),
        });
        assert_eq!(runtime.snapshot().workspaces, Some(snapshot));
        let changed = runtime.reconcile_applied();
        assert!(changed.workspaces);
        assert!(!runtime.dirty().any());
    }

    #[test]
    fn panel_snapshot_signal_updates_task_model_without_fetch() {
        let mut runtime = ShellRuntime::default();
        let window = flamewm_api::WindowRef::new(2, 1);
        let snapshot = flamewm_api::panels::PanelsSnapshot {
            revision: 6,
            panels: Vec::new(),
            tasks: vec![flamewm_api::panels::TaskEntry {
                id: flamewm_api::TaskEntryId::window(window),
                app_id: flamewm_api::DesktopAppId::new("term"),
                kind: flamewm_api::panels::TaskEntryKind::Window { window },
                order_index: 0,
            }],
            pinned_apps: Vec::new(),
        };
        runtime.mark_signal(&ControlSignal::PanelsSnapshotChanged {
            snapshot: snapshot.clone(),
        });
        assert_eq!(runtime.snapshot().panels, snapshot);
        let changed = runtime.reconcile_applied();
        assert!(changed.panels);
        assert!(!runtime.dirty().any());
    }

    #[test]
    fn identical_snapshot_signal_sets_no_flag() {
        let mut runtime = ShellRuntime::default();
        let windows = runtime.snapshot().windows.clone();
        runtime.mark_signal(&ControlSignal::WindowsSnapshotChanged {
            revision: 1,
            windows,
        });
        assert!(!runtime.dirty().any());
    }

    fn panel_output(id: &str, geometry: flamewm_api::Rect) -> flamewm_api::display::OutputSnapshot {
        flamewm_api::display::OutputSnapshot {
            id: flamewm_api::OutputId::new(id),
            connector: id.to_owned(),
            edid_identity: String::new(),
            connected: true,
            primary: true,
            geometry,
            current_mode: flamewm_api::ModeId(1),
            modes: Vec::new(),
            shell_scale_percent: 100,
        }
    }

    fn panel_entry(
        output: &str,
        edge: flamewm_api::PanelEdge,
        geometry: flamewm_api::Rect,
    ) -> flamewm_api::panels::PanelSnapshot {
        flamewm_api::panels::PanelSnapshot {
            output: flamewm_api::OutputId::new(output),
            edge,
            logical_size: 44,
            visible: true,
            geometry,
        }
    }

    fn placement_snapshot(
        output_geo: flamewm_api::Rect,
        panel_geo: flamewm_api::Rect,
        edge: flamewm_api::PanelEdge,
        generation: u64,
        revision: u64,
    ) -> ShellSnapshot {
        let mut snapshot = ShellSnapshot::default();
        snapshot.displays.generation = generation;
        snapshot.displays.outputs = vec![panel_output("HDMI-1", output_geo)];
        snapshot.panels.revision = revision;
        snapshot.panels.panels = vec![panel_entry("HDMI-1", edge, panel_geo)];
        snapshot
    }

    #[test]
    fn panel_context_top_to_bottom_recompute() {
        use flamewm_api::PanelEdge;
        let top = placement_snapshot(
            flamewm_api::Rect::new(0, 0, 1920, 1080),
            flamewm_api::Rect::new(0, 0, 1920, 44),
            PanelEdge::Top,
            1,
            1,
        );
        let ctx = panel_context_for_snapshot(&top, PanelEdge::Top).expect("top context");
        assert_eq!(ctx.work_area, flamewm_api::Rect::new(0, 44, 1920, 1036));
        assert_eq!((ctx.display_generation, ctx.panels_revision), (1, 1));
        // Mixed edge rejected: Bottom snapshot against Top expectation.
        let bottom = placement_snapshot(
            flamewm_api::Rect::new(0, 0, 1920, 1080),
            flamewm_api::Rect::new(0, 1036, 1920, 44),
            PanelEdge::Bottom,
            2,
            2,
        );
        assert!(panel_context_for_snapshot(&bottom, PanelEdge::Top).is_none());
        let ctx2 = panel_context_for_snapshot(&bottom, PanelEdge::Bottom).expect("bottom context");
        assert_eq!(ctx2.work_area, flamewm_api::Rect::new(0, 0, 1920, 1036));
    }

    #[test]
    fn panel_context_bottom_excludes_panel_and_starts_above() {
        use flamewm_api::PanelEdge;
        let snapshot = placement_snapshot(
            flamewm_api::Rect::new(0, 0, 1920, 1080),
            flamewm_api::Rect::new(0, 1036, 1920, 44),
            PanelEdge::Bottom,
            3,
            5,
        );
        let ctx = panel_context_for_snapshot(&snapshot, PanelEdge::Bottom).expect("context");
        assert_eq!(ctx.work_area.bottom(), 1036);
        assert!(ctx.work_area.height < 1080);
        // Start anchor (static layout) sits at the panel; the layout
        // popover must lie inside the work area, not overlap the panel.
        assert_eq!(ctx.start_layout.anchor.bottom(), 1080);
        assert!(ctx.start_layout.popover.bottom() <= ctx.work_area.bottom());
    }

    #[test]
    fn panel_context_quick_anchor_above_and_negative_origins() {
        use flamewm_api::PanelEdge;
        use flamewm_shell_core::popup::fitted_start_placement;
        let output = flamewm_api::Rect::new(-1920, 0, 1920, 1080);
        let snapshot = placement_snapshot(
            output,
            flamewm_api::Rect::new(-1920, 1036, 1920, 44),
            PanelEdge::Bottom,
            4,
            6,
        );
        let ctx = panel_context_for_snapshot(&snapshot, PanelEdge::Bottom).expect("context");
        assert_eq!(ctx.work_area, flamewm_api::Rect::new(-1920, 0, 1920, 1036));
        // Quick anchor above the panel: end-aligned popover stays inside
        // the work area and above the panel top.
        let anchor = flamewm_api::Rect::new(-100, 1036, 30, 44);
        let placed = fitted_start_placement(
            anchor,
            flamewm_api::Size::new(310, 200),
            PanelEdge::Bottom,
            ctx.work_area,
            8,
        )
        .expect("placed");
        assert!(placed.rect.bottom() <= anchor.y - 8);
        assert!(placed.rect.x >= ctx.work_area.x);
    }

    #[test]
    fn panel_context_hidpi_panel_rect_used_as_is() {
        use flamewm_api::PanelEdge;
        let snapshot = placement_snapshot(
            flamewm_api::Rect::new(0, 0, 3840, 2160),
            flamewm_api::Rect::new(0, 2072, 3840, 88),
            PanelEdge::Bottom,
            7,
            8,
        );
        let ctx = panel_context_for_snapshot(&snapshot, PanelEdge::Bottom).expect("context");
        assert_eq!(ctx.work_area, flamewm_api::Rect::new(0, 0, 3840, 2072));
    }

    #[test]
    fn f10_top_to_bottom_resync_uses_bottom() {
        use flamewm_api::PanelEdge;
        use flamewm_shell_core::popup::work_area_for_panel;
        let output = flamewm_api::Rect::new(0, 0, 1920, 1080);
        let top = placement_snapshot(
            output,
            flamewm_api::Rect::new(0, 0, 1920, 44),
            PanelEdge::Top,
            1,
            1,
        );
        let initial = panel_context_for_snapshot(&top, PanelEdge::Top).expect("top context");
        assert_eq!(initial.panel.edge, PanelEdge::Top);
        let bottom = placement_snapshot(
            output,
            flamewm_api::Rect::new(0, 1036, 1920, 44),
            PanelEdge::Bottom,
            2,
            2,
        );
        // Next Start/Quick resolve through the synced resolver: stored Top
        // misses the gated path, the geometry-driven fallback returns the
        // Bottom context with Bottom generation/revision/work area.
        let synced =
            synced_placement_for_snapshot(&bottom, initial.panel.edge).expect("resync to bottom");
        assert_eq!(synced.panel.edge, PanelEdge::Bottom);
        assert_eq!((synced.display_generation, synced.panels_revision), (2, 2));
        assert_eq!(
            synced.work_area,
            work_area_for_panel(output, synced.panel.geometry, PanelEdge::Bottom)
        );
        assert_eq!(synced.work_area, flamewm_api::Rect::new(0, 0, 1920, 1036));
        // Gate the open paths would check: stale Top edge against the new
        // Bottom geometry is inconsistent; synced Bottom is consistent.
        assert!(!edge_geometry_consistent(
            PanelEdge::Top,
            synced.panel.geometry,
            synced.output.geometry
        ));
        assert!(edge_geometry_consistent(
            PanelEdge::Bottom,
            synced.panel.geometry,
            synced.output.geometry
        ));
    }

    #[test]
    fn panel_context_missing_output_returns_none() {
        use flamewm_api::PanelEdge;
        let mut snapshot = placement_snapshot(
            flamewm_api::Rect::new(0, 0, 1920, 1080),
            flamewm_api::Rect::new(0, 1036, 1920, 44),
            PanelEdge::Bottom,
            1,
            1,
        );
        snapshot.displays.outputs.clear();
        assert!(panel_context_for_snapshot(&snapshot, PanelEdge::Bottom).is_none());
        let mut empty = ShellSnapshot::default();
        empty.displays.outputs = vec![panel_output(
            "HDMI-1",
            flamewm_api::Rect::new(0, 0, 1920, 1080),
        )];
        assert!(panel_context_for_snapshot(&empty, PanelEdge::Bottom).is_none());
    }
}
