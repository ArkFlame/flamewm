//! Frame controller: pure INPUT -> SESSION -> GEOMETRY -> effect plan reducer.
//!
//! No X calls, no x11rb, no live-frame mutation. Composes the `input`
//! registry view (source-XID-is-truth), the `session` state machine, and the
//! `geometry` planners into one changeset per event plus a native effect
//! list applied by the live owner.

use std::collections::HashMap;

use flamewm_render_core::CursorKind;

use super::coords::{ClientRootRect, RootPoint, RootRect};
use super::geometry::{
    FrameExtents, GeometryReason, GeometryRequest, frame_to_client_local, frame_to_client_root,
    plan_client_configure, plan_move, plan_resize,
};
use super::input::target_for_xid;
use super::layout::TITLEBAR_H;
use super::model::{
    FrameControl, FrameRegion, PlacementMode, PlacementSnapshot, PlacementState, ResizeEdges,
    SnapTarget,
};
use super::resources::cursor_for_region;
use super::session::{
    ControlPressSession, InteractionSession, MoveAnchor, MoveSession, ResizeSession,
    maximized_restore_rect,
};
use crate::size_hints::ClientSizeHints;

/// Native effect intents executed by the live owner. Descriptions only.
///
/// Domain split: `ConfigureClientLocal` carries client-local geometry
/// (origin (0, titlebar) + client size) for the reparented window;
/// `ConfigureFrameRoot` carries the frame outer rect in root coords;
/// `NotifyClientRoot` carries the synthetic client-root rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameEffect {
    Grab { window: u32, cursor: CursorKind },
    Ungrab,
    ConfigureFrameRoot { x: i32, y: i32, w: i32, h: i32 },
    ConfigureClientLocal { x: i32, y: i32, w: u32, h: u32 },
    LayoutInput,
    PaintChrome,
    ApplyShape,
    NotifyClientRoot { x: i32, y: i32, w: u32, h: u32 },
    SnapPreview { target: Option<SnapTarget> },
    Noop,
}

/// Pure controller state: session + placement + registry view + geometry ctx.
#[derive(Debug, Clone)]
pub struct ControllerState {
    pub session: InteractionSession,
    pub placement: PlacementState,
    pub registry: HashMap<u32, FrameRegion>,
    pub client: u32,
    pub grab: u32,
    pub work_area: RootRect,
    pub screen_rect: RootRect,
    pub hints: ClientSizeHints,
    pub extents: FrameExtents,
}

impl ControllerState {
    #[must_use]
    pub fn new(
        client: u32,
        grab: u32,
        placement: PlacementState,
        work_area: RootRect,
        hints: ClientSizeHints,
        extents: FrameExtents,
    ) -> Self {
        Self {
            session: InteractionSession::Idle,
            placement,
            registry: HashMap::new(),
            client,
            grab,
            work_area,
            screen_rect: work_area,
            hints,
            extents,
        }
    }

    #[must_use]
    pub fn with_screen_rect(mut self, screen_rect: RootRect) -> Self {
        self.screen_rect = screen_rect;
        self
    }

    #[must_use]
    pub fn with_registry(mut self, registry: HashMap<u32, FrameRegion>) -> Self {
        self.registry = registry;
        self
    }
}

/// One input event per reducer step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameEvent {
    Press {
        source: u32,
        pointer: RootPoint,
    },
    Motion {
        pointer: RootPoint,
    },
    Release {
        pointer: RootPoint,
    },
    Enter,
    Leave,
    Configure {
        origin: RootPoint,
        size: Option<(u32, u32)>,
    },
    Snap {
        target: SnapTarget,
    },
    ToggleMax,
    ToggleFullscreen,
    Cancel,
}

fn request(
    state: &ControllerState,
    reason: GeometryReason,
    start: PlacementSnapshot,
    delta: (i32, i32),
    edges: ResizeEdges,
) -> GeometryRequest {
    GeometryRequest {
        reason,
        start,
        pointer_delta: delta,
        edges,
        work_area: state.work_area,
        hints: state.hints,
        frame_extents: state.extents,
    }
}

/// Full plan split: frame root rect + client local size + client root notify.
fn configure_effects(frame: RootRect, extents: FrameExtents) -> Vec<FrameEffect> {
    let local = frame_to_client_local(frame, extents);
    let root: ClientRootRect = frame_to_client_root(frame, extents);
    vec![
        FrameEffect::ConfigureFrameRoot {
            x: frame.x,
            y: frame.y,
            w: frame.w,
            h: frame.h,
        },
        FrameEffect::ConfigureClientLocal {
            x: local.x,
            y: local.y,
            w: local.w.max(1) as u32,
            h: local.h.max(1) as u32,
        },
        FrameEffect::NotifyClientRoot {
            x: root.x,
            y: root.y,
            w: root.w.max(1) as u32,
            h: root.h.max(1) as u32,
        },
    ]
}

