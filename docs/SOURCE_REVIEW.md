# Source Review — 0.0.2

## FlameWM Render 0.0.6 — promoted renderer source

The strongest new evidence is the supplied 0.0.6 renderer experiment. It achieved near-reference Xephyr appearance and diagnosed a real direct-manipulation latency defect rather than treating it as subjective UI polish.

Root causes fixed upstream and promoted here:

- debug binary used for interactive testing;
- rendering every stale queued MotionNotify coordinate;
- unnecessary GraphicsExpose/NoExpose event generation from cached XCopyArea operations.

0.0.6 also provides the best current V8 resource mapping, transparency-mask path, rounded stroke path and native Rust popup/window/desktop interaction controller. These exact owners are now the active WebRender baseline.

## SourcePort post-mortem — methodology

The failed IceWM Rust experiment demonstrated that architecture-first parity ports accumulate structural entropy. The winning method carried exact source identity, owners/resources/defaults and differential evidence.

FlameWM applies that learning selectively:

- **V8 visual/behavior port:** identity-preserving parity method.
- **FlameWM Render integration:** source promotion before product-specific extension.
- **new FlameWM WM:** independent architecture; IceWM is reference/inspiration only, not runtime/source authority.

## Historical FlameWM C / FlameWM Rust

These remain the broad feature-completeness and product-contract references for windows, taskbar, workspaces, settings, desktop, integrations and session behavior. They do not execute underneath 0.0.2.

## IceWM / IceWM Rust / SourcePort

These remain valuable protocol/edge-case references for ICCCM/EWMH/X11 behavior and mature WM semantics. They are not Cargo members, not linked, not spawned and not the architectural base.

## V8 prototype

The browser prototype remains the user-visible behavior authority. The 0.0.6 native showcase is currently the highest-value translation oracle because its screenshot is already extremely close to the approved target.
