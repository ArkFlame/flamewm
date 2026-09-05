# RustWebRender 0.0.8 Implementation Report

## Fixed-build addendum — sticky-note startup crash

A release-blocking startup failure was traced to the fourth sticky-note wrapping slot. `sticky-line-4` was emitted as an empty HTML element. The HTML parser intentionally drops empty/whitespace-only text, while `RuntimeDocument::set_text()` requires either a text node or an element with exactly one text child. Initial `sync_sticky_text()` therefore terminated the demo before the first frame with `node 'sticky-line-4' has no text child`.

The fixed 0.0.8 source seeds that dynamic slot with U+200B (`&#8203;`). This creates a zero-width text child at compile time, and initial synchronization immediately replaces it with the real wrapped line or an empty override. A compiler regression test now locks this contract. No renderer or prototype behavior was otherwise changed.


**Release:** 0.0.8  
**Date:** 2026-09-04  
**Role:** standalone FlameWM V8 HTML/CSS/native-Rust renderer experiment; not the production IceWM/FlameWM port.

## Assumptions and authority

1. `reference/flamewm-html-css-js-v8/` is the functionality/visual specification for this experiment.
2. The renderer remains a self-contained virtual desktop. No showcase action may invoke or reconfigure the host OS.
3. HTML/CSS remains ahead-of-time input; Rust owns runtime state and input behavior.
4. Existing 0.0.6 release-mode low-latency rendering is retained.

## 1. Transparency root cause and repair

0.0.7 introduced XRender source-over composition but supplied straight RGB channels together with fractional alpha. The observed result was exactly the reported failure: selection fills looked too bright rather than like the browser prototype's translucent accent.

0.0.8 converts straight 8-bit runtime colors to premultiplied 16-bit XRender source channels before creating cached solid Pictures. Alpha-bearing rectangle borders now use the same XRender path rather than the opaque Xlib stroke path.

The V8 desktop states are represented as typed runtime colors:

```text
hover       accent @ 15% fill, 30% border
selected    accent @ 28% fill, 65% border
selection   accent @ configured 0..60% fill
snap        accent @ configured 0..60% fill
```

The previous `sel-*` child overlays were deleted. Selection now overrides background/border on the same desktop-entry node used by `:hover`; clearing selection removes those overrides and returns control to compiled hover CSS.

## 2. Desktop entry geometry and interaction

Desktop entries now follow the V8 tile contract:

```text
tile              82 x 82
image             44 x 44
horizontal padding 3 px each side
label row         76 px wide
column gap         4 px
grid pitch        92 x 91
```

The label row is itself centered through flex layout because the constrained renderer intentionally does not implement the browser's complete text-align/ellipsis stack.

New Folder uses the normal folder image, matching the web prototype's dynamically created folder model.

Right-click targets the clicked desktop item and opens a clamped context menu beside it. Trash receives `Empty Trash`; normal entries receive `Create Shortcut`, separator and `Delete`. All actions mutate virtual state only.

Multi-selection group drag remains transactional: the drag captures every selected original rectangle, moves them by one shared delta, clamps the group to the work area, and grid-snaps the complete set while avoiding unselected occupied slots.

## 3. Taskbar model

The previous static task buttons are now backed by session-local application state.

Initial pinned activities:

```text
Firefox
Dolphin / Files
Konsole / Terminal
Code
```

Settings, Elisa and generic apps have latent task slots that become visible while running and remain if pinned.

For each task:

```text
pinned || running  -> button visible
running            -> indicator visible
running inactive   -> 24 x 2 #87919b
running focused    -> 32 x 3 current accent
```

Right-click follows the V8 reference controller contract:

```text
pinned + closed   -> Open / Unpin
pinned + running  -> Unpin / Maximize|Minimize / Close
running unpinned  -> Pin / Maximize|Minimize / Close
```

The selected Maximize/Minimize row is derived from the fake window's real virtual state.

## 4. Window geometry fixes

The outer fake window has always been mutable, but several application body nodes still retained their original fixed height. Therefore maximizing could resize the frame while leaving the lower portion of the content short.

`apply_window()` now updates both:

```text
outer width/height = current window rectangle
body width         = current window width
body height        = max(window height - 31px titlebar, 1px)
```

This applies consistently to Browser, Files, Terminal, Code, Settings, Music and Generic windows.

Close-button hover uses a rounded red background to avoid the square red corner that escaped the visual top-right rounding in the small renderer.

## 5. Sticky notes

The reference uses browser `white-space: pre-wrap` and a contenteditable editor. RustWebRender intentionally has no general browser paragraph engine yet, so 0.0.8 implements deterministic lightweight line wrapping for the showcase. The text width and active font size determine a bounded character estimate and the copy is split across four native line nodes.

Right-click on the note now opens a local context menu at the pointer. Settings exposes virtual background, text color and text-size cycles; Delete hides the note. No file or OS state is touched.

## 6. Clock and workspace topology

Clock time and date each occupy a 66px centered flex row inside the 74px taskbar clock button.

The four default workspaces now use the V8 two-row topology:

```text
1  2
3  4
```

The DOM is represented as two vertical columns (`1/3`, `2/4`) because the current renderer supports Flexbox but not CSS Grid. The visible result and directional-navigation topology are equivalent to the browser prototype.

## 7. Accent propagation

One `DemoState::accent` remains authoritative. `set_accent()` updates typed CSS variables and then resynchronizes:

- workspace pager;
- running/focused task indicators;
- selected desktop entries;
- display preview state;
- Settings controls;
- selection/snap overlay variables.

This avoids disconnected hardcoded accent colors.

## 8. Host isolation

The showcase controller is explicitly guarded against:

```text
std::process::Command
Command::new
xdg-open
gio open
system(...)
```

About, session, application, display, network and file-browser actions remain fake/native prototype state inside the renderer window.

## 9. Verification status

The package is statically checked for:

- unique HTML IDs;
- local asset closure;
- action-handler closure;
- runtime literal ID closure;
- supported CSS property profile;
- exact PPM payload lengths;
- modified Rust source delimiter balance;
- shell syntax;
- virtual-only source guard;
- no distributed font binaries;
- manifest integrity and fresh archive extraction.

The artifact assembly environment does not provide `cargo`/`rustc`, therefore this report does not claim an executed Rust compilation. On the target machine, `./check` is the mandatory compile/unit/release-smoke gate before `./xephyr`.
