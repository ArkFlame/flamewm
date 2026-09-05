# RustWebRender 0.0.9 Implementation Report

**Release date:** 2026-09-05  
**Input baseline:** RustWebRender 0.0.8-fixed  
**Target:** FlameWM V8 virtual desktop parity / X11 renderer convergence  
**Runtime boundary:** self-contained virtual desktop simulation; no host application, display, filesystem, session or taskbar mutation

## Release contract

0.0.9 addresses the nineteen requested defects/interaction gaps while preserving the existing 0.0.8 renderer/compiler architecture. The change stays narrow: the runtime document gains a UI scale, the X11 backend gains retained-frame presentation and keyboard/hover input, XRender removes overlapping translucent composition, and the V8 controller/CSS/HTML implement the requested shell behavior.

## Requirement closure

| # | Requested behavior | 0.0.9 implementation |
|---:|---|---|
| 1 | Selection rectangle behind windows and above desktop elements | Explicit runtime z order: desktop entries inherit desktop base; `desktop-selection=50`; sticky note `80`; managed virtual windows begin above `100`; shell popovers `1200+`; taskbar `1300`. |
| 2 | Eliminate flicker while moving mouse/windows/elements | X11 now paints the complete frame into a retained full-screen Pixmap and presents it to the native window in one `XCopyArea`. Existing MotionNotify coalescing remains. Xft/XRender are rebound when the backbuffer is recreated. |
| 3 | Align Home/Projects/Firefox desktop icon and label geometry | Every desktop entry now uses the same 82x82 box, 44x44 icon and centered 76px label geometry. Runtime-created New Folder uses the same contract. |
| 4 | Correct Pin / Unpin assets | Pin uses the V8 `list-add-visible.svg` semantic role; Unpin uses `window-minimize.svg`, compiled to `task-pin.ppm` / `task-unpin.ppm`. |
| 5 | Correct clock/date taskbar color | Clock and date inherit the taskbar text foreground on a transparent clock button rather than a muted/gray surface. |
| 6 | Remove Dolphin black bar below explorer | The file sidebar no longer forces an erroneous full-height child that overflowed the fake window body. Window body resize continues to follow outer fake-window geometry. |
| 7 | Round only the outer top-right close-hover corner | Close hover itself remains square on all four button corners. A keyed 9x9 top-right mask removes only the outer top-right red corner so top-left/bottom-left/bottom-right stay square. |
| 8 | Pin/Unpin actions reflect active window pin state | Context policy is state-derived: pinned+closed = Open/Unpin; pinned+running = Unpin + Maximize/Minimize + Close; unpinned+running = Pin + Maximize/Minimize + Close. |
| 9 | Remove bright vertical bars in desktop/start red selections | XRender rounded translucent fill no longer draws overlapping rectangles. Each destination pixel is source-over composed at most once. The center band is one request; only corner rows are rasterized individually. |
| 10 | Start categories open on hover | Native X11 hover transitions dispatch `ActionPhase::Hover`; `start.category.*` opens its submenu on Hover or Release. |
| 11 | Start category content fits its entries | Submenu height is derived from visible row count rather than a large fixed content surface. |
| 12 | Settings switch knob physically moves | Watermark, sticky-note and global-bold toggle knobs move to x=20 when enabled and x=2 when disabled; track color remains independent. |
| 13 | Drag taskbar to top/bottom/left/right | Blank taskbar surface starts a 6-logical-pixel drag gesture, shows an edge preview, and commits Bottom/Top/Left/Right. Taskbar, work area, child flow and popup anchors reflow by orientation. |
| 14 | Align System Settings with V8 prototype | Preserves V8 720x480 window, 192px nav, 142px wordmark, 23x27 content inset, 21px titles, semantic Settings icons, V5 selected-display-first structure, V5 About actions, accent navigation, aligned monitor preview geometry and shared control metrics. Runtime no longer applies the stale 190px display-preview width. |
| 15 | Scale the complete virtual desktop | `RuntimeDocument::ui_scale` drives logical viewport size; painting, fonts, image geometry, strokes/radii, hit testing and action coordinates use the same scale. Controller reflows taskbar/work area, windows, sticky note, desktop entries and popovers on 100/125/150/175/200%. |
| 16 | Move workspace indicator to taskbar right side beside media | The 2x2 workspace pager is after the flexible spacer and immediately before the media/status cluster. It also reflows vertically with a side taskbar. |
| 17 | Sticky note content editable | Clicking note content enters edit mode; printable input, Backspace and Enter update bounded text and rewrap it to the eight native line slots. Title strip remains the move surface. |
| 18 | Writable Start search with example results | Start search owns focus state and keyboard input, supports Backspace/Escape/Enter, and filters Firefox, Dolphin, Konsole, Visual Studio Code, System Settings, Elisa, Kate, KWrite, Ark and Okular examples. |
| 19 | Fix calendar | Calendar is September 2026 for the release reference, highlights September 5, uses five visible week rows and compact gap-free calendar spacing inside a 286x250 popup. Popup anchoring follows every taskbar edge and scaled viewport. |

