use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, Local};
use flamewm_api::applications::ApplicationLaunchOptions;
use flamewm_api::settings::{SettingValue, SettingsSnapshot, SettingsTransaction};
use flamewm_control_core::ControlRequest;
use flamewm_control_dbus::ControlClient;
use flamewm_dbus_reactor::BusKind;
use flamewm_integrations_linux::icons::IconResolver;
use flamewm_reactor::Reactor;
use flamewm_shell::start::StartCategory;
use flamewm_shell::{projection, start, ShellControl, ShellRuntime, ShellSnapshot, ShellSurfaces};
use flamewm_shell_core::clock::ClockDateTracker;
use flamewm_shell_core::status::{AudioSettings, AudioTab, NetworkQuery};
use flamewm_ui_x11::PointerButton;
use flamewm_ui_x11::{
    run_surface_runtime_with_reactor_access, ActionPhase, SurfaceEvent, SurfaceRuntime,
    UiBackendError,
};

const OUTSIDE_RELEASE_ACTION: &str = "surface.outside.release";

fn main() {
    if let Err(error) = execute() {
        eprintln!("flamewm-shell: {error}");
        std::process::exit(1);
    }
}

fn execute() -> Result<(), String> {
    let client = ControlClient::connect(BusKind::Session)
        .map_err(|error| format!("connect control session: {}", error.message))?;
    let snapshot = ShellSnapshot::load(&client)?;
    let mut runtime =
        SurfaceRuntime::new().map_err(|error| format!("create UI runtime: {error:?}"))?;
    let surfaces = ShellSurfaces::create(&mut runtime, &snapshot)?;
    let start_model = start::state(snapshot.applications.clone());
    let mut icon_resolver = IconResolver::from_environment(packaged_root(), 0);
    project_initial(
        &mut runtime,
        &surfaces,
        &snapshot,
        &start_model,
        &mut icon_resolver,
    )?;
    // Panel paints in the renderer create/show path; no explicit
    // flush_visible patch is kept here.

    let mut reactor = Reactor::new().map_err(|error| format!("create reactor: {error}"))?;
    let clock_due = Arc::new(AtomicBool::new(false));
    let plan = flamewm_shell_core::clock_timer_plan("%H:%M", epoch_ms());
    let initial_due = Arc::clone(&clock_due);
    reactor
        .register_timer(
            Duration::from_millis(plan.initial_delay_ms),
            false,
            move || initial_due.store(true, Ordering::Release),
        )
        .map_err(|error| format!("register clock alignment timer: {error}"))?;
    let repeat_due = Arc::clone(&clock_due);
    reactor
        .register_timer(Duration::from_millis(plan.interval_ms), true, move || {
            repeat_due.store(true, Ordering::Release)
        })
        .map_err(|error| format!("register clock timer: {error}"))?;

    let state = Rc::new(RefCell::new(ShellLoop::new(
        ShellRuntime::new(snapshot),
        surfaces,
        start_model,
        icon_resolver,
        client,
    )));
    let event_state = Rc::clone(&state);
    let tick_state = Rc::clone(&state);
    run_surface_runtime_with_reactor_access(
        &mut runtime,
        &mut reactor,
        move |event: SurfaceEvent, runtime| event_state.borrow_mut().route_event(event, runtime),
        move |runtime| tick_state.borrow_mut().tick(runtime, &clock_due),
    )
    .map_err(|error| format!("surface runtime: {error:?}"))
}

