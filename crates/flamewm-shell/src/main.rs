use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{Datelike, Local};
use flamewm_api::applications::ApplicationLaunchOptions;
use flamewm_api::ports::FdEvents;
use flamewm_api::settings::{SettingValue, SettingsSnapshot, SettingsTransaction};
use flamewm_control_core::ControlRequest;
use flamewm_control_dbus::{ControlClient, ControlSignalClient};
use flamewm_control_wire::ControlSignal;
use flamewm_dbus_reactor::BusKind;
use flamewm_reactor::Reactor;
use flamewm_shell::async_projection::{self, StartViewNote};
use flamewm_shell::icon_loader::IconLoader;
use flamewm_shell::start::StartCategory;
use flamewm_shell::{projection, start, ShellControl, ShellRuntime, ShellSnapshot, ShellSurfaces};
use flamewm_shell_core::clock::ClockDateTracker;
use flamewm_shell_core::popup::PopupRefusal;
use flamewm_shell_core::status::{AudioSettings, AudioTab, NetworkQuery};
use flamewm_ui_x11::PointerButton;
use flamewm_ui_x11::{
    run_surface_runtime_with_reactor_access, ActionPhase, SurfaceEvent, SurfaceRuntime,
    UiBackendError,
};

const OUTSIDE_RELEASE_ACTION: &str = "surface.outside.release";

/// C07: one shell operation owns exactly one total span; nested totals must
/// not double-own. `operation_total_active` on the `ShellLoop` event turn
/// marks the outer owner; inner paths check it and skip their own total.
const SHELL_SLOW_TOTAL_NS: u128 = 16_670_000;
const SHELL_SLOW_COOLDOWN_MS: u64 = 500;

/// C07 slow-operation note: when `started`->now exceeds one 60Hz frame,
/// emit `shell.operation.slow` debug with a 500ms per-op cooldown. The
/// cooldown key is a per-op static atomic (millis since an arbitrary
/// epoch); the emit itself also passes the 500ms cooldown to the
/// rate-limited debug writer. Static labels only.
fn shell_slow_note(op: &'static str, started: Instant) {
    if started.elapsed().as_nanos() <= SHELL_SLOW_TOTAL_NS {
        return;
    }
    fn last_for(op: &str) -> Option<&'static std::sync::atomic::AtomicU64> {
        use std::sync::atomic::AtomicU64;
        static START: AtomicU64 = AtomicU64::new(0);
        static CONTEXT: AtomicU64 = AtomicU64::new(0);
        static WORKSPACE: AtomicU64 = AtomicU64::new(0);
        match op {
            "start" => Some(&START),
            "context" => Some(&CONTEXT),
            "workspace" => Some(&WORKSPACE),
            _ => None,
        }
    }
    let wall_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    if let Some(last) = last_for(op) {
        let prev = last.load(std::sync::atomic::Ordering::Relaxed);
        if wall_ms.saturating_sub(prev) < SHELL_SLOW_COOLDOWN_MS {
            return;
        }
        last.store(wall_ms, std::sync::atomic::Ordering::Relaxed);
    }
    flamewm_debug::emit(
        flamewm_debug::DebugEventId("shell.operation.slow"),
        Duration::from_millis(SHELL_SLOW_COOLDOWN_MS),
        || format!("op={op} slow"),
    );
}

/// C07 stage helper: open a static profiler span. Static labels only:
/// no `format!` labels on hot paths.
macro_rules! shell_stage {
    ($label:expr) => {{
        let __point = flamewm_profiler::ProfilePoint::new($label);
        __point.start()
    }};
}

fn main() {
    // J07 role branching at process entry: `--quick-control-host` runs the
    // helper role (profiler `flamewm-quick-control`, owns
    // audio/network/calendar surfaces); otherwise the normal shell role
    // (profiler `flamewm-shell`, owns panel/start/task menu/media).
    if std::env::args().any(|arg| arg == "--quick-control-host") {
        if let Err(error) = flamewm_shell::quick_controls::run_quick_control_host() {
            eprintln!("flamewm-quick-control: {error}");
            std::process::exit(1);
        }
        return;
    }
    if let Err(error) = execute() {
        eprintln!("flamewm-shell: {error}");
        std::process::exit(1);
    }
}

