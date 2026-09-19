//! Interaction orchestration: native pointer transaction / grab / effect-commit order.
//!
//! Pure orchestration over the frame reducer. No X calls, no x11rb, no geometry
//! math here: geometry lives in `frame::geometry` via `controller::reduce`.
//! The live `wm` owner applies the returned [`PointerCommit`] against the
//! server; J19 wires this module without signature churn.

use std::collections::HashMap;

use crate::client::ManagedClient;
use crate::frame::controller::{ControllerState, FrameEffect, FrameEvent, reduce};
use crate::frame::coords::RootRect;
use crate::frame::geometry::FrameExtents;
use crate::frame::model::{FrameRegion, PlacementState};
use crate::size_hints::ClientSizeHints;

/// Native pointer-commit order derived from one reducer effect plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointerCommit {
    /// Grab target + cursor when the plan opens with `Grab`.
    pub grab: Option<(u32, flamewm_render_core::CursorKind)>,
    /// Plan ends with (or contains) `Ungrab`.
    pub ungrab: bool,
    /// Grab failed at commit time: session must cancel to idle.
    pub abort_on_grab_fail: bool,
    /// Ordered effect plan, verbatim from `reduce`.
    pub effects: Vec<FrameEffect>,
}

impl PointerCommit {
    #[must_use]
    pub fn plan(effects: Vec<FrameEffect>) -> Self {
        let mut grab = None;
        let mut ungrab = false;
        for effect in &effects {
            match *effect {
                FrameEffect::Grab { window, cursor } => {
                    if grab.is_none() {
                        grab = Some((window, cursor));
                    }
                }
                FrameEffect::Ungrab => ungrab = true,
                _ => {}
            }
        }
        Self {
            grab,
            ungrab,
            abort_on_grab_fail: grab.is_some(),
            effects,
        }
    }
}

/// Build the region view for one client from the shared registry map.
#[must_use]
pub fn region_map_for(
    by_xid: &HashMap<u32, FrameRegion>,
    client_children: &[u32],
) -> HashMap<u32, FrameRegion> {
    let mut out = HashMap::with_capacity(client_children.len());
    for xid in client_children {
        if let Some(region) = by_xid.get(xid).copied() {
            out.insert(*xid, region);
        }
    }
    out
}

/// Build a controller snapshot from explicit live state. No hidden reads.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn build_controller(
    client: u32,
    grab: u32,
    placement: PlacementState,
    work_area: RootRect,
    screen_rect: RootRect,
    hints: ClientSizeHints,
    extents: FrameExtents,
    session: crate::frame::session::InteractionSession,
    region_map: HashMap<u32, FrameRegion>,
) -> ControllerState {
    let mut controller = ControllerState::new(client, grab, placement, work_area, hints, extents)
        .with_screen_rect(screen_rect)
        .with_registry(region_map);
    controller.session = session;
    controller
}

/// One reducer step plus its pointer-commit order. Pure.
#[must_use]
pub fn step(state: &ControllerState, event: FrameEvent) -> (ControllerState, PointerCommit) {
    let (next, effects) = reduce(state, event);
    let commit = PointerCommit::plan(effects);
    (next, commit)
}

/// Commit the next placement/session into the session map and client mirror.
pub fn store_controller(
    sessions: &mut HashMap<u32, ControllerState>,
    client_state: Option<&mut ManagedClient>,
    client: u32,
    next: &ControllerState,
) {
    let entry = sessions.entry(client).or_insert_with(|| next.clone());
    entry.session = next.session;
    entry.placement = next.placement;
    if let Some(state) = client_state {
        state.apply_placement(next.placement);
    }
}