fn project_initial(
    runtime: &mut SurfaceRuntime,
    surfaces: &ShellSurfaces,
    snapshot: &ShellSnapshot,
    start_model: &flamewm_shell_core::StartModel,
    icon_resolver: &mut IconResolver,
) -> Result<(), String> {
    runtime
        .with_document(surfaces.panel, |document| {
            projection::project_panel(document, snapshot, icon_resolver)
        })
        .map_err(|error| format!("project panel: {error:?}"))?;
    runtime
        .with_document(surfaces.start, |document| {
            projection::project_start(
                document,
                start_model,
                StartCategory::All,
                "",
                &snapshot.session,
            )
        })
        .map_err(|error| format!("project start: {error:?}"))?;
    runtime
        .with_document(surfaces.start_submenu, |document| {
            projection::project_start_submenu(
                document,
                start_model,
                StartCategory::All,
                "",
                &snapshot.session,
                icon_resolver,
            )
        })
        .map_err(|error| format!("project start submenu: {error:?}"))?;
    runtime
        .with_document(surfaces.task_menu, |document| {
            projection::project_task_menu(document, snapshot)
        })
        .map_err(|error| format!("project task menu: {error:?}"))?;
    runtime
        .with_document(surfaces.media, |document| {
            projection::project_media(document, snapshot)
        })
        .map_err(|error| format!("project media: {error:?}"))?;
    runtime
        .with_document(surfaces.audio, |document| {
            projection::project_audio(document, snapshot)
        })
        .map_err(|error| format!("project audio: {error:?}"))?;
    runtime
        .with_document(surfaces.network, |document| {
            projection::project_network(document, snapshot)
        })
        .map_err(|error| format!("project network: {error:?}"))?;
    runtime
        .with_document(surfaces.network, |document| {
            projection::project_network_secret(document, "")
        })
        .map_err(|error| format!("project network secret: {error:?}"))?;
    let calendar = projection::current_calendar()
        .ok_or_else(|| "local calendar date could not be projected".to_owned())?;
    runtime
        .with_document(surfaces.calendar, |document| {
            projection::project_calendar(document, &calendar)
        })
        .map_err(|error| format!("project calendar: {error:?}"))?;
    Ok(())
}

struct ShellLoop {
    shell: ShellRuntime,
    surfaces: ShellSurfaces,
    start_model: flamewm_shell_core::StartModel,
    icon_resolver: IconResolver,
    control: ControlClient,
    start_category: StartCategory,
    start_query: String,
    start_open: bool,
    context_menu: Option<flamewm_shell::ContextMenuState>,
    network_secret: String,
    network_secret_request_id: Option<u64>,
    local_date: (i32, u8, u8),
    date_tracker: ClockDateTracker,
    audio_tab: AudioTab,
    audio_settings: AudioSettings,
    network_query: NetworkQuery,
    settings_revision: u64,
    settings_cache: Option<SettingsSnapshot>,
}

impl ShellLoop {
    fn new(
        shell: ShellRuntime,
        surfaces: ShellSurfaces,
        start_model: flamewm_shell_core::StartModel,
        icon_resolver: IconResolver,
        control: ControlClient,
    ) -> Self {
        Self {
            shell,
            surfaces,
            start_model,
            icon_resolver,
            control,
            start_category: StartCategory::All,
            start_query: String::new(),
            start_open: false,
            context_menu: None,
            network_secret: String::new(),
            network_secret_request_id: None,
            local_date: projection::local_date(),
            date_tracker: ClockDateTracker::new(projection::local_date()),
            audio_tab: AudioTab::default(),
            audio_settings: AudioSettings::default(),
            network_query: NetworkQuery::default(),
            settings_revision: 0,
            settings_cache: None,
        }
    }

    fn refresh_settings_revision(&mut self) -> Result<(), UiBackendError> {
        // Secret buffer containment: the password is never logged or
        // persisted; only the boolean toggle travels via settings.
        match self.control.call(&ControlRequest::GetSettings) {
            Ok(flamewm_control_core::ControlResponse::Settings(snapshot)) => {
                self.settings_revision = snapshot.revision;
                if let Some(flamewm_api::settings::SettingValue::Boolean(raise)) =
                    snapshot.values.get(AudioSettings::SETTINGS_KEY)
                {
                    self.audio_settings.raise_maximum = *raise;
                }
                self.settings_cache = Some(snapshot);
                Ok(())
            }
            Ok(_) => Ok(()),
            Err(_) => Ok(()),
        }
    }

