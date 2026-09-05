# Changelog

## 0.0.9 — 2026-09-05

### Retained-frame X11 rendering

- Added a retained full-screen X11 pixmap backbuffer. Each redraw now paints the complete virtual desktop off-screen and presents it with one `XCopyArea`.
- Rebinds Xft and XRender draw targets when the native window/backbuffer is resized.
- Keeps pointer hit testing and dispatched logical coordinates consistent with display scale.
- Preserves motion-event coalescing while adding native hover action dispatch for Start categories.

### Desktop selection, entries and overlays

- Enforced desktop entries < selection rectangle < sticky note < application windows < shell popovers < taskbar stacking.
- Unified Home, Downloads, Projects, Firefox, Trash and New Folder on the same desktop-entry/icon/label geometry.
- Reworked translucent rounded XRender fills to paint each destination pixel once, removing bright vertical edge strips caused by overlapping source-over composition.
- Corrected task Pin and Unpin artwork to the V8 semantic assets.

### Taskbar, Start and calendar

- Added direct taskbar drag docking to Bottom, Top, Left and Right with orientation-aware layout, work area and popup anchoring.
- Moved the 2x2 workspace pager into the taskbar's right-side cluster next to media/status controls.
- Added pinned/running-aware task context menus with Open/Pin/Unpin/Maximize/Minimize/Close policy.
- Start categories now open on hover and submenus size to their actual entry count.
- Start search is writable and filters virtual example applications; Backspace, Enter and Escape are handled natively.
- Corrected clock/date foreground and compact calendar popup spacing/geometry so all rows remain inside the popup.

### Settings, scaling and virtual content

- Settings toggle knobs now move left/right with state instead of changing only track color.
- Display scale now drives the complete virtual desktop coordinate system, including layout viewport, painting, fonts, assets, hit testing, windows, desktop entries, sticky note and shell popovers.
- Corrected the Settings display preview geometry so the runtime no longer overrides its 150px monitor preview with the previous wider value.
- Sticky-note body text is keyboard editable with printable input, Backspace and Enter while the title area remains draggable.
- Removed the Dolphin/sidebar overflow that created a black strip beneath the file-manager surface.
- Close-button destructive hover remains square on three inner corners and masks only the outer top-right corner against the rounded window frame.

### Isolation

- All Start results, application actions, pin state, taskbar docking, display scaling, session actions and filesystem-like interactions remain virtual simulation state only. No host application/process/display/filesystem mutation path was added.

## 0.0.8 — 2026-09-04

### Fixed build — sticky note startup crash

- Fixed `rwr-flamewm-demo: node 'sticky-line-4' has no text child` during initial virtual-desktop synchronization.
- Root cause: the fourth sticky-note wrap slot was intentionally empty in HTML, so the compiler correctly emitted no text child; runtime `set_text()` requires a concrete text-node target.
- Seeded the fourth line with an invisible U+200B zero-width text node (`&#8203;`), which is replaced immediately by `sync_sticky_text()` before the first interactive frame.
- Added a compiler regression test proving a zero-width seeded runtime text slot survives compilation as a mutable `NodeKind::Text` child.
- No visual, interaction, host-isolation, taskbar, window, desktop, workspace, or Settings behavior changed.

### Alpha and desktop-entry rendering

- Corrected XRender translucent solid colors to premultiplied source channels before `PictOpOver`, eliminating the overly bright selection/entry overlays seen in 0.0.7.
- Routed alpha-bearing rectangle borders through XRender as well as fills.
- Removed the separate per-icon selection-overlay nodes; hover and selected state now paint the exact same 82x82 parent desktop-entry box.
- Matched V8 desktop hover/selected alpha values and kept selection/snap opacity independently configurable.
- Centered 44x44 desktop artwork and labels inside a consistent 82x82 tile; New Folder now uses the normal folder artwork.

### Taskbar activity model

- Added virtual pinned/running state for Browser, Files, Terminal, Code, Settings, Elisa and generic activities.
- Running indicators now use the V8 24x2 gray bar and focused entries use the 32x3 live-accent bar.
- Added task-entry right-click context menu with V8 Open/Pin/Unpin/Maximize/Minimize/Close policy.
- Pin/unpin remains prototype-only in-memory state.
- Centered time and date in the clock button.

### Window and sticky-note parity

- Maximize/snap now resize the fake application body with the outer frame so the window reaches the complete work area.
- Rounded close-button hover no longer paints a square red corner beyond the rounded titlebar treatment.
- Added deterministic sticky-note text wrapping.
- Added sticky-note right-click menu with Settings/Delete and virtual background/text/text-size controls.

### Desktop context and workspaces

- Added desktop-entry right-click menus, including Trash-specific `Empty Trash`.
- Retained multi-selection group dragging and grid-snap behavior while moving selection rendering onto the parent entry.
- Rebuilt the default four-workspace pager as the V8 two-row 2x2 topology and aligned directional workspace navigation to that topology.
- Accent changes now refresh task indicators, workspace state, desktop selection/hover variables, display selection and settings controls through one runtime authority.

## 0.0.7 — 2026-09-04

### Fixed build revision

- Fixed the release-blocking Rust E0689 error in desktop group grid placement by explicitly typing the neighborhood-search radius as `i32`, which also fixes `ox.abs()` / `oy.abs()` inference.
- Removed the `root_background` unused-variable warning while preserving the existing bounds guard before `runtime_style`.
- No runtime behavior, prototype isolation boundary, UI state, or assets were changed by this repair.