/// Move plan: frame position + root notify only. No client configure,
/// no layout, no paint, no shape.
fn move_effects(frame: RootRect, extents: FrameExtents) -> Vec<FrameEffect> {
    let root: ClientRootRect = frame_to_client_root(frame, extents);
    vec![
        FrameEffect::ConfigureFrameRoot {
            x: frame.x,
            y: frame.y,
            w: frame.w,
            h: frame.h,
        },
        FrameEffect::NotifyClientRoot {
            x: root.x,
            y: root.y,
            w: root.w.max(1) as u32,
            h: root.h.max(1) as u32,
        },
    ]
}

fn snap_rect(work_area: RootRect, target: SnapTarget) -> RootRect {
    let (x, y, w, h) = (work_area.x, work_area.y, work_area.w, work_area.h);
    let hw = (w / 2).max(1);
    let hh = (h / 2).max(1);
    match target {
        SnapTarget::Left => RootRect::new(x, y, hw, h),
        SnapTarget::Right => RootRect::new(x.saturating_add(w.saturating_sub(hw)), y, hw, h),
        SnapTarget::Top => RootRect::new(x, y, w, hh),
        SnapTarget::Bottom => RootRect::new(x, y.saturating_add(h.saturating_sub(hh)), w, hh),
        SnapTarget::TopLeft => RootRect::new(x, y, hw, hh),
        SnapTarget::TopRight => RootRect::new(x.saturating_add(w.saturating_sub(hw)), y, hw, hh),
        SnapTarget::BottomLeft => RootRect::new(x, y.saturating_add(h.saturating_sub(hh)), hw, hh),
        SnapTarget::BottomRight => RootRect::new(
            x.saturating_add(w.saturating_sub(hw)),
            y.saturating_add(h.saturating_sub(hh)),
            hw,
            hh,
        ),
    }
}

fn move_anchor(pointer: RootPoint, rect: RootRect) -> MoveAnchor {
    let den = rect.w.max(1) as u32;
    let num = pointer.x.saturating_sub(rect.x).clamp(0, rect.w.max(0)) as u32;
    let vert = pointer.y.saturating_sub(rect.y).clamp(0, TITLEBAR_H);
    MoveAnchor::new(num, den, vert)
}

