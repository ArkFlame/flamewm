# FlameWM V8 Final Porting Contract

V8 is the final browser reference. HTML/CSS/JavaScript is disposable specification code.

## Authority

1. Newest explicit V8 behavior.
2. Verified IceWM 4.1.0 source behavior.
3. `FLAMEWM_V8_TASKBAR_DRAG_REORDER_FINAL_QUIRKS_REPORT.md`.
4. V6/V5/V4 native reports for non-conflicting behavior.
5. Broader FlameWM project-source reports.

## Taskbar application/window model

### Vocabulary

`active` in V8 means **the app has an open window**. It does not mean focused.

### Entry identity

- Pinning is application-level, keyed by stable desktop application identity.
- Running activity entries are window/frame-level.
- `TaskBarTaskGrouping` must be effectively disabled for the Flame task strip: one open frame/window → one task entry.
- Never collapse multiple same-app windows into one icon, count bubble, breadcrumb group or popup group.

### Exact context menus

Pinned, no represented open window:

```text
Open
Unpin
```

Pinned, represented open window:

```text
Unpin
Maximize | Minimize   # state dependent
Close
```

Unpinned, represented open window:

```text
Pin
Maximize | Minimize   # state dependent
Close
```

State rule:

```text
visible maximized -> Minimize
normal/minimized  -> Maximize
```

Pin/Unpin updates application pin state only. Maximize/Minimize/Close applies only to the clicked window entry.

### Multiple windows

- Launching an already-running app may create another window.
- Every resulting window gets its own taskbar entry.
- Clicking an entry activates/restores that exact window; clicking its already-focused visible entry minimizes that exact window.
- If a pinned app has no open window, its persistent launcher slot remains visible.
- When the first window appears, that stable pinned slot becomes one window entry instead of rendering a duplicate launcher plus task entry.
- Further windows become additional independent entries.
- When the last window closes, the pinned launcher returns to its stable slot.

### Reordering

- Left-button press-hold and drag rearranges application/window entries directly.
- No taskbar edit mode.
- Horizontal taskbars reorder on X; vertical taskbars reorder on Y.
- Active windows may be reordered independently, including multiple windows from the same application.
- Inactive pinned launcher entries are reorderable in the same strip.
- Drop semantics are insertion-based: before first, between any two entries, or after last.
- Reordering is previewed during movement and committed only on release; pointer cancellation leaves order unchanged.
- A small movement threshold separates normal click activation from drag/reorder.
- Pinned slot ordering persists.
- Live window-entry ordering only persists while those windows exist; never persist raw X11 window IDs across sessions.
- Native FlameWM should reuse IceWM task dragging. The browser-specific V8 rule to keep the source DOM node stationary exists only to preserve web pointer capture.

## Native reuse rules

IceWM 4.1.0 already provides:

- `TaskBarTaskGrouping=0` to disable class grouping;
- one `TaskButton` per `TaskBarApp`/frame when grouping is disabled;
- `TaskButton::handleBeginDrag()`;
- `TaskPane::startDrag()` / `processDrag()` / `endDrag()` and array swapping for mouse reordering;
- focused/minimized/visible frame state and task activation/minimization behavior.

Reuse those mechanics. FlameWM must add the unified pinned-placeholder + running-window strip semantics and V8 context policy rather than reimplementing window state.

## All V6 contracts remain

All non-conflicting V6 rules remain authoritative: Start search, taskbar docking/settings, workspace minimum/pager hiding, desktop/sticky-note layers, audio pointer capture, display model, hotkeys, Appearance/Icon theme, About links and the rest of the accepted prototype.
