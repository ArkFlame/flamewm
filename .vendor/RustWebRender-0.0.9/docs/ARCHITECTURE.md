# RustWebRender 0.0.4 Architecture

## Boundary

RustWebRender treats HTML/CSS as a build language. The runtime receives a compact typed document and never tokenizes markup, parses CSS, performs selector matching, or executes JavaScript.

```text
                 build / packaging
 HTML ---> parser -----------+
 CSS  ---> parser/cascade ---+--> typed node/style tree
 PPM  ---> asset loader -----+--> RWRB/2

                 runtime
 RWRB/2 --> RuntimeDocument --> LayoutEngine --> PaintCommand[] --> backend
                  ^                 |
                  |                 +--> hit testing
             native controller
```

## Crates

### `rustwebrender-core`

Dependency-free runtime contract:

- compiled nodes/styles/assets;
- RWRB/2 encode/decode;
- ID/variable indexes;
- visibility, text and geometry overrides;
- layout;
- hit-testing;
- renderer-neutral paint commands.

### `rustwebrender-compiler`

Build-time only:

- controlled HTML parser;
- strict CSS parser;
- selector matching and cascade;
- typed color-variable resolution;
- local P6 image decoding;
- binary compilation.

### `rustwebrender-x11`

Native proof backend:

- one Xlib window;
- input loop;
- standard X cursors;
- Xft-first antialiased UTF-8 text with IBM Plex Sans preference and core-X fallback;
- rectangles/borders;
- embedded raster upload/cached Pixmaps;
- `ActionEvent` dispatch to a native Rust controller.

The FlameWM demo controller is a binary in this crate; it is not part of the generic renderer core.

## RWRB/2

Header:

- `RWRB` magic;
- format version 2;
- flags;
- source fingerprint;
- root node;
- typed color-variable count;
- image-asset count;
- node count.

Asset records contain source name, dimensions and opaque RGB8 payload. Node records contain topology, ID/action/text, optional asset index, base style and optional precomputed hover/active styles.

Changing compiled layout/style semantics requires a format-version decision. A 0.0.1 RWRB/1 document is not silently decoded as RWRB/2.

## Runtime mutation model

The runtime intentionally does not expose arbitrary DOM mutation. 0.0.4 has four narrow native state channels:

1. text override by ID;
2. visibility by ID;
3. typed color-variable override;
4. geometry override by ID.

These cover clocks/status text, popovers/pages, theme accent and native placement/maximize experiments without retaining a browser DOM/cascade engine.

## Input/action model

`data-action` is compiled to an action string on a node. Hit-testing returns the topmost effectively visible action node. The X11 shell emits:

```rust
ActionEvent {
    action: String,
    button: u32,
    x: f32,
    y: f32,
}
```

The native controller decides what that action means and mutates only the narrow runtime state channels.

## Cursor model

CSS cursor values compile to `CursorKind`; the generic runtime stores only the semantic cursor. The X11 backend maps that semantic value to X cursor-font shapes. This keeps X11-specific cursor IDs out of the compiled UI format.

## Image model

Image decoding remains build-time. The runtime asset is already raw RGB. X11 creates a depth-correct XImage, uses the visual RGB masks to convert pixels, uploads once to a Pixmap, then reuses that server-side Pixmap for normal repaints.

This is intentionally simple and deterministic. Alpha-preserving assets should be added as a renderer/asset-format evolution, not by importing a resident browser image stack.
