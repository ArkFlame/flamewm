# Blitz and External Reference Evaluation

## Supplied Blitz checkout

The supplied `blitz-main.zip` was inspected directly.

Observed in the supplied source:

- workspace version: `0.3.0-beta.2`;
- workspace Rust version: `1.91.0`;
- 26 workspace members in the root member list;
- `blitz-dom` integrates Stylo, selectors/cssparser, Taffy, Parley, optional AccessKit, media/image facilities and supporting crates;
- `blitz-html` uses `html5ever` and `xml5ever`;
- `blitz-paint` maps Blitz DOM output to AnyRender;
- `blitz-shell` uses Winit and optional desktop integrations;
- the supplied architecture diagram separates DOM/style/layout, HTML parsing, painting and shell integration.

### What RustWebRender keeps from the idea

The separation itself is correct:

```text
HTML parser -> DOM/style -> layout -> paint -> native shell
```

It makes each stage replaceable and testable.

### What RustWebRender rejects for this experiment

FlameWM's proposed use differs from a browser engine in one decisive way: product-owned shell markup is known before runtime.

Therefore 0.0.1 does not keep browser-grade parsing/style infrastructure resident. It moves parsing, selector matching and cascade resolution to compilation.

No source file from Blitz is copied into RustWebRender 0.0.1.

## External projects studied

### Blitz

https://github.com/DioxusLabs/blitz

Reference value:

- modular native HTML/CSS engine architecture;
- proof that HTML/CSS can be separated from browser extras;
- Stylo/Taffy/Parley composition;
- clean renderer/shell boundaries.

Decision: architecture reference only for 0.0.1.

### Taffy

https://github.com/DioxusLabs/taffy

Taffy implements CSS Block, Flexbox and Grid layout algorithms and is designed as an embeddable Rust layout library.

Decision: strong candidate to replace RustWebRender's deliberately small 0.0.1 layout implementation once profiling shows that standards-correct Flex/Grid is worth its code/memory cost. The IR boundary is designed so this swap does not require reintroducing HTML/CSS parsing at runtime.

### Servo cssparser

https://github.com/servo/rust-cssparser

Reference value:

- spec-oriented CSS Syntax tokenization;
- clean tokenizer/parser layering.

Decision: candidate build-time parser dependency, not a required runtime dependency.

### html5ever

https://github.com/servo/html5ever

Reference value:

- standards-oriented HTML parsing;
- proven Servo ecosystem parser.

Decision: candidate build-time parser replacement when malformed/real-world HTML compatibility becomes useful. FlameWM-owned markup does not require the full parser to prove 0.0.1.

### Lightning CSS

https://github.com/parcel-bundler/lightningcss

Reference value:

- very fast typed CSS parsing;
- transformation/minification;
- built on Servo/Mozilla parsing infrastructure.

Decision: particularly attractive for a future build-time compiler because its cost does not need to be resident in FlameWM. It is broader than the 0.0.1 system-UI CSS profile, so adopting it should be justified by authoring capability rather than runtime concerns.

### x11rb

https://github.com/psychon/x11rb

Reference value:

- safe Rust X11 protocol binding;
- avoids direct C Xlib FFI for a mature Rust codebase.

Decision: likely preferable for a long-lived standalone Rust X11 backend. 0.0.1 uses direct Xlib only to establish the absolute-smallest dependency-floor proof and because a future IceWM-Rust integration may already own its X11 abstraction.

### tiny-skia

https://github.com/linebender/tiny-skia

Reference value:

- CPU software 2D rasterization;
- useful candidate for rounded geometry, alpha and deterministic headless screenshots.

Decision: evaluate after 0.0.1 memory/binary measurements. Do not import it merely for convenience before the dependency-free baseline exists.

## Recommended direction after 0.0.1

Do not evolve RustWebRender by blindly implementing a browser.

The likely optimal stack is:

```text
Build time:
  standards parser if needed (html5ever)
  typed CSS parser if needed (Lightning CSS / cssparser)
  selector/cascade normalization
  asset decoding/rasterization
  -> compact RWR IR

Runtime:
  RWR IR
  + measured layout backend (small custom or Taffy)
  + native text shaping/raster path
  + FlameWM-owned X11 canvas/window integration
```

This keeps the ergonomic part—HTML/CSS authoring—without paying for arbitrary web execution at runtime.
