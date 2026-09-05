# Changelog

## 0.0.4 — 2026-09-05

### Product identity

- Promoted FlameWM Rust WebRender workspace identity to 0.0.4.
- Added canonical `FLAMEWM_VERSION` and pinned workspace `calloop` to 0.14.4.

## 0.0.2 — 2026-09-05

### Architecture

- Removed the external `/engine` design completely.
- Removed `icewm-rust` as an active dependency/runtime base.
- Established `crates/flamewm-wm` as FlameWM's independent Rust/X11 WM.
- Made `.vendor/` strictly reference-only; active source/path dependency checks reject vendor/engine reach-through.

### Build fixes

- Fixed the 0.0.1 invalid Cargo metadata (`rust-version = "0.0.1"`).
- FlameWM-owned product crates use Rust 1.85 / edition 2024. Promoted renderer/shell crates preserve edition 2021 during parity convergence instead of mixing an edition migration into the port.
- Fixed false `libfontconfig.so.1` doctor failures by using loader-backed library discovery.
- Removed the dead-code-failing IceWM Rust build path instead of suppressing its warnings.

### FlameWM Render 0.0.6 convergence

- Source-promoted the 0.0.6 renderer implementation into active FlameWM WebRender crates.
- Added ordered pointer-motion coalescing without crossing release/event boundaries.
- Added release-optimized interactive build contract (`opt-level=3`, Thin LTO, one codegen unit).
- Disabled unused X11 graphics-exposure traffic on cached image copy operations.
- Added cached transparency masks and rounded border strokes.
- Added press/motion/release interaction phases required by real drag/selection semantics.
- Promoted the 0.0.6 V8 HTML/CSS/assets/controller behavior as the visual baseline.
- Added `./verify-visual` against the approved 1350x641 Xephyr reference.

### Methodology

- Integrated the SourcePort post-mortem as a permanent parity-development rule.
- For visual/prototype parity: exact source owner -> mechanical translation/copy -> wire -> compile -> differential -> refactor.
- IceWM remains useful engineering reference material, but no longer defines FlameWM's runtime architecture or source ownership.
