# Provenance — 0.0.2

## Active production source

- FlameWM independent WM: new FlameWM-owned Rust source under `crates/flamewm-wm`.
- FlameWM product/platform models: derived from the FlameWM Rust project, now product-owned in this workspace.
- WebRender core/compiler/X11: identity-preserving promotion from supplied RustWebRender 0.0.6, MIT lineage retained.
- V8 presentation/assets: supplied FlameWM browser prototype plus the visually converged RustWebRender 0.0.6 asset/presentation mapping.

## Reference-only corpora

All historical implementations remain fully decompressed under `.vendor/`, including:

- FlameWM C original;
- FlameWM Rust 0.1.3;
- IceWM Rust 0.0.8;
- IceWM Rust SourcePort 0.1.3;
- RustWebRender 0.0.4;
- RustWebRender 0.0.6;
- FlameWM Web Prototype V8.

No active Cargo/source dependency is allowed to reach into `.vendor/`.

The supplied SourcePort post-mortem is preserved under `project_sources/reports/ICEWM_RUST_PORT_POST_MORTEM_SOURCEPORT_LAW.md` as methodology evidence, not as a statement that FlameWM remains an IceWM port.