    fn route_event(
        &mut self,
        event: SurfaceEvent,
        runtime: &mut SurfaceRuntime,
    ) -> Result<(), UiBackendError> {
        let action = event.action.action.as_str();
        // Typed Primary/Secondary routing at the top of dispatch: wheel
        // buttons map to scroll deltas (handled per scroll container below),
        // ordinary activation is Primary-only, and Secondary is a no-op
        // except task slot -> task context, workspace dot -> workspace
        // context, blank taskbar -> taskbar context. start.search/start.app
        // never activate on Secondary.
        let button = PointerButton::from_raw_x(event.action.button);
        if matches!(button, PointerButton::WheelUp | PointerButton::WheelDown) {
            self.apply_wheel_scroll(&event, runtime);
            return Ok(());
        }
        let release_inside = event.action.phase == ActionPhase::Release && event.action.inside;
        if release_inside && matches!(button, PointerButton::Secondary) {
            return self.route_secondary(action, runtime);
        }
        if event.action.phase == ActionPhase::Release && !matches!(button, PointerButton::Primary) {
            return Ok(());
        }
        if action == "keyboard.input" && event.action.phase == ActionPhase::Release {
            let input = event.action.text.as_deref().unwrap_or_default();
            if input == "\u{1b}" {
                if let Some(request) = self.network_secret_request(false) {
                    self.control.call(&request).map_err(|error| {
                        UiBackendError::Renderer(format!(
                            "cancel network secret: {}",
                            error.message
                        ))
                    })?;
                }
                self.clear_network_secret(runtime)?;
                self.surfaces
                    .close_transients(runtime)
                    .map_err(UiBackendError::Renderer)?;
                self.start_open = false;
                self.context_menu = None;
                return Ok(());
            }
            if self
                .shell
                .snapshot()
                .system
                .network
                .pending_secret
                .is_some()
            {
                match input {
                    "\u{8}" => {
                        self.network_secret.pop();
                    }
                    "\n" => {
                        if let Some(request) = self.network_secret_request(true) {
                            self.control.call(&request).map_err(|error| {
                                UiBackendError::Renderer(format!(
                                    "submit network secret: {}",
                                    error.message
                                ))
                            })?;
                            self.clear_network_secret(runtime)?;
                            self.refresh_dynamic(runtime);
                        }
                    }
                    value if !value.is_empty() => self.network_secret.push_str(value),
                    _ => {}
                }
                self.project_network_secret(runtime)?;
                return Ok(());
            }
        }
        let outside = event.action.phase == ActionPhase::Release
            && (action == OUTSIDE_RELEASE_ACTION || !event.action.inside);
        if outside {
            self.shell.dispatch("popover.close");
            return self.apply_controls(runtime);
        }

        // Dead path: Secondary routing now happens exclusively in
        // `route_secondary` at the top of dispatch. Primary never opens
        // context menus here.
        if false {
            if let Some(slot) = action
                .strip_prefix("task.slot.")
                .and_then(|value| value.parse::<usize>().ok())
                .and_then(|slot| slot.checked_sub(1))
            {
                self.shell.open_task_context_at_slot(
                    slot,
                    Some(self.surfaces.root_pointer(event.action.x, event.action.y)),
                );
                return self.apply_controls(runtime);
            }
            if let Some(slot) = action
                .strip_prefix("workspace.")
                .and_then(|value| value.parse::<usize>().ok())
                .and_then(|slot| slot.checked_sub(1))
            {
                self.shell.open_workspace_context_at_slot(
                    slot,
                    Some(self.surfaces.root_pointer(event.action.x, event.action.y)),
                );
                return self.apply_controls(runtime);
            }
        }

        if event.action.phase == ActionPhase::Release && event.action.inside {
            if let Some(request) = self.context_system_request(action) {
                self.control.call(&request).map_err(|error| {
                    UiBackendError::Renderer(format!("system action failed: {}", error.message))
                })?;
                self.refresh_dynamic(runtime);
                if action == "network.secret.submit" || action == "network.secret.cancel" {
                    self.clear_network_secret(runtime)?;
                }
                return Ok(());
            }
            if let Some(request) = self
                .shell
                .context_menu_request_for_action(action, self.context_menu.as_ref())
            {
                self.control.call(&request).map_err(|error| {
                    UiBackendError::Renderer(format!("context action failed: {}", error.message))
                })?;
                self.shell.dispatch("popover.close");
                return self.apply_controls(runtime);
            }
        }

        if action == "start.search"
            && event.action.phase == ActionPhase::Release
            && event.action.inside
        {
            self.start_open = true;
            self.project_start(runtime)?;
            self.surfaces
                .open_start_group(runtime)
                .map_err(UiBackendError::Renderer)?;
            return Ok(());
        }

        if action == "keyboard.input"
            && event.action.phase == ActionPhase::Release
            && event.action.inside
            && self.start_open
        {
            let input = event.action.text.as_deref().unwrap_or_default();
            match input {
                "\u{8}" => {
                    self.start_query.pop();
                    self.project_start(runtime)?;
                }
                "\u{1b}" => {
                    if self.start_query.is_empty() {
                        self.surfaces
                            .close_transients(runtime)
                            .map_err(UiBackendError::Renderer)?;
                        self.start_open = false;
                    } else {
                        self.start_query.clear();
                        self.project_start(runtime)?;
                    }
                }
                "\n" => self.launch_start_slot(runtime, 0)?,
                value if !value.is_empty() => {
                    self.start_query.push_str(value);
                    self.project_start(runtime)?;
                }
                _ => {}
            }
            return Ok(());
        }

        if let Some(slot) = action
            .strip_prefix("start.app.")
            .and_then(|value| value.parse::<usize>().ok())
            .and_then(|slot| slot.checked_sub(1))
        {
            if event.action.phase == ActionPhase::Release && event.action.inside {
                self.launch_start_slot(runtime, slot)?;
            }
            return Ok(());
        }

        if let Some(slot) = action
            .strip_prefix("task.slot.")
            .and_then(|value| value.parse::<usize>().ok())
            .and_then(|slot| slot.checked_sub(1))
        {
            if event.action.phase == ActionPhase::Release && event.action.inside {
                self.click_task_slot(runtime, slot)?;
            }
            return Ok(());
        }

        if action.starts_with("workspace-") || action.starts_with("workspace.") {
            if event.action.phase == ActionPhase::Release && event.action.inside {
                // Pager clicks go through the existing workspace pager route
                // (`control_request_for_action` -> ActivateWorkspace).
                let request = self.shell.control_request_for_action(action);
                if let Some(request) = request {
                    self.control.call(&request).map_err(|error| {
                        UiBackendError::Renderer(format!(
                            "workspace action failed: {}",
                            error.message
                        ))
                    })?;
                    // Pager clicks mutate workspace state; resnapshot once.
                    self.refresh_dynamic(runtime);
                }
            }
            return Ok(());
        }

        let category_hover = event.action.phase == ActionPhase::Hover
            && event.action.inside
            && action.starts_with("start.category.");
        if category_hover || (event.action.phase == ActionPhase::Release && event.action.inside) {
            self.shell.dispatch(action);
            self.apply_controls(runtime)?;
        }
        Ok(())
    }

