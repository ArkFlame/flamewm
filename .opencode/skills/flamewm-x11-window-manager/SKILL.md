---
name: flamewm-x11-window-manager
description: Change FlameWM X11 window-manager behavior while preserving coordinate, grab, and EWMH authority rules.
---

`flamewm-wm` owns X11 WM authority through safe `x11rb`. Keep root, client, frame, and event coordinates explicit; translate exactly once at each boundary. Acquire and release pointer/keyboard grabs on every success, cancel, and teardown path. Maintain ICCCM SaveSet/reparent/frame lifetime and EWMH root/client state atomically with WM state. UI projects typed state; it never becomes WM authority.
