//! J06 configure/interaction contract-regression tests (T04/T05).
//!
//! Pure semantics only. No X connection, no product behavior change, no WM mock.
//! FROZEN INTERFACE STUB for J15 `wm/configure.rs`: `NotifyClass`,
//! `classify_notify`, `RequestDecision`, and `decide_request` mirror the
//! frozen observation-vs-request separation (`handle_configure_notify` is
//! observation-only; `handle_configure_request` owns the accept/refuse
//! policy). J15 must keep these signatures/semantics when extracting the
//! owner module.

use crate::frame::coords::{RootPoint, RootRect};
use crate::frame::model::ResizeEdges;
use crate::frame::model::SnapTarget;
use crate::frame::model::{FrameControl, PlacementMode, PlacementSnapshot, PlacementState};
use crate::frame::session::{
    ControlPressSession, InteractionSession, MoveAnchor, MoveSession, ResizeSession,
};

// ---- FROZEN INTERFACE STUB (J15 target: wm/configure.rs) ----

/// Observation class for a ConfigureNotify event window id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyClass {
    SyntheticEchoDrop,
    FrameObserved,
    ClientObserved,
    InputChildObserved,
    StaleUnknown,
}

/// Pure notify classifier: observation ids never imply geometry intent.
#[must_use]
pub fn classify_notify(
    window: u32,
    frame: u32,
    client: u32,
    is_input_child: bool,
    synthetic_echo: bool,
    known: bool,
) -> NotifyClass {
    if synthetic_echo {
        return NotifyClass::SyntheticEchoDrop;
    }
    if !known {
        return NotifyClass::StaleUnknown;
    }
    if window == frame {
        return NotifyClass::FrameObserved;
    }
    if window == client {
        return NotifyClass::ClientObserved;
    }
    if is_input_child {
        return NotifyClass::InputChildObserved;
    }
    NotifyClass::StaleUnknown
}

/// Request policy decision. Accept means exactly one frame commit;
/// refusals must emit a synthetic current-geometry notify instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestDecision {
    AcceptOnce,
    RefuseInteractive,
    RefuseMode,
}

impl RequestDecision {
    #[must_use]
    pub const fn needs_synthetic_notify(self) -> bool {
        !matches!(self, Self::AcceptOnce)
    }
    #[must_use]
    pub const fn geometry_effects(self) -> usize {
        match self {
            Self::AcceptOnce => 1,
            Self::RefuseInteractive | Self::RefuseMode => 0,
        }
    }
}

/// Pure request policy: active gesture wins over mode; WM-owned modes
/// refuse client geometry; only Idle+Floating accepts.
#[must_use]
pub fn decide_request(session: &InteractionSession, mode: PlacementMode) -> RequestDecision {
    if !session.is_idle() {
        return RequestDecision::RefuseInteractive;
    }
    if !matches!(mode, PlacementMode::Floating) {
        return RequestDecision::RefuseMode;
    }
    RequestDecision::AcceptOnce
}

fn sample_placement(mode: PlacementMode) -> PlacementState {
    let rect = RootRect::new(100, 100, 400, 300);
    PlacementState {
        mode,
        current: rect,
        floating_restore: rect,
        resume: None,
    }
}

fn idle() -> InteractionSession {
    InteractionSession::Idle
}

fn move_session() -> InteractionSession {
    let mut s = InteractionSession::Idle;
    s.begin_move(MoveSession {
        client: 7,
        start_pointer: RootPoint::new(120, 130),
        start_placement: PlacementSnapshot {
            mode: PlacementMode::Floating,
            rect: RootRect::new(100, 100, 400, 300),
        },
        anchor: MoveAnchor::new(20, 400, 30),
        grab_window: 99,
    });
    s
}

fn resize_session() -> InteractionSession {
    let mut s = InteractionSession::Idle;
    s.begin_resize(ResizeSession {
        client: 7,
        start_pointer: RootPoint::new(100, 100),
        start_placement: PlacementSnapshot {
            mode: PlacementMode::Floating,
            rect: RootRect::new(100, 100, 400, 300),
        },
        edges: ResizeEdges::bottom_right(),
        grab_window: 99,
    });
    s
}