/// Pure reducer: one event -> (next state, native effect plan).
#[must_use]
pub fn reduce(state: &ControllerState, event: FrameEvent) -> (ControllerState, Vec<FrameEffect>) {
    let mut next = state.clone();
    match event {
        FrameEvent::Press { source, pointer } => {
            let target = target_for_xid(&state.registry, state.client, source);
            match target.region {
                FrameRegion::TitleDrag => {
                    // Maximized drag re-anchors to the floating restore rect.
                    let start_rect = if state.placement.mode == PlacementMode::Maximized {
                        maximized_restore_rect(
                            state.placement.floating_restore,
                            pointer,
                            move_anchor(pointer, state.placement.current).frac_num,
                            move_anchor(pointer, state.placement.current).frac_den,
                            pointer.y.saturating_sub(state.placement.current.y),
                            TITLEBAR_H,
                        )
                    } else {
                        state.placement.current
                    };
                    let anchor = move_anchor(pointer, start_rect);
                    next.session.begin_move(MoveSession {
                        client: state.client,
                        start_pointer: pointer,
                        start_placement: PlacementSnapshot {
                            mode: state.placement.mode,
                            rect: start_rect,
                        },
                        anchor,
                        grab_window: state.grab,
                    });
                    let effects = vec![FrameEffect::Grab {
                        window: state.grab,
                        cursor: CursorKind::Move,
                    }];
                    (next, effects)
                }
                FrameRegion::Resize(edges) => {
                    next.session.begin_resize(ResizeSession {
                        client: state.client,
                        start_pointer: pointer,
                        start_placement: PlacementSnapshot {
                            mode: state.placement.mode,
                            rect: state.placement.current,
                        },
                        edges,
                        grab_window: state.grab,
                    });
                    let effects = vec![FrameEffect::Grab {
                        window: state.grab,
                        cursor: cursor_for_region(FrameRegion::Resize(edges)),
                    }];
                    (next, effects)
                }
                FrameRegion::Control(control) => {
                    next.session.begin_control(ControlPressSession::new(
                        state.client,
                        control,
                        state.grab,
                    ));
                    let effects = vec![
                        FrameEffect::Grab {
                            window: state.grab,
                            cursor: cursor_for_region(FrameRegion::Control(control)),
                        },
                        FrameEffect::PaintChrome,
                    ];
                    (next, effects)
                }
                FrameRegion::Client => (next, vec![FrameEffect::Noop]),
            }
        }
        FrameEvent::Motion { pointer } => match state.session {
            InteractionSession::Move(m) => {
                let delta = (
                    pointer.x.saturating_sub(m.start_pointer.x),
                    pointer.y.saturating_sub(m.start_pointer.y),
                );
                let req = request(
                    state,
                    GeometryReason::InteractiveMove,
                    m.start_placement,
                    delta,
                    ResizeEdges::none(),
                );
                let plan = plan_move(&req);
                if plan.noop {
                    return (next, vec![FrameEffect::Noop]);
                }
                next.placement.current = plan.frame;
                (next, move_effects(plan.frame, state.extents))
            }
            InteractionSession::Resize(r) => {
                let delta = (
                    pointer.x.saturating_sub(r.start_pointer.x),
                    pointer.y.saturating_sub(r.start_pointer.y),
                );
                let req = request(
                    state,
                    GeometryReason::InteractiveResize,
                    r.start_placement,
                    delta,
                    r.edges,
                );
                let plan = plan_resize(&req);
                if plan.noop {
                    return (next, vec![FrameEffect::Noop]);
                }
                next.placement.current = plan.frame;
                (next, configure_effects(plan.frame, state.extents))
            }
            InteractionSession::ControlPress(_) | InteractionSession::Idle => {
                (next, vec![FrameEffect::Noop])
            }
        },
        FrameEvent::Release { pointer } => match state.session {
            InteractionSession::Move(m) => {
                let delta = (
                    pointer.x.saturating_sub(m.start_pointer.x),
                    pointer.y.saturating_sub(m.start_pointer.y),
                );
                let req = request(
                    state,
                    GeometryReason::InteractiveMove,
                    m.start_placement,
                    delta,
                    ResizeEdges::none(),
                );
                let plan = plan_move(&req);
                next.session = InteractionSession::Idle;
                if plan.noop && next.placement.current == m.start_placement.rect {
                    return (next, vec![FrameEffect::Ungrab]);
                }
                next.placement.current = plan.frame;
                // Final geometry first, Ungrab last.
                let mut effects = vec![FrameEffect::LayoutInput, FrameEffect::PaintChrome];
                effects.extend(move_effects(plan.frame, state.extents));
                effects.push(FrameEffect::Ungrab);
                (next, effects)
            }
            InteractionSession::Resize(r) => {
                let delta = (
                    pointer.x.saturating_sub(r.start_pointer.x),
                    pointer.y.saturating_sub(r.start_pointer.y),
                );
                let req = request(
                    state,
                    GeometryReason::InteractiveResize,
                    r.start_placement,
                    delta,
                    r.edges,
                );
                let plan = plan_resize(&req);
                next.session = InteractionSession::Idle;
                if plan.noop && next.placement.current == r.start_placement.rect {
                    return (next, vec![FrameEffect::Ungrab]);
                }
                next.placement.current = plan.frame;
                // Final geometry first, Ungrab last.
                let mut effects = vec![
                    FrameEffect::LayoutInput,
                    FrameEffect::PaintChrome,
                    FrameEffect::ApplyShape,
                ];
                effects.extend(configure_effects(plan.frame, state.extents));
                effects.push(FrameEffect::Ungrab);
                (next, effects)
            }
            InteractionSession::ControlPress(c) => {
                next.session = InteractionSession::Idle;
                if !c.armed {
                    return (next, vec![FrameEffect::Ungrab, FrameEffect::PaintChrome]);
                }
                match c.control {
                    FrameControl::MaximizeRestore => {
                        let (mut out, mut effects) = toggle_max(&next);
                        let mut head = vec![FrameEffect::Ungrab];
                        head.append(&mut effects);
                        out.session = InteractionSession::Idle;
                        (out, head)
                    }
                    FrameControl::Minimize | FrameControl::Close => {
                        (next, vec![FrameEffect::Ungrab, FrameEffect::PaintChrome])
                    }
                }
            }
            InteractionSession::Idle => (next, vec![FrameEffect::Noop]),
        },
        FrameEvent::Enter => {
            if matches!(next.session, InteractionSession::ControlPress(_)) {
                next.session.control_enter();
                (next, vec![FrameEffect::PaintChrome])
            } else {
                (next, vec![FrameEffect::Noop])
            }
        }
        FrameEvent::Leave => {
            if matches!(next.session, InteractionSession::ControlPress(_)) {
                next.session.control_leave();
                (next, vec![FrameEffect::PaintChrome])
            } else {
                (next, vec![FrameEffect::Noop])
            }
        }
        FrameEvent::Configure { origin, size } => {
            let req = request(
                state,
                GeometryReason::ClientConfigure,
                PlacementSnapshot {
                    mode: state.placement.mode,
                    rect: state.placement.current,
                },
                (0, 0),
                ResizeEdges::none(),
            );
            let plan = plan_client_configure(&req, origin, size);
            if plan.noop {
                return (next, vec![FrameEffect::Noop]);
            }
            next.placement.current = plan.frame;
            let mut effects = vec![FrameEffect::LayoutInput, FrameEffect::PaintChrome];
            effects.extend(configure_effects(plan.frame, state.extents));
            (next, effects)
        }
        FrameEvent::Snap { target } => {
            let rect = snap_rect(state.work_area, target);
            next.session = InteractionSession::Idle;
            if next.placement.mode != PlacementMode::Floating {
                next.placement.resume = Some(PlacementSnapshot {
                    mode: next.placement.mode,
                    rect: next.placement.current,
                });
            } else {
                next.placement.resume = Some(PlacementSnapshot {
                    mode: PlacementMode::Floating,
                    rect: next.placement.floating_restore,
                });
            }
            next.placement.mode = PlacementMode::Snapped(target);
            next.placement.current = rect;
            // Final geometry first, Ungrab last.
            let mut effects = vec![
                FrameEffect::LayoutInput,
                FrameEffect::PaintChrome,
                FrameEffect::ApplyShape,
                FrameEffect::SnapPreview {
                    target: Some(target),
                },
            ];
            effects.extend(configure_effects(rect, state.extents));
            effects.push(FrameEffect::Ungrab);
            (next, effects)
        }
        FrameEvent::ToggleMax => {
            let (out, effects) = toggle_max(&next);
            (out, effects)
        }
        FrameEvent::ToggleFullscreen => {
            let (out, effects) = toggle_fullscreen(&next);
            (out, effects)
        }
        FrameEvent::Cancel => {
            if state.session.is_idle() {
                (next, vec![FrameEffect::Noop])
            } else {
                next.session.cancel();
                (
                    next,
                    vec![
                        FrameEffect::Ungrab,
                        FrameEffect::SnapPreview { target: None },
                    ],
                )
            }
        }
    }
}