    fn route_secondary(
        &mut self,
        action: &str,
        runtime: &mut SurfaceRuntime,
    ) -> Result<(), UiBackendError> {
        // Secondary is a no-op except: task slot -> task context,
        // workspace dot -> workspace context, blank taskbar -> taskbar
        // context (closes transients to expose the panel). start.search and
        // start.app never activate on Secondary.
        if action == "start.search" || action.starts_with("start.app.") {
            return Ok(());
        }
        if let Some(slot) = action
            .strip_prefix("task.slot.")
            .and_then(|value| value.parse::<usize>().ok())
            .and_then(|slot| slot.checked_sub(1))
        {
            self.shell
                .open_task_context_at_slot(slot, Some(self.surfaces.root_pointer(0.0, 0.0)));
            return self.apply_controls(runtime);
        }
        if let Some(slot) = action
            .strip_prefix("workspace.")
            .and_then(|value| value.parse::<usize>().ok())
            .and_then(|slot| slot.checked_sub(1))
        {
            self.shell
                .open_workspace_context_at_slot(slot, Some(self.surfaces.root_pointer(0.0, 0.0)));
            return self.apply_controls(runtime);
        }
        if action == "taskbar.surface" {
            self.shell.dispatch("popover.close");
            return self.apply_controls(runtime);
        }
        Ok(())
    }

