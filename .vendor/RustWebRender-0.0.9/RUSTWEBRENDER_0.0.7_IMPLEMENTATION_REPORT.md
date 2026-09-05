# RustWebRender 0.0.7 — Virtual FlameWM V8 Interaction Convergence

## Scope

0.0.7 is a standalone RustWebRender experiment. It is not the production FlameWM/IceWM port and deliberately does not control the host desktop. The preserved `reference/flamewm-html-css-js-v8/` HTML/CSS/JavaScript implementation is the behavioral reference.

## Root correction: prototype isolation

0.0.6 still had showcase actions that could delegate to the host through external-launch concepts inherited from early prototypes. That violated the experiment boundary. 0.0.7 makes the boundary explicit:

```text
web-prototype intent -> DemoState -> RuntimeDocument -> native repaint
```

There is no host application/session/display authority in the demo controller. Browser, Files, Terminal, Code, Settings, music, power/session and About actions stay internal.

## True overlay alpha

The selection/snap fill bug was caused by fallback alpha being pre-blended against black. Over a wallpaper that produces a dark opaque-looking rectangle instead of compositing the accent over the existing pixel.

0.0.7 adds `crates/rustwebrender-x11/src/xrender.rs`, dynamically loads `libXrender.so.1`, creates a Picture for the renderer window, caches solid RGBA Pictures and uses `PictOpOver` for alpha fills. Opaque paint remains on the existing Xlib path.

Runtime fallback is explicit: if XRender cannot load, the renderer logs the fallback. `./doctor` marks XRender runtime availability as required for faithful prototype transparency.

## Runtime mutation surface

`RuntimeDocument` now supports typed runtime overrides for:

- background/border colors;
- opacity;
- flex direction;
- font size/weight;
- global UI font family/offset/bold;
- z-order.

Layout, hit testing and paint consume the same runtime style authority. Z-order is inherited from a window/surface root so its descendants remain above lower windows without rewriting DOM order.

## Window/activity simulation

The native controller owns seven internal window classes:

- Firefox/browser;
- Dolphin/files;
- Konsole;
- Code;
- System Settings;
- Elisa/music;
- generic application activity.

Each owns floating geometry, restore geometry, open/minimized/maximized/snapped state, virtual desktop membership and z-order. Static taskbar entries operate on these internal objects. Start applications create/activate internal activities; none open a real process or URL.

The X11 backend adds titlebar double-click recognition so the same titlebar action can perform the V8 maximize/restore behavior without adding a JavaScript runtime.

## Desktop selection and group drag

The desktop state holds a `HashSet<IconKind>` selection and an `Icons` drag transaction containing the original rectangle of every selected icon.

On motion, one delta is clamped against the union bounds of the full selection and applied to all originals. On release, one grid delta is selected for the group, collision-tested against non-selected desktop items, then applied to every selected item. Relative spacing is never recomputed item-by-item.

A moved group dropped on the Trash is hidden as virtual Trash state instead of touching the real filesystem.

Single click selects. A second click within the X11 double-click interval opens the corresponding fake activity. Dragging cancels the click-open path.

## Settings

Settings is a real controller for the virtual environment, not a static screenshot. Changes feed typed runtime slots and `DemoState` immediately. Implemented domains:

- Appearance/accent/icon-theme state;
- Desktop watermark, sticky-note enablement, selection and snap opacity;
- Taskbar color/position/size/opacity;
- selected virtual display, resolution and scale preview state;
- Xft font family/bold/size offset;
- Start and directional desktop hotkey variants.

The display model is simulation-only by design; no XRandR mutation is performed.

## Keyboard integration

The handwritten Xlib ABI now includes `XKeyEvent`, `KeyPress`, `KeyRelease` and `XLookupKeysym`. The implementation preserves the V8 special Super behavior: a bare Super release toggles Start, while a Super chord suppresses that release action. Alternate virtual bindings are emitted distinctly so Settings can enable the selected variant or disable an action with `Not assigned`.

The `XKeyEvent` layout was checked against the installed Xlib headers: `sizeof(XEvent)=192`, `sizeof(XKeyEvent)=96`, with state/keycode/same_screen offsets 80/84/88 on the target x86_64 ABI.

## Low-latency path

0.0.6's release-mode and motion-coalescing work is retained. 0.0.7 does not add timers, polling, a browser engine or a second event loop. XRender is invoked only for alpha rectangles; image and text caches remain retained.

## Verification policy

The assembly environment for this artifact does not include Cargo/rustc, so no native Rust build result is fabricated. Static verification performed before packaging covers:

- shell syntax;
- HTML ID uniqueness/nesting;
- action/ID closure against controller mutations;
- CSS property profile compatibility;
- Rust delimiter/token sanity;
- Xlib key-event ABI against system headers;
- dynamic XRender runtime symbol availability;
- no host-launch primitives in the virtual controller;
- prepared PPM payload validity;
- manifest/package integrity.

The target-machine executable gate remains:

```bash
./doctor
./check
./xephyr
```
## Fixed build revision — 2026-09-04

A user-side Rust build exposed one compiler error and one warning that static packaging checks could not catch in the assembly environment:

- `rwr-flamewm-demo.rs`: the `1..=8` search radius had no concrete integer type before calling `abs()` on derived offsets, producing Rust `E0689`. The range is now explicitly `i32`, matching the grid delta contract (`dc`/`dr`) and the candidate arithmetic.
- `lib.rs`: `root_background` retained a bounds-check binding named `node` that was intentionally unused. The guard is retained as `let _ = ...?;`, removing the warning without weakening the safety behavior.

This repair does not change the virtual-only prototype behavior or visual contract.

