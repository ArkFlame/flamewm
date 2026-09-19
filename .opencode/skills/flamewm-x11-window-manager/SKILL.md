---
name: flamewm-x11-window-manager
description: Change FlameWM X11 window-manager behavior while preserving coordinate, grab, and EWMH authority rules.
---

## Authority

`flamewm-wm` owns WM decisions and canonical state through safe `x11rb`. Reactor owns native event source/typed dispatch; UI/WebRender may project state and submit typed intent only. Never make renderer, UI, or client callback authoritative.

## Coordinate and event discipline

- Label root, client, frame, window-local, logical, and physical coordinates at boundaries.
- Translate exactly once; preserve event timestamp, target, and coordinate space needed for later decisions.
- Validate event target/window lifetime before acting. Ignore or fence stale events after unmanage, reparent, or restart.
- Keep configure, map, unmap, destroy, focus, and client-message ordering explicit; do not infer intent from one event alone.

## ICCCM, EWMH, and lifetime

- Maintain SaveSet, reparenting, frame creation/destruction, border/input properties, focus, and cleanup across every manage/unmanage/error path.
- Update EWMH root and client properties atomically with canonical WM state; remove stale client/list/active-window state during teardown.
- Respect client configure requests and synthetic notifications according to negotiated WM ownership, not renderer geometry.
- Acquire pointer/keyboard grabs only with defined owner and release on success, cancellation, error, unmanage, and shutdown.

## Validation

Read ICCCM/EWMH contract, current state transitions, and callers before edits. Use isolated nested-X runtime evidence for behavior claims. Report display isolation, commands, observed protocol state, cleanup, and untested interactions.