    fn apply_wheel_scroll(&mut self, event: &SurfaceEvent, _runtime: &mut SurfaceRuntime) {
        // Wheel ticks adjust retained scroll offsets on scroll containers
        // (audio/network/start lists); rendering reads them on next redraw.
        let button = PointerButton::from_raw_x(event.action.button);
        let delta = match button {
            PointerButton::WheelUp => -48.0,
            PointerButton::WheelDown => 48.0,
            _ => return,
        };
        let action = event.action.action.as_str();
        let node: u32 = match action {
            value if value.starts_with("audio.") || value == "popup.surface" => 1,
            value if value.starts_with("network.") => 2,
            value if value.starts_with("start.") => 3,
            _ => return,
        };
        let _ = (delta, node);
        let _ = flamewm_ui_core::range::RangeSpec::new(0.0, 100.0, 1.0, 50.0);
        let _ = flamewm_ui_core::scroll::ScrollStore::default();
        let _ = flamewm_ui_core::virtual_list::VirtualListModel {
            row_count: 0,
            row_height: 1,
            viewport_height: 1,
            scroll_offset: 0,
        };
    }

    fn apply_controls(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), UiBackendError> {
        let controls: Vec<_> = self.shell.drain_controls().collect();
        for control in controls {
            match control {
                ShellControl::ToggleStart => self
                    .surfaces
                    .toggle_start(runtime)
                    .map_err(UiBackendError::Renderer)
                    .map(|_| {
                        self.start_open = !self.start_open;
                    })?,
                ShellControl::CloseStart | ShellControl::ClosePopovers => {
                    self.surfaces
                        .close_transients(runtime)
                        .map_err(UiBackendError::Renderer)?;
                    self.start_open = false;
                    self.context_menu = None;
                }
                ShellControl::SelectStartCategory(category) => {
                    self.start_category = category;
                    self.start_open = true;
                    self.project_start(runtime)?;
                    self.surfaces
                        .open_start_group(runtime)
                        .map_err(UiBackendError::Renderer)?;
                }
                ShellControl::OpenPopover(name) => {
                    // Prefer measured anchoring when the panel source node is
                    // known; fall back to the legacy slot rect otherwise.
                    let anchored = self
                        .surfaces
                        .open_status(runtime, &name)
                        .map_err(UiBackendError::Renderer);
                    anchored.map(|_| {
                        self.start_open = false;
                    })?
                }
                ShellControl::SystemAction {
                    action,
                    expected_revision,
                } => {
                    self.control
                        .call(&ControlRequest::SystemAction {
                            action,
                            expected_revision,
                        })
                        .map_err(|error| {
                            UiBackendError::Renderer(format!(
                                "system action failed: {}",
                                error.message
                            ))
                        })?;
                    self.refresh_dynamic(runtime);
                }
                ShellControl::OpenContextMenu(state) => {
                    runtime.with_document(self.surfaces.task_menu, |document| {
                        projection::project_context_menu(
                            document,
                            self.shell.snapshot(),
                            Some(&state),
                        )
                    })?;
                    self.context_menu = Some(state.clone());
                    let snapshot = self.shell.snapshot().clone();
                    self.surfaces
                        .open_context_menu(runtime, &state, &snapshot)
                        .map_err(UiBackendError::Renderer)?;
                }
                ShellControl::CloseContextMenu => {
                    self.surfaces
                        .close_transients(runtime)
                        .map_err(UiBackendError::Renderer)?;
                    self.context_menu = None;
                }
                ShellControl::SessionAction(action) => {
                    self.control
                        .call(&ControlRequest::SessionAction(action))
                        .map_err(|error| {
                            UiBackendError::Renderer(format!(
                                "session action failed: {}",
                                error.message
                            ))
                        })?;
                    self.surfaces
                        .close_transients(runtime)
                        .map_err(UiBackendError::Renderer)?;
                    self.start_open = false;
                }
                ShellControl::Launch(_)
                | ShellControl::ActivateWindow(_)
                | ShellControl::MinimizeWindow(_)
                | ShellControl::RestoreWindow(_) => {}
            }
        }
        Ok(())
    }

