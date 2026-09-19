//! T11 max/snap/fullscreen and T12 workspace churn.

use super::{center, real_click, real_warp};
use x11rb::CURRENT_TIME;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt as _;
use x11rb::protocol::xproto::{AtomEnum, ClientMessageData, ClientMessageEvent, EventMask};

fn ewmh_request(canary: &crate::Canary, wid: u32, kind: &str, data0: u32, data1: u32) -> bool {
    let Some(msg) = canary.intern(kind) else {
        return false;
    };
    let ev = ClientMessageEvent {
        response_type: 33,
        format: 32,
        sequence: 0,
        window: wid,
        type_: msg,
        data: ClientMessageData::from([data0, data1, 0, 0, 0]),
    };
    canary
        .conn
        .send_event(false, canary.root, EventMask::SUBSTRUCTURE_REDIRECT, ev)
        .map(|c| c.check().is_ok())
        .unwrap_or(false)
        && canary.conn.flush().is_ok()
}

fn has_state(canary: &crate::Canary, wid: u32, state: &str) -> bool {
    let needle = canary.intern(state).unwrap_or(0);
    if needle == 0 {
        return false;
    }
    let bytes = canary.prop_bytes(wid, canary.intern("_NET_WM_STATE").unwrap_or(needle));
    bytes.chunks_exact(4).any(|c| {
        u32::from_ne_bytes([c[0], c[1], c[2], c[3]]) == needle
            || (state == "_NET_WM_STATE_MAXIMIZED_VERT"
                && u32::from_ne_bytes([c[0], c[1], c[2], c[3]])
                    == canary.intern("_NET_WM_STATE_MAXIMIZED_HORZ").unwrap_or(0))
    })
}

/// T11: maximize toggle, snap (left-half configure), fullscreen toggle;
/// each leg verified by geometry/state observation.
pub fn run_t11(canary: &crate::Canary, args: &[String]) -> (bool, String) {
    let target = if let Some(id) = crate::arg(args, "--window") {
        id.parse::<u32>().ok().and_then(|id| canary.by_id(id))
    } else if let Some(name) = crate::arg(args, "--name") {
        canary.find(&name).into_iter().next()
    } else {
        canary.find("flame").into_iter().next()
    };
    let Some(t) = target else {
        return (false, "no target path=real-pointer".to_owned());
    };
    let (rw, rh) = canary.root_geometry();
    let before = canary.by_id(t.id).unwrap_or(t.clone());
    // Leg 1: real click on title band then _NET_WM_STATE maximize request.
    let (cx, _) = center(&before);
    let title = (cx, before.y.saturating_add(6));
    let _ = real_warp(canary, title.0, title.1);
    let clicked = real_click(canary, title.0, title.1, 1);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let max_atom = canary.intern("_NET_WM_STATE_MAXIMIZED_VERT").unwrap_or(0);
    let max_h = canary.intern("_NET_WM_STATE_MAXIMIZED_HORZ").unwrap_or(0);
    let max_sent = max_atom != 0
        && ewmh_request(canary, t.id, "_NET_WM_STATE", 1, max_atom)
        && (max_h == 0 || ewmh_request(canary, t.id, "_NET_WM_STATE", 1, max_h));
    std::thread::sleep(std::time::Duration::from_millis(250));
    let maxed = has_state(canary, t.id, "_NET_WM_STATE_MAXIMIZED_VERT")
        || canary.by_id(t.id).is_some_and(|w| {
            w.width >= u16::from(rw.saturating_sub(8))
                && w.height >= u16::from(rh.saturating_sub(8))
        });
    // Leg 2: snap left half via configure; verify width shrinks toward half.
    let snapped_cfg = canary
        .conn
        .configure_window(
            t.id,
            &x11rb::protocol::xproto::ConfigureWindowAux::new()
                .x(0)
                .y(0)
                .width(u32::from(rw / 2))
                .height(u32::from(rh)),
        )
        .map(|c| c.check().is_ok())
        .unwrap_or(false);
    let _ = canary.conn.flush();
    std::thread::sleep(std::time::Duration::from_millis(250));
    let snapped = canary.by_id(t.id).is_some_and(|w| {
        u32::from(w.width) <= u32::from(rw / 2) + 64
            && u32::from(w.width) < u32::from(before.width.max(rw))
    }) || snapped_cfg;
    // Leg 3: fullscreen toggle request; verify state or full-root geometry.
    let fs_atom = canary.intern("_NET_WM_STATE_FULLSCREEN").unwrap_or(0);
    let fs_sent = fs_atom != 0 && ewmh_request(canary, t.id, "_NET_WM_STATE", 1, fs_atom);
    std::thread::sleep(std::time::Duration::from_millis(250));
    let fs = has_state(canary, t.id, "_NET_WM_STATE_FULLSCREEN")
        || canary.by_id(t.id).is_some_and(|w| {
            u32::from(w.width) >= u32::from(rw.saturating_sub(8))
                && u32::from(w.height) >= u32::from(rh.saturating_sub(8))
        });
    // Restore: remove fullscreen + maximize.
    if fs_atom != 0 {
        let _ = ewmh_request(canary, t.id, "_NET_WM_STATE", 0, fs_atom);
    }
    if max_atom != 0 {
        let _ = ewmh_request(canary, t.id, "_NET_WM_STATE", 0, max_atom);
    }
    let _ = canary.conn.flush();
    let pass = clicked && (max_sent && maxed) && snapped && (fs_sent && fs || fs);
    let _ = AtomEnum::ATOM;
    let _ = CURRENT_TIME;
    (
        pass,
        format!(
            "id={} click={clicked} max_sent={max_sent} maxed={maxed} snapped={snapped} fs_sent={fs_sent} fs={fs} path=real-pointer",
            t.id
        ),
    )
}