fn toggle_max(state: &ControllerState) -> (ControllerState, Vec<FrameEffect>) {
    let mut next = state.clone();
    next.session = InteractionSession::Idle;
    if state.placement.mode == PlacementMode::Maximized {
        let snap = state.placement.resume.unwrap_or(PlacementSnapshot {
            mode: PlacementMode::Floating,
            rect: state.placement.floating_restore,
        });
        next.placement.mode = snap.mode;
        next.placement.current = snap.rect;
        next.placement.resume = None;
    } else {
        next.placement.resume = Some(PlacementSnapshot {
            mode: state.placement.mode,
            rect: state.placement.current,
        });
        if state.placement.mode == PlacementMode::Floating {
            next.placement.floating_restore = state.placement.current;
        }
        next.placement.mode = PlacementMode::Maximized;
        next.placement.current = state.work_area;
    }
    let frame = next.placement.current;
    // Final geometry first, Ungrab last.
    let mut effects = vec![
        FrameEffect::LayoutInput,
        FrameEffect::PaintChrome,
        FrameEffect::ApplyShape,
    ];
    effects.extend(configure_effects(frame, state.extents));
    effects.push(FrameEffect::Ungrab);
    (next, effects)
}

fn toggle_fullscreen(state: &ControllerState) -> (ControllerState, Vec<FrameEffect>) {
    let mut next = state.clone();
    next.session = InteractionSession::Idle;
    if state.placement.mode == PlacementMode::Fullscreen {
        let snap = state.placement.resume.unwrap_or(PlacementSnapshot {
            mode: PlacementMode::Floating,
            rect: state.placement.floating_restore,
        });
        next.placement.mode = snap.mode;
        next.placement.current = snap.rect;
        next.placement.resume = None;
    } else {
        next.placement.resume = Some(PlacementSnapshot {
            mode: state.placement.mode,
            rect: state.placement.current,
        });
        if state.placement.mode == PlacementMode::Floating {
            next.placement.floating_restore = state.placement.current;
        }
        next.placement.mode = PlacementMode::Fullscreen;
        next.placement.current = state.screen_rect;
    }
    let frame = next.placement.current;
    // Final geometry first, Ungrab last.
    let mut effects = vec![
        FrameEffect::LayoutInput,
        FrameEffect::PaintChrome,
        FrameEffect::ApplyShape,
    ];
    effects.extend(configure_effects(frame, state.extents));
    effects.push(FrameEffect::Ungrab);
    (next, effects)
}

