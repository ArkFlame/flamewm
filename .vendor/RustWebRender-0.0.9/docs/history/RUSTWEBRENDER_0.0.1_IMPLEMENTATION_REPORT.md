# RustWebRender 0.0.1 Implementation Report

## Decision

Use HTML/CSS as a **compile-time system-UI authoring language** and keep the window-manager runtime native. Do not embed a browser engine.

The 0.0.1 pipeline is:

```text
HTML + CSS
   -> build-time parser / cascade
   -> normalized typed RWRB/1 document
   -> native RuntimeDocument
   -> layout + hit testing + paint commands
   -> FlameWM-owned renderer or minimal X11 proof backend
```

This removes HTML tokenization, CSS parsing, selector matching, stylesheet I/O, JavaScript, networking, WebView machinery, and browser state from the steady-state runtime.

## Blitz disposition

The supplied Blitz checkout was useful as an architectural reference but was not copied. Its parse/style/layout/paint/shell separation is sound; its browser-oriented Stylo/Taffy/Parley/AnyRender/Winit composition is broader than required for a precompiled desktop shell.

RustWebRender deliberately establishes a smaller dependency floor first. Mature parsers/layout engines can later replace individual build-time/runtime stages behind the existing IR boundary if measurements justify them.

## Implemented in 0.0.1

- dependency-free runtime core;
- build-time HTML parser for controlled shell markup;
- build-time CSS parser, selectors, specificity and cascade for a strict system-UI subset;
- local linked stylesheets and embedded `<style>`;
- typed color custom properties;
- RWRB/1 deterministic binary codec;
- block/flex/absolute layout;
- native action hit testing via `data-action`;
- precompiled `:hover` and `:active` styles;
- text updates by stable `id`;
- subtree visibility updates by stable `id`;
- runtime color-variable updates;
- renderer-neutral paint-command output;
- minimal direct Xlib demonstration backend;
- FlameWM panel/full-shell examples;
- doctor/check/demo/Xephyr/PSS helper scripts;
- integration, compatibility, benchmark and roadmap documentation.

## FlameWM integration rule

Do not use `rustwebrender-x11` as a second X11 owner inside the real WM. The Rust port should decode the compiled document once and adapt `LayoutEngine`, `LayoutResult::hit_test_action`, and paint commands into the X11/canvas infrastructure the WM already owns.

Native services remain authoritative. HTML contains presentation and semantic action IDs, not shell commands or direct WM mutations.

## Immediate known limits

0.0.1 is not a general browser and is intentionally not standards-complete. It lacks Grid, text shaping/wrapping, clipping, images/SVG, keyboard focus/navigation, accessibility, DPI abstraction, stacking contexts, shadows, transforms, animations, media queries, and a production Unicode text backend.

The Xlib backend is only a dependency-floor proof. Final FlameWM text should reuse the Rust port's native text path or a measured Xft/FreeType equivalent.

## Validation status

See `docs/VALIDATION.md`. Source/ABI/profile checks were executed, but this sandbox had no Rust compiler or Cargo, so a genuine Rust build/test result is not claimed.