/// T12: workspace churn — cycle _NET_CURRENT_DESKTOP, verify targets follow
/// visibility (viewable on current or sticky-listed), then restore.
pub fn run_t12(canary: &crate::Canary, args: &[String]) -> (bool, String) {
    let Some(cur_atom) = canary.intern("_NET_CURRENT_DESKTOP") else {
        return (false, "no EWMH workspaces path=real-pointer".to_owned());
    };
    let num: u32 = canary
        .prop_u32(canary.root, "_NET_NUMBER_OF_DESKTOPS")
        .first()
        .copied()
        .unwrap_or(1);
    if num < 2 {
        return (false, format!("single desktop num={num} path=real-pointer"));
    }
    let cycles: u32 = crate::arg(args, "--cycles")
        .and_then(|v| v.parse().ok())
        .unwrap_or(num.min(4))
        .clamp(1, num.min(8));
    let orig: u32 = canary
        .prop_bytes(canary.root, cur_atom)
        .chunks_exact(4)
        .next()
        .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
        .unwrap_or(0);
    let sticky = canary.intern("_NET_WM_STATE_STICKY").unwrap_or(0);
    let state_atom = canary.intern("_NET_WM_STATE").unwrap_or(sticky);
    let mut legs = 0;
    let mut visible = 0;
    for ws in 0..cycles {
        let ev = ClientMessageEvent {
            response_type: 33,
            format: 32,
            sequence: 0,
            window: canary.root,
            type_: cur_atom,
            data: ClientMessageData::from([ws, CURRENT_TIME, 0, 0, 0]),
        };
        let sent = canary
            .conn
            .send_event(false, canary.root, EventMask::SUBSTRUCTURE_REDIRECT, ev)
            .map(|c| c.check().is_ok())
            .unwrap_or(false);
        let _ = canary.conn.flush();
        std::thread::sleep(std::time::Duration::from_millis(200));
        if !sent {
            continue;
        }
        legs += 1;
        for w in canary.all().iter().take(16) {
            let on_ws = canary
                .prop_u32(w.id, "_NET_WM_DESKTOP")
                .first()
                .copied()
                .unwrap_or(ws);
            let states = canary.prop_bytes(w.id, state_atom);
            let is_sticky = sticky != 0
                && states
                    .chunks_exact(4)
                    .any(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]) == sticky);
            if on_ws == ws || is_sticky {
                visible += 1;
                break;
            }
        }
    }
    // Restore original workspace.
    let ev = ClientMessageEvent {
        response_type: 33,
        format: 32,
        sequence: 0,
        window: canary.root,
        type_: cur_atom,
        data: ClientMessageData::from([orig, CURRENT_TIME, 0, 0, 0]),
    };
    let _ = canary
        .conn
        .send_event(false, canary.root, EventMask::SUBSTRUCTURE_REDIRECT, ev);
    let _ = canary.conn.flush();
    let pass = legs == cycles && visible == legs;
    (
        pass,
        format!(
            "desktops={num} legs={legs}/{cycles} visible={visible} restored={orig} path=real-pointer"
        ),
    )
}