#[cfg(test)]
mod tests {
    use super::super::geometry::FrameExtents;
    use super::*;

    const WA: RootRect = RootRect::new(0, 0, 1920, 1080);
    const EXT: FrameExtents = FrameExtents::new(31, 1);
    const START: RootRect = RootRect::new(100, 100, 400, 300);

    const SCREEN: RootRect = RootRect::new(0, 0, 1920, 1080);

    fn floating_state() -> ControllerState {
        let placement = PlacementState {
            mode: PlacementMode::Floating,
            current: START,
            floating_restore: START,
            resume: None,
        };
        let mut registry = HashMap::new();
        registry.insert(1, FrameRegion::TitleDrag);
        registry.insert(2, FrameRegion::Resize(ResizeEdges::bottom_right()));
        registry.insert(3, FrameRegion::Control(FrameControl::MaximizeRestore));
        ControllerState::new(7, 71, placement, WA, ClientSizeHints::default(), EXT)
            .with_screen_rect(SCREEN)
            .with_registry(registry)
    }

    fn has_frame(target: &[FrameEffect], x: i32, y: i32) -> bool {
        target.iter().any(|e| {
            matches!(
                e,
                FrameEffect::ConfigureFrameRoot { x: fx, y: fy, .. } if *fx == x && *fy == y
            )
        })
    }

    fn count_local(target: &[FrameEffect]) -> usize {
        target
            .iter()
            .filter(|e| matches!(e, FrameEffect::ConfigureClientLocal { .. }))
            .count()
    }

    #[test]
    fn press_motion_release_move() {
        let s0 = floating_state();
        let (s1, e1) = reduce(
            &s0,
            FrameEvent::Press {
                source: 1,
                pointer: RootPoint::new(150, 120),
            },
        );
        assert!(matches!(s1.session, InteractionSession::Move(_)));
        assert!(e1.iter().any(|e| matches!(e, FrameEffect::Grab { .. })));
        let (s2, e2) = reduce(
            &s1,
            FrameEvent::Motion {
                pointer: RootPoint::new(200, 170),
            },
        );
        assert_eq!(s2.placement.current, RootRect::new(150, 150, 400, 300));
        assert!(has_frame(&e2, 150, 150));
        let (s3, e3) = reduce(
            &s2,
            FrameEvent::Release {
                pointer: RootPoint::new(200, 170),
            },
        );
        assert!(s3.session.is_idle());
        assert!(e3.contains(&FrameEffect::Ungrab));
        assert_eq!(s3.placement.current, RootRect::new(150, 150, 400, 300));
    }

    #[test]
    fn press_motion_release_resize() {
        let s0 = floating_state();
        let (s1, e1) = reduce(
            &s0,
            FrameEvent::Press {
                source: 2,
                pointer: RootPoint::new(500, 400),
            },
        );
        assert!(matches!(s1.session, InteractionSession::Resize(_)));
        assert!(e1.iter().any(|e| matches!(e, FrameEffect::Grab { .. })));
        let (s2, _) = reduce(
            &s1,
            FrameEvent::Motion {
                pointer: RootPoint::new(550, 460),
            },
        );
        assert_eq!(s2.placement.current, RootRect::new(100, 100, 450, 360));
        let (s3, e3) = reduce(
            &s2,
            FrameEvent::Release {
                pointer: RootPoint::new(550, 460),
            },
        );
        assert!(s3.session.is_idle());
        assert!(e3.contains(&FrameEffect::Ungrab));
        assert!(e3.contains(&FrameEffect::ApplyShape));
        assert_eq!(s3.placement.current, RootRect::new(100, 100, 450, 360));
    }

    #[test]
    fn control_click_toggles_max() {
        let s0 = floating_state();
        let (s1, _) = reduce(
            &s0,
            FrameEvent::Press {
                source: 3,
                pointer: RootPoint::new(300, 110),
            },
        );
        assert!(matches!(s1.session, InteractionSession::ControlPress(_)));
        let (s2, e2) = reduce(
            &s1,
            FrameEvent::Release {
                pointer: RootPoint::new(300, 110),
            },
        );
        assert_eq!(s2.placement.mode, PlacementMode::Maximized);
        assert_eq!(s2.placement.current, WA);
        assert!(e2.contains(&FrameEffect::Ungrab));
        assert!(has_frame(&e2, 0, 0));
    }