fn execute() -> Result<(), String> {
    flamewm_profiler::init_process("flamewm-shell");
    flamewm_debug::init_process("flamewm-shell");
    let total = flamewm_profiler::ProfilePoint::new("shell.startup.total");
    let total_guard = total.start();
    let client = {
        let scope = flamewm_profiler::ProfilePoint::new("shell.startup.control_connect");
        let _s = scope.start();
        ControlClient::connect(BusKind::Session)
            .map_err(|error| format!("connect control session: {}", error.message))?
    };
    let snapshot = {
        let scope = flamewm_profiler::ProfilePoint::new("shell.startup.bootstrap_call");
        let _s = scope.start();
        ShellSnapshot::load(&client)?
    };
    let mut runtime = {
        let scope = flamewm_profiler::ProfilePoint::new("shell.startup.runtime_create");
        let _s = scope.start();
        SurfaceRuntime::new().map_err(|error| format!("create UI runtime: {error:?}"))?
    };
    let surfaces = {
        let scope = flamewm_profiler::ProfilePoint::new("shell.startup.surface_create");
        let _s = scope.start();
        ShellSurfaces::create(&mut runtime, &snapshot)?
    };
    let start_model = start::state(snapshot.applications.clone());
    {
        let scope = flamewm_profiler::ProfilePoint::new("shell.startup.panel_project");
        let _s = scope.start();
        project_panel_critical(&mut runtime, &surfaces, &snapshot, &start_model)?;
    }
    {
        let scope = flamewm_profiler::ProfilePoint::new("shell.startup.panel_show");
        let _s = scope.start();
        runtime
            .show(surfaces.panel)
            .map_err(|e| format!("show panel: {e:?}"))?;
    }
    let loader_root = packaged_root();
    // flush_visible patch is kept here.

    let profile_interval = profile_interval_secs();
    let profile_due = Arc::new(AtomicBool::new(false));
    let profile_flag = Arc::clone(&profile_due);

    let dispatcher = flamewm_control_dbus::MutationDispatcher::start(BusKind::Session, 64).ok();
    let mut reactor = {
        let scope = flamewm_profiler::ProfilePoint::new("shell.startup.reactor_enter");
        let _s = scope.start();
        Reactor::new().map_err(|error| format!("create reactor: {error}"))?
    };
    drop(total_guard);
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

    if let Some(dispatcher) = dispatcher.as_ref() {
        let watch_fd = dispatcher.wake_fd();
        let wake_dispatcher = std::sync::Arc::clone(dispatcher);
        reactor
            .register_raw_fd_with_action(watch_fd, calloop::Interest::READ, move |_, _| {
                flamewm_shell::control_actions::drain_results(Some(&wake_dispatcher));
                flamewm_reactor::FdAction::Continue
            })
            .map_err(|error| format!("register mutation result source: {error}"))?;
    }
    let state = Rc::new(RefCell::new(ShellLoop::new(
        ShellRuntime::new(snapshot),
        surfaces,
        start_model,
        client,
        dispatcher,
        loader_root,
        current_exe_program(),
    )));
    // J07 parent: start the helper asynchronously after panel show. The
    // parent never blocks on the helper; spawn failure is nonfatal.
    state.borrow_mut().start_helper_after_panel();
    let event_state = Rc::clone(&state);
    let tick_state = Rc::clone(&state);
    let signal_state = Rc::clone(&state);
    // Signal subscription into the same Reactor: snapshot signals apply
    // their payload directly in `mark_signal` (zero fetch); legacy
    // revision-only signals are ignored for normal refresh. The tick
    // drains applied domains plus fetch-owned system state once per turn.
    let signal_client = ControlSignalClient::connect(BusKind::Session)
        .map_err(|error| format!("subscribe control signals: {}", error))?;
    // Leak the client for process lifetime: its pump fd must stay open for the
    // reactor registration below, and the reactor never owns the descriptor.
    let signal_client: &'static ControlSignalClient = Box::leak(Box::new(signal_client));
    {
        let watch = signal_client.watch();
        let interest = signal_interest(watch.events);
        reactor
            .register_raw_fd_with_action(watch.fd, interest, move |_, _| {
                let _ = signal_client.on_ready(|signal: ControlSignal| {
                    signal_state.borrow_mut().note_signal(&signal);
                });
                flamewm_reactor::FdAction::Continue
            })
            .map_err(|error| format!("register control signal source: {error}"))?;
    }
    let tick_profile_flag = Arc::clone(&profile_due);
    reactor
        .register_timer(Duration::from_secs(5), false, move || {
            tick_profile_flag.store(true, Ordering::Release);
        })
        .map_err(|error| format!("register profile one-shot: {error}"))?;
    // Canonical IconService wake FD: registered before the run loop so
    // worker completions wake the reactor; the tick drains them. No Shell
    // icon forward duplication exists: this is the single registration.
    {
        let icon_fd = state.borrow_mut().icon_wake_fd();
        let icon_state = Rc::clone(&state);
        reactor
            .register_raw_fd_with_action(icon_fd, calloop::Interest::READ, move |_, _| {
                icon_state.borrow_mut().drain_icons_pending();
                flamewm_reactor::FdAction::Continue
            })
            .map_err(|error| format!("register icon result source: {error}"))?;
    }
    reactor
        .register_timer(Duration::from_secs(profile_interval), true, move || {
            profile_flag.store(true, Ordering::Release)
        })
        .map_err(|error| format!("register profile timer: {error}"))?;
    run_surface_runtime_with_reactor_access(
        &mut runtime,
        &mut reactor,
        move |event: SurfaceEvent, runtime| event_state.borrow_mut().route_event(event, runtime),
        {
            let tick_profile = Arc::clone(&profile_due);
            move |runtime| {
                if tick_profile.swap(false, Ordering::Acquire) {
                    let _ = flamewm_profiler::report_window();
                }
                tick_state.borrow_mut().tick(runtime, &clock_due)
            }
        },
    )
    .map_err(|error| format!("surface runtime: {error:?}"))
}

/// Parent helper program: re-exec this binary with the helper role flag so
/// no second binary or install path is required.
fn current_exe_program() -> String {
    std::env::current_exe()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "flamewm-shell".to_owned())
}

fn project_panel_critical(
    runtime: &mut SurfaceRuntime,
    surfaces: &ShellSurfaces,
    snapshot: &ShellSnapshot,
    start_model: &flamewm_shell_core::StartModel,
) -> Result<(), String> {
    runtime
        .with_document(surfaces.panel, |document| {
            async_projection::project_panel_async(document, snapshot, None)
        })
        .map_err(|error| format!("project panel: {error:?}"))?;
    runtime
        .with_document(surfaces.start, |document| {
            async_projection::project_start_unified(
                document,
                start_model,
                StartCategory::All,
                "",
                &snapshot.session,
                None,
            )
        })
        .map_err(|error| format!("project start: {error:?}"))?;
    Ok(())
}

struct ShellLoop {
    shell: ShellRuntime,
    surfaces: ShellSurfaces,
    start_model: flamewm_shell_core::StartModel,
    control: ControlClient,
    dispatcher: Option<std::sync::Arc<flamewm_control_dbus::MutationDispatcher>>,
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
    icon_loader: Option<IconLoader>,
    loader_root: std::path::PathBuf,
    start_view: StartViewNote,
    quick_control: flamewm_shell::quick_controls::QuickControlSupervisor,
}