### Virtual-only prototype boundary

- Removed the host-desktop integration model from the showcase controller: fake Browser, Files, Terminal, Code, Settings, music, session and About actions remain inside RustWebRender.
- `./check` now fails if host-launch primitives such as `std::process::Command`, `Command::new`, `xdg-open`, `gio open` or `system()` appear in the showcase controller.
- About links render prototype feedback only; the Website destination remains `https://wm.arkflame.com`.

### Selection/snap transparency

- Added a dynamically-loaded XRender backend (`libXrender.so.1`) for true source-over RGBA rectangle composition.
- Selection rectangle and snap-preview fills now blend over the already-painted wallpaper/window pixels instead of being pre-blended against black.
- XRender solid Pictures are cached by color; opaque painting stays on the existing Xlib path.
- `./doctor` reports XRender as required for faithful overlay transparency.

### Virtual Settings

- Added runtime style overrides for color, border, opacity, flex direction, font metrics and z-order.
- Appearance accent changes propagate live to selection, snap preview, Settings selection and workspace state.
- Desktop controls now change watermark, sticky-note enablement and independent selection/snap fill opacity.
- Taskbar settings now mutate virtual panel color, position, size and opacity.
- Displays keeps a selected eDP/HDMI model and cycles simulated resolution/scale without touching host XRandR.
- Fonts change the Xft preferred family plus global weight/size offset.
- Hotkeys now align with the V8 Start + desktop-left/right/up/down model and support default, alternate and `Not assigned` states.

### Internal fake activities

- Browser/Firefox, Dolphin/Files, Konsole, Code, Settings, Elisa/music and generic applications are internal window state, never real OS processes.
- Added inherited runtime z-order so focused fake windows and all descendants paint/hit-test above lower windows.
- Added titlebar double-click maximize/restore.
- Retained move, minimize, close, snap halves/quarters/top maximize, preview and drag-away restore.

### Desktop parity

- Single-click selects desktop icons; double-click opens the fake activity.
- Rectangle selection selects every intersecting visible icon.
- Dragging any member of a multi-selection moves the full selection transaction together.
- Group drag clamps and grid-snaps as one unit while avoiding collisions with unselected items.
- Dropping selected desktop items on Trash moves them into virtual Trash state; opening Trash uses fake Dolphin at `trash:/`.
- New Folder and sticky notes remain prototype-local.

### Keyboard/X11

- Added XKeyEvent/KeyRelease/XLookupKeysym handling to the handwritten Xlib FFI.
- Bare Super toggles Start while Super chords suppress the release toggle.
- Ctrl+Super+Arrow and alternate Alt+Arrow variants drive only virtual workspaces.
- Alt+Tab cycles internal fake windows.

### Performance retained from 0.0.6

- Interactive paths remain release-mode `opt-level=3`.
- Contiguous MotionNotify coalescing, cached pixmaps/masks/fonts/colors and disabled GraphicsExpose traffic remain active.


## 0.0.5 — 2026-09-04

- Ported the preserved FlameWM V8 HTML/CSS/JavaScript showcase much more completely into the constrained RustWebRender document/runtime model.
- Added visible minimize, maximize/restore and close titlebar controls for Settings and terminal prototype windows.
- Added native titlebar window dragging with floating-state restoration.
- Added edge/corner snapping: left/right halves, four quarters and top-edge maximize.
- Added live semantic-accent snap preview geometry while dragging.
- Added desktop drag-selection rectangle and hit-based desktop icon selection.
- Added draggable desktop icons with bounded 82px grid snapping.
- Added dynamic New Folder desktop entry from the context menu.
- Rebuilt prototype assets from the V8 source set, including corrected New Folder and Settings Displays/Fonts/Hotkeys icons.
- Added pure-magenta RGB8 transparency key and cached X11 1-bit clip masks, removing black backgrounds around transparent assets including Trash.
- Recolored dark symbolic Breeze-derived assets for the dark FlameWM shell during deterministic asset preparation.
- Added Xlib rounded-stroke support so floating windows and shell surfaces can match the prototype's subtle rounding.
- Expanded Settings content to V8-style Appearance, Desktop, Taskbar, Displays, Fonts, Hotkeys and About pages.
- Added live typed accent presets; selection, navigation, task indicators and snap preview share the same mutable accent authority.
- Rebuilt Start as the V8 category menu with right-hand application/session submenus and search footer.
- Made bottom-right media, volume and network icons opaque white rather than low-contrast/translucent.
- Updated website destination to `https://wm.arkflame.com`.
- Preserved IBM Plex Sans Xft rendering and the persistent `./xephyr` lifecycle fix from 0.0.4.

## 0.0.4 — 2026-09-04

- Fixed `./xephyr` black-window-then-exit failure caused by `-terminate` plus an `xdpyinfo` readiness client.
- Interactive Xephyr now uses `-noreset`, checks spawned-server liveness, and keeps the Rust renderer in the foreground.

## 0.0.3 — 2026-09-04

- Fixed top-level symlink project-root resolution.
- Fixed ImageMagick 7 visual verification discovery/capture paths.

## 0.0.2 — 2026-09-04

- Added the first FlameWM V8 document adaptation, RWRB/2 image assets, Xft/IBM Plex Sans text, cursor handling and native action controller.

## 0.0.1 — 2026-09-04

- Initial dependency-free build-time HTML/CSS compiler + compact runtime + direct X11 rendering experiment.