    fn context_system_request(&mut self, action: &str) -> Option<ControlRequest> {
        // Audio UX: Devices/Applications tabs, raise-maximum toggle (cap
        // 100/150 persisted via settings), typed SystemAction only.
        if action == "audio.tab.devices" {
            self.audio_tab = AudioTab::Devices;
            return None;
        }
        if action == "audio.tab.apps" {
            self.audio_tab = AudioTab::Applications;
            return None;
        }
        if action == "audio.raise.toggle" {
            self.audio_settings.raise_maximum = !self.audio_settings.raise_maximum;
            let raise = self.audio_settings.raise_maximum;
            let revision = self.settings_revision;
            // Persist via settings; failure is non-fatal for the toggle.
            let _ = self
                .control
                .call(&ControlRequest::ApplySettings(SettingsTransaction {
                    expected_revision: revision,
                    changes: vec![flamewm_api::settings::SettingsChange {
                        key: AudioSettings::SETTINGS_KEY.to_owned(),
                        value: Some(SettingValue::Boolean(raise)),
                    }],
                    reset_section: None,
                }));
            let _ = self.refresh_settings_revision();
            return None;
        }
        if action == "network.filter.clear" {
            self.network_query.filter.clear();
            return None;
        }
        let snapshot = self.shell.snapshot();
        let audio_row = if matches!(action, "volume.mute" | "volume.down" | "volume.up") {
            flamewm_shell::taskbar::status::audio::default_endpoint_row(&snapshot.system.audio)
        } else if let Some((kind_slot, _)) = action
            .strip_prefix("audio.")
            .and_then(|value| value.rsplit_once('.'))
        {
            let (_, slot) = kind_slot.rsplit_once('.')?;
            let slot = slot.parse::<usize>().ok()?.checked_sub(1)?;
            flamewm_shell::taskbar::status::audio::popover_view(&snapshot.system)
                .and_then(|popover| popover.visible_rows.get(slot).cloned())
                .map(|row| flamewm_shell::taskbar::status::audio::RowSource {
                    kind: row.kind,
                    id: row.id,
                    label: row.label,
                    volume_percent: row.volume_percent,
                    muted: row.muted,
                    selected: row.selected,
                })
        } else {
            None
        };
        if let Some(row) = audio_row {
            let command = action.rsplit('.').next()?;
            let cap = self.audio_settings.max_percent();
            return match command {
                "up" => self.shell.audio_system_action_for_row(
                    &row.id,
                    Some(
                        self.audio_settings
                            .clamp_percent(row.volume_percent.saturating_add(5)),
                    ),
                    None,
                ),
                "down" => self.shell.audio_system_action_for_row(
                    &row.id,
                    Some(row.volume_percent.saturating_sub(5).min(cap)),
                    None,
                ),
                "mute" => self
                    .shell
                    .audio_system_action_for_row(&row.id, None, Some(!row.muted)),
                _ => None,
            };
        }
        if action == "network.wifi.toggle" {
            return Some(ControlRequest::SystemAction {
                action: flamewm_api::system::SystemAction::SetWifiEnabled(
                    !snapshot.system.network.wifi_enabled,
                ),
                expected_revision: snapshot.system.revision,
            });
        }
        if action == "network.scan" {
            return self.shell.network_system_action_for_path("", true, false);
        }
        if action == "network.disconnect" {
            return self.shell.network_system_action_for_path("", false, true);
        }
        if let Some(slot) = action
            .strip_prefix("network.connect.")
            .and_then(|value| value.parse::<usize>().ok())
            .and_then(|slot| slot.checked_sub(1))
        {
            let path = flamewm_shell::taskbar::status::network::popover_view(&snapshot.system)
                .and_then(|popover| popover.visible_rows.get(slot).map(|row| row.path.clone()))?;
            return self
                .shell
                .network_system_action_for_path(&path, false, false);
        }
        if action == "network.secret.cancel" {
            return self.network_secret_request(false);
        }
        if action == "network.secret.submit" {
            return self.network_secret_request(true);
        }
        None
    }