## Renderer changes

### Retained presentation

`crates/rustwebrender-x11/src/lib.rs`

- Allocates one depth-matched full-window Pixmap after creating the X11 window.
- Xft and XRender draw to that Pixmap, not directly to the visible window.
- Core-X fills, strokes, text fallback and images also draw to the same Pixmap.
- Redraw completes off-screen, resets the GC clip state, then performs one full-window `XCopyArea` and flush.
- ConfigureNotify recreates the Pixmap and safely retargets Xft/XRender before freeing the old one.
- Destruction explicitly drops Xft/XRender before freeing their retained drawable.

This closes the prior partial-presentation mechanism where a user could observe individual elements being cleared/repainted during rapid pointer-driven redraws.

### Scale authority

`crates/rustwebrender-core/src/model.rs`

- Adds finite positive `ui_scale` state, clamped to 0.5..4.0.

`crates/rustwebrender-x11/src/lib.rs`

- Layout computes in logical pixels (`native_size / ui_scale`).
- Paint commands are scaled to physical output once.
- Pointer hit testing and all action coordinates are converted back to logical pixels.
- Xft text size/position and image/shape geometry use the same scale.

### Hover and text input

- Adds `ActionPhase::Hover` when the actionable node under the pointer changes while no pointer drag is active.
- `ActionEvent` carries optional text.
- Native key translation handles printable Latin-1, Backspace, Return and Escape for virtual text controls.

### XRender alpha correctness

`crates/rustwebrender-x11/src/xrender.rs`

The previous rounded-fill implementation constructed the shape from overlapping center and middle rectangles. With translucent `PictOpOver`, overlap areas were composed twice and became visible as bright vertical/horizontal strips. 0.0.9 uses a non-overlapping middle band plus rounded-corner scanlines; every destination pixel is covered once.

## V8 controller changes

`crates/rustwebrender-x11/src/bin/rwr-flamewm-demo.rs`

- Adds taskbar dock drag state and four-edge layout authority.
- Adds scale-aware viewport/work-area calculations.
- Recomputes snapped/maximized windows after scale or panel-edge changes.
- Moves the workspace pager into the right-side taskbar cluster.
- Adds Start category hover state and content-sized submenu geometry.
- Adds Start search focus/query/result filtering/activation.
- Adds sticky-note editing state and keyboard mutations.
- Applies switch knob position independently from track color.
- Derives task context rows from pinned/running/maximized state.
- Uses explicit shell z-order and scaled popup positioning.

## Asset changes

New generated source assets:

- `examples/flamewm-v8/assets/task-pin.ppm`
- `examples/flamewm-v8/assets/task-unpin.ppm`
- `examples/flamewm-v8/assets/close-top-right-mask.ppm`

`scripts/prepare-demo-assets` is the authority that regenerates all three from the packaged V8 references / deterministic ImageMagick operation.

## Verification performed in the release workspace

Static and packaging-level verification completed:

- 196 HTML IDs / 196 unique / 0 duplicates.
- 168 HTML image references / 0 missing assets.
- 74 literal runtime document node-ID references / 0 missing HTML IDs.
- 77 binary P6 PPM assets structurally validated with exact pixel payload lengths.
- CSS delimiter/brace scan passed.
- Changed Rust sources passed delimiter/string/comment structural scan.
- `bash -n` passed for all packaged Bash scripts.
- Virtual-only host-isolation grep passed: no `std::process::Command`, `Command::new`, `xdg-open`, `gio open` or `system()` path in the showcase controller.
- Distribution font-binary policy passed: no TTF/OTF/WOFF/WOFF2 packaged outside ignored build storage.
- `check`, `doctor` and `xephyr` symlinks resolve and retain executable targets.
- Active package/compiler/script version metadata is 0.0.9; 0.0.8 appears only in historical release documentation/changelog sections.

### Compile/runtime gate availability

The artifact-generation environment used for this release does not contain `cargo` or `rustc`, and outbound toolchain installation is unavailable. Therefore this report does **not** claim an executed Rust compile/test/Xephyr PASS in this environment. The packaged `./check` remains the authoritative Rust test + release-build + compile/inspect smoke gate and `./xephyr` remains the interactive runtime gate on a development host with the documented dependencies.

The release archive itself is verified after packaging by fresh extraction and `sha256sum -c MANIFEST.sha256`.
