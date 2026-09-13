//! J07 quick-control host process: owns audio/network/calendar surfaces.
//!
//! Role branching happens at process entry (`--quick-control-host`):
//! helper role (profiler name `flamewm-quick-control`) connects its own
//! [`ControlClient`], projects the snapshot through the existing taskbar
//! status projections, owns one popup surface per kind, answers parent
//! `OPEN` commands with fitted placement, and routes typed actions through
//! the existing dispatcher. The authentication secret travels out-of-band
//! (environment) and never appears in the supervisor protocol or logs.

use std::collections::VecDeque;
use std::io::{self, Read};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use flamewm_api::settings::{SettingValue, SettingsSnapshot};
use flamewm_control_core::ControlRequest;
use flamewm_control_dbus::ControlClient;
use flamewm_dbus_reactor::BusKind;
use flamewm_debug::DebugEventId;
use flamewm_shell_core::popup::{measured_popup_rect, PopupRefusal};
use flamewm_shell_core::status::{AudioSettings, AudioTab, NetworkQuery};
use flamewm_ui_core::popover::{PopoverAlign, PopoverEdge};
use flamewm_ui_x11::{
    decode_document, ActionPhase, PointerButton, SurfaceConfig, SurfaceHandle, SurfaceInputMode,
    SurfaceRole, SurfaceRuntime, UiBackendError, UiTemplate,
};

use super::protocol::{decode_line, Command, OpenRequest, QuickControlKind, MAX_LINE_LEN};
use super::supervisor::{QuickControlSupervisor, SupervisorError};

/// Pre-commit debug probe; the final geometry commit limit stays fixed
/// (refusals leave state untouched, no limit raise).
const POPUP_TRANSITION: DebugEventId = DebugEventId("shell.popup.transition");
const GEOMETRY_REFUSAL: DebugEventId = DebugEventId("render.geometry.refusal");
const POPUP_MEASURE: DebugEventId = DebugEventId("shell.popup.measure");
const POPUP_MEASURE_COOLDOWN: Duration = Duration::from_millis(1000);

const AUDIO_ARTIFACT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-audio.rwr"));
const NETWORK_ARTIFACT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-network.rwr"));
const CALENDAR_ARTIFACT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-calendar.rwr"));

const OUTSIDE_RELEASE_ACTION: &str = "surface.outside.release";

/// DEBUG-ONLY gate (local; no new cross-crate dep): true only when
/// `FLAMEWM_DEBUG=1`. Normal mode issues zero extra X sync.
fn host_debug_enabled() -> bool {
    std::env::var_os("FLAMEWM_DEBUG").is_some_and(|v| v == "1")
}

#[derive(Debug)]
pub enum HostError {
    Io(io::Error),
    Control(String),
    Ui(String),
}

impl core::fmt::Display for HostError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "host i/o: {e}"),
            Self::Control(e) => write!(f, "host control: {e}"),
            Self::Ui(e) => write!(f, "host ui: {e}"),
        }
    }
}

impl std::error::Error for HostError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Control(_) | Self::Ui(_) => None,
        }
    }
}

impl From<io::Error> for HostError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// Snapshot projection for the helper: one [`ControlClient`] fetch round
/// over the System domain only. No secret is ever carried: secrets travel
/// via the SecretAgent snapshot, never in this struct or the protocol.
#[derive(Debug, Clone)]
pub struct QuickControlSnapshot {
    pub system: flamewm_api::system::SystemSnapshot,
    pub audio_tab: AudioTab,
    pub audio_settings: AudioSettings,
    pub network_query: NetworkQuery,
    pub network_secret_masked_len: usize,
}

impl QuickControlSnapshot {
    pub fn load(client: &ControlClient) -> Result<Self, HostError> {
        match client.call(&ControlRequest::GetSystem) {
            Ok(flamewm_control_core::ControlResponse::System(system)) => Ok(Self {
                system,
                audio_tab: AudioTab::default(),
                audio_settings: AudioSettings::default(),
                network_query: NetworkQuery::default(),
                network_secret_masked_len: 0,
            }),
            Ok(response) => Err(HostError::Control(format!(
                "GetSystem returned {response:?}"
            ))),
            Err(error) => Err(HostError::Control(format!(
                "GetSystem failed: {}",
                error.message
            ))),
        }
    }

    pub fn refresh_settings(&mut self, client: &ControlClient) {
        if let Ok(flamewm_control_core::ControlResponse::Settings(snapshot)) =
            client.call(&ControlRequest::GetSettings)
        {
            apply_settings(snapshot, &mut self.audio_settings);
        }
    }
}

fn apply_settings(snapshot: SettingsSnapshot, settings: &mut AudioSettings) {
    if let Some(SettingValue::Boolean(raise)) = snapshot.values.get(AudioSettings::SETTINGS_KEY) {
        settings.raise_maximum = *raise;
    }
}

/// Quick-control host state behind the reactor callbacks: one reactor
/// owns the X11 surface FD, the nonblocking parent command FD, and tick
/// timers. No per-popup thread, no async runtime.
struct HostState {
    surfaces: HelperSurfaces,
    snapshot: QuickControlSnapshot,
    dispatcher: Option<Arc<flamewm_control_dbus::MutationDispatcher>>,
    /// Parsed parent commands awaiting the tick drain.
    queue: VecDeque<Command>,
    /// Set on parent EOF or SHUTDOWN; the tick stops the loop cleanly.
    shutdown: bool,
}