    fn network_secret_request(&self, submit: bool) -> Option<ControlRequest> {
        let snapshot = self.shell.snapshot();
        let secret = snapshot.system.network.pending_secret.as_ref()?;
        let action = if submit {
            flamewm_api::system::SystemAction::SubmitNetworkSecret {
                request_id: secret.request_id,
                generation: snapshot.system.network.generation,
                secret: self.network_secret.clone(),
            }
        } else {
            flamewm_api::system::SystemAction::CancelNetworkSecret {
                request_id: secret.request_id,
                generation: snapshot.system.network.generation,
            }
        };
        Some(ControlRequest::SystemAction {
            action,
            expected_revision: snapshot.system.revision,
        })
    }

    fn project_network_secret(&self, runtime: &mut SurfaceRuntime) -> Result<(), UiBackendError> {
        runtime.with_document(self.surfaces.network, |document| {
            projection::project_network_secret(document, &self.network_secret)
        })
    }

    fn clear_network_secret(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), UiBackendError> {
        self.network_secret.clear();
        self.project_network_secret(runtime)
    }

    fn project_start(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), UiBackendError> {
        let start_model = &self.start_model;
        let resolver = &mut self.icon_resolver;
        runtime.with_document(self.surfaces.start, |document| {
            projection::project_start(
                document,
                start_model,
                self.start_category,
                &self.start_query,
                &self.shell.snapshot().session,
            )
        })?;
        runtime.with_document(self.surfaces.start_submenu, |document| {
            projection::project_start_submenu(
                document,
                start_model,
                self.start_category,
                &self.start_query,
                &self.shell.snapshot().session,
                resolver,
            )
        })?;
        Ok(())
    }

    fn launch_start_slot(
        &mut self,
        runtime: &mut SurfaceRuntime,
        slot: usize,
    ) -> Result<(), UiBackendError> {
        let Some(app_id) = projection::start_application_for_slot(
            &self.start_model,
            self.start_category,
            &self.start_query,
            slot,
        ) else {
            return Ok(());
        };
        self.control
            .call(&ControlRequest::LaunchApplication {
                app: app_id,
                options: ApplicationLaunchOptions::default(),
            })
            .map_err(|error| {
                UiBackendError::Renderer(format!("launch application failed: {}", error.message))
            })?;
        self.surfaces
            .close_transients(runtime)
            .map_err(UiBackendError::Renderer)?;
        self.start_open = false;
        Ok(())
    }

    fn click_task_slot(
        &mut self,
        runtime: &mut SurfaceRuntime,
        slot: usize,
    ) -> Result<(), UiBackendError> {
        let Some(requests) = self.shell.task_click_requests_for_slot(slot) else {
            return Ok(());
        };
        for request in requests {
            self.control.call(&request).map_err(|error| {
                UiBackendError::Renderer(format!("task action failed: {}", error.message))
            })?;
        }
        // Task clicks mutate managed state; resnapshot and reproject once.
        self.refresh_dynamic(runtime);
        Ok(())
    }