impl ShellLoop {
    fn new(
        shell: ShellRuntime,
        surfaces: ShellSurfaces,
        start_model: flamewm_shell_core::StartModel,
        control: ControlClient,
        dispatcher: Option<std::sync::Arc<flamewm_control_dbus::MutationDispatcher>>,
        loader_root: std::path::PathBuf,
        quick_program: String,
    ) -> Self {
        let quick_control =
            flamewm_shell::quick_controls::QuickControlSupervisor::new(quick_program);
        Self {
            shell,
            surfaces,
            start_model,
            control,
            dispatcher,
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
            icon_loader: None,
            loader_root,
            start_view: StartViewNote::default(),
            quick_control,
        }
    }

    /// J07: start the helper asynchronously after panel show. Spawn failure
    /// is nonfatal: the shell keeps running with in-process media only.
    fn start_helper_after_panel(&mut self) {
        if let Err(error) = self.quick_control.start_after_panel() {
            eprintln!("flamewm-shell: quick-control helper spawn failed (nonfatal): {error}");
        }
    }

    fn note_signal(&mut self, signal: &ControlSignal) {
        self.shell.mark_signal(signal);
    }

    fn refresh_settings_revision(&mut self) -> Result<(), UiBackendError> {
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

    /// J07 nonblocking submit: enqueue onto the mutation queue and return
    /// immediately. Full/closed queue is a nonfatal diagnostic (counter +
    /// stderr in `control_actions`); the event loop always survives.
    fn submit(&self, request: ControlRequest) {
        flamewm_shell::control_actions::enqueue_action(self.dispatcher.as_ref(), request);
    }

    fn route_event(
        &mut self,
        event: SurfaceEvent,
        runtime: &mut SurfaceRuntime,
    ) -> Result<(), UiBackendError> {
        let action = event.action.action.as_str();
        // never activate on Secondary.
        let button = PointerButton::from_raw_x(event.action.button);
        if matches!(button, PointerButton::WheelUp | PointerButton::WheelDown) {
            self.apply_wheel_scroll(&event, runtime);
            return Ok(());
        }
        let release_inside = event.action.phase == ActionPhase::Release && event.action.inside;
        if release_inside && matches!(button, PointerButton::Secondary) {
            return self.route_secondary(&event, runtime);
        }
        if event.action.phase == ActionPhase::Release && !matches!(button, PointerButton::Primary) {
            return Ok(());
        }
        if action == "keyboard.input" && event.action.phase == ActionPhase::Release {
            let input = event.action.text.as_deref().unwrap_or_default();
            if input == "\u{1b}" {
                if let Some(request) = self.network_secret_request(false) {
                    self.submit(request);
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
                            self.submit(request);
                            self.clear_network_secret(runtime)?;
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
            // C04: geometry-invalidating or menu-opening paths also drop
            // any helper popup on the same turn (nonfatal, loop survives).
            self.close_quick_control_nonfatal();
            return self.apply_controls(runtime);
        }

        // context menus here.
        if false {
            if let Some(slot) = action
                .strip_prefix("task.slot.")
                .and_then(|value| value.parse::<usize>().ok())
                .and_then(|slot| slot.checked_sub(1))
            {
                self.shell.open_task_context_at_slot(
                    slot,
                    Some(
                        self.surfaces
                            .root_pointer(runtime, event.action.x, event.action.y),
                    ),
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
                    Some(
                        self.surfaces
                            .root_pointer(runtime, event.action.x, event.action.y),
                    ),
                );
                return self.apply_controls(runtime);
            }
        }

        if event.action.phase == ActionPhase::Release && event.action.inside {
            if let Some(request) = self.context_system_request(action) {
                self.submit(request);
                if action == "network.secret.submit" || action == "network.secret.cancel" {
                    self.clear_network_secret(runtime)?;
                }
                return Ok(());
            }
            if let Some(request) = self
                .shell
                .context_menu_request_for_action(action, self.context_menu.as_ref())
            {
                self.submit(request);
                self.shell.dispatch("popover.close");
                return self.apply_controls(runtime);
            }
        }

        if action == "start.search"
            && event.action.phase == ActionPhase::Release
            && event.action.inside
        {
            // C07: start open via the search affordance owns the same
            // start total as the ToggleStart path (project/measure/place/
            // prepare/present). No nested total inside `project_start`.
            // F10: read current context generation; refuse on inconsistency.
            self.note_popup_generation("start");
            if self.refuse_popup_on_inconsistent_geometry().is_some() {
                return Ok(());
            }
            self.start_open = true;
            let started = Instant::now();
            let _total = shell_stage!("shell.start.open.total");
            self.project_start(runtime)?;
            let _measure = shell_stage!("shell.start.open.measure");
            let _place = shell_stage!("shell.start.open.place");
            let _prepare = shell_stage!("shell.start.open.prepare");
            let opened = self.surfaces.open_start_group(runtime);
            let _present = shell_stage!("shell.start.open.present");
            shell_slow_note("start", started);
            match opened {
                Ok(()) => {}
                Err(error) if is_popup_refusal(&error) => {}
                Err(error) => return Err(UiBackendError::Renderer(error)),
            }
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
                // C07: workspace action owns one operation: submit stage
                // around the submit, signal-apply stage around the helper
                // anchor invalidation, project stage around the (no-op
                // here) projection tail. The normal path performs no
                // control fetch; the fetch counter stays 0.
                let started = Instant::now();
                {
                    let _submit = shell_stage!("shell.workspace.action.submit");
                    if let Some(request) = self.shell.control_request_for_action(action) {
                        self.submit(request);
                    }
                }
                {
                    let _apply = shell_stage!("shell.workspace.action.signal.apply");
                    // C04: workspace change invalidates the helper anchor.
                    self.close_quick_control_nonfatal();
                }
                {
                    let _project = shell_stage!("shell.workspace.action.project");
                }
                shell_slow_note("workspace", started);
            }
            return Ok(());
        }

        let category_hover = event.action.phase == ActionPhase::Hover
            && event.action.inside
            && action.starts_with("start.category.");
        if category_hover {
            // Parse the target category from the action and compare the
            // target StartView: same-view hovers are free, only a real
            // target change updates state and reprojects ONE unified
            // document (one intrinsic union, one move_resize, one show).
            let Some(target) = flamewm_shell::start::category_for_action(action) else {
                return Ok(());
            };
            if !self.start_view.note_start_view(target, &self.start_query) {
                return Ok(());
            }
            let transition = flamewm_profiler::ProfilePoint::new("shell.category_transition");
            let _s = transition.start();
            self.shell.dispatch(action);
            self.apply_controls(runtime)?;
            return Ok(());
        }
        if event.action.phase == ActionPhase::Release && event.action.inside {
            self.shell.dispatch(action);
            self.apply_controls(runtime)?;
        }
        Ok(())
    }

    fn route_secondary(
        &mut self,
        event: &SurfaceEvent,
        runtime: &mut SurfaceRuntime,
    ) -> Result<(), UiBackendError> {
        let action = event.action.action.as_str();
        if action == "start.search" || action.starts_with("start.app.") {
            return Ok(());
        }
        if let Some(slot) = action
            .strip_prefix("task.slot.")
            .and_then(|value| value.parse::<usize>().ok())
            .and_then(|slot| slot.checked_sub(1))
        {
            self.shell.open_task_context_at_slot(
                slot,
                Some(
                    self.surfaces
                        .root_pointer(runtime, event.action.x, event.action.y),
                ),
            );
            self.close_quick_control_nonfatal();
            return self.apply_controls(runtime);
        }
        if let Some(slot) = action
            .strip_prefix("workspace.")
            .and_then(|value| value.parse::<usize>().ok())
            .and_then(|slot| slot.checked_sub(1))
        {
            self.shell.open_workspace_context_at_slot(
                slot,
                Some(
                    self.surfaces
                        .root_pointer(runtime, event.action.x, event.action.y),
                ),
            );
            self.close_quick_control_nonfatal();
            return self.apply_controls(runtime);
        }
        if action == "taskbar.surface" {
            self.shell.dispatch("popover.close");
            return self.apply_controls(runtime);
        }
        Ok(())
    }

    fn apply_wheel_scroll(&mut self, event: &SurfaceEvent, _runtime: &mut SurfaceRuntime) {
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
                ShellControl::ToggleStart => {
                    // C07: start open owns one total around the open path
                    // (project/measure/place/prepare/present). The close
                    // branch owns only its close stage. Refusal leaves
                    // state untouched.
                    // F10: read current context generation; refuse when
                    // edge/geometry is inconsistent.
                    self.note_popup_generation("start");
                    if self.refuse_popup_on_inconsistent_geometry().is_some() {
                        continue;
                    }
                    let was_open = self.surfaces.is_start_group_open();
                    if !was_open {
                        let started = Instant::now();
                        let _total = shell_stage!("shell.start.open.total");
                        self.project_start(runtime)?;
                        let _measure = shell_stage!("shell.start.open.measure");
                        let _place = shell_stage!("shell.start.open.place");
                        let _prepare = shell_stage!("shell.start.open.prepare");
                        let opened = self.surfaces.toggle_start(runtime);
                        let _present = shell_stage!("shell.start.open.present");
                        shell_slow_note("start", started);
                        match opened {
                            Ok(()) => {
                                self.start_open = !was_open;
                            }
                            Err(error) if is_popup_refusal(&error) => {}
                            Err(error) => return Err(UiBackendError::Renderer(error)),
                        }
                    } else {
                        let _close = shell_stage!("shell.start.open.close");
                        match self.surfaces.toggle_start(runtime) {
                            Ok(()) => {
                                self.start_open = !was_open;
                            }
                            Err(error) if is_popup_refusal(&error) => {}
                            Err(error) => return Err(UiBackendError::Renderer(error)),
                        }
                    }
                }
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
                    // F10: read current context generation; refuse when
                    // edge/geometry is inconsistent.
                    self.note_popup_generation("start");
                    if self.refuse_popup_on_inconsistent_geometry().is_some() {
                        continue;
                    }
                    // C07: SelectStartCategory is its own operation: it owns
                    // the start total (project/measure/place/prepare/
                    // present). `project_start` is only the project stage,
                    // so no nested total double-owns the operation.
                    // Project-first, then single measured show: already-open
                    // category switches resize transactionally inside
                    // open_start_group, so no 0,0 or half-resized frame.
                    let started = Instant::now();
                    let _total = shell_stage!("shell.start.open.total");
                    self.project_start(runtime)?;
                    let _measure = shell_stage!("shell.start.open.measure");
                    let _place = shell_stage!("shell.start.open.place");
                    let _prepare = shell_stage!("shell.start.open.prepare");
                    let opened = self.surfaces.open_start_group(runtime);
                    let _present = shell_stage!("shell.start.open.present");
                    shell_slow_note("start", started);
                    match opened {
                        Ok(()) => {}
                        Err(error) if is_popup_refusal(&error) => {}
                        Err(error) => return Err(UiBackendError::Renderer(error)),
                    }
                }
                ShellControl::OpenPopover(name) => {
                    // J07: media stays in-process; audio/network/clock
                    // compute the retained anchor + work area + edge and
                    // forward to the supervisor, then return immediately.
                    // Supervisor failures are nonfatal. Opening media or
                    // any in-process popup closes a helper popup first.
                    if matches!(name.as_str(), "volume" | "audio" | "network" | "clock") {
                        self.open_quick_control(runtime, &name);
                        self.start_open = false;
                        continue;
                    }
                    self.close_quick_control_nonfatal();
                    // F10: read current context generation; refuse when
                    // edge/geometry is inconsistent.
                    self.note_popup_generation("media");
                    if self.refuse_popup_on_inconsistent_geometry().is_some() {
                        continue;
                    }
                    // Single measured show: project first (model -> document),
                    // then measure (intrinsic) -> anchor (panel node rect) ->
                    // prepare (geometry/backbuffer/shape) -> map/raise/grab ->
                    // present. No fixed sizes, no 0,0 fallback; measurement
                    // refusal leaves the current state untouched.
                    self.project_status_content(runtime, &name)?;
                    match self.surfaces.open_status(runtime, &name) {
                        Ok(()) => {}
                        Err(error) if is_popup_refusal(&error) => {}
                        Err(error) => return Err(UiBackendError::Renderer(error)),
                    }
                    self.start_open = false;
                }
                ShellControl::SystemAction {
                    action,
                    expected_revision,
                } => {
                    self.submit(ControlRequest::SystemAction {
                        action,
                        expected_revision,
                    });
                }
                ShellControl::OpenContextMenu(state) => {
                    // C04: task/shell context menus invalidate the helper
                    // anchor; send quick-control CLOSE on the same turn.
                    // C07: this branch owns the context total (project/
                    // measure/place/prepare/present). Nested totals must
                    // not double-own: no other total starts inside.
                    // F10: refuse when edge/geometry is inconsistent.
                    if self.refuse_popup_on_inconsistent_geometry().is_some() {
                        continue;
                    }
                    let started = Instant::now();
                    let _total = shell_stage!("shell.context.open.total");
                    self.close_quick_control_nonfatal();
                    {
                        let _project = shell_stage!("shell.context.open.project");
                        runtime.with_document(self.surfaces.task_menu, |document| {
                            projection::project_context_menu(
                                document,
                                self.shell.snapshot(),
                                Some(&state),
                            )
                        })?;
                    }
                    self.context_menu = Some(state.clone());
                    let snapshot = self.shell.snapshot().clone();
                    let _measure = shell_stage!("shell.context.open.measure");
                    let _place = shell_stage!("shell.context.open.place");
                    let _prepare = shell_stage!("shell.context.open.prepare");
                    let opened = self
                        .surfaces
                        .open_context_menu(runtime, &state, &snapshot)
                        .map_err(UiBackendError::Renderer);
                    let _present = shell_stage!("shell.context.open.present");
                    shell_slow_note("context", started);
                    opened?;
                }
                ShellControl::CloseContextMenu => {
                    let _close = shell_stage!("shell.context.open.close");
                    self.surfaces
                        .close_transients(runtime)
                        .map_err(UiBackendError::Renderer)?;
                    self.context_menu = None;
                }
                ShellControl::SessionAction(action) => {
                    self.submit(ControlRequest::SessionAction(action));
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
            self.submit(ControlRequest::ApplySettings(SettingsTransaction {
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

    fn project_network_secret(&self, _runtime: &mut SurfaceRuntime) -> Result<(), UiBackendError> {
        // J07: the secret editor lives in the helper process; the parent
        // keeps the masked-length-free no-op so keyboard paths stay total.
        Ok(())
    }

    fn clear_network_secret(
        &mut self,
        _runtime: &mut SurfaceRuntime,
    ) -> Result<(), UiBackendError> {
        self.network_secret.clear();
        Ok(())
    }

    fn project_status_content(
        &mut self,
        runtime: &mut SurfaceRuntime,
        name: &str,
    ) -> Result<(), UiBackendError> {
        // Project-first: the measured open reads intrinsic size from the
        // already-projected document, so content must land before measure.
        // J07: only media projects in-process; helper kinds are unreachable
        // here (routed to the supervisor before this call).
        let snapshot = self.shell.snapshot().clone();
        match name {
            "media" => runtime.with_document(self.surfaces.media, |document| {
                projection::project_media(document, &snapshot)
            })?,
            _ => {}
        }
        Ok(())
    }

    /// C04: send quick-control CLOSE; close signals are owned in main
    /// (workspace change, menus, media open, outside release, geometry
    /// invalidation). Best-effort and nonfatal: the loop always survives.
    fn close_quick_control_nonfatal(&mut self) {
        if let Err(error) = self.quick_control.close() {
            let message = error.to_string();
            // NotRunning = no helper/harmless; log the rest as diagnostics.
            if !message.contains("not running") {
                eprintln!("flamewm-shell: quick-control close failed (nonfatal): {message}");
            }
        }
    }

    /// J07: retained anchor + work area + edge -> supervisor.open, then
    /// return immediately. Failures are nonfatal (logged, loop survives).
    /// F10: always reads the current context generation first and
    /// refuses when edge/geometry is inconsistent.
    fn open_quick_control(&mut self, runtime: &SurfaceRuntime, name: &str) {
        use flamewm_shell::quick_controls::QuickControlKind;
        self.note_popup_generation("quick");
        if self.refuse_popup_on_inconsistent_geometry().is_some() {
            return;
        }
        let kind = match name {
            "volume" | "audio" => QuickControlKind::Audio,
            "network" => QuickControlKind::Network,
            "clock" => QuickControlKind::Calendar,
            _ => return,
        };
        let source_id = match kind {
            QuickControlKind::Audio => "tray-volume",
            QuickControlKind::Network => "tray-network",
            QuickControlKind::Calendar => "clock-button",
        };
        // Retained anchor preferred; snapshot geometry is the sole
        // pre-map fallback (same contract as the in-process path).
        let anchor =
            self.surfaces
                .quick_anchor(runtime, &self.pending_anchor_fallback(), source_id);
        let work_area = self.surfaces.work_area();
        let panel_edge = self.surfaces.panel_edge();
        flamewm_shell::quick_controls::host::open_via_supervisor_nonfatal(
            &mut self.quick_control,
            kind,
            anchor,
            work_area,
            panel_edge,
        );
    }

    /// Pre-map anchor fallback: the panel snapshot geometry. Used only
    /// when no retained layout exists yet (same sole-fallback contract as
    /// the in-process popup path).
    fn pending_anchor_fallback(&self) -> flamewm_api::Rect {
        self.shell
            .snapshot()
            .panels
            .panels
            .first()
            .map(|panel| panel.geometry)
            .unwrap_or(flamewm_api::Rect::new(0, 0, 1, 1))
    }

    fn project_start(&mut self, runtime: &mut SurfaceRuntime) -> Result<(), UiBackendError> {
        // C07: project stage only. The caller owns the operation total, so
        // nested totals never double-own one operation.
        let _project = shell_stage!("shell.start.open.project");
        let start_model = self.start_model.clone();
        let category = self.start_category;
        let query = self.start_query.clone();
        let session = self.shell.snapshot().session.clone();
        self.ensure_loader();
        let loader = self.icon_loader.as_mut();
        // ONE unified document on the single Start surface: categories +
        // search + app rows + power rows project in one pass, so one
        // intrinsic union, one move_resize, one show/present follow.
        runtime.with_document(self.surfaces.start, |document| {
            async_projection::project_start_unified(
                document,
                &start_model,
                category,
                &query,
                &session,
                loader,
            )
        })?;
        Ok(())
    }

    fn ensure_loader(&mut self) {
        if self.icon_loader.is_none() {
            self.icon_loader = Some(IconLoader::spawn(self.loader_root.clone()));
        }
    }

    /// Canonical IconService wake FD for the reactor loop. Ensures the
    /// loader exists (single owner) and returns the service descriptor;
    /// the reactor only wakes on it, `drain_icons` drains on the tick.
    fn icon_wake_fd(&mut self) -> std::os::unix::io::RawFd {
        self.ensure_loader();
        self.icon_loader
            .as_ref()
            .map(|loader| loader.wake_fd())
            .expect("icon loader present after ensure")
    }

    /// FD wake path: drain the wake pipe only. Result payloads stay queued
    /// for the tick (`drain_icons`), which owns document mutation. No Shell
    /// forward duplication: the service FD is the single wake source.
    fn drain_icons_pending(&mut self) {
        self.ensure_loader();
        if let Some(loader) = self.icon_loader.as_mut() {
            loader.wake_drain();
        }
    }

    fn drain_icons(&mut self, runtime: &mut SurfaceRuntime) {
        self.ensure_loader();
        let results = self
            .icon_loader
            .as_mut()
            .map(|loader| loader.drain_ready())
            .unwrap_or_default();
        if self
            .icon_loader
            .as_ref()
            .map(|l| l.queue_full_drops)
            .unwrap_or(0)
            > 0
            || self
                .icon_loader
                .as_ref()
                .map(|l| l.stale_drops)
                .unwrap_or(0)
                > 0
        {
            if let Some(loader) = self.icon_loader.as_ref() {
                async_projection::note_loader_stats(loader);
            }
        }
        if results.is_empty() {
            return;
        }
        // Exact-target completion: worker result -> wake -> reactor ->
        // verify (key + generation + view epoch) -> cache -> mutate the
        // one image node only, then redraw its surface. No whole-start or
        // whole-panel reproject per icon.
        let mut need_panel = false;
        let mut need_submenu = false;
        for res in &results {
            let raster = match &res.raster {
                Some(r) => r.clone(),
                None => continue,
            };
            let image = flamewm_ui_x11::RuntimeImage {
                source: raster.source.clone(),
                width: raster.width,
                height: raster.height,
                pixels: raster.pixels.clone(),
            };
            async_projection::note_icon_result(
                &res.name,
                raster.width,
                raster.height,
                image.clone(),
            );
            let surface = match res.target {
                flamewm_shell::icon_loader::IconTarget::StartSlot(_) if !self.start_open => {
                    continue;
                }
                target => flamewm_shell::icon_loader::IconLoader::dependent_surface(target),
            };
            let applied = match surface {
                flamewm_shell::icon_loader::DependentSurface::Panel => {
                    runtime.with_document(self.surfaces.panel, |document| {
                        async_projection::apply_icon_result(
                            document,
                            res.target,
                            &res.name,
                            res.width,
                            res.height,
                            res.generation,
                            image.clone(),
                        )
                    })
                }
                flamewm_shell::icon_loader::DependentSurface::Start => {
                    runtime.with_document(self.surfaces.start, |document| {
                        async_projection::apply_icon_result(
                            document,
                            res.target,
                            &res.name,
                            res.width,
                            res.height,
                            res.generation,
                            image.clone(),
                        )
                    })
                }
            };
            let mutated = match applied {
                Ok(true) => true,
                Ok(false) => continue,
                Err(_) => continue,
            };
            // apply_icon_result already counted stale drops for mismatched
            // bindings; only mutate+redraw on a verified binding (dirty via
            // existing node invalidation inside image replace + redraw).
            if !mutated {
                continue;
            }
            match surface {
                flamewm_shell::icon_loader::DependentSurface::Panel => need_panel = true,
                flamewm_shell::icon_loader::DependentSurface::Start => need_submenu = true,
            }
        }
        if need_panel {
            let _ = runtime.redraw(self.surfaces.panel);
        }
        if need_submenu {
            let _ = runtime.redraw(self.surfaces.start);
        }
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
        self.submit(ControlRequest::LaunchApplication {
            app: app_id,
            options: ApplicationLaunchOptions::default(),
        });
        self.surfaces
            .close_transients(runtime)
            .map_err(UiBackendError::Renderer)?;
        self.start_open = false;
        Ok(())
    }

    fn click_task_slot(
        &mut self,
        _runtime: &mut SurfaceRuntime,
        slot: usize,
    ) -> Result<(), UiBackendError> {
        let Some(requests) = self.shell.task_click_requests_for_slot(slot) else {
            return Ok(());
        };
        for request in requests {
            self.submit(request);
        }
        Ok(())
    }

    /// Signal-driven dynamic refresh (§6): reconcile dirty domains flagged
    /// by `ControlSignal` delivery (at most one fetch round), then project
    /// only the surfaces each changed domain owns. Windows->task slots on
    /// the panel only, Workspaces->pager on the panel only,
    /// Panels->panel, System->open status popups only, Applications->Start
    /// only when the app set changed. Hidden surfaces are never
    /// reprojected. StartModel replaces only on app generation change
    /// (content inequality). Steady state never polls.
    fn refresh_dynamic(&mut self, runtime: &mut SurfaceRuntime) {
        use flamewm_shell::runtime::{ChangedDomains, ProjectionKind};
        let applications_dirty = self.shell.dirty().applications;
        let applications_changed = if applications_dirty {
            match self.shell.refresh_applications(&self.control) {
                Ok(applications_changed) => applications_changed,
                Err(_) => false,
            }
        } else {
            false
        };
        let mut changed = match self.shell.reconcile_dirty(&self.control) {
            Ok(changed) => changed,
            Err(_) => ChangedDomains::default(),
        };
        changed.applications = applications_changed;
        // F10: display geometry change may arrive without a panels dirty
        // flag, so check generation/revision/geometry drift even when no
        // changed domain fired. Sync happens BEFORE next open/projection.
        let mut snapshot = self.shell.snapshot().clone();
        let resynced = self.resync_panel_geometry(runtime, &snapshot);
        if resynced {
            snapshot = self.shell.snapshot().clone();
        }
        if !changed.any() && !resynced {
            return;
        }
        if applications_changed {
            self.start_model
                .replace_applications(self.shell.snapshot().applications.clone());
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
        let mut snapshot = self.shell.snapshot().clone();
        let start_model = self.start_model.clone();
        let category = self.start_category;
        let query = self.start_query.clone();
        let session = snapshot.session.clone();
        self.ensure_loader();
        // Windows or panels or workspaces own the visible panel document.
        // No broad refresh of hidden surfaces: only the panel reprojects
        // here; open popups reproject on their own open path.
        if changed.windows || changed.workspaces || (changed.panels && !resynced) {
            // C04: workspace/output/panel geometry changes invalidate the
            // helper anchor; workspaces also imply a pager change.
            // F10: when the pre-projection resync above did not fire but
            // panels changed, resync now (sync BEFORE projection,
            // move_resize panel, reproject) and skip the duplicate
            // project below when the resync already reprojected.
            if changed.panels && self.resync_panel_geometry(runtime, &snapshot) {
                if changed.workspaces || changed.panels {
                    self.close_quick_control_nonfatal();
                }
            } else {
                let loader = self.icon_loader.as_mut();
                if runtime
                    .with_document(self.surfaces.panel, |document| {
                        async_projection::project_panel_for_changed(
                            document, &snapshot, loader, changed,
                        )
                    })
                    .is_ok()
                {
                    flamewm_shell::runtime::projection_counter(ProjectionKind::Partial).increment();
                    let _ = runtime.redraw(self.surfaces.panel);
                }
            }
        }
        // System owns only the currently open in-process status popup
        // (media). Helper-owned popups (audio/network/calendar) reproject
        // on their own open path inside the helper process; the parent
        // never reprojects hidden or helper-owned surfaces.
        if changed.system {
            let open = self.open_popup_name();
            let projected = match open {
                Some("media") => runtime.with_document(self.surfaces.media, |document| {
                    projection::project_media(document, &snapshot)
                }),
                _ => Ok(()),
            };
            if projected.is_ok() && open.is_some() {
                flamewm_shell::runtime::projection_counter(ProjectionKind::Popup).increment();
                let surface = match open {
                    Some("media") => Some(self.surfaces.media),
                    _ => None,
                };
                if let Some(surface) = surface {
                    let _ = runtime.redraw(surface);
                }
            }
        }
        // Applications own the unified Start document, only when the app
        // set changed and Start is open; hidden Start stays untouched.
        if changed.applications && self.start_open {
            let loader = self.icon_loader.as_mut();
            let projected = runtime.with_document(self.surfaces.start, |document| {
                async_projection::project_start_unified(
                    document,
                    &start_model,
                    category,
                    &query,
                    &session,
                    loader,
                )
            });
            if projected.is_ok() {
                flamewm_shell::runtime::projection_counter(ProjectionKind::Start).increment();
                let _ = runtime.redraw(self.surfaces.start);
            }
        }
    }

    /// F10: panel/display geometry resync. Close popup/quick helper,
    /// sync the placement context BEFORE the next open/projection,
    /// move_resize the panel native surface when geometry changed,
    /// then reproject the panel. Returns true when a resync happened.
    fn resync_panel_geometry(
        &mut self,
        runtime: &mut SurfaceRuntime,
        snapshot: &ShellSnapshot,
    ) -> bool {
        let (display_generation, panels_revision) = self.surfaces.placement_generation();
        let geometry_changed = snapshot.displays.generation != display_generation
            || snapshot.panels.revision != panels_revision
            || snapshot
                .panels
                .panels
                .first()
                .map(|panel| panel.geometry != self.surfaces.panel_geometry())
                .unwrap_or(false);
        if !geometry_changed {
            return false;
        }
        // Close popup/quick helper first so no open surface keeps a
        // stale anchor while the context swaps underneath it.
        let _ = self.surfaces.close_transients(runtime);
        self.close_quick_control_nonfatal();
        self.start_open = false;
        self.context_menu = None;
        // Sync BEFORE next open/projection; refusal keeps old context.
        if !self.surfaces.sync_panel_context(snapshot) {
            eprintln!("flamewm-shell: panel context sync refused (missing output/panel)");
            return false;
        }
        if let Err(error) = self.surfaces.move_panel_to_synced(runtime) {
            eprintln!("flamewm-shell: panel move_resize failed (nonfatal): {error}");
        }
        let loader = self.icon_loader.as_mut();
        let changed = flamewm_shell::runtime::ChangedDomains {
            panels: true,
            ..Default::default()
        };
        if runtime
            .with_document(self.surfaces.panel, |document| {
                async_projection::project_panel_for_changed(document, snapshot, loader, changed)
            })
            .is_ok()
        {
            flamewm_shell::runtime::projection_counter(
                flamewm_shell::runtime::ProjectionKind::Partial,
            )
            .increment();
            let _ = runtime.redraw(self.surfaces.panel);
        }
        flamewm_debug::emit(
            flamewm_debug::DebugEventId("shell.panel.resync"),
            Duration::from_millis(500),
            || {
                let (generation, revision) = self.surfaces.placement_generation();
                format!(
                    "F10 resync generation={generation} revision={revision} edge={:?}",
                    self.surfaces.panel_edge()
                )
            },
        );
        true
    }

    /// F10: edge/geometry gate shared by every popup open path.
    /// Refuses with a diagnostic when edge != geometry (e.g. Bottom
    /// without panel.bottom == output.bottom).
    fn refuse_popup_on_inconsistent_geometry(&self) -> Option<String> {
        match self.surfaces.check_edge_consistent() {
            Ok(()) => None,
            Err(diagnostic) => {
                debug_assert!(false, "{diagnostic}");
                eprintln!("flamewm-shell: popup open refused: {diagnostic}");
                Some(diagnostic)
            }
        }
    }

    /// F10: read the current context generation on every Start/Quick
    /// open so the open provably uses the post-sync context.
    fn note_popup_generation(&self, what: &'static str) {
        let (generation, revision) = self.surfaces.placement_generation();
        flamewm_debug::emit(
            flamewm_debug::DebugEventId("shell.popup.open.generation"),
            Duration::from_millis(500),
            || format!("F10 {what} generation={generation} revision={revision}"),
        );
    }

    /// Name of the currently open in-process status popup, if any.
    /// Helper-owned popups are never in-process, so only media qualifies.
    fn open_popup_name(&self) -> Option<&'static str> {
        if self.surfaces.is_media_open() {
            return Some("media");
        }
        None
    }

    fn tick(
        &mut self,
        runtime: &mut SurfaceRuntime,
        clock_due: &AtomicBool,
    ) -> Result<(), UiBackendError> {
        self.drain_icons(runtime);
        // J07: nonblocking helper poll; restart is bounded, failures are
        // nonfatal diagnostics, the shell loop always survives.
        flamewm_shell::quick_controls::host::poll_supervisor_nonfatal(&mut self.quick_control);
        let clock_fired = clock_due.swap(false, Ordering::Acquire);
        if clock_fired {
            let now = Local::now();
            runtime.with_document(self.surfaces.panel, |document| {
                projection::project_clock_at(document, now)
            })?;
            runtime.redraw(self.surfaces.panel)?;

            let date = (now.year(), now.month() as u8, now.day() as u8);
            if self.date_tracker.date_changed(date) {
                self.local_date = date;
            }
        }
        // Steady state never polls: reconcile signal-flagged domains at most
        // once per tick, after the clock projection above.
        self.refresh_dynamic(runtime);
        Ok(())
    }
}

fn epoch_ms() -> u64 {
    Local::now().timestamp_millis().max(0) as u64
}

fn signal_interest(events: FdEvents) -> calloop::Interest {
    match (
        events.contains(FdEvents::READABLE),
        events.contains(FdEvents::WRITABLE),
    ) {
        (true, true) => calloop::Interest::BOTH,
        (false, true) => calloop::Interest::WRITE,
        _ => calloop::Interest::READ,
    }
}

fn profile_interval_secs() -> u64 {
    flamewm_profiler::profile_interval_secs()
}

/// Popup measurement refusal: layout pending or anchor pending. The caller
/// swallows these (state untouched, nothing mapped) instead of mapping at
/// 0,0 or a fixed size. Pointer-grab contention (e.g. the WM holding a grab
/// or a synthetic-click race) is the same class of transient refusal: the
/// popup open is retried on the next toggle, and must never abort the event
/// loop (fatal `Renderer` -> process exit). Refusals arrive as the typed
/// `PopupRefusal` display strings mapped at the grab call site; no renderer
/// symbol matching here.
fn is_popup_refusal(error: &str) -> bool {
    error == PopupRefusal::PendingLayout.to_string()
        || error == PopupRefusal::PendingAnchor.to_string()
        || error == PopupRefusal::PointerGrabRefused.to_string()
}

fn packaged_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("flamewm workspace root")
}