    #[test]
    fn cancel_aborts_move() {
        let s0 = floating_state();
        let (s1, _) = reduce(
            &s0,
            FrameEvent::Press {
                source: 1,
                pointer: RootPoint::new(150, 120),
            },
        );
        let (s2, e2) = reduce(&s1, FrameEvent::Cancel);
        assert!(s2.session.is_idle());
        assert_eq!(s2.placement.current, START);
        assert!(e2.contains(&FrameEffect::Ungrab));
        let (_, e3) = reduce(&s2, FrameEvent::Cancel);
        assert_eq!(e3, vec![FrameEffect::Noop]);
    }

    #[test]
    fn snap_finalizes_left_half() {
        let s0 = floating_state();
        let (s1, e1) = reduce(
            &s0,
            FrameEvent::Snap {
                target: SnapTarget::Left,
            },
        );
        assert_eq!(s1.placement.mode, PlacementMode::Snapped(SnapTarget::Left));
        assert_eq!(s1.placement.current, RootRect::new(0, 0, 960, 1080));
        assert!(e1.contains(&FrameEffect::SnapPreview {
            target: Some(SnapTarget::Left)
        }));
        assert!(has_frame(&e1, 0, 0));
    }

    #[test]
    fn max_restore_returns_to_floating() {
        let s0 = floating_state();
        let (s1, _) = reduce(&s0, FrameEvent::ToggleMax);
        assert_eq!(s1.placement.mode, PlacementMode::Maximized);
        let (s2, e2) = reduce(&s1, FrameEvent::ToggleMax);
        assert_eq!(s2.placement.mode, PlacementMode::Floating);
        assert_eq!(s2.placement.current, START);
        assert!(has_frame(&e2, 100, 100));
    }

    #[test]
    fn noop_geometry_yields_noop() {
        let s0 = floating_state();
        // Motion with zero delta while moving: plan equals start -> Noop.
        let (s1, _) = reduce(
            &s0,
            FrameEvent::Press {
                source: 1,
                pointer: RootPoint::new(150, 120),
            },
        );
        let (s2, e2) = reduce(
            &s1,
            FrameEvent::Motion {
                pointer: RootPoint::new(150, 120),
            },
        );
        assert_eq!(e2, vec![FrameEffect::Noop]);
        assert_eq!(s2.placement.current, START);
        // Bare configure with no movement on idle session -> Noop path.
        let (_, e3) = reduce(
            &s0,
            FrameEvent::Motion {
                pointer: RootPoint::new(0, 0),
            },
        );
        assert_eq!(e3, vec![FrameEffect::Noop]);
    }

    #[test]
    fn effect_order_manage_chrome_paint_retarget_first_flush_last() {
        use super::super::chrome::{effects_for, plan_scene};
        use super::super::resources::{CONTROL_ORDER, FrameResources, RESIZE_ORDER};
        let res = FrameResources::new(7, 70, 71, [72, 73, 74], [80, 81, 82, 83, 84, 85, 86, 87]);
        let children = res.children();
        assert_eq!(children.len(), 12);
        assert_eq!(children[8], 71);
        assert_eq!(&children[9..12], &[72, 73, 74]);
        assert_eq!(RESIZE_ORDER.len(), 8);
        assert_eq!(CONTROL_ORDER.len(), 3);
        let scene = plan_scene(400, 300, "app", true, None, None, false, false);
        let fx = effects_for(70, &scene);
        let kind = |e: &super::super::chrome::ChromeEffect| match e {
            super::super::chrome::ChromeEffect::Retarget { .. } => 0,
            super::super::chrome::ChromeEffect::FillTitlebar { .. } => 1,
            super::super::chrome::ChromeEffect::DrawTitle { .. } => 2,
            super::super::chrome::ChromeEffect::BlitIcon => 3,
            super::super::chrome::ChromeEffect::BlitControls { .. } => 4,
            super::super::chrome::ChromeEffect::ApplyShape { .. } => 5,
            super::super::chrome::ChromeEffect::Flush => 6,
        };
        let order: Vec<i32> = fx.iter().map(kind).collect();
        assert_eq!(*order.first().expect("nonempty"), 0);
        assert_eq!(*order.last().expect("nonempty"), 6);
        let mut sorted = order.clone();
        sorted.sort_unstable();
        assert_eq!(
            order, sorted,
            "chrome effects must be monotonically ordered"
        );
    }

