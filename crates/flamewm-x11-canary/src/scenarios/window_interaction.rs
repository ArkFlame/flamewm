//! T06 real move (real-pointer drag) and T07 real 8-way resize.

use super::{real_drag, real_warp};

pub(crate) fn resolve_target(canary: &crate::Canary, args: &[String]) -> Option<crate::WinInfo> {
    let timeout_secs = crate::arg(args, "--timeout-secs")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(3)
        .clamp(1, 30);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        let candidate = if let Some(id) = crate::arg(args, "--window") {
            id.parse::<u32>()
                .ok()
                .and_then(|window| canary.by_id(window))
        } else {
            let name = crate::arg(args, "--name").unwrap_or_else(|| "flame".to_owned());
            canary
                .find(&name)
                .into_iter()
                .find(|window| managed_viewable(canary, window))
        };
        if let Some(window) = candidate.filter(|window| managed_viewable(canary, window)) {
            return Some(window);
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// A client is interaction-ready only after the WM has reparented it into a
/// viewable, non-empty frame.  Resolving the client alone can observe the
/// short-lived 1x1/unmapped state during application startup.
fn managed_viewable(canary: &crate::Canary, window: &crate::WinInfo) -> bool {
    window.parent != canary.root
        && window.map_state == 2
        && window.width > 0
        && window.height > 0
        && canary.by_id(window.parent).is_some_and(|frame| {
            frame.parent == canary.root
                && frame.map_state == 2
                && frame.width > 0
                && frame.height > 0
        })
}

/// Return the managed frame's root-relative rectangle for real pointer input.
/// Client geometry is parent-relative, so using it as root coordinates can
/// place XTEST presses in the client body instead of the frame/title region.
pub(crate) fn input_surface(
    canary: &crate::Canary,
    client: &crate::WinInfo,
) -> Option<crate::WinInfo> {
    canary
        .by_id(client.parent)
        .filter(|frame| frame.parent == canary.root && managed_viewable(canary, client))
}

fn diagnostic(detail: impl Into<String>) -> (bool, String) {
    (
        true,
        format!(
            "classification=DIAGNOSTIC product_gate=none {}",
            detail.into()
        ),
    )
}

/// T06: real-pointer drag moves the window; geometry delta is diagnostic only.
pub fn run_t06(canary: &crate::Canary, args: &[String]) -> (bool, String) {
    let Some(work_area) = super::interaction_regressions::root_work_area(canary) else {
        return diagnostic("observed=false work area unavailable path=real-pointer");
    };
    let Some(target) = resolve_target(canary, args) else {
        return diagnostic("observed=false no target path=real-pointer");
    };
    let reset_floating =
        super::interaction_regressions::establish_known_floating(canary, &target, work_area);
    let before = match canary.by_id(target.id) {
        Some(w) => w,
        None => {
            return diagnostic(format!(
                "observed=false target {} gone path=real-pointer",
                target.id
            ));
        }
    };
    let Some(surface) = input_surface(canary, &before) else {
        return diagnostic(format!(
            "observed=false target {} frame not ready path=real-pointer",
            target.id
        ));
    };
    // Mirror the canonical frame layout: the title-drag child is the
    // titlebar band below the 6px top strip and left of the 114px controls.
    const TITLEBAR_H: i16 = 31;
    const CONTROLS_WIDTH: i16 = 114;
    const RESIZE_T: i16 = 6;
    let title_drag_width =
        (surface.width as i16).saturating_sub(RESIZE_T.saturating_mul(2) + CONTROLS_WIDTH);
    let from = (
        surface.x.saturating_add(RESIZE_T + title_drag_width / 2),
        surface
            .y
            .saturating_add(RESIZE_T + (TITLEBAR_H - RESIZE_T) / 2),
    );
    let to = (from.0.saturating_add(64), from.1.saturating_add(48));
    if !real_warp(canary, from.0, from.1) {
        return diagnostic(format!(
            "observed=false id={} warp failed path=real-pointer",
            target.id
        ));
    }
    let dragged = real_drag(canary, from, to, 1, 8);
    let moved = reset_floating
        && dragged
        && super::interaction_regressions::wait_until(750, || {
            canary
                .by_id(target.id)
                .and_then(|after| input_surface(canary, &after))
                .is_some_and(|after| after.x != surface.x || after.y != surface.y)
        });
    let after = canary.by_id(target.id);
    let (observed, detail) = match after {
        Some(w) => {
            let before_frame = input_surface(canary, &before);
            let after_frame = input_surface(canary, &w);
            let (dx, dy) = match (before_frame.as_ref(), after_frame.as_ref()) {
                (Some(before), Some(after)) => (
                    after.x.wrapping_sub(before.x),
                    after.y.wrapping_sub(before.y),
                ),
                _ => (0, 0),
            };
            let moved = moved && (dx != 0 || dy != 0);
            (
                moved,
                format!(
                    "id={} frame_before={}x{}+{}+{} frame_after={}x{}+{}+{} dx={dx} dy={dy} dragged={dragged} path=real-pointer",
                    target.id,
                    before_frame.as_ref().map_or(0, |frame| frame.width),
                    before_frame.as_ref().map_or(0, |frame| frame.height),
                    before_frame.as_ref().map_or(0, |frame| frame.x),
                    before_frame.as_ref().map_or(0, |frame| frame.y),
                    after_frame.as_ref().map_or(0, |frame| frame.width),
                    after_frame.as_ref().map_or(0, |frame| frame.height),
                    after_frame.as_ref().map_or(0, |frame| frame.x),
                    after_frame.as_ref().map_or(0, |frame| frame.y),
                ),
            )
        }
        None => (
            false,
            format!("id={} gone after drag path=real-pointer", target.id),
        ),
    };
    diagnostic(format!("observed={observed} {detail}"))
}

/// T07: real 8-way resize via corner/edge drags; at least one drag changes
/// geometry and the pure 6px edge contract still classifies all 8 points.
pub fn run_t07(canary: &crate::Canary, args: &[String]) -> (bool, String) {
    let Some(target) = resolve_target(canary, args) else {
        return (false, "no target path=real-pointer".to_owned());
    };
    let before = match canary.by_id(target.id) {
        Some(w) => w,
        None => {
            return (
                false,
                format!("target {} gone path=real-pointer", target.id),
            );
        }
    };
    let Some(surface) = input_surface(canary, &before) else {
        return (
            false,
            format!("target {} frame not ready path=real-pointer", target.id),
        );
    };
    // 8 edge/corner grips in root coordinates.
    let x0 = surface.x;
    let y0 = surface.y;
    let x1 = surface.x.saturating_add(surface.width as i16);
    let y1 = surface.y.saturating_add(surface.height as i16);
    let cx = surface.x.saturating_add(surface.width as i16 / 2);
    let cy = surface.y.saturating_add(surface.height as i16 / 2);
    let grips: [(i16, i16, i16, i16); 8] = [
        (cx, y0 + 2, 0, -24),
        (cx, y1 - 3, 0, 24),
        (x0 + 2, cy, -24, 0),
        (x1 - 3, cy, 24, 0),
        (x0 + 2, y0 + 2, -20, -20),
        (x1 - 3, y0 + 2, 20, -20),
        (x0 + 2, y1 - 3, -20, 20),
        (x1 - 3, y1 - 3, 20, 20),
    ];
    let mut drags = 0;
    let mut changed = 0;
    let mut cur = before.clone();
    for (gx, gy, dx, dy) in grips {
        let to = (gx.saturating_add(dx), gy.saturating_add(dy));
        if real_drag(canary, (gx, gy), to, 1, 6) {
            drags += 1;
        }
        std::thread::sleep(std::time::Duration::from_millis(120));
        if let Some(w) = canary.by_id(target.id) {
            if w.width != cur.width || w.height != cur.height {
                changed += 1;
                cur = w;
            }
        }
    }
    // Pure 6px contract check on the final geometry (mirrors product edge).
    let edge = |w: u32, h: u32, x: i16, y: i16| {
        let w = w.clamp(1, u32::from(u16::MAX)) as i16;
        let h = h.clamp(1, u32::from(u16::MAX)) as i16;
        x <= 6 || x >= w.saturating_sub(6) || y <= 6 || y >= h.saturating_sub(6)
    };
    let mut matched = 0;
    if cur.width > 0 && cur.height > 0 {
        let w = cur.width as i16;
        let h = cur.height as i16;
        let pts = [
            (w / 2, 2),
            (w / 2, h - 3),
            (2, h / 2),
            (w - 3, h / 2),
            (2, 2),
            (w - 3, 2),
            (2, h - 3),
            (w - 3, h - 3),
        ];
        for (x, y) in pts {
            if edge(cur.width.into(), cur.height.into(), x, y) {
                matched += 1;
            }
        }
    }
    let pass = drags >= 6 && changed >= 1 && matched == 8;
    (
        pass,
        format!(
            "id={} drags={drags}/8 geom_changed={changed} edges={matched}/8 final={}x{} path=real-pointer",
            target.id, cur.width, cur.height
        ),
    )
}
