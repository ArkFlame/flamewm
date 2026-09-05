# FlameWM Desktop Prototype V8 Verification

## Static gates

- `node --check app.js` -> PASS.
- V8 storage namespace is `flamewm-v8-*`.
- Unified `#task-entries` strip remains the single task-entry visual owner.
- The V7 drag implementation that called `insertBefore(button, ...)` / `appendChild(button)` while dragging is gone.
- V8 keeps the real source entry stationary and uses `.task-drag-ghost` + `.task-drop-indicator`.
- Pointer motion/release/cancel listeners are registered on `window` in capture phase for the lifetime of the gesture.
- Drop index is midpoint-derived on X for horizontal taskbars and Y for vertical taskbars.
- Persistent order is mutated only on successful pointer release.

## Executed Chromium semantic gate

The environment blocks browser navigation to localhost/file URLs, so verification used Playwright `page.set_content()` with the packaged `index.html`, `styles.css` and `app.js` inlined into one self-contained document. This executes the actual packaged JavaScript and CSS without replacing the taskbar implementation.

Viewport: `1400x900`.

Executed behavior:

```text
Initial:
[pin:browser, pin:files, pin:terminal, pin:code]

Drag browser after terminal:
[pin:files, pin:terminal, pin:browser, pin:code]

Drag code before first:
[pin:code, pin:files, pin:terminal, pin:browser]

Launch Dolphin two additional times:
[pin:code, pin:files, pin:terminal, pin:browser, win:3, win:4]

Drag win:4 before first:
[win:4, pin:code, pin:files, pin:terminal, pin:browser, win:3]
```

During the first drag the test asserted exactly one `.task-drag-ghost` and one `.task-drop-indicator`; both were removed after release.

Result: **PASS**.

No page errors or JavaScript console errors were emitted.

## Native acceptance

Native FlameWM should reuse IceWM's existing task state and drag machinery, but must satisfy the same externally visible insertion/release semantics. See `FLAMEWM_V8_TASKBAR_DRAG_REORDER_FINAL_QUIRKS_REPORT.md`.