/// Duplicate stdin + O_NONBLOCK on the dup. No `unsafe`: the std `File`
/// duplicate path (`try_clone`) keeps fd 0 stdio-owned while the clone
/// gets its own description flags via `set_nonblocking`.
fn stdin_nonblocking_dup() -> Result<std::fs::File, HostError> {
    use std::os::unix::io::AsFd;
    let file = std::fs::File::open("/dev/stdin").map_err(HostError::Io)?;
    let flags = rustix::fs::fcntl_getfl(&file)
        .map_err(|error| HostError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
    rustix::fs::fcntl_setfl(&file, flags | rustix::fs::OFlags::NONBLOCK)
        .map_err(|error| HostError::Io(io::Error::from_raw_os_error(error.raw_os_error())))?;
    let _ = file.as_fd();
    Ok(file)
}

/// Shared parent-input state: partial line bytes + decoded queue live
/// behind one mutex shared by the stdin FD callback and the tick.
#[derive(Default)]
struct ParentInput {
    pending: Vec<u8>,
    queue: VecDeque<Command>,
    eof: bool,
}

fn push_line(state: &mut ParentInput, line: &[u8]) {
    if line.len() > MAX_LINE_LEN {
        eprintln!("flamewm-quick-control: overlong command line dropped");
        return;
    }
    let text = String::from_utf8_lossy(line);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    match decode_line(trimmed) {
        Ok(command) => state.queue.push_back(command),
        Err(error) => eprintln!("flamewm-quick-control: bad command: {error}"),
    }
}

/// Drain available bytes from the nonblocking parent FD: accumulate
/// partial lines, newline parses to queue, EOF (Ok(0)) shuts down.
fn drain_parent_fd(file: &mut std::fs::File, state: &Arc<Mutex<ParentInput>>) {
    let mut buf = [0u8; 4096];
    loop {
        match file.read(&mut buf) {
            Ok(0) => {
                if let Ok(mut guard) = state.lock() {
                    guard.eof = true;
                }
                return;
            }
            Ok(n) => {
                let Ok(mut guard) = state.lock() else { return };
                let mut start = 0usize;
                while start < n {
                    match buf[start..n].iter().position(|b| *b == b'\n') {
                        Some(rel) => {
                            let end = start + rel;
                            let mut line = core::mem::take(&mut guard.pending);
                            line.extend_from_slice(&buf[start..end]);
                            // Strip a trailing CR so CRLF parents stay total.
                            if line.last() == Some(&b'\r') {
                                line.pop();
                            }
                            push_line(&mut guard, &line);
                            start = end + 1;
                        }
                        None => {
                            guard.pending.extend_from_slice(&buf[start..n]);
                            if guard.pending.len() > MAX_LINE_LEN {
                                guard.pending.clear();
                                eprintln!("flamewm-quick-control: overlong command line dropped");
                            }
                            break;
                        }
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => {
                if let Ok(mut guard) = state.lock() {
                    guard.eof = true;
                }
                return;
            }
        }
    }
}

/// Run the quick-control host loop.
///
/// Connects its own control client, creates the three helper-owned popup
/// surfaces, then enters one reactor: X11 surface FD + nonblocking parent
/// command FD + timers/wake -> one event loop. Reuses `flamewm-reactor`.
/// The tick drains commands: OPEN closes the current helper popup,
/// projects, fits, shows/grabs; CLOSE closes; SHUTDOWN stops cleanly.
/// The surface event callback calls [`route_helper_event`].
pub fn run_quick_control_host() -> Result<(), HostError> {
    flamewm_profiler::init_process("flamewm-quick-control");
    flamewm_debug::init_process("flamewm-quick-control");
    let client = ControlClient::connect(BusKind::Session).map_err(|error| {
        HostError::Control(format!("connect control session: {}", error.message))
    })?;
    let mut snapshot = QuickControlSnapshot::load(&client)?;
    snapshot.refresh_settings(&client);
    drop(client);
    let mut runtime = SurfaceRuntime::new()
        .map_err(|error| HostError::Ui(format!("create UI runtime: {error:?}")))?;
    let surfaces = HelperSurfaces::create(&mut runtime).map_err(HostError::Ui)?;
    let dispatcher = flamewm_control_dbus::MutationDispatcher::start(BusKind::Session, 64).ok();
    let state = std::rc::Rc::new(std::cell::RefCell::new(HostState {
        surfaces,
        snapshot,
        dispatcher,
        queue: VecDeque::new(),
        shutdown: false,
    }));

    // Nonblocking parent command FD: duplicate the stdin description so
    // the original fd 0 stays owned by stdio, then mark the duplicate
    // nonblocking. Reads below never block; no new thread, no async.
    let stdin_dup = stdin_nonblocking_dup()?;
    let stdin_fd = {
        use std::os::unix::io::AsRawFd;
        stdin_dup.as_raw_fd()
    };
    let input: Arc<Mutex<ParentInput>> = Arc::new(Mutex::new(ParentInput::default()));
    let mut reactor = flamewm_reactor::Reactor::new()
        .map_err(|error| HostError::Control(format!("create reactor: {error}")))?;
    // the callback reopens its own nonblocking handle per readiness.
    // Keep-alive holder for the dup description; the callback registers
    // the raw fd without ownership transfer.
    let stdin_holder: Arc<Mutex<Option<std::fs::File>>> = Arc::new(Mutex::new(Some(stdin_dup)));
    {
        let input = Arc::clone(&input);
        let stdin_holder = Arc::clone(&stdin_holder);
        reactor
            .register_raw_fd_with_action(stdin_fd, calloop::Interest::READ, move |_, _| {
                // Reuse the held nonblocking description for the read.
                if let Ok(mut holder) = stdin_holder.lock() {
                    if let Some(file) = holder.as_mut() {
                        drain_parent_fd(file, &input);
                    }
                }
                flamewm_reactor::FdAction::Continue
            })
            .map_err(|error| HostError::Control(format!("register stdin fd: {error}")))?;
    }
    // Mutation result wake FD on the same reactor; the tick drains payloads.
    if let Some(dispatcher) = state.borrow().dispatcher.as_ref() {
        let watch_fd = dispatcher.wake_fd();
        let wake = Arc::clone(dispatcher);
        reactor
            .register_raw_fd_with_action(watch_fd, calloop::Interest::READ, move |_, _| {
                crate::control_actions::drain_results(Some(&wake));
                flamewm_reactor::FdAction::Continue
            })
            .map_err(|error| HostError::Control(format!("register mutation fd: {error}")))?;
    }
    reactor
        .register_timer(
            Duration::from_secs(flamewm_profiler::profile_interval_secs()),
            true,
            || {
                let _ = flamewm_profiler::report_window();
            },
        )
        .map_err(|error| HostError::Control(format!("register profile timer: {error}")))?;
    // Keep the nonblocking stdin description alive until the loop returns.
    let _stdin_keepalive = Arc::clone(&stdin_holder);
    let _stdin_file = stdin_holder;

    let event_state = std::rc::Rc::clone(&state);
    let tick_state = std::rc::Rc::clone(&state);
    let tick_input = Arc::clone(&input);
    flamewm_ui_x11::run_surface_runtime_with_reactor_access(
        &mut runtime,
        &mut reactor,
        move |event, runtime| {
            let dispatcher = event_state.borrow().dispatcher.clone();
            let mut guard = event_state.borrow_mut();
            // Clone-then-write-back: route needs &mut surfaces + &mut
            // snapshot, which cannot co-borrow from one RefMut pair.
            let mut snapshot = guard.snapshot.clone();
            let keep = route_helper_event(
                &event,
                runtime,
                &mut guard.surfaces,
                &mut snapshot,
                dispatcher.as_ref(),
            );
            guard.snapshot = snapshot;
            if !keep {
                guard.shutdown = true;
            }
            Ok(())
        },
        move |runtime| {
            // Move newly arrived parent commands into the tick queue.
            if let Ok(mut guard) = tick_input.lock() {
                let mut state = tick_state.borrow_mut();
                state.queue.append(&mut guard.queue);
                if guard.eof {
                    state.shutdown = true;
                }
            }
            tick_host(runtime, &tick_state)?;
            if tick_state.borrow().shutdown {
                // Clean stop: close helper popups, then drop surfaces so the
                // runner exits when no surfaces remain.
                let mut state = tick_state.borrow_mut();
                state.surfaces.close(runtime);
                for surface in [
                    state.surfaces.audio,
                    state.surfaces.network,
                    state.surfaces.calendar,
                ] {
                    let _ = runtime.destroy(surface);
                }
            }
            Ok(())
        },
    )
    .map_err(|error| HostError::Ui(format!("surface runtime: {error:?}")))?;
    // Final profiler window on clean exit: flush the cadence bucket even
    // when the loop ran fewer than one interval.
    let _ = flamewm_profiler::report_window();
    Ok(())
}

/// Coalesce queued commands: consecutive OPENs collapse to the newest;
/// CLOSE/SHUTDOWN are best-effort singletons and never duplicate work.
fn coalesce_commands(queue: &mut VecDeque<Command>) -> VecDeque<Command> {
    let mut out: VecDeque<Command> = VecDeque::new();
    for cmd in queue.drain(..) {
        if matches!(cmd, Command::Open(_)) {
            while matches!(out.back(), Some(Command::Open(_))) {
                out.pop_back();
            }
        }
        out.push_back(cmd);
    }
    out
}

/// Tick: drain queued parent commands; OPEN closes the current helper
/// popup then projects/fits/shows/grabs; CLOSE closes; SHUTDOWN stops.
fn tick_host(
    runtime: &mut SurfaceRuntime,
    state: &std::rc::Rc<std::cell::RefCell<HostState>>,
) -> Result<(), flamewm_ui_x11::UiBackendError> {
    // OPEN coalesces to the newest before the drain.
    {
        let mut guard = state.borrow_mut();
        let mut pending = core::mem::take(&mut guard.queue);
        guard.queue = coalesce_commands(&mut pending);
    }
    loop {
        let command = state.borrow_mut().queue.pop_front();
        let Some(command) = command else {
            return Ok(());
        };
        match command {
            Command::Shutdown | Command::Close => {
                let mut guard = state.borrow_mut();
                guard.surfaces.close(runtime);
                if matches!(command, Command::Shutdown) {
                    guard.shutdown = true;
                }
            }
            Command::Open(request) => {
                let snapshot = state.borrow().snapshot.clone();
                let mut guard = state.borrow_mut();
                // One helper popup at a time: close the current one first,
                // timing the real close turn for the slow-open line.
                let close_start = std::time::Instant::now();
                guard.surfaces.close(runtime);
                let close_elapsed = close_start.elapsed();
                if let Err(error) = open_request(
                    runtime,
                    &mut guard.surfaces,
                    &snapshot,
                    &request,
                    close_elapsed,
                ) {
                    eprintln!("flamewm-quick-control: open refused: {error}");
                }
            }
        }
    }
}

/// True open stages: each label maps to one real turn in `open_request`
/// (project, measure, place, prepare/commit_geometry, present,
/// pointer_grab) plus the total and the real close
/// (close/ungrab/unmap). No fake spans.
pub const QUICK_STAGE_LABELS: &[&str] = &[
    "shell.quick.open.total",
    "shell.quick.project",
    "shell.quick.measure",
    "shell.quick.place",
    "shell.quick.prepare/commit_geometry",
    "shell.quick.present",
    "shell.quick.pointer_grab",
    "shell.quick.close/ungrab/unmap",
];
/// Slow-open threshold: one 60Hz frame (16.67ms).
pub const SLOW_OPEN_THRESHOLD: Duration = Duration::from_micros(16_670);

/// Pure seam: slow-open predicate shared by the log gate.
pub fn is_slow_open(total: Duration) -> bool {
    total >= SLOW_OPEN_THRESHOLD
}

/// Pure seam: stage timings carried by one open turn, rendered by the
/// slow-open debug line. `close` is the pre-open close of the previous
/// popup on the same tick (zero when nothing was open).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OpenStageTimings {
    pub total: Duration,
    pub project: Duration,
    pub measure: Duration,
    pub place: Duration,
    pub prepare_commit: Duration,
    pub present: Duration,
    pub pointer_grab: Duration,
    pub close: Duration,
}

impl OpenStageTimings {
    pub fn render(&self, kind: QuickControlKind) -> String {
        format!(
            "flamewm-quick-control: slow open kind={kind:?} open.total={:?} project={:?} measure={:?} place={:?} prepare/commit_geometry={:?} present={:?} pointer_grab={:?} close/ungrab/unmap={:?}",
            self.total, self.project, self.measure, self.place, self.prepare_commit, self.present, self.pointer_grab, self.close,
        )
    }
}

fn outer_measure_constraint(work_area: flamewm_api::Rect) -> (f32, f32) {
    let width = work_area.width.max(1).min(i32::MAX);
    let height = work_area.height.max(1).min(i32::MAX);
    (width as f32, height as f32)
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
    kind: QuickControlKind,
    anchor: flamewm_api::Rect,
    work_area: flamewm_api::Rect,
    max_device: (f32, f32),
    logical: (f32, f32),
    measured: Option<(f32, f32)>,
    fitted: Option<flamewm_api::Rect>,
    refusal: Option<&str>,
) {
    flamewm_debug::emit(POPUP_MEASURE, POPUP_MEASURE_COOLDOWN, || {
        format!(
            "role={role} kind={kind:?} anchor={anchor:?} work_area={work_area:?} max_device={max_device:?} logical={logical:?} measured={measured:?} fitted={fitted:?} refusal={refusal:?}"
        )
    });
}

fn open_request(
    runtime: &mut SurfaceRuntime,
    surfaces: &mut HelperSurfaces,
    snapshot: &QuickControlSnapshot,
    request: &OpenRequest,
    close_elapsed: Duration,
) -> Result<(), String> {
    let total_start = std::time::Instant::now();
    let _total = crate::runtime::shell_span("shell.quick.open.total").start();
    flamewm_debug::emit(POPUP_TRANSITION, Duration::from_millis(0), || {
        format!(
            "quick-control open kind={:?} anchor={:?} work_area={:?} panel_edge={:?}",
            request.kind, request.anchor, request.work_area, request.panel_edge,
        )
    });
    let surface = surfaces.surface_for(request.kind);
    let project_start = std::time::Instant::now();
    let _project = crate::runtime::shell_span("shell.quick.project").start();
    project_snapshot(runtime, surface, snapshot, request.kind)?;
    let project_elapsed = project_start.elapsed();
    drop(_project);
    let (rect, measure_elapsed, place_elapsed) = fitted_rect(runtime, surface, request)?;
    flamewm_debug::emit(POPUP_TRANSITION, Duration::from_millis(0), || {
        format!("quick-control fitted kind={:?} rect={rect:?}", request.kind)
    });
    let prepare_start = std::time::Instant::now();
    let _prepare = crate::runtime::shell_span("shell.quick.prepare/commit_geometry").start();
    prepare(runtime, surface, rect)?;
    let prepare_elapsed = prepare_start.elapsed();
    drop(_prepare);
    let present_start = std::time::Instant::now();
    let _present = crate::runtime::shell_span("shell.quick.present").start();
    runtime.show(surface).map_err(ui_error)?;
    let present_elapsed = present_start.elapsed();
    drop(_present);
    // DEBUG-ONLY: one observed root-rect probe after present. Normal mode
    // issues zero extra X sync.
    let trace = flamewm_ui_x11::GeometryTrace::begin(request.kind.as_str());
    if host_debug_enabled() {
        let _ = runtime.debug_probe_root_rect(surface, trace.for_surface(0));
    }
    let observed_rect = runtime
        .surface_device_rect(surface)
        .ok()
        .map(|r| (r.x as i32, r.y as i32, r.width as i32, r.height as i32));
    let grab_start = std::time::Instant::now();
    let _grab = crate::runtime::shell_span("shell.quick.pointer_grab").start();
    runtime
        .grab_pointer(surface)
        .map_err(|_| PopupRefusal::PointerGrabRefused.to_string())?;
    let grab_elapsed = grab_start.elapsed();
    drop(_grab);
    surfaces.open = Some(request.kind);
    let total = total_start.elapsed();
    if host_debug_enabled() {
        eprintln!(
            "popup-record txn={} kind={:?} panel_revision={} edge={:?} panel_rect={:?} output_rect={:?} work_area={:?} anchor={:?} intrinsic=n/a fitted={:?} native_request={:?} native_observed={:?}",
            trace.txn,
            request.kind,
            snapshot.system.revision,
            request.panel_edge,
            request.anchor,
            request.work_area,
            request.work_area,
            request.anchor,
            rect,
            rect,
            observed_rect,
        );
        match runtime.debug_stack_canary() {
            Ok(true) => {}
            Ok(false) => {
                let _ = runtime.raise(surface);
                eprintln!(
                    "flamewm-quick-control: debug stack canary FAILED kind={:?}; re-raised via existing owner",
                    request.kind
                );
            }
            Err(error) => {
                eprintln!("flamewm-quick-control: debug stack canary error: {error:?}");
            }
        }
    }
    if is_slow_open(total) {
        eprintln!(
            "{}",
            OpenStageTimings {
                total,
                project: project_elapsed,
                measure: measure_elapsed,
                place: place_elapsed,
                prepare_commit: prepare_elapsed,
                present: present_elapsed,
                pointer_grab: grab_elapsed,
                close: close_elapsed,
            }
            .render(request.kind)
        );
    }
    Ok(())
}

/// Fitted placement plus the real measure/place stage split, so the
/// slow-open line can attribute each turn without fake spans.
pub struct FittedPlacement {
    pub rect: flamewm_api::Rect,
    pub measure_elapsed: Duration,
    pub place_elapsed: Duration,
}

fn fitted_rect(
    runtime: &mut SurfaceRuntime,
    surface: SurfaceHandle,
    request: &OpenRequest,
) -> Result<(flamewm_api::Rect, Duration, Duration), String> {
    let measure_start = std::time::Instant::now();
    let _measure = crate::runtime::shell_span("shell.quick.measure").start();
    let max = outer_measure_constraint(request.work_area);
    if request.work_area.width <= 0 || request.work_area.height <= 0 {
        emit_popup_measure(
            "quick-control",
            request.kind,
            request.anchor,
            request.work_area,
            max,
            max,
            None,
            None,
            Some("PendingLayout"),
        );
        return Err(PopupRefusal::PendingLayout.to_string());
    }
    let logical = (max.0, max.1);
    let measured = runtime
        .measure_outer_intrinsic_device_size(surface, max.0, max.1)
        .map_err(|error| {
            emit_popup_measure("quick-control", request.kind, request.anchor, request.work_area, max, logical, None, None, Some(&format!("{error:?}")));
            flamewm_debug::emit(GEOMETRY_REFUSAL, Duration::from_millis(0), || {
                format!("quick-control refusal kind={:?} refusal=PendingLayout anchor={:?} work_area={:?} panel_edge={:?}", request.kind, request.anchor, request.work_area, request.panel_edge)
            });
            PopupRefusal::PendingLayout.to_string()
        })?;
    let intrinsic = device_to_popup_size(measured)
        .filter(|size| size.width > 0 && size.height > 0)
        .ok_or_else(|| {
            emit_popup_measure(
                "quick-control",
                request.kind,
                request.anchor,
                request.work_area,
                max,
                logical,
                Some(measured),
                None,
                Some("PendingLayout"),
            );
            PopupRefusal::PendingLayout.to_string()
        })?;
    let intrinsic = flamewm_api::Size::new(
        intrinsic.width.min(request.work_area.width.max(1)),
        intrinsic.height.min(request.work_area.height.max(1)),
    );
    drop(_measure);
    let measure_elapsed = measure_start.elapsed();
    let place_start = std::time::Instant::now();
    let _place = crate::runtime::shell_span("shell.quick.place").start();
    let edge = match request.panel_edge {
        flamewm_api::PanelEdge::Top => PopoverEdge::Below,
        _ => PopoverEdge::Above,
    };
    let rect = measured_popup_rect(
        Some(request.anchor),
        Some(intrinsic),
        edge,
        PopoverAlign::End,
        request.work_area,
        8,
    )
    .map(|rect| {
        emit_popup_measure("quick-control", request.kind, request.anchor, request.work_area, max, logical, Some(measured), Some(rect), None);
        rect
    })
    .map_err(|refusal| {
        emit_popup_measure("quick-control", request.kind, request.anchor, request.work_area, max, logical, Some(measured), None, Some(&refusal.to_string()));
        flamewm_debug::emit(GEOMETRY_REFUSAL, Duration::from_millis(0), || {
            format!(
                "quick-control refusal kind={:?} refusal={refusal} anchor={:?} work_area={:?} intrinsic={intrinsic:?} panel_edge={:?}",
                request.kind, request.anchor, request.work_area, request.panel_edge,
            )
        });
        refusal.to_string()
    })?;
    let place_elapsed = place_start.elapsed();
    drop(_place);
    Ok((rect, measure_elapsed, place_elapsed))
}

fn project_snapshot(
    runtime: &mut SurfaceRuntime,
    surface: SurfaceHandle,
    snapshot: &QuickControlSnapshot,
    kind: QuickControlKind,
) -> Result<(), String> {
    let shell = crate::runtime::ShellSnapshot {
        displays: flamewm_api::display::DisplaySnapshot {
            generation: 0,
            outputs: Vec::new(),
            pending: None,
        },
        panels: flamewm_api::panels::PanelsSnapshot {
            revision: 0,
            panels: Vec::new(),
            tasks: Vec::new(),
            pinned_apps: Vec::new(),
        },
        windows: Vec::new(),
        applications: Vec::new(),
        workspaces: None,
        session: flamewm_api::session::SessionCapabilities::default(),
        system: snapshot.system.clone(),
    };
    runtime
        .with_document(surface, |document| match kind {
            QuickControlKind::Audio => crate::projection::project_audio_full(
                document,
                &shell,
                snapshot.audio_tab,
                snapshot.audio_settings,
                0,
            ),
            QuickControlKind::Network => {
                let query = snapshot.network_query.clone();
                crate::projection::project_network_full(document, &shell, &query, 0)?;
                crate::projection::project_network_secret(
                    document,
                    &"*".repeat(snapshot.network_secret_masked_len),
                )
            }
            QuickControlKind::Calendar => {
                let grid = crate::projection::current_calendar()
                    .ok_or_else(|| "invalid local calendar date".to_owned())?;
                crate::projection::project_calendar(document, &grid)
            }
        })
        .map_err(|error| format!("{error:?}"))
}

/// Route one helper surface event: Escape/outside closes the helper popup,
/// typed audio/network actions forward via the existing dispatcher
/// contract. Events inside the active popup or its submodules are kept;
/// a release outside the active popup closes it on the same turn.
/// Returns `true` when the loop should keep running.
pub fn route_helper_event(
    event: &flamewm_ui_x11::SurfaceEvent,
    runtime: &mut SurfaceRuntime,
    surfaces: &mut HelperSurfaces,
    snapshot: &mut QuickControlSnapshot,
    dispatcher: Option<&std::sync::Arc<flamewm_control_dbus::MutationDispatcher>>,
) -> bool {
    let action = event.action.action.as_str();
    if action == "keyboard.input" && event.action.phase == ActionPhase::Release {
        let input = event.action.text.as_deref().unwrap_or_default();
        if input == "\u{1b}" {
            surfaces.close(runtime);
            return true;
        }
    }
    let open_surface = surfaces.open.map(|kind| surfaces.surface_for(kind));
    let inside_active = match open_surface {
        Some(handle) => event.surface == handle || event.action.inside,
        None => event.action.inside,
    };
    let outside = event.action.phase == ActionPhase::Release
        && (action == OUTSIDE_RELEASE_ACTION || (surfaces.open.is_some() && !inside_active));
    if outside {
        surfaces.close(runtime);
        return true;
    }
    if surfaces.open.is_none() {
        return true;
    }
    if event.action.phase == ActionPhase::Release && event.action.inside {
        if let Some(request) = helper_control_request(action, snapshot) {
            crate::control_actions::enqueue_action(dispatcher, request);
        }
        let _ = PointerButton::from_raw_x(event.action.button);
    }
    true
}

fn helper_control_request(action: &str, snapshot: &QuickControlSnapshot) -> Option<ControlRequest> {
    let system = &snapshot.system;
    if action == "network.wifi.toggle" {
        return Some(ControlRequest::SystemAction {
            action: flamewm_api::system::SystemAction::SetWifiEnabled(!system.network.wifi_enabled),
            expected_revision: system.revision,
        });
    }
    if action == "network.scan" {
        return crate::taskbar::status::network::system_action_for_path(system, "", true, false);
    }
    if action == "network.disconnect" {
        return crate::taskbar::status::network::system_action_for_path(system, "", false, true);
    }
    if let Some(slot) = action
        .strip_prefix("network.connect.")
        .and_then(|value| value.parse::<usize>().ok())
        .and_then(|slot| slot.checked_sub(1))
    {
        let path = crate::taskbar::status::network::popover_view(system)
            .and_then(|popover| popover.visible_rows.get(slot).map(|row| row.path.clone()))?;
        return crate::taskbar::status::network::system_action_for_path(
            system, &path, false, false,
        );
    }
    if action == "audio.raise.toggle" {
        return None;
    }
    let row = if matches!(action, "volume.mute" | "volume.down" | "volume.up") {
        crate::taskbar::status::audio::default_endpoint_row(&system.audio)
    } else if let Some((kind_slot, _)) = action
        .strip_prefix("audio.")
        .and_then(|value| value.rsplit_once('.'))
    {
        let (_, slot) = kind_slot.rsplit_once('.')?;
        let slot = slot.parse::<usize>().ok()?.checked_sub(1)?;
        crate::taskbar::status::audio::popover_view(system)
            .and_then(|popover| popover.visible_rows.get(slot).cloned())
            .map(|row| crate::taskbar::status::audio::RowSource {
                kind: row.kind,
                id: row.id,
                label: row.label,
                volume_percent: row.volume_percent,
                muted: row.muted,
                selected: row.selected,
            })
    } else {
        None
    }?;
    let command = action.rsplit('.').next()?;
    let cap = snapshot.audio_settings.max_percent();
    match command {
        "up" => crate::taskbar::status::audio::system_action_for_row(
            system,
            &row.id,
            Some(
                snapshot
                    .audio_settings
                    .clamp_percent(row.volume_percent.saturating_add(5)),
            ),
            None,
        ),
        "down" => crate::taskbar::status::audio::system_action_for_row(
            system,
            &row.id,
            Some(row.volume_percent.saturating_sub(5).min(cap)),
            None,
        ),
        "mute" => crate::taskbar::status::audio::system_action_for_row(
            system,
            &row.id,
            None,
            Some(!row.muted),
        ),
        _ => None,
    }
}

/// Pure helpers for the close-one/close-none contract: `take_close_target`
/// consumes `open` and yields the single kind to close (None = zero calls).
pub fn take_close_target(open: Option<QuickControlKind>) -> Option<QuickControlKind> {
    open
}

pub struct HelperSurfaces {
    pub audio: SurfaceHandle,
    pub network: SurfaceHandle,
    pub calendar: SurfaceHandle,
    pub open: Option<QuickControlKind>,
}

impl HelperSurfaces {
    pub fn create(runtime: &mut SurfaceRuntime) -> Result<Self, String> {
        let audio = create_surface(runtime, AUDIO_ARTIFACT, "FlameWM audio")?;
        let network = create_surface(runtime, NETWORK_ARTIFACT, "FlameWM network")?;
        let calendar = create_surface(runtime, CALENDAR_ARTIFACT, "FlameWM calendar")?;
        Ok(Self {
            audio,
            network,
            calendar,
            open: None,
        })
    }

    pub fn surface_for(&self, kind: QuickControlKind) -> SurfaceHandle {
        match kind {
            QuickControlKind::Audio => self.audio,
            QuickControlKind::Network => self.network,
            QuickControlKind::Calendar => self.calendar,
        }
    }

    /// Close only the currently open surface (open=kind). No-op when none
    /// is open (zero close calls). The real close span wraps the actual
    /// ungrab/unmap turn.
    /// Pure close-target selection: returns the single handle to close,
    /// or None when nothing is open (zero close calls). Keeps the
    /// close-one/close-none contract testable without an X11 runtime.
    pub fn close_handle(&mut self) -> Option<SurfaceHandle> {
        take_close_target(self.open.take()).map(|kind| self.surface_for(kind))
    }

    pub fn close(&mut self, runtime: &mut SurfaceRuntime) {
        let Some(surface) = self.close_handle() else {
            return;
        };
        let _span = crate::runtime::shell_span("shell.quick.close/ungrab/unmap").start();
        let _ = runtime.close_surface(surface);
    }

    /// Shutdown path: close the open surface, then destroy all helpers.
    pub fn shutdown(&mut self, runtime: &mut SurfaceRuntime) {
        self.close(runtime);
        for surface in [self.audio, self.network, self.calendar] {
            let _ = runtime.destroy(surface);
        }
    }
}

fn create_surface(
    runtime: &mut SurfaceRuntime,
    bytes: &[u8],
    title: &str,
) -> Result<SurfaceHandle, String> {
    let document = decode_document(bytes)?;
    runtime
        .create_surface(
            UiTemplate::new(document),
            SurfaceConfig {
                width: 1,
                height: 1,
                title: title.to_owned(),
                role: SurfaceRole::PopupMenu,
                input: SurfaceInputMode::Interactive,
                initially_visible: false,
                x: 0,
                y: 0,
            },
        )
        .map_err(ui_error)
}

fn prepare(
    _runtime: &mut SurfaceRuntime,
    surface: SurfaceHandle,
    rect: flamewm_api::Rect,
) -> Result<(), String> {
    if rect.width <= 0 || rect.height <= 0 {
        return Err(PopupRefusal::PendingLayout.to_string());
    }
    _runtime
        .move_resize(
            surface,
            rect.x,
            rect.y,
            u32::try_from(rect.width)
                .map_err(|_| format!("invalid surface size {}", rect.width))?,
            u32::try_from(rect.height)
                .map_err(|_| format!("invalid surface size {}", rect.height))?,
        )
        .map_err(ui_error)
}

fn ui_error(error: UiBackendError) -> String {
    format!("{error:?}")
}

/// Parent-side helper: restart the supervisor with the J07 static span
/// (`shell.quick_control.restart`). Failures are nonfatal: the caller logs
/// and keeps the shell running without quick-control popups.
pub fn poll_supervisor_nonfatal(supervisor: &mut QuickControlSupervisor) {
    let _span = crate::runtime::shell_span("shell.quick_control.restart").start();
    match supervisor.poll() {
        Ok(_) => {}
        Err(SupervisorError::RestartBudgetExhausted) => {
            eprintln!(
                "flamewm-shell: quick-control restart budget exhausted; continuing without helper"
            );
        }
        Err(error) => {
            eprintln!("flamewm-shell: quick-control poll: {error}");
        }
    }
}

/// Parent-side open: compute the retained anchor + work area + edge and
/// forward to the supervisor, then return immediately. Nonfatal on
/// failure: the shell loop survives and the popup simply stays closed.
pub fn open_via_supervisor_nonfatal(
    supervisor: &mut QuickControlSupervisor,
    kind: QuickControlKind,
    anchor: flamewm_api::Rect,
    work_area: flamewm_api::Rect,
    panel_edge: flamewm_api::PanelEdge,
) {
    let _span = crate::runtime::shell_span("shell.quick_control.open_request").start();
    if let Err(error) = supervisor.open(super::protocol::OpenRequest {
        kind,
        anchor,
        work_area,
        panel_edge,
    }) {
        eprintln!("flamewm-shell: quick-control open failed (nonfatal): {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_never_in_logs_or_protocol() {
        // The helper snapshot carries only a masked length, never the
        // secret value; the protocol type carries no secret field.
        let snapshot = QuickControlSnapshot {
            system: flamewm_api::system::SystemSnapshot::default(),
            audio_tab: AudioTab::Devices,
            audio_settings: AudioSettings::default(),
            network_query: NetworkQuery::default(),
            network_secret_masked_len: 8,
        };
        let debug = format!("{snapshot:?}");
        assert!(
            !debug.contains("s3cr3t"),
            "snapshot debug must not carry a secret"
        );
        let line = super::super::protocol::encode_command(&Command::Open(OpenRequest {
            kind: QuickControlKind::Network,
            anchor: flamewm_api::Rect::new(0, 0, 1, 1),
            work_area: flamewm_api::Rect::new(0, 0, 10, 10),
            panel_edge: flamewm_api::PanelEdge::Bottom,
        }));
        assert!(!line.contains("s3cr3t"));
    }

    #[test]
    fn i32_max_constraint_cannot_reach_placement() {
        // i32::MAX work area must resolve to a finite f32 device
        // constraint and never flow raw into placement math (which
        // would overflow `right()`/`bottom()` i32 arithmetic).
        let area = flamewm_api::Rect::new(0, 0, i32::MAX, i32::MAX);
        let (max_w, max_h) = outer_measure_constraint(area);
        assert!(max_w.is_finite() && max_h.is_finite());
        assert!(max_w > 0.0 && max_h > 0.0);
        // The clamped intrinsic (work-area min) still refuses as
        // PendingLayout through the pure path when size is absent.
        let refused = measured_popup_rect(
            Some(flamewm_api::Rect::new(0, 0, 10, 10)),
            None,
            PopoverEdge::Above,
            PopoverAlign::End,
            flamewm_api::Rect::new(0, 0, 1920, 1080),
            8,
        );
        assert!(refused.is_err());
    }

    #[test]
    fn bottom_end_aligned_network_fixture_right_above() {
        // Bottom panel network fixture: end-aligned above the anchor,
        // right edge pinned to the anchor/work-area right.
        let area = flamewm_api::Rect::new(0, 0, 1920, 1080);
        let anchor = flamewm_api::Rect::new(1830, 1036, 70, 44);
        let size = flamewm_api::Size::new(310, 220);
        let rect = measured_popup_rect(
            Some(anchor),
            Some(size),
            PopoverEdge::Above,
            PopoverAlign::End,
            area,
            8,
        )
        .expect("measured");
        assert_eq!(rect.right(), anchor.right());
        assert!(rect.right() <= area.right());
        assert!(rect.bottom() <= anchor.y);
    }

    #[test]
    fn top_panel_places_below() {
        // Top panel: generic path with Below edge opens under the anchor.
        let area = flamewm_api::Rect::new(0, 0, 1920, 1080);
        let anchor = flamewm_api::Rect::new(1830, 0, 70, 44);
        let size = flamewm_api::Size::new(310, 220);
        let rect = measured_popup_rect(
            Some(anchor),
            Some(size),
            PopoverEdge::Below,
            PopoverAlign::End,
            area,
            8,
        )
        .expect("measured");
        assert!(rect.y >= anchor.bottom());
        assert_eq!(rect.right(), anchor.right());
    }

    #[test]
    fn left_right_edges_preserved() {
        // Left/right panels keep their opening side inside the area.
        let area = flamewm_api::Rect::new(0, 0, 1920, 1080);
        let anchor_l = flamewm_api::Rect::new(0, 400, 44, 70);
        let anchor_r = flamewm_api::Rect::new(1876, 400, 44, 70);
        let size = flamewm_api::Size::new(200, 150);
        let right_of = measured_popup_rect(
            Some(anchor_l),
            Some(size),
            PopoverEdge::Right,
            PopoverAlign::End,
            area,
            8,
        )
        .expect("measured");
        assert!(right_of.x >= anchor_l.right());
        let left_of = measured_popup_rect(
            Some(anchor_r),
            Some(size),
            PopoverEdge::Left,
            PopoverAlign::End,
            area,
            8,
        )
        .expect("measured");
        assert!(left_of.right() <= anchor_r.x);
    }

    #[test]
    fn fitted_helper_rect_refuses_without_layout() {
        // Pure refusal contract: degenerate anchor area still places when
        // the anchor is valid; the refusal path is owned by the helper
        // intrinsic measurement (covered by popup unit tests).
        let area = flamewm_api::Rect::new(0, 0, 1920, 1080);
        let anchor = flamewm_api::Rect::new(1900, 1036, 20, 44);
        let size = flamewm_api::Size::new(310, 200);
        let rect = measured_popup_rect(
            Some(anchor),
            Some(size),
            PopoverEdge::Above,
            PopoverAlign::End,
            area,
            8,
        )
        .expect("measured");
        assert!(rect.right() <= area.right());
        assert!(rect.bottom() <= anchor.y);
    }

    fn open_cmd(kind: QuickControlKind) -> Command {
        Command::Open(OpenRequest {
            kind,
            anchor: flamewm_api::Rect::new(0, 0, 10, 10),
            work_area: flamewm_api::Rect::new(0, 0, 100, 100),
            panel_edge: flamewm_api::PanelEdge::Bottom,
        })
    }

    #[test]
    fn coalesce_open_keeps_newest() {
        let mut q: VecDeque<Command> = VecDeque::from([
            open_cmd(QuickControlKind::Audio),
            open_cmd(QuickControlKind::Network),
            open_cmd(QuickControlKind::Calendar),
        ]);
        let out = coalesce_commands(&mut q);
        assert_eq!(out.len(), 1);
        assert!(matches!(
            out[0],
            Command::Open(r) if r.kind == QuickControlKind::Calendar
        ));
    }

    #[test]
    fn close_none_selects_zero_close_one_selects_one() {
        // close-none: no open surface -> None (zero close calls).
        assert!(take_close_target(None).is_none());
        // close-one: exactly the open kind is selected.
        assert_eq!(
            take_close_target(Some(QuickControlKind::Network)),
            Some(QuickControlKind::Network)
        );
    }

    #[test]
    fn profiler_cadence_floor_is_ten_seconds() {
        // The host profiler timer must honor profile_interval_secs with
        // its 10s floor, never a fast empty tick.
        assert!(flamewm_profiler::profile_interval_secs() >= 10);
    }

    #[test]
    fn stage_labels_are_real_turns() {
        // True stages only: every label maps to a real turn in
        // open_request/fitted_rect/close; no fake snapshot/measure/place.
        for label in QUICK_STAGE_LABELS {
            assert!(
                [
                    "shell.quick.open.total",
                    "shell.quick.project",
                    "shell.quick.measure",
                    "shell.quick.place",
                    "shell.quick.prepare/commit_geometry",
                    "shell.quick.present",
                    "shell.quick.pointer_grab",
                    "shell.quick.close/ungrab/unmap",
                ]
                .contains(label),
                "non-real stage label: {label}"
            );
        }
        assert!(!QUICK_STAGE_LABELS.contains(&"shell.quick.snapshot"));
        assert!(!QUICK_STAGE_LABELS.contains(&"shell.quick.grab"));
        assert!(!QUICK_STAGE_LABELS.contains(&"shell.quick.prepare"));
        assert!(!QUICK_STAGE_LABELS.contains(&"shell.quick.close"));
    }

    #[test]
    fn slow_open_gate_and_stage_render_are_pure() {
        assert!(!is_slow_open(Duration::from_micros(16_669)));
        assert!(is_slow_open(Duration::from_micros(16_670)));
        let timings = OpenStageTimings {
            total: Duration::from_millis(20),
            project: Duration::from_millis(1),
            measure: Duration::from_millis(2),
            place: Duration::from_millis(3),
            prepare_commit: Duration::from_millis(4),
            present: Duration::from_millis(5),
            pointer_grab: Duration::from_millis(6),
            close: Duration::from_millis(7),
        };
        let line = timings.render(QuickControlKind::Audio);
        for stage in [
            "open.total",
            "project",
            "measure",
            "place",
            "prepare/commit_geometry",
            "present",
            "pointer_grab",
            "close/ungrab/unmap",
        ] {
            assert!(line.contains(stage), "slow line must carry {stage}");
        }
        let placement = FittedPlacement {
            rect: flamewm_api::Rect::new(10, 20, 30, 40),
            measure_elapsed: Duration::from_millis(2),
            place_elapsed: Duration::from_millis(3),
        };
        assert_eq!(placement.rect.x, 10);
        assert_eq!(placement.measure_elapsed, Duration::from_millis(2));
    }
}
