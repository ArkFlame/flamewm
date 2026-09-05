# RustWebRender 0.0.9

RustWebRender is an experimental lightweight native Rust HTML/CSS renderer for system UI. The bundled FlameWM V8 showcase is a **self-contained virtual desktop simulation**: HTML/CSS is compiled ahead of time, Rust owns behavior, X11/Xft/XRender paint it, and showcase actions never launch or reconfigure the host operating system.

This is not the production IceWM/FlameWM window manager. It is the standalone renderer/prototype used to test whether precompiled HTML/CSS can reproduce the FlameWM web prototype without Chromium, WebKit, JavaScript, GTK, Qt, Winit or WGPU.

## Run

Ubuntu/Debian development dependencies:

```bash
sudo apt install cargo rustc pkg-config libx11-dev libxft2 libxrender1 libfontconfig1 xserver-xephyr x11-utils
```

IBM Plex Sans is preferred. If Fontconfig already sees it, no extra action is required. Otherwise place a user-owned `IBM_Plex_Sans*.zip` beside the project or in `~/Downloads`; `./xephyr` stages those faces below ignored `target/` storage for the renderer process.

```bash
./doctor
./check
./xephyr
```

`./xephyr` builds the interactive renderer in release mode, opens a persistent 1350x641 nested desktop, and stays attached until Xephyr closes or Ctrl+C is pressed.

## 0.0.9 interaction, rendering and layout convergence

0.0.9 closes the current FlameWM V8 simulation gaps without crossing the virtual-only boundary. The renderer now presents complete frames through a retained full-screen X11 pixmap, scales the entire virtual coordinate system, and keeps pointer hit testing/action coordinates in the same logical space. The showcase adds four-edge taskbar docking, editable sticky notes, writable Start search, category hover, content-sized Start submenus, state-correct Pin/Unpin menus, corrected calendar geometry, and the requested desktop/window layering and alignment fixes.

Key 0.0.9 behavior:

- Desktop entries use one shared 82x82 / 44x44 geometry for Home, Downloads, Projects, Firefox, Trash and New Folder.
- Desktop selection is above desktop entries but below sticky notes and application windows.
- Translucent rounded XRender fills no longer compose overlapping rectangles over the same pixels, removing bright left/right strips.
- The X11 backend paints a complete frame into one retained pixmap and presents it with one `XCopyArea`, eliminating partial-window presentation during high-frequency movement.
- Task context actions follow pinned/running state and use the V8 Pin/Unpin Breeze assets.
- Start categories activate on hover, submenu height follows its entries, and the search field accepts text/Backspace/Enter/Escape and filters example applications.
- Settings switch knobs physically move with their boolean state.
- The taskbar can be dragged to Bottom, Top, Left or Right and reflows its children/popovers for orientation.
- 100/125/150/175/200% display scale changes the virtual viewport, drawing, fonts, assets, hit testing, work area and shell geometry together.
- The workspace pager is in the right-side cluster beside media/status controls.
- Sticky-note content is keyboard editable while its title strip remains the drag surface.
- Clock/date foreground, Dolphin body/sidebar sizing, close-hover corner masking and calendar row/popup sizing are aligned to the V8 target.

### 0.0.8 fixed startup

The fixed 0.0.8 archive repairs the startup crash:

```text
rwr-flamewm-demo: node 'sticky-line-4' has no text child
```

The fourth sticky-note wrap line is a legitimate dynamic text slot that is empty at the default font size but can become populated at larger virtual font sizes. It is now compiled with an invisible zero-width seed so `RuntimeDocument::set_text()` always has a concrete text node to mutate. The seed is replaced during initial state synchronization before the desktop is presented.

Visual comparison remains available with:

```bash
./verify-visual
```

## 0.0.8 prototype parity

### Correct alpha composition

0.0.8 fixes the bright pseudo-transparency visible in 0.0.7. Translucent colors are converted to the premultiplied representation used by the XRender source-over path before solid Pictures are composed over the already-painted desktop. Translucent borders also use XRender instead of the opaque core-X stroke path.

The desktop rectangle and desktop-entry states now match the V8 reference values:

```text
hover fill             15%
hover border           30%
selected fill          28%
selected border        65%
selection rectangle    configurable 0..60%, default 20%
snap preview           configurable 0..60%, default 20%
```