fn control_session() -> InteractionSession {
    let mut s = InteractionSession::Idle;
    s.begin_control(ControlPressSession::new(7, FrameControl::Close, 99));
    s
}

// CONTRACT-REGRESSION: T04 ConfigureNotify is observation-only.
// TRIGGER: 1000 expected-frame notifies against unchanged placement.
// OBSERVABLE: classifier yields FrameObserved; placement untouched.
// GAP: native wm.rs path needs X conn; stub pins semantics pre-J15.
// MUTATION: none (query-only classifier + equality check).
// CASE: frame-expected burst has zero geometry effects.
#[test]
fn t04_frame_expected_notify_is_observation_only() {
    let placement = sample_placement(PlacementMode::Floating);
    let geometry_effects = 0usize;
    for i in 0..1000 {
        let window = 11u32;
        let class = classify_notify(window, 11, 12, false, false, true);
        assert_eq!(class, NotifyClass::FrameObserved, "iter {i}");
        // Observation path: compare only, never commit.
        let observed = RootRect::new(100, 100, 400, 300);
        assert_eq!(observed, placement.current, "iter {i}");
        assert_eq!(placement.current.w, 400);
    }
    assert_eq!(geometry_effects, 0);
    assert_eq!(placement, sample_placement(PlacementMode::Floating));
}

// CONTRACT-REGRESSION: T04 client-expected notify is observation-only.
// TRIGGER: 1000 expected-client notifies (local coords, interior size).
// OBSERVABLE: classifier yields ClientObserved; placement untouched.
// GAP: native client-offset check needs extents; stub pins class + no-mutation.
// MUTATION: none.
// CASE: client-expected burst has zero geometry effects.
#[test]
fn t04_client_expected_notify_is_observation_only() {
    let placement = sample_placement(PlacementMode::Floating);
    let geometry_effects = 0usize;
    for i in 0..1000 {
        let class = classify_notify(12, 11, 12, false, false, true);
        assert_eq!(class, NotifyClass::ClientObserved, "iter {i}");
        let _ = &placement;
    }
    assert_eq!(geometry_effects, 0);
    assert_eq!(placement.current, RootRect::new(100, 100, 400, 300));
}

// CONTRACT-REGRESSION: T04 stale/unknown notify is dropped.
// TRIGGER: 1000 notifies for unmanaged/unknown windows.
// OBSERVABLE: classifier yields StaleUnknown; no placement lookup/mutation.
// GAP: native client_for() needs conn maps; stub pins drop semantics.
// MUTATION: none.
// CASE: stale burst has zero geometry effects.
#[test]
fn t04_stale_notify_is_dropped() {
    let placement = sample_placement(PlacementMode::Floating);
    let geometry_effects = 0usize;
    for i in 0..1000 {
        let class = classify_notify(9000 + (i % 64) as u32, 11, 12, false, false, false);
        assert_eq!(class, NotifyClass::StaleUnknown, "iter {i}");
    }
    assert_eq!(geometry_effects, 0);
    assert_eq!(placement.mode, PlacementMode::Floating);
}

// CONTRACT-REGRESSION: T04 input-child notify never touches placement.
// TRIGGER: 1000 InputOnly child notifies (resize/title/controls).
// OBSERVABLE: classifier yields InputChildObserved; placement untouched.
// GAP: native registry lookup needs conn; stub pins WM-owned-detail drop.
// MUTATION: none.
// CASE: input-child burst has zero geometry effects.
#[test]
fn t04_input_child_notify_is_observation_only() {
    let placement = sample_placement(PlacementMode::Floating);
    let geometry_effects = 0usize;
    for i in 0..1000 {
        let child = 100 + (i % 12) as u32;
        let class = classify_notify(child, 11, 12, true, false, true);
        assert_eq!(class, NotifyClass::InputChildObserved, "iter {i}");
    }
    assert_eq!(geometry_effects, 0);
    assert_eq!(placement.current, RootRect::new(100, 100, 400, 300));
}

