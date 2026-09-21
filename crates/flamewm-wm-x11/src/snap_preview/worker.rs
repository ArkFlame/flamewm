//! Off-WM-thread snap-preview backdrop readback.
//!
//! The mailbox is deliberately one-slot: pointer motion replaces pending work
//! instead of building a queue. The worker owns its Xlib connection through
//! `capture_root_rect_auto`; it never receives the WM connection or a
//! `SurfaceRuntime`.

use std::io::{Read, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::JoinHandle;

use flamewm_api::Rect;
use flamewm_render_x11::backdrop;
use flamewm_window_core::SnapTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BackdropRequest {
    pub(crate) generation: u64,
    pub(crate) candidate: SnapTarget,
    pub(crate) geometry: Rect,
    pub(crate) opacity: u8,
    pub(crate) capture: (i32, i32, u32, u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackdropResult {
    pub(crate) request: BackdropRequest,
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// `None` means capture failed; no black/opaque fallback is synthesized.
    pub(crate) pixels: Option<Vec<u8>>,
    /// Preserve the originating capture/composite failure for the WM boundary.
    pub(crate) error: Option<String>,
}

impl BackdropResult {
    /// Generation, candidate, geometry, and opacity are one stale-state fence.
    #[must_use]
    pub(crate) fn matches(&self, request: BackdropRequest) -> bool {
        self.request == request
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum FailureStage {
    InvalidGeometry,
    SurfaceEnsure,
    SurfaceCreate,
    SurfaceUnavailable,
    MoveResize,
    Style,
    Redraw,
    Show,
    ApplyError,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum HideReason {
    Request,
    Replace,
    InvalidGeometry,
    SurfaceFailure,
    MoveResizeFailure,
    StyleFailure,
    RedrawFailure,
    ShowFailure,
    ApplyError,
}

pub(crate) struct SnapPreviewWorker {
    requests: Arc<Mutex<Option<BackdropRequest>>>,
    request_ready: Arc<Condvar>,
    stopped: Arc<AtomicBool>,
    last_request: Mutex<Option<BackdropRequest>>,
    result: Arc<Mutex<Option<BackdropResult>>>,
    wake_reader: UnixStream,
    worker: Option<JoinHandle<()>>,
}

struct SnapTelemetry {
    request: flamewm_profiler::CounterPoint,
    request_coalesced: flamewm_profiler::CounterPoint,
    result_ready: flamewm_profiler::CounterPoint,
    result_stale_drop: flamewm_profiler::CounterPoint,
    result_apply: flamewm_profiler::CounterPoint,
    failure: flamewm_profiler::CounterPoint,
    hide: flamewm_profiler::CounterPoint,
    failure_invalid_geometry: flamewm_profiler::CounterPoint,
    failure_surface_ensure: flamewm_profiler::CounterPoint,
    failure_surface_create: flamewm_profiler::CounterPoint,
    failure_surface_unavailable: flamewm_profiler::CounterPoint,
    failure_move_resize: flamewm_profiler::CounterPoint,
    failure_style: flamewm_profiler::CounterPoint,
    failure_redraw: flamewm_profiler::CounterPoint,
    failure_show: flamewm_profiler::CounterPoint,
    capture_error: flamewm_profiler::CounterPoint,
    composite_error: flamewm_profiler::CounterPoint,
    result_ready_no_pixels: flamewm_profiler::CounterPoint,
    result_stale: flamewm_profiler::CounterPoint,
    result_take_count: flamewm_profiler::CounterPoint,
    result_take_empty: flamewm_profiler::CounterPoint,
    result_match_count: flamewm_profiler::CounterPoint,
    result_match_stale: flamewm_profiler::CounterPoint,
    apply_error: flamewm_profiler::CounterPoint,
    hide_request: flamewm_profiler::CounterPoint,
    hide_replace: flamewm_profiler::CounterPoint,
    hide_invalid_geometry: flamewm_profiler::CounterPoint,
    hide_surface_failure: flamewm_profiler::CounterPoint,
    hide_move_resize_failure: flamewm_profiler::CounterPoint,
    hide_style_failure: flamewm_profiler::CounterPoint,
    hide_redraw_failure: flamewm_profiler::CounterPoint,
    hide_show_failure: flamewm_profiler::CounterPoint,
    hide_apply_error: flamewm_profiler::CounterPoint,
    capture_worker: flamewm_profiler::ProfilePoint,
    composite_worker: flamewm_profiler::ProfilePoint,
    wake_drain: flamewm_profiler::ProfilePoint,
    result_take: flamewm_profiler::ProfilePoint,
    result_match: flamewm_profiler::ProfilePoint,
}

const RESULT_TRACE_COOLDOWN: std::time::Duration = std::time::Duration::from_millis(100);
const RESULT_TRACE_READY: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.result.ready.trace");
const RESULT_TRACE_WAKE: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.result.wake.trace");
const RESULT_TRACE_WAKE_DRAIN: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.result.wake_drain.trace");
const RESULT_TRACE_TAKE: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.result.take.trace");
const RESULT_TRACE_MATCH: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.result.match.trace");
const RESULT_TRACE_STALE_DROP: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.result.stale_drop.trace");
const RESULT_TRACE_APPLY: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.result.apply.trace");
const RESULT_TRACE_FAILURE: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.result.failure.trace");
const REQUEST_TRACE_SUBMIT: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.request.submit.trace");
const REQUEST_TRACE_COALESCE: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.request.coalesce.trace");
const HIDE_TRACE: flamewm_debug::DebugEventId = flamewm_debug::DebugEventId("wm.snap.hide.trace");
const CAPTURE_TRACE_START: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.capture.start.trace");
const CAPTURE_TRACE_END: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.capture.end.trace");
const COMPOSITE_TRACE_END: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.composite.end.trace");
const RESULT_TRACE_STORE: flamewm_debug::DebugEventId =
    flamewm_debug::DebugEventId("wm.snap.result.store.trace");

static TRACE_EPOCH: OnceLock<std::time::Instant> = OnceLock::new();

fn trace_mono_us() -> u128 {
    TRACE_EPOCH
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_micros()
}

fn result_trace_id(stage: &'static str) -> flamewm_debug::DebugEventId {
    match stage {
        "ready" => RESULT_TRACE_READY,
        "wake" => RESULT_TRACE_WAKE,
        "wake_drain" => RESULT_TRACE_WAKE_DRAIN,
        "take" => RESULT_TRACE_TAKE,
        "match" => RESULT_TRACE_MATCH,
        "stale_drop" => RESULT_TRACE_STALE_DROP,
        "apply" => RESULT_TRACE_APPLY,
        "failure" => RESULT_TRACE_FAILURE,
        "submit" => REQUEST_TRACE_SUBMIT,
        "coalesce" => REQUEST_TRACE_COALESCE,
        "capture_start" => CAPTURE_TRACE_START,
        "capture_end" => CAPTURE_TRACE_END,
        "composite_end" => COMPOSITE_TRACE_END,
        "store" => RESULT_TRACE_STORE,
        _ => RESULT_TRACE_FAILURE,
    }
}

fn candidate_identity(candidate: SnapTarget) -> &'static str {
    match candidate {
        SnapTarget::None => "none",
        SnapTarget::LeftHalf => "left",
        SnapTarget::RightHalf => "right",
        SnapTarget::TopHalf => "top",
        SnapTarget::BottomHalf => "bottom",
        SnapTarget::TopLeftQuarter => "top_left",
        SnapTarget::TopRightQuarter => "top_right",
        SnapTarget::BottomLeftQuarter => "bottom_left",
        SnapTarget::BottomRightQuarter => "bottom_right",
        SnapTarget::Maximize => "maximize",
    }
}

pub(crate) fn request_identity(request: BackdropRequest) -> String {
    format!(
        "generation={} candidate={} geometry=({},{} {}x{}) opacity={} capture=({},{} {}x{})",
        request.generation,
        candidate_identity(request.candidate),
        request.geometry.x,
        request.geometry.y,
        request.geometry.width,
        request.geometry.height,
        request.opacity,
        request.capture.0,
        request.capture.1,
        request.capture.2,
        request.capture.3,
    )
}

pub(crate) fn compact_error(error: &str) -> String {
    error.chars().take(256).collect()
}

pub(crate) fn emit_result_trace(
    stage: &'static str,
    request: BackdropRequest,
    detail: impl FnOnce() -> String,
) {
    flamewm_debug::emit(result_trace_id(stage), RESULT_TRACE_COOLDOWN, || {
        format!(
            "stage={stage} mono_us={} {} {}",
            trace_mono_us(),
            request_identity(request),
            detail()
        )
    });
}

pub(crate) fn emit_hide_trace(
    generation_before: u64,
    generation_after: u64,
    candidate: Option<SnapTarget>,
    geometry: Option<Rect>,
    opacity: u8,
    reason: HideReason,
) {
    flamewm_debug::emit(HIDE_TRACE, RESULT_TRACE_COOLDOWN, || {
        let geometry = geometry.map_or_else(
            || "none".to_owned(),
            |rect| format!("({},{} {}x{})", rect.x, rect.y, rect.width, rect.height),
        );
        format!(
            "stage=hide mono_us={} generation={} generation_next={} candidate={} geometry={} opacity={} reason={reason:?}",
            trace_mono_us(),
            generation_before,
            generation_after,
            candidate.map_or("none", candidate_identity),
            geometry,
            opacity,
        )
    });
}

fn telemetry() -> &'static SnapTelemetry {
    static SET: OnceLock<SnapTelemetry> = OnceLock::new();
    SET.get_or_init(|| SnapTelemetry {
        request: flamewm_profiler::CounterPoint::new("wm.snap.request"),
        request_coalesced: flamewm_profiler::CounterPoint::new("wm.snap.request.coalesced"),
        result_ready: flamewm_profiler::CounterPoint::new("wm.snap.result.ready"),
        result_stale_drop: flamewm_profiler::CounterPoint::new("wm.snap.result.stale_drop"),
        result_apply: flamewm_profiler::CounterPoint::new("wm.snap.result.apply"),
        failure: flamewm_profiler::CounterPoint::new("wm.snap.failure"),
        hide: flamewm_profiler::CounterPoint::new("wm.snap.hide"),
        failure_invalid_geometry: flamewm_profiler::CounterPoint::new(
            "wm.snap.failure.invalid_geometry",
        ),
        failure_surface_ensure: flamewm_profiler::CounterPoint::new(
            "wm.snap.failure.surface.ensure",
        ),
        failure_surface_create: flamewm_profiler::CounterPoint::new(
            "wm.snap.failure.surface.create",
        ),
        failure_surface_unavailable: flamewm_profiler::CounterPoint::new(
            "wm.snap.failure.surface.unavailable",
        ),
        failure_move_resize: flamewm_profiler::CounterPoint::new("wm.snap.failure.move_resize"),
        failure_style: flamewm_profiler::CounterPoint::new("wm.snap.failure.style"),
        failure_redraw: flamewm_profiler::CounterPoint::new("wm.snap.failure.redraw"),
        failure_show: flamewm_profiler::CounterPoint::new("wm.snap.failure.show"),
        capture_error: flamewm_profiler::CounterPoint::new("wm.snap.failure.capture_error"),
        composite_error: flamewm_profiler::CounterPoint::new("wm.snap.failure.composite_error"),
        result_ready_no_pixels: flamewm_profiler::CounterPoint::new(
            "wm.snap.result.ready.no_pixels",
        ),
        result_stale: flamewm_profiler::CounterPoint::new("wm.snap.result.stale"),
        result_take_count: flamewm_profiler::CounterPoint::new("wm.snap.result.take.count"),
        result_take_empty: flamewm_profiler::CounterPoint::new("wm.snap.result.take.empty"),
        result_match_count: flamewm_profiler::CounterPoint::new("wm.snap.result.match.count"),
        result_match_stale: flamewm_profiler::CounterPoint::new("wm.snap.result.match.stale"),
        apply_error: flamewm_profiler::CounterPoint::new("wm.snap.failure.apply_error"),
        hide_request: flamewm_profiler::CounterPoint::new("wm.snap.hide.request"),
        hide_replace: flamewm_profiler::CounterPoint::new("wm.snap.hide.replace"),
        hide_invalid_geometry: flamewm_profiler::CounterPoint::new("wm.snap.hide.invalid_geometry"),
        hide_surface_failure: flamewm_profiler::CounterPoint::new("wm.snap.hide.surface_failure"),
        hide_move_resize_failure: flamewm_profiler::CounterPoint::new(
            "wm.snap.hide.move_resize_failure",
        ),
        hide_style_failure: flamewm_profiler::CounterPoint::new("wm.snap.hide.style_failure"),
        hide_redraw_failure: flamewm_profiler::CounterPoint::new("wm.snap.hide.redraw_failure"),
        hide_show_failure: flamewm_profiler::CounterPoint::new("wm.snap.hide.show_failure"),
        hide_apply_error: flamewm_profiler::CounterPoint::new("wm.snap.hide.apply_error"),
        capture_worker: flamewm_profiler::ProfilePoint::new("render.overlay.capture.worker"),
        composite_worker: flamewm_profiler::ProfilePoint::new("render.overlay.composite.worker"),
        wake_drain: flamewm_profiler::ProfilePoint::new("wm.snap.wake.drain"),
        result_take: flamewm_profiler::ProfilePoint::new("wm.snap.result.take"),
        result_match: flamewm_profiler::ProfilePoint::new("wm.snap.result.match"),
    })
}

pub(crate) fn record_request() {
    telemetry().request.increment();
}

pub(crate) fn record_request_coalesced() {
    telemetry().request_coalesced.increment();
}

pub(crate) fn record_result_ready() {
    telemetry().result_ready.increment();
}

pub(crate) fn record_result_stale_drop() {
    telemetry().result_stale_drop.increment();
}

pub(crate) fn record_result_apply() {
    telemetry().result_apply.increment();
}

pub(crate) fn record_stage(stage: FailureStage) {
    match stage {
        FailureStage::InvalidGeometry => telemetry().failure_invalid_geometry.increment(),
        FailureStage::SurfaceEnsure => telemetry().failure_surface_ensure.increment(),
        FailureStage::SurfaceCreate => telemetry().failure_surface_create.increment(),
        FailureStage::SurfaceUnavailable => telemetry().failure_surface_unavailable.increment(),
        FailureStage::MoveResize => telemetry().failure_move_resize.increment(),
        FailureStage::Style => telemetry().failure_style.increment(),
        FailureStage::Redraw => telemetry().failure_redraw.increment(),
        FailureStage::Show => telemetry().failure_show.increment(),
        FailureStage::ApplyError => telemetry().apply_error.increment(),
    }
}

pub(crate) fn record_failure(stage: FailureStage) {
    telemetry().failure.increment();
    record_stage(stage);
}

pub(crate) fn record_capture_error() {
    telemetry().capture_error.increment();
}

pub(crate) fn record_composite_error() {
    telemetry().composite_error.increment();
}

pub(crate) fn record_result_ready_no_pixels() {
    telemetry().result_ready_no_pixels.increment();
}

pub(crate) fn record_result_stale_stage() {
    telemetry().result_stale.increment();
}

pub(crate) fn record_result_take(has_result: bool) {
    if has_result {
        telemetry().result_take_count.increment();
    } else {
        telemetry().result_take_empty.increment();
    }
}

pub(crate) fn record_result_match(matches: bool) {
    if matches {
        telemetry().result_match_count.increment();
    } else {
        telemetry().result_match_stale.increment();
    }
}

pub(crate) fn record_hide_reason(reason: HideReason) {
    telemetry().hide.increment();
    match reason {
        HideReason::Request => telemetry().hide_request.increment(),
        HideReason::Replace => telemetry().hide_replace.increment(),
        HideReason::InvalidGeometry => telemetry().hide_invalid_geometry.increment(),
        HideReason::SurfaceFailure => telemetry().hide_surface_failure.increment(),
        HideReason::MoveResizeFailure => telemetry().hide_move_resize_failure.increment(),
        HideReason::StyleFailure => telemetry().hide_style_failure.increment(),
        HideReason::RedrawFailure => telemetry().hide_redraw_failure.increment(),
        HideReason::ShowFailure => telemetry().hide_show_failure.increment(),
        HideReason::ApplyError => telemetry().hide_apply_error.increment(),
    }
}

pub(crate) fn start_capture_worker() -> flamewm_profiler::SpanGuard {
    telemetry().capture_worker.start()
}

pub(crate) fn start_composite_worker() -> flamewm_profiler::SpanGuard {
    telemetry().composite_worker.start()
}

pub(crate) fn start_wake_drain() -> flamewm_profiler::SpanGuard {
    telemetry().wake_drain.start()
}

pub(crate) fn start_result_take() -> flamewm_profiler::SpanGuard {
    telemetry().result_take.start()
}

pub(crate) fn start_result_match() -> flamewm_profiler::SpanGuard {
    telemetry().result_match.start()
}

impl SnapPreviewWorker {
    #[must_use]
    pub(crate) fn new() -> Self {
        let (wake_reader, wake_writer) =
            UnixStream::pair().expect("snap preview worker wake pipe must exist");
        wake_reader
            .set_nonblocking(true)
            .expect("snap preview worker wake reader must be nonblocking");
        wake_writer
            .set_nonblocking(true)
            .expect("snap preview worker wake writer must be nonblocking");

        let requests = Arc::new(Mutex::new(None));
        let request_ready = Arc::new(Condvar::new());
        let stopped = Arc::new(AtomicBool::new(false));
        let result = Arc::new(Mutex::new(None));
        let worker_requests = Arc::clone(&requests);
        let worker_ready = Arc::clone(&request_ready);
        let worker_stopped = Arc::clone(&stopped);
        let worker_result = Arc::clone(&result);
        let worker = std::thread::spawn(move || {
            worker_loop(
                worker_requests,
                worker_ready,
                worker_stopped,
                worker_result,
                wake_writer,
            );
        });

        Self {
            requests,
            request_ready,
            stopped,
            last_request: Mutex::new(None),
            result,
            wake_reader,
            worker: Some(worker),
        }
    }

    /// Replace pending work without waiting for the worker or X11.
    pub(crate) fn try_send(&self, request: BackdropRequest) -> bool {
        if self.stopped.load(Ordering::Acquire) {
            return false;
        }
        let mut last = self
            .last_request
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if last.as_ref() == Some(&request) {
            record_request_coalesced();
            emit_result_trace("coalesce", request, || {
                "source=mailbox.same_state".to_owned()
            });
            return false;
        }
        let mut latest = self
            .requests
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *latest = Some(request);
        *last = Some(request);
        self.request_ready.notify_one();
        record_request();
        emit_result_trace("submit", request, || "source=mailbox.latest".to_owned());
        true
    }

    /// Raw fd signalled by a completed worker result.
    #[must_use]
    pub(crate) fn wake_fd(&self) -> RawFd {
        self.wake_reader.as_raw_fd()
    }

    /// Drain wake bytes without blocking. Result ownership remains in the
    /// latest result slot until the WM loop boundary consumes it.
    pub(crate) fn drain_wake(&mut self) {
        let mut bytes = [0_u8; 64];
        let mut signaled = false;
        loop {
            match self.wake_reader.read(&mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(_) => signaled = true,
            }
        }
        if signaled {
            let request = self
                .result
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .as_ref()
                .map(|result| result.request);
            if let Some(request) = request {
                emit_result_trace("wake_drain", request, || "slot=result".to_owned());
            }
        }
    }

    pub(crate) fn take_result(&self) -> Option<BackdropResult> {
        let result = self
            .result
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .take();
        record_result_take(result.is_some());
        if let Some(result) = result.as_ref() {
            emit_result_trace("take", result.request, || "slot=take".to_owned());
        } else {
            flamewm_debug::emit(RESULT_TRACE_TAKE, RESULT_TRACE_COOLDOWN, || {
                format!("stage=take mono_us={} slot=empty", trace_mono_us())
            });
        }
        result
    }
}

impl Drop for SnapPreviewWorker {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        self.request_ready.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn worker_loop(
    requests: Arc<Mutex<Option<BackdropRequest>>>,
    request_ready: Arc<Condvar>,
    stopped: Arc<AtomicBool>,
    result: Arc<Mutex<Option<BackdropResult>>>,
    mut wake_writer: UnixStream,
) {
    loop {
        let request = {
            let mut latest = requests.lock().unwrap_or_else(|poison| poison.into_inner());
            loop {
                if stopped.load(Ordering::Acquire) {
                    return;
                }
                if let Some(request) = latest.take() {
                    break request;
                }
                latest = request_ready
                    .wait(latest)
                    .unwrap_or_else(|poison| poison.into_inner());
            }
        };

        // No lock is held over XOpenDisplay/XGetImage or pixel conversion.
        emit_result_trace("capture_start", request, || "owner=worker".to_owned());
        let capture_started = std::time::Instant::now();
        let captured = {
            let _guard = start_capture_worker();
            backdrop::capture_root_rect_auto(request.capture)
        };
        emit_result_trace("capture_end", request, || match &captured {
            Ok(capture) => format!(
                "outcome=ok duration_us={} size={}x{}",
                capture_started.elapsed().as_micros(),
                capture.width,
                capture.height
            ),
            Err(error) => format!(
                "outcome=error duration_us={} error={}",
                capture_started.elapsed().as_micros(),
                compact_error(error)
            ),
        });
        let completed = match captured {
            Ok(capture) => {
                let dimensions = (capture.width, capture.height);
                let (fill, border) = backdrop::selection_material(request.opacity);
                let composite_started = std::time::Instant::now();
                let pixels = {
                    let _guard = start_composite_worker();
                    backdrop::composite_selection_material(&capture, Some(fill), border)
                };
                let expected = dimensions.0 as usize * dimensions.1 as usize * 4;
                let pixel_len = pixels.len();
                emit_result_trace("composite_end", request, || {
                    format!(
                        "outcome={} duration_us={} bytes={} expected={expected}",
                        if pixel_len == expected { "ok" } else { "error" },
                        composite_started.elapsed().as_micros(),
                        pixel_len,
                    )
                });
                if pixel_len == expected {
                    BackdropResult {
                        request,
                        width: dimensions.0,
                        height: dimensions.1,
                        pixels: Some(pixels),
                        error: None,
                    }
                } else {
                    record_composite_error();
                    BackdropResult {
                        request,
                        width: dimensions.0,
                        height: dimensions.1,
                        pixels: None,
                        error: Some(format!(
                            "backdrop composite produced {} bytes; expected {expected}",
                            pixels.len()
                        )),
                    }
                }
            }
            Err(error) => {
                record_capture_error();
                BackdropResult {
                    request,
                    width: request.capture.2,
                    height: request.capture.3,
                    pixels: None,
                    error: Some(error),
                }
            }
        };
        let result_request = completed.request;
        let result_ok = completed.pixels.is_some();
        let result_error = completed.error.clone();
        let (overwritten, previous_request) = {
            let mut slot = result.lock().unwrap_or_else(|poison| poison.into_inner());
            let previous_request = slot.as_ref().map(|result| result.request);
            let overwritten = previous_request.is_some();
            *slot = Some(completed);
            (overwritten, previous_request)
        };
        record_result_ready();
        if !result_ok {
            record_result_ready_no_pixels();
        }
        emit_result_trace("store", result_request, || match previous_request {
            Some(previous) => format!(
                "slot=store overwritten={overwritten} previous={}",
                request_identity(previous)
            ),
            None => format!("slot=store overwritten={overwritten}"),
        });
        emit_result_trace("ready", result_request, || match result_error.as_deref() {
            Some(error) => format!("outcome=failure error={}", compact_error(error)),
            None if result_ok => "outcome=ok".to_owned(),
            None => "outcome=failure error=missing-pixels".to_owned(),
        });
        let _ = wake_writer.write(&[1]);
        emit_result_trace("wake", result_request, || "signal=pipe".to_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(generation: u64) -> BackdropRequest {
        BackdropRequest {
            generation,
            candidate: SnapTarget::LeftHalf,
            geometry: Rect::new(0, 0, 960, 1080),
            opacity: 20,
            capture: (0, 0, 960, 1080),
        }
    }

    #[test]
    fn result_fence_includes_generation_and_visual_state() {
        let request = request(7);
        let result = BackdropResult {
            request,
            width: 960,
            height: 1080,
            pixels: None,
            error: None,
        };
        assert!(result.matches(request));
        assert!(!result.matches(BackdropRequest {
            generation: 8,
            ..request
        }));
        assert!(!result.matches(BackdropRequest {
            candidate: SnapTarget::RightHalf,
            ..request
        }));
        assert!(!result.matches(BackdropRequest {
            geometry: Rect::new(960, 0, 960, 1080),
            ..request
        }));
        assert!(!result.matches(BackdropRequest {
            opacity: 40,
            ..request
        }));
    }

    #[test]
    fn same_state_request_is_coalesced_before_capture() {
        let worker = SnapPreviewWorker::new();
        let request = request(1);
        assert!(worker.try_send(request));
        assert!(!worker.try_send(request));
    }

    #[test]
    fn result_preserves_capture_error_identity() {
        let result = BackdropResult {
            request: request(3),
            width: 960,
            height: 1080,
            pixels: None,
            error: Some("backdrop capture: XOpenDisplay failed".to_owned()),
        };
        assert_eq!(
            result.error.as_deref(),
            Some("backdrop capture: XOpenDisplay failed")
        );
    }
}