Hover and selected state are painted on the same 82x82 desktop-entry box. There is no second child selection overlay, so their geometry cannot drift apart or double-compose alpha.

### Desktop entries

- 82x82 V8 tiles with 44x44 icons.
- A fixed 76px label row centers Home, Downloads, Projects, Firefox, Trash and New Folder consistently.
- New Folder uses the normal folder artwork, matching the browser prototype's dynamically created folder entries.
- Single click selects; double-click opens the internal fake activity.
- Empty-space drag creates the translucent multi-selection rectangle.
- Dragging one member of a selected set moves the complete set while preserving relative geometry.
- Group release snaps to the V8 92x91 grid and avoids occupied slots.
- Right-click on a desktop entry opens its entry menu; Trash gets `Empty Trash`, other entries get `Create Shortcut` and `Delete`.
- All deletion/trash operations are virtual state only.

### Taskbar activity model

The taskbar now models the web preview's pin/running state rather than treating each icon as a static launcher.

- Firefox, Files, Terminal and Code begin pinned.
- Settings, Elisa and generic application slots appear while running or after pinning.
- Running activity indicator: 24x2 gray line.
- Focused activity indicator: 32x3 current-accent line.
- Right-click task context menu follows V8 policy:
  - pinned + closed: `Open`, `Unpin`;
  - pinned + running: `Unpin`, `Maximize|Minimize`, `Close`;
  - unpinned + running: `Pin`, `Maximize|Minimize`, `Close`.
- Pin/unpin is session-local virtual state and never edits the host desktop.

### Two-row workspace pager

The default four virtual desktops render as the V8 2x2 topology:

```text
1  2
3  4
```

Directional virtual hotkeys use the same topology. Active workspace fill/border uses the live accent.

### Window fixes

- Maximize and snap resize both the outer fake window and its application body, so content reaches the bottom of the work area rather than retaining the floating height.
- Floating geometry is retained for restore/drag-away.
- Close-button hover has a rounded red surface so it respects the rounded top-right window treatment.
- Existing movement, stacking, minimize, maximize/restore, close, titlebar double-click, half/quarter snapping and live rectangle preview remain.

### Sticky note fixes

- Sticky copy is wrapped into deterministic native text lines within the 166px content width.
- Right-click opens a note-local context menu at the pointer.
- `Settings` exposes prototype-local background, text color and text-size choices.
- `Delete Note` removes only the virtual note.

### Clock

Time and date now use centered flex rows inside the taskbar clock container instead of being offset toward one edge.

### Accent authority

Changing the virtual accent updates all participating runtime surfaces from one `DemoState` authority:

- Settings navigation/controls;
- workspace state;
- running/focused task indicators;
- desktop hover/selection;
- selection rectangle;
- snap preview;
- display-selection preview.

## Virtual-desktop isolation rule

The showcase intentionally has no `xdg-open`, `Command::new`, shell launch, NetworkManager mutation, XRandR mutation, host filesystem browser launch, logout, reboot or shutdown action path. Browser, Dolphin, Konsole, Code, Settings, media/network/session actions and About links are simulated inside the one nested prototype.

`./check` fails if host-launch primitives are reintroduced into the showcase controller.

## Performance path

Interactive runs retain the 0.0.6 low-latency path:

- release `opt-level=3` renderer;
- contiguous `MotionNotify` coalescing without skipping release events;
- cached image pixmaps and transparency masks;
- cached Xft fonts and colors;
- cached XRender solid Pictures;
- disabled `GraphicsExpose` traffic.

## Architecture

```text
HTML + constrained CSS + prepared assets
              |
              | build time
              v
       rustwebrender-compiler
              |
              v
           RWRB/2
              |
              | runtime
              v
       RuntimeDocument
       typed style/text/geometry/z slots
              |
              v
       virtual DemoState
       Press / Motion / Release
              |
              v
       rustwebrender-x11
       Xlib + Xft + XRender
```

HTML tokenization, CSS parsing, selector matching and cascade are not steady-state runtime work.

## Validation

`./check` performs workspace unit tests, release compiler/demo builds, full V8 HTML/CSS -> RWRB/2 compilation, RWR inspection, the virtual-only source invariant and the font redistribution guard.

`./doctor` treats XRender as required for the faithful transparency path.

The preserved browser implementation remains under `reference/flamewm-html-css-js-v8/` as the functionality and visual authority.
