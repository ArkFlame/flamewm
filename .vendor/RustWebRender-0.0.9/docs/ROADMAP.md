# RustWebRender Roadmap

## 0.0.1 — architecture proof

Build-time HTML/CSS -> compact runtime tree -> direct native renderer.

## 0.0.2 — FlameWM visual proof

Current milestone:

- RWRB/2 raster assets;
- supplied 1350x641 FlameWM target composition;
- Start/settings/context-menu native controller;
- runtime geometry overrides;
- explicit X11 cursors;
- Xft antialiased UTF-8 text;
- IBM Plex Sans process-local loading;
- deterministic reference screenshot harness.

## 0.0.3 — launcher/verification reliability

Bugfix milestone:

- symlink-safe package-root discovery for all top-level launchers;
- `./check` and `./xephyr` always execute inside the extracted release;
- ImageMagick 7 `magick import` support;
- legacy ImageMagick 6 capture support;
- `xwd` fallback when ImageMagick lacks X11 capture support;
- stronger process/readiness diagnostics.

No renderer-format change; `RWRB/2` is retained.

## 0.0.4 — interactive Xephyr lifecycle reliability

- eliminate readiness-probe server termination;
- keep the manual Xephyr session persistent for hands-on testing;
- choose/validate nested displays safely;
- keep visual verification ephemeral and separate.

## 0.0.5 — renderer depth, only after measured 0.0.4 convergence

1. alpha-preserving runtime raster assets instead of semantic-surface precomposition;
2. clip/overflow primitives required by long Settings lists;
3. native keyboard focus/key actions;
4. optional shaped-text boundary where complex scripts actually require it;
5. profile first-frame raster upload separately from steady-state repaint;
6. PSS/startup/idle wakeup measurements;
7. decide whether the current hand-written layout subset remains sufficient or measured Taffy adoption is justified.

Do not add a JS VM, network loader or WebView merely to gain UI convenience. Native Rust remains the controller/application layer.