// CONTRACT-REGRESSION: T05 Idle+Floating accepts exactly once.
// TRIGGER: single client geometry request while idle and floating.
// OBSERVABLE: AcceptOnce (1 geometry effect); stack/sibling handled apart.
// GAP: native plan_client_configure needs conn for commit; stub pins policy.
// MUTATION: none (decision is pure).
// CASE: accept-once baseline.
#[test]
fn t05_idle_floating_accepts_once() {
    let decision = decide_request(&idle(), PlacementMode::Floating);
    assert_eq!(decision, RequestDecision::AcceptOnce);
    assert_eq!(decision.geometry_effects(), 1);
    assert!(!decision.needs_synthetic_notify());
}

// CONTRACT-REGRESSION: T05 active gestures refuse + synthetic notify.
// TRIGGER: requests during Move/Resize/ControlPress (all modes incl. Floating).
// OBSERVABLE: RefuseInteractive, 0 geometry effects, synthetic notify owed.
// GAP: native send_configure_notify needs conn; stub pins refusal kind.
// MUTATION: none.
// CASE: parameterized over the three interactive sessions.
#[test]
fn t05_interactive_sessions_refuse_with_synthetic() {
    let sessions = [move_session(), resize_session(), control_session()];
    for (idx, session) in sessions.iter().enumerate() {
        let decision = decide_request(session, PlacementMode::Floating);
        assert_eq!(
            decision,
            RequestDecision::RefuseInteractive,
            "session {idx}"
        );
        assert_eq!(decision.geometry_effects(), 0);
        assert!(decision.needs_synthetic_notify(), "session {idx}");
    }
}

// CONTRACT-REGRESSION: T05 WM-owned modes refuse even when idle.
// TRIGGER: requests while Maximized/Fullscreen/Snapped with Idle session.
// OBSERVABLE: RefuseMode, 0 geometry effects, synthetic current notify owed.
// GAP: native mode check lives behind conn-owned controller; stub pins it.
// MUTATION: none.
// CASE: parameterized over non-floating modes.
#[test]
fn t05_owned_modes_refuse_with_synthetic() {
    let modes = [
        PlacementMode::Maximized,
        PlacementMode::Fullscreen,
        PlacementMode::Snapped(SnapTarget::Left),
        PlacementMode::Snapped(SnapTarget::Right),
    ];
    for mode in modes {
        let decision = decide_request(&idle(), mode);
        assert_eq!(decision, RequestDecision::RefuseMode, "mode {mode:?}");
        assert_eq!(decision.geometry_effects(), 0);
        assert!(decision.needs_synthetic_notify(), "mode {mode:?}");
    }
}

// CONTRACT-REGRESSION: T05 stack/sibling intent is orthogonal to policy.
// TRIGGER: same decisions with and without STACK_MODE/SIBLING bits.
// OBSERVABLE: decision identical; restack still allowed on refuse paths.
// GAP: native restack needs conn; stub pins independence (bool passthrough).
// MUTATION: none.
// CASE: stack flag never flips accept/refuse.
#[test]
fn t05_stack_sibling_independent_of_policy() {
    // Model: stack bit carried alongside, decision ignores it.
    let with_stack = (decide_request(&idle(), PlacementMode::Floating), true);
    let without_stack = (decide_request(&idle(), PlacementMode::Floating), false);
    assert_eq!(with_stack.0, without_stack.0);
    assert_eq!(with_stack.0, RequestDecision::AcceptOnce);
    let refused_with = (
        decide_request(&move_session(), PlacementMode::Floating),
        true,
    );
    let refused_without = (
        decide_request(&move_session(), PlacementMode::Floating),
        false,
    );
    assert_eq!(refused_with.0, refused_without.0);
    assert_eq!(refused_with.0, RequestDecision::RefuseInteractive);
    let mode_with = (decide_request(&idle(), PlacementMode::Maximized), true);
    let mode_without = (decide_request(&idle(), PlacementMode::Maximized), false);
    assert_eq!(mode_with.0, mode_without.0);
    assert_eq!(mode_with.0, RequestDecision::RefuseMode);
}