    #[test]
    fn effect_order_move_motion_has_no_layout_or_paint() {
        let s0 = floating_state();
        let (s1, _) = reduce(
            &s0,
            FrameEvent::Press {
                source: 1,
                pointer: RootPoint::new(150, 120),
            },
        );
        let (_, e2) = reduce(
            &s1,
            FrameEvent::Motion {
                pointer: RootPoint::new(200, 170),
            },
        );
        assert_eq!(e2.len(), 2);
        assert!(matches!(
            e2[0],
            FrameEffect::ConfigureFrameRoot { x: 150, y: 150, .. }
        ));
        // Client local origin is (0,titlebar); synthetic root notify at frame offset.
        assert!(matches!(
            e2[1],
            FrameEffect::NotifyClientRoot {
                x: 150,
                y: 181,
                w: 400,
                h: 269
            }
        ));
        assert_eq!(count_local(&e2), 0);
        assert!(!e2.contains(&FrameEffect::LayoutInput));
        assert!(!e2.contains(&FrameEffect::PaintChrome));
        assert!(!e2.contains(&FrameEffect::ApplyShape));
        assert!(!e2.contains(&FrameEffect::Ungrab));
    }

    #[test]
    fn effect_order_resize_release_has_frame_client_layout_chrome() {
        let s0 = floating_state();
        let (s1, _) = reduce(
            &s0,
            FrameEvent::Press {
                source: 2,
                pointer: RootPoint::new(500, 400),
            },
        );
        let (s2, e_motion) = reduce(
            &s1,
            FrameEvent::Motion {
                pointer: RootPoint::new(550, 460),
            },
        );
        // Resize motion carries frame + local + root notify.
        assert_eq!(count_local(&e_motion), 1);
        assert!(
            e_motion
                .iter()
                .any(|e| matches!(e, FrameEffect::ConfigureClientLocal { x: 0, y: 31, .. }))
        );
        let (_, e3) = reduce(
            &s2,
            FrameEvent::Release {
                pointer: RootPoint::new(550, 460),
            },
        );
        assert!(e3.contains(&FrameEffect::LayoutInput));
        assert!(e3.contains(&FrameEffect::PaintChrome));
        assert!(e3.contains(&FrameEffect::ApplyShape));
        assert!(
            e3.iter()
                .any(|e| matches!(e, FrameEffect::ConfigureFrameRoot { .. }))
        );
        assert!(
            e3.iter()
                .any(|e| matches!(e, FrameEffect::ConfigureClientLocal { .. }))
        );
        let ungrab = e3
            .iter()
            .position(|e| *e == FrameEffect::Ungrab)
            .expect("ungrab");
        let layout = e3
            .iter()
            .position(|e| *e == FrameEffect::LayoutInput)
            .expect("layout");
        let paint = e3
            .iter()
            .position(|e| *e == FrameEffect::PaintChrome)
            .expect("paint");
        let frame = e3
            .iter()
            .position(|e| matches!(e, FrameEffect::ConfigureFrameRoot { .. }))
            .expect("frame");
        assert_eq!(ungrab, e3.len() - 1, "geometry precedes Ungrab on release");
        assert!(layout < paint && paint < frame && frame < ungrab);
    }

    #[test]
    fn effect_order_release_geometry_precedes_ungrab() {
        let s0 = floating_state();
        let (s1, _) = reduce(
            &s0,
            FrameEvent::Press {
                source: 1,
                pointer: RootPoint::new(150, 120),
            },
        );
        let (s2, _) = reduce(
            &s1,
            FrameEvent::Motion {
                pointer: RootPoint::new(200, 170),
            },
        );
        let (s3, e3) = reduce(
            &s2,
            FrameEvent::Release {
                pointer: RootPoint::new(200, 170),
            },
        );
        assert_eq!(s3.placement.current, RootRect::new(150, 150, 400, 300));
        assert_eq!(*e3.last().expect("nonempty"), FrameEffect::Ungrab);
        let tail_frame = e3
            .iter()
            .find_map(|e| match *e {
                FrameEffect::ConfigureFrameRoot { x, y, w, h } => Some((x, y, w, h)),
                _ => None,
            })
            .expect("final frame geometry");
        assert_eq!(tail_frame, (150, 150, 400, 300));
        let frame_pos = e3
            .iter()
            .position(|e| matches!(e, FrameEffect::ConfigureFrameRoot { .. }))
            .expect("frame");
        let ungrab_pos = e3
            .iter()
            .position(|e| *e == FrameEffect::Ungrab)
            .expect("ungrab");
        assert!(frame_pos < ungrab_pos);
    }

