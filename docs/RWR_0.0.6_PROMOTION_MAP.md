# RustWebRender 0.0.6 Promotion Map

This map enforces the supplied port post-mortem's core rule on the WebRender parity surface: preserve source-owner identity first, extend only proven owners, and do not redesign a near-parity renderer before differential evidence exists.

Reference corpus: `.vendor/RustWebRender-0.0.6/`  
Active namespace: `crates/flamewm-render-*`  
Development verifier: `tools/verify_rwr_promotion.py`

## Exact after mechanical crate rename

| 0.0.6 owner | Active owner | Status |
|---|---|---|
| `rustwebrender-core/src/codec.rs` | `flamewm-render-core/src/codec.rs` | exact |
| `rustwebrender-core/src/layout.rs` | `flamewm-render-core/src/layout.rs` | exact |
| `rustwebrender-core/src/lib.rs` | `flamewm-render-core/src/lib.rs` | exact |
| `rustwebrender-core/src/paint.rs` | `flamewm-render-core/src/paint.rs` | exact |
| `rustwebrender-compiler/src/compile.rs` | `flamewm-render-compiler/src/compile.rs` | exact |
| `rustwebrender-compiler/src/css.rs` | `flamewm-render-compiler/src/css.rs` | exact |
| `rustwebrender-compiler/src/html.rs` | `flamewm-render-compiler/src/html.rs` | exact |
| `rustwebrender-compiler/src/lib.rs` | `flamewm-render-compiler/src/lib.rs` | exact |
| `rustwebrender-compiler/src/bin/rwr-inspect.rs` | `flamewm-render-compiler/src/bin/flamewm-render-inspect.rs` | renamed product target |
| `rustwebrender-x11/src/xft.rs` | `flamewm-render-x11/src/xft.rs` | exact |
| `rustwebrender-x11/src/bin/rwr-x11.rs` | `flamewm-render-x11/src/bin/flamewm-render-x11.rs` | renamed product target |

## Intentionally extended owners

### `model.rs`

Retains the 0.0.6 runtime document model and adds only product-needed typed mutations:

- `set_flex_direction`
- `set_background_color`
- `set_border_color`

These are used to mutate compiled FlameWM shell state without adding a browser DOM/runtime.

### `x11/lib.rs` + `x11/xlib.rs`

Retains the 0.0.6 performance fixes exactly, including:

- contiguous same-window `MotionNotify` coalescing;
- `XEventsQueued(..., QueuedAfterReading)`;
- `XPeekEvent` ordering barrier so release/non-motion events are never crossed;
- `XSetGraphicsExposures(..., False)`;
- cached images, keyed-transparency masks, Xft fonts and colors;
- hover redraw only on effective target change;
- `ActionPhase::{Press, Motion, Release}` and `inside` state;
- rounded fill/stroke drawing.

FlameWM adds one product integration: typed X11 window roles (`Normal`, `Desktop`, `Dock`) via `_NET_WM_WINDOW_TYPE`.

### V8 controller

`crates/flamewm-shell/src/main.rs` is the 0.0.6 `rwr-flamewm-demo` controller translated into the product executable. The intentional differences are:

- compile-time embedded `RWRB/2` instead of reading a `.rwr` path from argv;
- FlameWM product naming/log prefixes;
- desktop-role declaration for the shell surface;
- active crate namespaces.

The interaction/state ordering remains the 0.0.6 V8 oracle.

## Visual source identity

- `ui/flamewm.css` is byte-identical to 0.0.6 V8 CSS.
- `ui/index.html` is identical except for `id="taskbar"`, required for typed runtime orientation mutation.
- active raster assets are promoted from the 0.0.6 V8 preparation output.
- `./verify-visual` compares the standalone shell at **1350x641** directly against `project_sources/visual/rwr-0.0.6-target-xephyr.png` without resizing.

## Release rule

Source-identity verification is necessary but not compiler proof. `./check` must still compile/test/build the active workspace on a Rust-capable machine, and `./verify-visual` must still execute the pixel differential on an X11-capable machine.
