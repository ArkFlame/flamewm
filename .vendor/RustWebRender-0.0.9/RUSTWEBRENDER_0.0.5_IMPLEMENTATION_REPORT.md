# RustWebRender 0.0.5 — FlameWM V8 Full Showcase Port

## Scope

0.0.5 is the next R&D prototype of RustWebRender. Its purpose is to reproduce the supplied FlameWM V8 browser specification using precompiled HTML/CSS plus a native Rust/X11 controller. It is not a replacement IceWM/FlameWM window manager and does not claim browser compatibility.

The preserved prototype under `reference/flamewm-html-css-js-v8/` is the implementation reference. Presentation is represented in `examples/flamewm-v8/index.html` + `flamewm.css`; interactive JavaScript concepts are translated into explicit Rust state transitions in `rwr-flamewm-demo.rs`.

## Requested gap closure

| Requirement | 0.0.5 implementation |
|---|---|
| Window minimize/maximize/close controls | Correct title assets, native click actions, restore glyph switching |
| New Folder icon | Rebuilt from the reference visible folder asset; context menu creates/reveals the desktop entry |
| Displays/Fonts/Hotkeys icons | Reference semantic SVGs prepared as high-contrast dark-shell assets |
| Website | `https://wm.arkflame.com` |
| Settings interior | Seven-page V8-style Settings layout, cards, display topology, font/hotkey content, About |
| Rounded windows | Radius-aware fill and stroke rendering, 6px floating shell radius |
| Movable windows | Press/motion/release native titlebar drag controller |
| White taskbar status icons | Prepared opaque white media/volume/Wi-Fi assets |
| Start menu | V8 category list, search footer, right-hand submenus and application/session actions |
| Window layouts | Edge/corner detection, live preview rectangle, half/quarter/maximize commit |
| Desktop selection rectangle | Native pointer drag rectangle + intersection selection |
| Trash transparency | Chroma-key prepared RGB8 asset + X11 1-bit clip mask |
| Draggable desktop icons | Native drag, bounds, grid snap and click-vs-drag threshold |
| Accent color | Typed runtime CSS variables, six preset mutations, no CSS reparsing |

## Native interaction model

0.0.5 extends the X11 controller callback from release-only clicks to a typed pointer phase contract:

```text
Press -> Motion* -> Release
```

An `ActionEvent` carries:

```text
action
phase
button
x/y
inside
```

For the primary button, the action selected on press remains the interaction owner until release. This supports bounded direct manipulation without moving a DOM node or introducing a JavaScript runtime.

## Window movement and snapping

Each prototype window has a compact native state:

```text
rect
restore rectangle
maximized
snapped
```

Titlebar drag updates geometry directly through `RuntimeDocument::set_position_px`. If the window enters an edge/corner target, `snap-preview` receives the candidate geometry. On release the geometry is committed.

Targets:

```text
top                maximize
left/right         50% half
four corners       25% quarter
```

Dragging a snapped/maximized window begins by restoring the saved floating rectangle, preserving the familiar web-prototype behavior.

## Desktop selection and icon dragging

Desktop pointer drag owns one rectangle from press to release. Its visual is drawn using `--accent`/`--accent-soft`. At release it intersects the current desktop icon rectangles and toggles each icon's precompiled selection overlay.

Desktop icon drag stores the original rectangle and pointer origin, updates the runtime slot during motion, clamps it to the desktop work area, then snaps to the 82px grid on release. Movement below the threshold remains a click.

## Image transparency

RWRB/2 intentionally remains RGB8. Rather than widening the binary format in this prototype milestone, the deterministic asset pipeline reserves pure magenta as an internal transparent key. The X11 renderer detects keyed pixels while building a resized image cache and creates a depth-1 clip mask. `XSetClipMask`/`XSetClipOrigin` bound each copy and the GC is restored immediately after the image draw.

This addresses transparent SVG/PNG-derived artwork without persistent per-frame alpha work.

## Settings and semantic icon preparation

The V8 source uses several Breeze symbolic SVGs that are dark by source design and become unreadable when naïvely flattened onto FlameWM's dark shell. The asset preparation step classifies these as symbolic and converts their dark source fills to the reference foreground before rasterization. Full-color/brand artwork remains full-color.

The Settings adaptation includes:

```text
Appearance
Desktop
Taskbar
Displays
Fonts
Hotkeys
About
```

The active sidebar backplate and accent stripe are one movable runtime element rather than seven duplicated states.

## Accent authority

The document declares typed color variables:

```text
--accent
--accent-soft
--accent-strong
```

Preset controls mutate all three through `RuntimeDocument::set_color_variable`. Relevant surfaces consume those variables so the color propagates without HTML/CSS parsing or selector matching at runtime.

## Start menu

The previous compact start mock was replaced with the V8 categorical shape. Categories are always in the primary surface; selecting one shows exactly one application/session group in the adjacent submenu. The native controller owns submenu visibility.

## Verification performed in the assembly environment

The assembly environment does not provide `rustc`/`cargo`, so a final native Rust compilation cannot be truthfully reported here. The following non-Rust checks were executed:

- all shipped shell scripts pass `bash -n`;
- all 71 image references in the 0.0.5 HTML resolve;
- all HTML IDs are unique;
- every literal runtime mutation ID used by the controller exists in the document;
- the 0.0.5 CSS property inventory stays inside the compiler's supported profile;
- the asset preparation pipeline generated 57 prototype PPM assets;
- the Rust controller's structural delimiter balance was checked;
- newly used Xlib `XDrawLine`/`XDrawArc`, clip-mask declarations are present in the handwritten FFI surface.

The authoritative target-machine gate is therefore:

```bash
./doctor
./check
./xephyr
```

and then, where desired:

```bash
./verify-visual
```

A source archive must never claim a native test result that was not actually executed.
