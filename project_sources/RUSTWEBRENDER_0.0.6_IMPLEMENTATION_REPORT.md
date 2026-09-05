# RustWebRender 0.0.6 — Low-Latency V8 Showcase Convergence

## Scope

0.0.6 continues the standalone RustWebRender experiment. The preserved FlameWM HTML/CSS/JavaScript V8 prototype is the showcase specification; RustWebRender compiles its constrained presentation ahead of time and implements interactive behavior in native Rust/X11. It is not the production FlameWM or IceWM Rust port.

## Root-cause analysis: delayed window/selection dragging

The 0.0.5 interaction architecture redraws after native `MotionNotify` state mutations. Two independent implementation details made the result feel substantially slower than the browser reference:

1. the normal interactive launcher built and executed `target/debug/rwr-flamewm-demo`, so layout, paint-command construction and controller mutation executed without release optimization;
2. X11 pointer motion can arrive faster than a full redraw. Processing each old queued `MotionNotify` in sequence means the UI renders historical coordinates and visually trails the physical pointer.

A third avoidable source of event traffic existed in the image path: the GC retained the Xlib default graphics-exposures behavior even though RustWebRender never consumes copy exposure notifications as repaint authority.

## 0.0.6 latency design

### Release is the interactive contract

The interactive scripts now compile both compiler and renderer with `--release` and execute `target/release/*` directly. The runtime release profile uses:

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

The compiler keeps its package-specific `opt-level=2` because parser throughput is not the steady-state interaction bottleneck.

### Ordered motion coalescing

When an X11 `MotionNotify` arrives, the event loop keeps the first motion as the interaction boundary, then peeks only at the queue head. While the next event is another motion for the same renderer window it consumes it and replaces the coordinate with the newer one.

The loop **stops immediately** when the queue head is a release or any other event. This matters: searching arbitrarily through the queue for the newest motion would reorder input and could move a window after its physical button release.

`XEventsQueued(..., QueuedAfterReading)` is used so already-arrived socket input can be brought into Xlib when its in-memory queue becomes empty. The frame therefore targets the newest currently available coordinate instead of deterministically drawing every stale coordinate.

### No unused copy exposure events

The image renderer uses server pixmaps and `XCopyArea`. After GC creation 0.0.6 calls:

```text
XSetGraphicsExposures(display, gc, False)
```

RustWebRender owns expose/repaint through ordinary window exposure and its retained document; it does not need `GraphicsExpose` or `NoExpose` from every image copy. Disabling them removes irrelevant queue work.

### Existing caches retained

The latency fix does not throw away the low-memory architecture:

- raster assets remain precompiled;
- X11 image pixmaps are cached by asset + target size;
- transparency masks are cached beside their pixmap;
- Xft font handles are cached by pixel size + weight;
- Xft colors are cached by RGBA;
- HTML/CSS parse/cascade remains build-time only.

## Showcase convergence added in 0.0.6

### Reference geometry

Native showcase constants now use the preserved V8 values:

```text
panel height = 44
X desktop grid step = 92
Y desktop grid step = 91
icon footprint = 82 x 82
```

### Taskbar and shell popovers

The bottom strip now includes:

```text
Start / Browser / Files / Terminal / Code / Settings / spacer / Media / Volume / Wi-Fi / Clock
```

Four native popup surfaces were added:

- media/Now Playing;
- volume;
- Wi-Fi;
- calendar.

Only one shell popover is authoritative at a time. Opening Start, a desktop context menu or a tray popup closes the competing popup state.

Media play/pause, previous/next demo actions, mute/volume state and selectable Wi-Fi demo rows are native Rust transitions, not JavaScript.

### Start alignment

The existing V8 category + right-hand submenu structure is expanded with more of the reference application rows, including KWrite, Ark, Okular, Spectacle, Gwenview, Kdenlive and Code/development entries.

### Icon correction

The deterministic asset map was reconciled against the preserved prototype icon definitions rather than using approximate theme names. Important corrections include:

- New Folder -> `folder-new-visible.svg`;
- Internet category -> `internet-visible.svg` symbolic treatment;
- Search -> `search-visible.svg`;
- terminal/application icons preserve their full Breeze artwork where the prototype uses image mode;
- missing taskbar Files/terminal/Code images;
- media transport and mute images;
- popup volume/mute images.

Dark symbolic source glyphs are recolored only when the prototype treats them as monochrome. Full-color/application artwork is preserved.

## Runtime behavior retained from 0.0.5

- IBM Plex Sans through Xft/Fontconfig;
- persistent Xephyr lifecycle;
- native pointer/cursor handling;
- minimize/maximize/restore/close controls;
- movable Settings/terminal windows;
- snap target preview and edge/corner layouts;
- drag-away floating restore;
- desktop rectangle selection;
- draggable desktop icons;
- transparent Trash and other alpha-like keyed assets;
- live semantic accent variables;
- V8 Settings/About presentation;
- exact `https://wm.arkflame.com` Website action.

## Verification contract

On a machine with the project development dependencies:

```bash
./doctor
./check
./xephyr
```

`./check` must now compile the release renderer in addition to tests. Interactive latency must be evaluated with `./xephyr`; debug performance is no longer accepted as the product path.

Visual measurement remains:

```bash
./verify-visual
```

This artifact environment does not contain Cargo/Rustc/Xephyr. Therefore this report does not fabricate a native compile, frame-time number or interactive PASS. Static source/resource/package checks are performed before packaging; the commands above are the target-machine executable gates.
