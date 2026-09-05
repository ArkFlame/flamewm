# FlameWM / IceWM-Rust Integration Blueprint

> **0.0.4 scope note:** This document describes a possible future integration boundary only. The 0.0.4 FlameWM demo is an isolated renderer prototype and is not integrated into IceWM/FlameWM production code.


## Objective

Replace hand-authored shell layout/paint code with precompiled HTML/CSS while keeping the native WM services authoritative.

RustWebRender should become a presentation asset engine, not a second shell process and not a browser process.

## Build-time integration

Add the compiler only as a build dependency/tool. A conceptual `build.rs`:

```rust
use std::{env, fs, path::Path};
use rustwebrender_compiler::{compile_file, encode, CompileOptions};

fn main() {
    println!("cargo:rerun-if-changed=ui/panel.html");
    println!("cargo:rerun-if-changed=ui/panel.css");

    let output = compile_file(
        Path::new("ui/panel.html"),
        &[],
        CompileOptions::default(),
    ).expect("panel HTML/CSS must compile");

    let bytes = encode(&output.document).expect("RWR encoding must succeed");
    let out_dir = env::var_os("OUT_DIR").unwrap();
    fs::write(Path::new(&out_dir).join("panel.rwr"), bytes).unwrap();
}
```

Runtime embedding:

```rust
static PANEL_UI: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/panel.rwr"));
```

This avoids runtime filesystem parsing entirely.

## Runtime ownership

Recommended ownership:

```text
Flame Panel Controller
  owns:
    current tasks
    focus/minimized state
    pinned apps
    workspaces
    clock/network/volume values
    action dispatch

RustWebRender RuntimeDocument
  owns:
    compiled visual tree
    text overrides
    color variable values
    hover/active presentation state

Native Flame/IceWM renderer
  owns:
    X11 surface/window
    font/icon resources
    clipping/damage
```

## Action dispatch

Markup:

```html
<button data-action="start.toggle">...</button>
<button data-action="workspace.next">...</button>
<button data-action="settings.open">...</button>
```

Rust controller:

```text
on_action("start.toggle")
    -> StartMenuController::toggle()

on_action("workspace.next")
    -> WorkspaceService::activate_next()
```

Do not encode service calls, shell commands or X11 operations into HTML attributes.

## Dynamic text

Markup:

```html
<span id="clock">00:00</span>
```

Controller:

```rust
ui.set_text("clock", formatted_time)?;
```

This is a targeted data patch followed by layout/paint, not DOM parsing.

## Dynamic theme/accent

Markup:

```css
:root {
  --accent: #e53935;
}
```

Controller:

```rust
ui.set_color_variable("--accent", accent)?;
```

A future version should add typed length/number variables only when a real FlameWM setting requires them.

## Existing IceWM-Rust canvas integration

The standalone `rustwebrender-x11` crate is not the desired final embedding path if IceWM-Rust already owns the X11 connection/window.

Instead:

1. decode RWR once;
2. call `LayoutEngine::compute` when geometry/content changes;
3. call `build_paint_commands` when presentation changes;
4. translate paint commands into the existing native canvas;
5. route pointer coordinates through `LayoutResult::hit_test_action`;
6. dispatch returned action IDs into FlameWM controllers.

This avoids a second X11 connection and avoids parallel native window ownership.

## Panel-specific performance rule

A panel should not relayout every frame because there should be no frame loop.

Relayout only on:

- output geometry change;
- task/pin count change;
- text width-affecting change;
- taskbar edge/size setting change;
- explicit style variable change that affects geometry.

Repaint only on:

- damage/expose;
- hover/active change;
- focus/minimized/task state change;
- clock/network/volume visual change;
- appearance change.

Idle means no animation timer and no renderer wakeup loop.