    #[test]
    fn move_thousand_samples_zero_client_local() {
        let s0 = floating_state();
        let (s1, _) = reduce(
            &s0,
            FrameEvent::Press {
                source: 1,
                pointer: RootPoint::new(150, 120),
            },
        );
        let mut total_local = 0;
        for i in 0..1000 {
            let dx = (i % 100) - 50;
            let dy = ((i * 7) % 100) - 50;
            let (_, e) = reduce(
                &s1,
                FrameEvent::Motion {
                    pointer: RootPoint::new(150 + dx, 120 + dy),
                },
            );
            total_local += count_local(&e);
        }
        assert_eq!(total_local, 0);
    }

    #[test]
    fn move_frame_at_origin_client_local_and_root() {
        // Start (100,100) + delta (300,100) => frame (400,200); client local
        // stays (0,titlebar), synthetic root = (400,231) size (400,269).
        let s0 = floating_state();
        let (s1, _) = reduce(
            &s0,
            FrameEvent::Press {
                source: 1,
                pointer: RootPoint::new(150, 120),
            },
        );
        let (s2, e2) = reduce(
            &s1,
            FrameEvent::Motion {
                pointer: RootPoint::new(450, 220),
            },
        );
        assert_eq!(s2.placement.current, RootRect::new(400, 200, 400, 300));
        assert!(matches!(
            e2[1],
            FrameEffect::NotifyClientRoot {
                x: 400,
                y: 231,
                w: 400,
                h: 269
            }
        ));
    }

    #[test]
    fn resize_right_and_bottom_right_local_origin_constant() {
        let s0 = floating_state();
        let (s1, _) = reduce(
            &s0,
            FrameEvent::Press {
                source: 2,
                pointer: RootPoint::new(500, 400),
            },
        );
        let (_, e) = reduce(
            &s1,
            FrameEvent::Motion {
                pointer: RootPoint::new(550, 460),
            },
        );
        assert!(
            e.iter()
                .any(|fx| matches!(fx, FrameEffect::ConfigureClientLocal { x: 0, y: 31, .. }))
        );
    }

    #[test]
    fn fullscreen_uses_screen_rect_and_restores_exact_snapshot() {
        let mut s0 = floating_state();
        s0.work_area = RootRect::new(0, 24, 1920, 1056);
        s0.screen_rect = RootRect::new(0, 0, 1920, 1080);
        let (s1, e1) = reduce(&s0, FrameEvent::ToggleFullscreen);
        assert_eq!(s1.placement.mode, PlacementMode::Fullscreen);
        assert_eq!(s1.placement.current, RootRect::new(0, 0, 1920, 1080));
        assert!(has_frame(&e1, 0, 0));
        let (s2, _) = reduce(&s0, FrameEvent::ToggleMax);
        assert_eq!(s2.placement.current, RootRect::new(0, 24, 1920, 1056));
        let (s3, _) = reduce(&s1, FrameEvent::ToggleFullscreen);
        assert_eq!(s3.placement.mode, PlacementMode::Floating);
        assert_eq!(s3.placement.current, START);
    }

    #[test]
    fn eight_edges_hold_opposite_anchor() {
        use super::super::geometry::{GeometryReason, GeometryRequest, plan_resize};
        use super::super::model::PlacementSnapshot;
        let edges = [
            ResizeEdges::left(),
            ResizeEdges::right(),
            ResizeEdges::top(),
            ResizeEdges::bottom(),
            ResizeEdges::top_left(),
            ResizeEdges::top_right(),
            ResizeEdges::bottom_left(),
            ResizeEdges::bottom_right(),
        ];
        for e in edges {
            let r = GeometryRequest {
                reason: GeometryReason::InteractiveResize,
                start: PlacementSnapshot {
                    mode: PlacementMode::Floating,
                    rect: START,
                },
                pointer_delta: (40, 30),
                edges: e,
                work_area: WA,
                hints: ClientSizeHints::default(),
                frame_extents: EXT,
            };
            let p = plan_resize(&r);
            if !e.left {
                assert_eq!(p.frame.x, START.x, "left anchor {e:?}");
            }
            if !e.right && !e.left {
                assert_eq!(p.frame.w, START.w, "width {e:?}");
            }
            if !e.top {
                assert_eq!(p.frame.y, START.y, "top anchor {e:?}");
            }
        }
    }

    #[test]
    fn border_does_not_change_local_geometry() {
        use super::super::geometry::frame_to_client_local;
        let thin = FrameExtents::new(31, 0);
        let thick = FrameExtents::new(31, 1);
        assert_eq!(
            frame_to_client_local(START, thin),
            frame_to_client_local(START, thick)
        );
    }
}