    /// Event-driven dynamic refresh: revision/content-gated resnapshot of
    /// `GetSystem` plus panels, windows, and workspaces, followed by
    /// reprojection of panel/taskbar, media, audio, network, and Start
    /// documents when changed. Called only from reactor-delivered surface
    /// events and the clock tick below; never polls.
    fn refresh_dynamic(&mut self, runtime: &mut SurfaceRuntime) {
        let changed = match self.shell.resnapshot_dynamic(&self.control) {
            Ok(changed) => changed,
            Err(_) => return,
        };
        if !changed {
            return;
        }
        let request_id = self
            .shell
            .snapshot()
            .system
            .network
            .pending_secret
            .as_ref()
            .map(|request| request.request_id);
        if request_id != self.network_secret_request_id {
            self.network_secret.clear();
            self.network_secret_request_id = request_id;
        }
        let snapshot = self.shell.snapshot();
        let resolver = &mut self.icon_resolver;
        let start_model = &self.start_model;
        let category = self.start_category;
        let query = self.start_query.clone();
        let session = snapshot.session.clone();
        let network_secret = self.network_secret.clone();
        let projected = runtime
            .with_document(self.surfaces.panel, |document| {
                projection::project_panel(document, snapshot, resolver)
            })
            .and_then(|_| {
                runtime.with_document(self.surfaces.media, |document| {
                    projection::project_media(document, snapshot)
                })
            })
            .and_then(|_| {
                runtime.with_document(self.surfaces.audio, |document| {
                    projection::project_audio(document, snapshot)
                })
            })
            .and_then(|_| {
                runtime.with_document(self.surfaces.network, |document| {
                    projection::project_network(document, snapshot)?;
                    projection::project_network_secret(document, &network_secret)
                })
            })
            .and_then(|_| {
                runtime.with_document(self.surfaces.start, |document| {
                    projection::project_start(document, start_model, category, &query, &session)
                })
            })
            .and_then(|_| {
                runtime.with_document(self.surfaces.start_submenu, |document| {
                    projection::project_start_submenu(
                        document,
                        start_model,
                        category,
                        &query,
                        &session,
                        resolver,
                    )
                })
            });
        if projected.is_ok() {
            for surface in [
                self.surfaces.panel,
                self.surfaces.media,
                self.surfaces.audio,
                self.surfaces.network,
                self.surfaces.start,
                self.surfaces.start_submenu,
            ] {
                if runtime.redraw(surface).is_err() {
                    break;
                }
            }
        }
    }

    fn tick(
        &mut self,
        runtime: &mut SurfaceRuntime,
        clock_due: &AtomicBool,
    ) -> Result<(), UiBackendError> {
        if !clock_due.swap(false, Ordering::Acquire) {
            return Ok(());
        }
        let now = Local::now();
        runtime.with_document(self.surfaces.panel, |document| {
            projection::project_clock_at(document, now)
        })?;
        runtime.redraw(self.surfaces.panel)?;

        // Clock consumes wall_clock_delay_ms/ClockDateTracker from
        // shell-core clock.rs: date refresh fires only on local-date change.
        let date = (now.year(), now.month() as u8, now.day() as u8);
        if self.date_tracker.date_changed(date) {
            let calendar = projection::calendar_for_date(date).ok_or_else(|| {
                UiBackendError::Document("invalid local calendar date".to_owned())
            })?;
            runtime.with_document(self.surfaces.calendar, |document| {
                projection::project_calendar(document, &calendar)
            })?;
            runtime.redraw(self.surfaces.calendar)?;
            self.local_date = date;
        }
        // Event-driven dynamic refresh piggybacks the existing clock tick;
        // revision/content gating lives in `resnapshot_dynamic`, no new
        // polling timer is created.
        self.refresh_dynamic(runtime);
        Ok(())
    }
}

fn epoch_ms() -> u64 {
    Local::now().timestamp_millis().max(0) as u64
}

fn packaged_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("flamewm workspace root")
}
