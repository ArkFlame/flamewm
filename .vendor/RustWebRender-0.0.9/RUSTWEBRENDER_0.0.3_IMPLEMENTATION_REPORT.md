# RustWebRender 0.0.3 Implementation Report

## Objective

Make the standalone Rust renderer reproduce the supplied FlameWM browser-prototype frame directly under Xephyr while keeping the architecture lightweight: HTML/CSS is compiled ahead of time; native Rust owns state/actions; X11 is the output backend; no browser runtime or JavaScript VM is resident.

The authoritative target is `reference/target-xephyr.png`, exactly **1350x641**.

## Root causes of the failed baseline

The original screenshot diverged for structural reasons rather than minor CSS tuning:

1. wrong viewport and dark-blue desktop default;
2. incomplete FlameWM composition/state;
3. no proper build-time image pipeline;
4. transparent SVGs flattened against incorrect backgrounds;
5. legacy core-X bitmap text instead of smooth UI typography;
6. inherited/default cursor rather than explicit X cursor ownership;
7. missing target geometry/palette details such as the 720x480 Settings frame, titlebar gradient, active-nav surface and one-pixel taskbar separator.

0.0.3 addresses those causes directly.

## Precompiled UI path

```text
HTML + CSS + prepared PPM assets
           |
           v
      rwr-compile
           |
           v
        RWRB/2
  resolved nodes/styles/images
           |
           v
 RuntimeDocument
   -> layout
   -> hit test
   -> paint commands
           |
           +-> Xlib rectangles/images/cursor
           +-> Xft UTF-8 text
```

No HTML/CSS parsing or selector matching remains in steady state.

## Image fidelity

The source FlameWM/Breeze assets remain under `reference/flamewm-html-css-js-v8/assets/`. `scripts/prepare-demo-assets` rasterizes them at the exact semantic size and composites source alpha against the exact surface where the asset is used.

This is important because RWRB/2 currently stores opaque RGB8 pixels. Naively discarding transparency produced visible white/wrong-color boxes. 0.0.3 instead precomposes against:

- desktop `#000000`;
- taskbar `#000000`;
- titlebar center `#272a2d`;
- sidebar `#1b1e20`;
- main surface `#202326`;
- active nav `#412527`;
- menu `#1e2123`;
- About button `#303438`.

The About logo is precomposed at 92% source opacity, sidebar brand at 96%, desktop watermark at 28%. The browser titlebar gradient is precomputed as a 1x31 RGB strip (`#292c2f` -> `#24272a`) and stretched horizontally by the native image renderer.

## IBM Plex Sans + Xft

The reference browser CSS chooses IBM Plex Sans for the interface. 0.0.3 now has a native Xft-first text backend instead of pretending X11 core fonts can visually converge.

`crates/rustwebrender-x11/src/xft.rs` dynamically resolves:

- `XftDrawCreate` / `XftDrawDestroy`;
- `XftDrawStringUtf8`;
- `XftFontOpenName` / `XftFontClose`;
- `XftColorAllocValue` / `XftColorFree`;
- `FcConfigGetCurrent`;
- `FcConfigAppFontAddFile`;
- `FcConfigBuildFonts`.

This preserves the zero third-party Rust-crate runtime baseline and avoids requiring Xft development headers merely to build the Rust crate. Xft runtime availability is checked by `./doctor`.

The text pattern prefers IBM Plex Sans, maps CSS weights to Regular/Medium/SemiBold/Bold, enables antialiasing/hinting and forces grayscale antialiasing for deterministic screenshots. Core X11 text remains only a fallback if Xft cannot initialize.

Font binaries are not part of the RustWebRender distribution. `scripts/prepare-ibm-plex` can extract the four static faces from a user-owned `IBM_Plex_Sans*.zip` into `target/rwr/fonts/`; the Rust process then registers those paths directly with Fontconfig. No global font installation is required.

## Mouse cursor

0.0.3 creates standard X cursor-font cursors and immediately installs `XC_LEFT_PTR` on the renderer window. Hover hit-testing maps compiled CSS cursor metadata to pointer, text, move and resize cursors with `XDefineCursor`. Leaving the window restores the default pointer.

This removes the baseline's dependency on Xephyr/root cursor inheritance.

## Native controller

The supplied JavaScript is reference behavior, not a runtime dependency. High-value state changes are represented directly in Rust:

- Start visibility/toggle;
- desktop context-menu position and visibility;
- Settings page visibility and active selection geometry;
- Settings hide/restore/maximize geometry;
- terminal mock visibility;
- safe URI/file launching through direct `xdg-open` argv.

The renderer core still knows nothing about FlameWM business logic.

## Visual acceptance tooling

`./verify-visual` is the convergence harness:

1. build compiler/demo;
2. start 1350x641 Xephyr at 96 DPI;
3. render default reference state;
4. capture nested root at native size;
5. reject geometry mismatch;
6. print ImageMagick RMSE and absolute changed-pixel count against `reference/target-xephyr.png`.

That allows future changes to be driven by measured visual difference rather than subjective iteration.

## Validation performed in the artifact environment

The environment does not contain Rust/Cargo/Xephyr, therefore a Rust build or Xephyr screenshot is not claimed.

The native text boundary itself was exercised with an independent C ABI probe using the same dynamic symbols/signatures. Under Xvfb the probe:

- loaded `libXft.so.2` and `libfontconfig.so.1`;
- registered the supplied IBM Plex Sans Regular file as an application font;
- opened `IBM Plex Sans:pixelsize=13:weight=semibold:...` through Xft;
- allocated an Xft color;
- rendered UTF-8 text;
- shut down cleanly.

All required dynamic symbols were also verified in the installed libraries with `nm -D`.

## Boundary

This is not production FlameWM and does not replace the IceWM renderer. It is the explicit R&D experiment requested to answer whether a precompiled HTML/CSS UI language can drive a very small native Rust desktop renderer without carrying a browser engine.

## 0.0.3 launcher-path and visual-gate repair

The 0.0.2 top-level commands are relative symlinks such as `check -> scripts/check`. The scripts used:

```bash
cd "$(dirname "$0")/.."
```

When Bash executes a script through `./check`, `$0` remains the symlink spelling (`./check`). `dirname "$0"` is therefore `.` and `/..` becomes the directory *above* the extracted project. This exactly explains both reported failures:

```text
cargo: could not find Cargo.toml in ~/Downloads
./xephyr: ./scripts/prepare-ibm-plex: No such file or directory
```

0.0.3 resolves the actual script file before changing directory:

```bash
SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}")"
ROOT="$(cd "$(dirname "$SCRIPT_PATH")/.." && pwd -P)"
cd "$ROOT"
```

The same invariant is applied to every shipped launcher/helper that depends on package-relative paths.

`verify-visual` had a second independent portability defect: it required both `import` and `magick`. ImageMagick 7 officially exposes the capture operation as `magick import`, and distributions are not required to install the ImageMagick 6 compatibility executable named `import`. 0.0.3 therefore uses this capture order:

```text
ImageMagick 7: magick import
-> legacy standalone import
-> xwd root capture + ImageMagick XWD decoding
```

Comparison uses `magick identify/compare` on ImageMagick 7 and standalone `identify/compare` on legacy ImageMagick.

No renderer semantics or `RWRB/2` layout changed in this repair release.
