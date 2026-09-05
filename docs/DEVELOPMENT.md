# Development

## Commands

```bash
./doctor         # dependency + source-boundary preflight
./check          # static audit + Cargo metadata/check/test + release build + UI compile/inspect
./xephyr         # independent FlameWM WM + compiled WebRender shell in nested X11
./verify-visual  # standalone WebRender V8 screenshot differential against the 0.0.6 oracle
./bench-memory   # PSS/RSS sampling for FlameWM-owned processes
./lint           # optional strict rustfmt + Clippy hygiene gate
```

There is deliberately no `xephyr-engine`, `xvfb-engine`, `/engine`, or external WM base in 0.0.2.

## Work on the correct owner

- X11/ICCCM/EWMH/window/workspace defect -> `crates/flamewm-wm`.
- Flame product/service/state policy -> Flame API/platform/core crate.
- Web presentation/layout/paint/hit-test defect -> the corresponding `flamewm-render-*` owner, preserving promoted 0.0.6 behavior until differential parity is protected.
- V8 interaction translation -> `crates/flamewm-shell` using the V8 prototype and the 0.0.6 native controller as exact behavioral oracles.
- Desktop filesystem semantics -> `flamewm-desktop-core` plus the eventual desktop surface/controller.
- Settings behavior -> `flamewm-settings-core` / platform control contract plus the Settings presentation.
- Audio/media/network -> integration core/reactor, event-driven.

## Porting rule

For a parity surface, do not redesign before matching it:

```text
COPY EXACT OWNER -> TRANSLATE -> WIRE -> COMPILE -> DIFFERENTIAL -> REFACTOR
```

For the independent FlameWM WM, historical IceWM/FlameWM implementations are edge-case and protocol references only. They are never active source dependencies.

## Red gate law

If `./check` fails, stop feature mutation and fix the first proven compiler/test failure, then rerun the same gate. Static audits are not substitutes for Cargo. If the target compiler was not executed, report the artifact as compiler-unverified rather than claiming runtime PASS.
