# FlameWM V4 — Complete Prototype-to-IceWM 4.1.0 Feature Gap & Native Implementation Report

**Project:** FlameWM  
**Studio:** ArkFlame Studios  
**Native base:** IceWM 4.1.0 / X11  
**Reference UX:** `breeze-desktop-prototype-v4`  
**Report role:** Current implementation contract for turning the verified IceWM 4.1.0 source into the native equivalent of the approved FlameWM V4 browser prototype.  
**Date:** 2026-09-02

---

# 0. Assumptions and authority

1. The current explicit V4 request is the highest product authority.
2. IceWM 4.1.0 source behavior is the implementation authority for what the base already does.
3. The browser prototype is a behavioral/visual specification only. HTML/CSS/JavaScript is **not** part of the production FlameWM runtime.
4. FlameWM remains an X11-first IceWM fork for this generation.
5. New product logic should be isolated in Flame-owned source; mature IceWM state owners remain authoritative.
6. No Qt/GTK/QML/Electron/WebView shell dependency is introduced merely to copy Plasma/Windows visual behavior.
7. The 40 MiB-class combined PSS objective remains a measured optimization target, not an unverified promise.
8. Latest V4 decisions supersede older project-source decisions where they conflict.

## 0.1 Explicit V4 supersessions

The following older ideas are **not** part of the current approved target:

- **No Activities/Overview taskbar button.** V2 removed it and V4 does not restore it.
- **No maximize-button hover snap-layout selector.** V4 explicitly removes it.
- **No cool-blue default accent.** Current default is Flame red.
- **No left-aligned title.** Current target centers the application title against the complete titlebar.
- **No light default.** Current prototype default shell is dark/pitch-black.
- **No fixed non-user-configurable shell font.** IBM Plex Sans remains default, but V4 exposes family, global bold and size offset.
- **No verbose About page.** Current user-facing About is logo + `A lightweight desktop by ArkFlame Studios` only.
- The current desktop context menu intentionally retains **Open Terminal, Create New Folder, Add Virtual Desktop, Desktop and Wallpaper**, because V4 explicitly requires their icons to work. That supersedes older narrower-menu wording.

This matters because several older FlameWM reports describe Overview and a maximize-hover layout chooser. A coding agent implementing those now would be implementing obsolete product behavior.

---

# 1. Audit basis

## 1.1 Native source audited

Extracted source:

```text
icewm-master(2).zip
VERSION -> PACKAGE=icewm, VERSION=4.1.0
```

Verified source owners include:

```text
src/default.h
src/themable.h
src/wmapp.cc/.h
src/wmmgr.cc/.h
src/workspaces.h
src/wmframe.cc/.h
src/movesize.cc
src/wmtitle.cc/.h
src/wmbutton.cc/.h
src/wmtaskbar.cc/.h
src/atasks.cc/.h
src/objbar.cc/.h
src/aworkspaces.cc/.h
src/aclock.cc/.h
src/aapm.cc/.h
src/apppstatus.cc/.h
src/wmprog.cc/.h
src/fdomenu.cc
src/wmswitch.cc/.h
src/preview.cc/.h
src/yicon.cc/.h
src/ywindow.cc/.h
src/yxtray.cc/.h
src/wmminiicon.cc/.h
src/icewmbg.cc
src/icesm.cc
src/CMakeLists.txt
src/Makefile.am
```

## 1.2 V4 prototype audited

Primary files:

```text
breeze-desktop-prototype-v4/index.html
breeze-desktop-prototype-v4/styles.css
breeze-desktop-prototype-v4/app.js
breeze-desktop-prototype-v4/PORTING_CONTRACT.md
```

The prototype currently models:

- pitch-black desktop;
- Flame red semantic accent;
- supplied FlameWM Start SVG;
- FlameWM desktop wordmark;
- compact cascading Start menu;
- app category menus and search;
- Power / Session actions;
- pinned/running task buttons;
- two-row virtual-desktop topology;
- workspace add/remove;
- `Ctrl + Super + Arrow` desktop navigation;
- four-edge taskbar docking by direct drag gesture;
- media/audio/Wi-Fi status popovers;
- centered time/date and calendar popup;
- movable/resizable/minimizable/maximizable windows;
- full-titlebar-centered application names;
- drag-to-edge half/quarter/maximize window layouts with preview;
- cross-workspace dragging by edge dwell;
- desktop filesystem-like icon grid;
- bounded icon positions;
- file-entry context actions;
- Trash and drag-to-Trash behavior;
- selection rectangle, multi-selection and group drag;
- Appearance/Desktop/Displays/Fonts/Hotkeys/About settings;
- per-monitor simulated resolution and shell scale;
- font family/global bold/size offset;
- Breeze SVG icon policy;
- local state persistence.

---

# 2. Classification vocabulary

The user requested three primary buckets. This report uses exactly these:

## A. **ICEWM HAS IT AND IT IS OK — KEEP**

The mature IceWM behavior already satisfies the functional requirement. FlameWM may change defaults/theme/branding, but must not write a second implementation.

## B. **ICEWM HAS A FOUNDATION BUT FLAMEWM MUST MODIFY IT — MODIFY**

IceWM owns the correct state/protocol/foundation, but the interaction, layout, rendering, product semantics or API is insufficient for V4.

## C. **ICEWM DOES NOT HAVE THE PRODUCT FEATURE — ADD**

A genuinely new FlameWM subsystem or host-service integration is required.

Where an external Linux service is involved, it remains in **ADD** because FlameWM must add the shell integration, even though FlameWM must not reimplement NetworkManager/PipeWire/etc.

---

# 3. Executive feature classification

## 3.1 Keep — mature IceWM behavior to preserve

IceWM already provides the hard low-level machinery for:

- X11 client management;
- EWMH/ICCCM integration;
- focus and stacking;
- interactive window move;
- interactive window resize;
- minimize/restore;
- maximize/restore;
- fullscreen;
- titlebar double-click maximize;
- work-area calculation;
- taskbar strut reservation for current top/bottom model;
- XRandR output discovery and monitor geometry tracking;
- virtual desktop/workspace engine;
- moving windows between workspaces;
- dynamic workspace-count growth/shrink foundation;
- existing half/quarter tile geometry;
- edge-switch timing infrastructure;
- task-window state tracking;
- task grouping foundation;
- system tray/XEmbed;
- clock formatting engine;
- battery/APM foundation;
- QuickSwitch / Alt+Tab;
- live QuickSwitch previews;
- XDG/Freedesktop application discovery;
- freedesktop icon-theme lookup including scalable formats;
- wallpaper/background process (`icewmbg`);
- session actions and commands for lock/logout/reboot/shutdown/suspend;
- preference/theme parsing;
- keyboard shortcut infrastructure.

These are the reasons IceWM is the correct base rather than a rewrite.

## 3.2 Modify — IceWM owners that need FlameWM product behavior

Major modifications are required around:

- titlebar centering semantics and visuals;
- window-control scale/visibility/hit targets;
- mouse edge/corner snap gestures and preview;
- snapped floating-geometry restore state;
- drag-only workspace edge traversal;
- workspace indexed insertion/removal and two-row pager UX;
- taskbar composition, modern metrics, spacing and four-edge orientation;
- pinned + running application unification;
- task context menus;
- compact Start tree/search presentation;
- power/session presentation and icons;
- clock/date renderer and popup anchoring;
- theme/font design tokens;
- icon fallback policy and semantic sizing;
- wallpaper/settings integration;
- XRandR output foundation into a user-facing display model;
- existing shortcut infrastructure into a curated GUI editor;
- IceWM configuration into a safe Flame-owned live-settings layer.

## 3.3 Add — genuinely new FlameWM systems

The largest new systems are:

- `flamewm-desktop` filesystem desktop;
- desktop grid/layout persistence;
- selection rectangle;
- multi-selection and group movement;
- freedesktop Trash implementation;
- desktop shortcut creation/file actions;
- FlameWM watermark layer;
- NetworkManager Wi-Fi integration;
- asynchronous PulseAudio-compatible audio integration;
- MPRIS media integration;
- calendar popup;
- `flamewm-settings` application;
- per-output logical shell scaling;
- resolution/monitor settings frontend;
- font user-settings abstraction;
- Flame semantic palette/accent system;
- left/right taskbar orientation and drag docking;
- native Flame Start launcher surface/search model if the classic `YMenu` cannot meet the approved compact behavior cleanly;
- Flame config + IPC/reload layer;
- runtime/packaging rules for critical Breeze fallback icons.

---

# 4. Complete prototype-to-source matrix

## 4.1 Window-management engine

| V4 behavior | IceWM 4.1.0 | Bucket | Source owner | Required FlameWM action |
|---|---|---|---|---|
| Manage normal X11 windows | Mature | KEEP | `wmmgr.*`, `wmclient.*`, `wmframe.*` | Preserve |
| Focus/raise/stack | Mature | KEEP | `wmmgr.*`, frame focus order | Preserve |
| Window drag | Mature | KEEP | `movesize.cc`, `wmframe.*` | Preserve; add narrow snap hook only |
| Resize from edges/corners | Mature | KEEP | `movesize.cc`, frame/container | Preserve |
| Minimize | Mature | KEEP | `wmframe.*` actions | Preserve |
| Maximize/restore | Mature | KEEP | `wmframe.*` | Preserve |
| Fullscreen | Mature | KEEP | `wmframe.*` | Preserve |
| Double-click titlebar maximize | Already implemented | KEEP | `wmtitle.cc:186+`, `TitleBarMaximizeButton` | Keep enabled and regression-test |
| Work-area-aware geometry | Mature | KEEP | `wmmgr.*` | All Flame layout code must call it |
| Monitor-aware placement | Mature | KEEP | `ywindow.cc` XRandR + `wmmgr.*` | Reuse |

### Critical rule

FlameWM must never introduce a second window registry or second focus/geometry authority. Browser prototype objects such as `state.windows` are simulation-only.

---

# 5. Titlebar and window decorations

## 5.1 Title text centering

### What IceWM already has

IceWM exposes:

```text
TitleBarJustify=0..100
```

and `wmtitle.cc` uses `titleBarJustify` to place title text.

### Why this is not exactly V4

Current source computes centering inside the **available region between titlebar controls/resources**, approximately:

```cpp
stringOffset = lLeft + (lRight - lLeft - textWidth) * titleBarJustify / 100;
```

Therefore `TitleBarJustify=50` does not guarantee:

```text
title text center == complete frame/titlebar center
```

when the left and right occupied widths differ.

### Classification

**MODIFY.** IceWM has centering machinery, but V4 requires true frame-center semantics.

### Native implementation

Add a Flame title-layout mode that computes:

```text
desiredX = (titlebarWidth - titleTextWidth) / 2
```

Then apply collision protection:

```text
minimumX = left occupied extent + padding
maximumX = right occupied extent - padding - textWidth
paintX = clamp(desiredX, minimumX, maximumX)
```

This preserves the V4 visual center when space permits without drawing through controls on very narrow windows.

Do not globally rewrite IceWM's existing `TitleBarJustify`; make the Flame mode explicit and leave expert/upstream behavior available.

## 5.2 Minimize/maximize/close controls

### IceWM

Existing `YFrameButton`/`YFrameTitleBar` already own the controls and their actions.

### FlameWM work

**MODIFY rendering, KEEP behavior.**

Required:

- guaranteed visible glyph fallback;
- Breeze/Flame flat glyph language;
- correct maximize vs restore glyph;
- scalable hit target separate from visible glyph size;
- high-contrast controls on dark titlebar;
- close hover destructive state;
- no hidden button because an icon resource failed to load.

The current project visual audit correctly identifies the need to separate:

```text
visual glyph size
button hit target
frame visual border
resize hit region
```

## 5.3 Font and titlebar scale

Existing raw theme metrics/fonts are global. V4 per-monitor scale requires resolving titlebar height/font/icon size against the output containing the frame. This is covered by the new Scale Manager later in the report.

---

# 6. Window layout/snap system

## 6.1 Existing IceWM geometry to reuse

`YFrameWindow::wmTile()` already implements:

- left half;
- right half;
- top half;
- bottom half;
- top-left quarter;
- top-right quarter;
- bottom-left quarter;
- bottom-right quarter;
- center.

It uses the current screen's **work area**.

Key actions also already exist in `src/default.h:504-512`.

### Classification

Geometry foundation: **KEEP/MODIFY**, not ADD from zero.

## 6.2 What V4 adds

V4 requires drag semantics:

```text
left edge       -> left half
right edge      -> right half
top-left        -> top-left quarter
top-right       -> top-right quarter
bottom-left     -> bottom-left quarter
bottom-right    -> bottom-right quarter
top center      -> maximize
```

IceWM does not currently provide this complete Windows/KWin-style mouse target/preview/commit state machine.

### Classification

**MODIFY** the interactive move path around existing geometry.

### Source touchpoint

`src/movesize.cc` should only call a Flame-owned controller, for example:

```text
flamewm::snap::Controller
```

Do not paste snap calculations into `movesize.cc`.

## 6.3 Snap preview

### IceWM

No approved V4 target preview overlay exists.

### Classification

**ADD.**

Implement a small override-redirect Flame window/overlay:

```text
accent border
accent-muted fill
no compositor requirement
no blur
pointer transparent
```

It should use the same semantic accent token as the V4 desktop selection rectangle.

## 6.4 Restore geometry

### IceWM

Existing tiling places geometry, but does not by itself encode the precise V4 contract:

> first snap stores floating rectangle; drag snapped/maximized titlebar away and restore it under pointer.

### Classification

**ADD minimal per-frame snap state**, integrated with existing `YFrameWindow` lifetime.

Required state:

```text
SnapTarget currentTarget
YRect unsnappedOuterGeometry
bool hasUnsnappedGeometry
```

Rules:

- first floating -> snap captures geometry;
- snap -> snap does not overwrite original floating geometry;
- manual resize exits snapped state;
- explicit maximize uses normal IceWM maximize state;
- drag-away restores floating rectangle;
- output removal clamps restoration to surviving work area.

## 6.5 Existing center-tile bug

Current 4.1.0 center branch uses:

```cpp
x = mx + (mx + Mx - w) / 2;
y = my + (my + My - h) / 2;
```

This adds monitor/work-area origin twice for non-zero origins.

Before any generalized layout code relies on center geometry, fix it as an upstreamable generic IceWM correctness patch:

```text
x = workLeft + (workWidth - targetWidth) / 2
y = workTop  + (workHeight - targetHeight) / 2
```

V4 does not expose center snap, but repairing source before building on the same math avoids latent multi-monitor defects.

## 6.6 Explicitly removed: maximize-hover layout chooser

Older reports proposed `FlameSnapPopup` on maximize hover. V4 explicitly removes it.

**Do not implement it.**

No hover timer, no chooser popup, no Super+Z requirement is needed for current V4 unless a later explicit product request restores it.

---

# 7. Virtual desktops/workspaces

## 7.1 Existing engine

IceWM already has:

- workspace model;
- `_NET_NUMBER_OF_DESKTOPS` handling (`wmmgr.cc:778+`);
- runtime extension (`extendWorkspaces`);
- runtime shrink (`lessenWorkspaces`);
- EWMH desktop property updates;
- moving windows across workspaces;
- workspace pager;
- edge-switch delay infrastructure;
- previous/next workspace key actions.

### Classification

Core engine: **KEEP**.

## 7.2 Runtime add/remove exact selected desktop

### Gap

Existing shrink semantics remove from the tail. V4 context UI means the desktop the user acts upon is the semantic target.

### Classification

**MODIFY** `YWindowManager` with a first-class indexed transaction.

Add:

```cpp
bool insertWorkspace(int index);
bool removeWorkspace(int index);
```

with one atomic transaction that updates:

- model names/count;
- every frame workspace index;
- active/last workspace;
- window visibility/focus;
- `_NET_NUMBER_OF_DESKTOPS`;
- `_NET_CURRENT_DESKTOP`;
- `_NET_DESKTOP_NAMES`;
- `_NET_DESKTOP_VIEWPORT`;
- `_NET_WORKAREA`;
- pager;
- window-list/move menus.

Hard invariant:

```text
workspaceCount >= 1
```

## 7.3 V4 two-row topology

IceWM has `TaskBarDoubleHeight` and workspace placement options, but the V4 pager is a compact **two-row workspace topology inside the product panel**, not simply “make the entire taskbar twice as tall.”

### Classification

**MODIFY** `WorkspacesPane`/Flame panel composition.

The topology should have a single model function:

```text
index -> row/column
row/column + direction -> index
```

so pointer rendering and keyboard navigation agree.

## 7.4 Ctrl + Super + Arrow navigation

IceWM already has configurable workspace key actions, primarily previous/next and explicit workspace numbers.

### Classification

**MODIFY/WRAP** existing shortcut infrastructure.

FlameWM needs product-level directional actions corresponding to its two-row topology:

```text
WorkspaceLeft
WorkspaceRight
WorkspaceUp
WorkspaceDown
```

Do not fake Up/Down by hardcoded desktop numbers in the UI.

## 7.5 Dragged-window edge workspace switching

IceWM already has edge switching and moving-window edge behavior (`movesize.cc`, `wmframe.cc`, `wmmgr.cc`).

### V4 difference

Passive pointer edge switching is not wanted. Only an actively dragged titlebar/window should traverse an adjacent desktop after dwell.

### Classification

**MODIFY existing edge infrastructure.**

Required conflict policy with snap:

1. corners always mean quarter snap;
2. side edge shows half-snap preview immediately;
3. quick release commits snap;
4. dwelling at extreme left/right for configured delay cancels preview and changes workspace while carrying the frame.

---

# 8. Taskbar/panel

## 8.1 Existing foundation

`TaskBar` already owns/hosts:

- Start button;
- show-desktop/window-list controls;
- toolbar launchers;
- workspaces;
- task buttons;
- tray;
- clock;
- battery;
- CPU/MEM/network monitors;
- other legacy applets.

### Classification

Core panel/window: **KEEP**, presentation/composition: **MODIFY**.

## 8.2 Current V4 composition

```text
Start
Pinned + running applications
Flexible unused drag surface
Virtual desktops
Media
Audio
Wi-Fi/network
System tray when needed
Centered clock/date
```

There is no Activities button.

## 8.3 Disable legacy clutter by default

Use existing preferences to default off:

```text
TaskBarShowCPUStatus=0
TaskBarShowMEMStatus=0
TaskBarShowNetStatus=0
TaskBarShowMailboxStatus=0
address bar off
legacy configuration controls hidden
```

Do not delete those mature upstream features unless a measured maintenance reason justifies deletion.

## 8.4 Four-edge taskbar

### IceWM

Current product preference is effectively:

```text
TaskBarAtTop = false/true
```

which is bottom/top. Current taskbar layout code is heavily horizontal.

### V4

```text
Bottom
Top
Left
Right
```

and direct drag docking.

### Classification

**ADD/MODIFY major taskbar capability.**

Recommended native model:

```cpp
enum class PanelEdge { Bottom, Top, Left, Right };
```

Do not overload `TaskBarAtTop` with magic integers.

Required work:

- generalize taskbar geometry;
- EWMH struts for vertical bars;
- work-area recomputation;
- horizontal/vertical layout axis;
- popover anchoring per edge;
- task button orientation;
- workspace pager reflow;
- clock/date vertical treatment;
- status icon flow;
- drag-target preview;
- output edge constraints.

## 8.5 Direct taskbar docking gesture

### IceWM

Missing.

### Classification

**ADD Flame taskbar interaction.**

Only unused panel space begins docking drag. Child controls must consume their normal clicks.

Flow:

```text
pointer press/hold blank taskbar
-> monitor root motion
-> nearest legal edge within threshold
-> draw edge preview
-> release
-> persist edge
-> update struts/work area
-> relayout shell
```

No panel-edit mode.

## 8.6 Right-side spacing

V3/V4 already corrected excessive gaps. Native taskbar should replace scattered legacy margins with semantic Flame metrics:

```text
statusIconGap
statusHitTarget
clockInset
trayGap
```

Do not tune individual applets with independent magic numbers.

---

# 9. Pinned/running application area

## 9.1 Existing foundations

IceWM currently separates:

- toolbar/ObjectBar launchers;
- running `TaskPane`/`TaskButton` windows;
- task grouping by class.

The task model already knows focused/minimized/normal states.

### Classification

State tracking: **KEEP**. Unified product model/rendering: **MODIFY**.

## 9.2 Required application identity

Use stable mapping in order:

1. desktop application ID;
2. `.desktop` `StartupWMClass`;
3. X11 `WM_CLASS`;
4. normalized fallback.

Never use current window title as application identity.

## 9.3 V4 task states

```text
pinned, not running     -> icon only, no activity indicator
running background      -> icon + neutral/subtle running indicator
focused                 -> icon + strong Flame accent indicator
minimized               -> indicator remains, icon de-emphasized
```

Reuse IceWM's existing frame/task state as authority.

## 9.4 V4 context-menu semantics

Pinned:

```text
Unpin only
```

Running non-pinned:

```text
Activate
Close
```

This is a deliberate V4 product rule. Do not expose Close on a pinned task just because a running instance exists.

### Classification

**MODIFY** task context-menu policy.

## 9.5 Exact context positioning

Bottom taskbar:

```text
menu.left == clicked task icon left
menu.bottom <= taskbar.top
```

Other edges anchor beside the relevant panel edge. Create one general edge-aware popover/menu positioning helper instead of task-specific coordinate hacks.

---

# 10. Start menu / application launcher

## 10.1 IceWM foundations to keep

IceWM already has:

- Start/root button;
- cascading `YMenu` infrastructure;
- application/program menu mechanism;
- XDG `.desktop` parsing through FDO menu code;
- application names/icons/categories;
- program launching;
- keyboard/menu navigation;
- logout/session menu actions.

### Classification

Discovery/launch/action engine: **KEEP**.

## 10.2 Why classic Start needs Flame modification

`StartMenu::refresh()` is architected for classic IceWM management and can append:

- Programs;
- Run;
- Windows;
- Settings;
- Focus;
- Themes;
- Help;
- filesystem browsing;
- Logout.

V4 wants a compact Windows-XP-like tree whose visible purpose is apps plus the deliberate session submenu.

### Classification

**MODIFY heavily or ADD a dedicated Flame launcher surface while reusing the FDO model.**

The correct implementation choice depends on whether `YMenu` can cleanly host the search field and exact V4 row behavior without invasive legacy changes.

Recommended architecture:

```text
FlameLauncher
    AppModel (extracted/reused FDO desktop parsing)
    SearchModel
    Category list
    Cascading app submenu
    PowerSession submenu
```

Do not hardcode production apps.

## 10.3 Search

V4 search field has a visible left search icon and filters local applications.

IceWM's classic Start has no product search input.

### Classification

**ADD launcher search UI**, reusing XDG app model.

Requirements:

- local-only;
- in-memory filtering after app model load;
- keyboard typing when Start opens;
- arrows + Enter + Escape;
- no web results;
- no shell interpolation of search input.

## 10.4 More real apps in V4

V4 hardcodes extra identities only to test visual density and icon correctness:

- Kate;
- KWrite;
- Ark;
- KCalc;
- Spectacle;
- Okular;
- Gwenview;
- Kdenlive.

### Native classification

**No production feature needs to hardcode these.** IceWM's existing XDG discovery already solves installed-app discovery. FlameWM must simply expose those real `.desktop` entries when installed.

This is a **KEEP existing discovery + MODIFY launcher presentation** item, not an ADD catalog.

## 10.5 Power / Session

IceWM already has:

- `actionLock`;
- `actionLogout`;
- `actionReboot`;
- `actionShutdown`;
- `actionSuspend`;
- corresponding command preferences (`LockCommand`, `LogoutCommand`, `RebootCommand`, `ShutdownCommand`, `SuspendCommand`).

### Classification

Action backend: **KEEP**. V4 tree/icon presentation: **MODIFY**.

Use valid Breeze high-contrast icons with Flame fallbacks. Do not reimplement power management in the WM.

---

# 11. Desktop filesystem shell

This is the largest V4 feature family that IceWM does not already provide.

## 11.1 What IceWM has

- root/background layer;
- `icewmbg` wallpaper process;
- minimized-window `MiniIcon` objects that demonstrate draggable desktop child behavior.

### Important correction

`MiniIcon` is **not** a filesystem desktop. It represents minimized windows.

## 11.2 Architecture decision

### Classification

Filesystem desktop: **ADD** as a separate process:

```text
flamewm-desktop
```

Rationale:

- file-system watching belongs outside the reliability-critical WM;
- file actions/DND/trash can fail without crashing window management;
- easier upstream merges;
- independently measurable memory;
- follows IceWM's existing process-separation philosophy (`icewmbg`).

## 11.3 Native source shape

```text
src/flamewm-desktop/
    main.cc
    desktopapp.*
    model.*
    view.*
    item.*
    layout.*
    watcher.*
    selection.*
    trash.*
    fileactions.*
    watermark.*
```

Use IceWM lightweight rendering primitives where feasible.

## 11.4 XDG Desktop directory

Resolve the standard user Desktop directory; do not hardcode `~/Desktop`.

Model item types:

```text
Directory
RegularFile
DesktopLauncher
Symlink/Shortcut
TrashPseudoItem
```

## 11.5 inotify

**ADD** event-driven filesystem watching.

React to:

- create;
- delete;
- rename/move;
- attribute changes that affect icon/launchability.

Coalesce event bursts. Do not poll the Desktop directory every second.

---

# 12. Desktop grid and placement

## 12.1 V4 contract

- grid always fits usable desktop work area;
- no item may be placed outside;
- taskbar edge changes recompute available cells;
- new items choose deterministic free cells;
- positions persist independently from filesystem object identity.

### IceWM

No general filesystem grid.

### Classification

**ADD.**

## 12.2 Grid model

Persist logical cells, not raw pixels:

```text
outputIdentity
column
row
```

Then derive pixels from current Flame desktop metrics and output scale.

This is essential for:

- taskbar movement;
- resolution changes;
- shell scale changes;
- monitor reconnect;
- DPI changes.

## 12.3 Output change policy

When output geometry changes:

1. recompute legal rows/columns;
2. retain cells that still exist;
3. clamp/reflow invalid positions deterministically;
4. items on removed output move to first-free cells on surviving primary output;
5. never silently drop layout metadata.

---

# 13. Selection rectangle and multi-selection — V4 new feature

## 13.1 IceWM

No filesystem desktop selection model.

### Classification

**ADD to `flamewm-desktop`.**

## 13.2 Selection overlay

Blank desktop primary-button drag creates a rectangle with:

```text
Flame accent border
accent-muted fill
same semantic palette as snap preview
```

The rectangle is presentation only and must not receive pointer input.

## 13.3 Selection model

One authoritative set:

```cpp
std::unordered_set<ItemId> selectedItems;
```

During rectangular drag, select items whose visual hit rectangles intersect the selection rectangle.

Required deterministic behavior:

- blank click without meaningful drag clears selection;
- a drag threshold prevents accidental 1px rectangles;
- clicking an unselected item normally selects it alone;
- group state is independent from item filesystem ordering.

## 13.4 Group drag

When dragging any member of the selected set:

- move the whole set;
- preserve cell-offset vectors;
- compute candidate delta once;
- clamp the **entire group's bounding cell rectangle**;
- reject candidate deltas that overlap unselected items;
- commit one transaction for the whole group.

Do not move each item independently and then attempt to repair collisions afterward.

## 13.5 Drop group onto Trash

If Trash is the drop target:

- exclude the Trash pseudo-item itself;
- trash every selected normal entry as one UI operation;
- if one filesystem operation fails, report exact failures rather than pretending the whole operation succeeded;
- refresh the Desktop model from actual filesystem state.

---

# 14. Trash semantics

## 14.1 Prototype

V4 has a simulated Trash state, drag-to-Trash and Empty Trash.

## 14.2 Native implementation

### Classification

**ADD.**

Implement the freedesktop.org Trash specification rather than inventing a private FlameWM trash directory.

Home trash is based on:

```text
$XDG_DATA_HOME/Trash/
    files/
    info/
```

and each trashed object gets a corresponding `.trashinfo` recording original path and deletion time. Cross-filesystem/top-directory trash rules must follow the specification.

This provides interoperability with KDE/GNOME/other desktop tooling.

## 14.3 Empty Trash

Enumerate supported user trash roots and erase their contents safely. Errors are surfaced; do not claim empty if files remain.

## 14.4 “Delete” context action

V4's label is `Delete`. Native semantics should be explicit:

- drag onto Trash = recoverable trash;
- Trash menu `Empty Trash` = permanent erase of trash contents;
- context `Delete` should either be a confirmed permanent erase or be renamed to `Move to Trash` in a future explicit product decision.

Do **not** silently map a destructive-looking `Delete` action to inconsistent behavior. Until changed by product direction, preserve the label but require a clear destructive confirmation for permanent deletion.

---

# 15. Create Shortcut

## 15.1 IceWM

No general desktop shortcut manager.

### Classification

**ADD to desktop file-actions layer.**

## 15.2 Native semantic policy

For an application `.desktop` launcher:

- create a collision-safe Desktop copy/launcher with executable/desktop-entry semantics preserved.

For a regular file/directory:

- create a filesystem symbolic link in the Desktop directory, with a collision-safe `… Shortcut`-style display name.

Never generate shell commands by concatenating filenames.

---

# 16. Desktop context menu

Current V4 actions:

```text
Open Terminal
Create New Folder
Add Virtual Desktop
Desktop and Wallpaper
```

## 16.1 Open Terminal

IceWM can launch programs/commands, but this exact contextual UX belongs to Flame desktop.

### Classification

**ADD desktop action, reuse IceWM/session launch helper.**

Open terminal with working directory set to the XDG Desktop directory if supported safely.

## 16.2 Create New Folder

### Classification

**ADD filesystem action.**

Requirements:

- create collision-safe default name;
- immediately refresh/select new item;
- choose first-free grid cell;
- optionally enter rename state later if product specifies it.

## 16.3 Add Virtual Desktop

Workspace engine exists.

### Classification

**MODIFY/WRAP existing workspace transaction** through Flame desktop context action.

Do not mutate workspace arrays in `flamewm-desktop`; send a narrow command to the WM authority.

## 16.4 Desktop and Wallpaper

### Classification

**ADD command route to `flamewm-settings`**, while wallpaper rendering continues through existing `icewmbg` initially.

---

# 17. Wallpaper and branding

## 17.1 Wallpaper

`icewmbg` already handles background image/color/scale behavior.

### Classification

Rendering backend: **KEEP**. Product UI/live setting: **MODIFY/ADD settings integration.**

Do not merge `icewmbg` into the WM until measurements prove the process separation is too expensive.

## 17.2 Pitch-black default

Ship FlameWM default with no wallpaper image / black background until the user chooses one. This is configuration/assets, not a new WM engine.

## 17.3 FlameWM desktop watermark

IceWM has no product watermark layer.

### Classification

**ADD to `flamewm-desktop`.**

Requirements:

- bottom-right of current usable desktop work area;
- adapt to panel edge;
- pointer transparent;
- behind normal windows;
- hidden during fullscreen;
- subtle opacity;
- scalable wordmark;
- user toggle retained.

---

# 18. Media control

## 18.1 IceWM

No MPRIS media applet.

### Classification

**ADD.**

## 18.2 Native integration

Use MPRIS over the session D-Bus with an IceWM-event-loop-compatible D-Bus dispatcher.

Required model:

```text
available players
active player
PlaybackStatus
Metadata title/artist
CanPlay/CanPause/CanGoNext/CanGoPrevious
```

No polling subprocess every second.

## 18.3 V4 taskbar icon state

The current visual contract explicitly says:

```text
Playing -> Play glyph
Paused  -> Pause glyph
```

This is unusual compared with “button shows the action you can take,” but it is the current explicit product behavior and must be treated as authoritative.

When external MPRIS state changes, update immediately.

---

# 19. Audio control

## 19.1 IceWM

No modern volume mixer/control applet.

### Classification

**ADD.**

## 19.2 Backend

Use libpulse asynchronous APIs against PulseAudio or PipeWire-Pulse compatibility:

- default sink;
- volume;
- mute;
- sink/server subscription events;
- set volume/mute.

Integrate watches/timers with IceWM's event loop rather than adding a GLib shell loop.

## 19.3 UI

- readable high-contrast audio glyph;
- volume popup;
- slider;
- mute state;
- wheel over taskbar icon can adjust volume if retained;
- no invisible slot if backend unavailable.

---

# 20. Wi-Fi/network control

## 20.1 Existing IceWM network applet

`NetStatusControl`/`apppstatus.cc` monitors interface throughput and reads network counters. It is not a Wi-Fi connection manager.

### Classification

Legacy throughput monitor: **KEEP but disable by default.**

V4 Wi-Fi product: **ADD.**

## 20.2 Backend

Use NetworkManager's system D-Bus API.

Required state:

```text
unavailable
disconnected
wired
wifi connecting
wifi connected
SSID
signal strength
available networks
```

Required operations:

- Wi-Fi enable/disable where appropriate;
- connect existing/saved profile;
- disconnect;
- refresh AP list;
- hand off new-secret creation to an appropriate secure path unless FlameWM later implements a proper NetworkManager SecretAgent.

No shell-string construction from SSID/password.

## 20.3 Icon contrast

The V4 dark-gray icon bug is a packaging/rendering failure mode that native FlameWM must structurally prevent:

- semantic `Network` icon role;
- tested dark/light fallback;
- symbolic icon recolor when available;
- no silent broken SVG currentColor handling.

---

# 21. Clock, date and calendar

## 21.1 Time/date engine

IceWM has `TimeFormat`, `TimeFormatAlt`, `DateFormat` and `strftime` rendering.

### Classification

Time engine: **KEEP**. Exact V4 visible layout: **MODIFY**.

V4 wants centered stacked/paired visible time and date. Add a Flame clock layout mode if current formatter cannot produce the exact two-line centered composition cleanly.

## 21.2 Calendar popup

IceWM's clock provides date tooltip/formatting but not the V4 month calendar popup.

### Classification

**ADD.**

Use standard calendar arithmetic/localtime; no external calendar daemon required.

## 21.3 Today circle

Paint the current date inside a square logical cell before applying 50% radius:

```text
cellWidth == cellHeight
border radius = 50%
```

Do not size the accent backplate by text width, which is what produces an oval.

---

# 22. Battery and tray

## 22.1 System tray

IceWM has XEmbed tray support.

### Classification

**KEEP.**

Do not replace it when adding native media/audio/network controls.

## 22.2 Battery

IceWM already has APM/battery taskbar support.

### Classification

Core state: **KEEP/MODIFY visual only**, unless modern power-service behavior is later requested.

V4 does not add a new battery feature, so do not expand scope.

---

# 23. FlameWM Settings application

IceWM has preferences/theme files, not the V4 product GUI.

### Classification

**ADD separate on-demand executable:**

```text
flamewm-settings
```

Why separate:

- zero memory when closed;
- failures do not crash WM;
- color picker/font/display controls can be complex without inflating WM owner classes;
- independently testable;
- long-term upstream isolation.

Recommended pages for current V4:

```text
Appearance
Desktop
Displays
Fonts
Hotkeys
About
```

Do not re-add obsolete pages/features simply because older reports contained them.

---

# 24. Appearance / accent system

## 24.1 IceWM

Theme system already supplies many colors/resources.

### Classification

Theme primitive: **KEEP**. Coherent product palette/live accent abstraction: **ADD/MODIFY.**

## 24.2 Semantic palette

Do not scatter `#ef4048` through C++.

Create:

```text
FlamePalette
    accent
    accentHover
    accentPressed
    accentMuted
    surface
    surfaceRaised
    textPrimary
    textSecondary
    border
    danger
```

All Flame shell surfaces consume those semantic tokens.

Default:

```text
Flame red
```

Presets + custom color picker update the same token authority.

---

# 25. Fonts — V4 expanded requirement

## 25.1 What IceWM already has

`src/themable.h` exposes separate font roles including:

```text
TitleFontName
MenuFontName
StatusFontName
QuickSwitchFontName
NormalButtonFontName
ActiveButtonFontName
NormalTaskBarFontName
ActiveTaskBarFontName
ToolButtonFontName
NormalWorkspaceFontName
ActiveWorkspaceFontName
ListBoxFontName
ToolTipFontName
ClockFontName
ApmFontName
InputFontName
LabelFontName
```

### Classification

Font rendering/roles: **KEEP**. V4 unified user abstraction: **MODIFY/ADD settings layer.**

## 25.2 V4 UI model

User chooses:

```text
font family
bold globally
size offset
```

Default family:

```text
IBM Plex Sans
```

## 25.3 Correct native implementation

Do not overwrite each raw font preference independently in arbitrary ways.

Define Flame semantic roles with base size deltas:

```text
UiBody             base + 0
UiSmall            base - 2
UiTitle            base + 0 or +1
UiSettingsHeading  base + 8
UiClock             base + 1
UiCaption           base - 3
```

Then apply:

```text
final family = selected family
final weight = globalBold ? Bold : role default
final size = role base size + user size offset
final physical size = logical final size * output scale
```

Map semantic roles to existing IceWM font owners.

## 25.4 Bold globally

Applies to **Flame-owned shell text** and Flame-controlled IceWM chrome/task/menu fonts. It cannot and should not force arbitrary third-party GTK/Qt application content bold.

## 25.5 No font binaries in FlameWM package unless separately licensed/required

Use installed fonts and robust fallback chain. Packaging may recommend/depend on IBM Plex Sans at the distribution level if desired.

---

# 26. Displays: resolution and per-monitor scale

This requirement needs careful separation of two different concepts.

## 26.1 What IceWM already has

IceWM already uses XRandR for:

- output discovery;
- output connection state;
- CRTC geometry;
- monitor work-area mapping;
- RR change notifications;
- relayout after screen-size changes.

Source examples:

```text
src/ywindow.cc:1873+
src/ywindow.cc:1883  XRRGetScreenResources
src/ywindow.cc:1890  XRRGetOutputInfo
src/ywindow.cc:1902  XRRGetCrtcInfo
src/wmmgr.cc:3904+   RR change handling
```

### Classification

Monitor discovery/geometry: **KEEP**.

User-facing resolution configuration and Flame per-output shell scale: **ADD/MODIFY.**

## 26.2 Resolution

Create a Flame output model over libXrandr:

```text
OutputId
connector name
EDID identity when available
connected
primary
available modes
current mode
CRTC
position
rotation
```

Settings must enumerate modes advertised by XRandR, not invent resolution strings.

Applying resolution/layout should use libXrandr output/CRTC APIs and then let existing IceWM RR-notify/work-area paths observe the committed change.

Required safety:

- validate mode belongs to output;
- never leave all outputs disabled accidentally;
- preview potentially disruptive display changes with an automatic timeout/revert transaction;
- persist only after confirmation;
- handle output disappearance while Settings is open.

## 26.3 Per-monitor FlameWM shell scale

IceWM 4.1.0 theme metrics are predominantly global raw integers. There is no coherent per-output logical scale system.

### Classification

**ADD `FlameScaleManager`.**

Recommended presets:

```text
100%
125%
150%
175%
200%
```

Persist by durable output identity:

```text
EDID hash + connector-name fallback
```

Do not persist only `monitor 0` / `monitor 1`.

## 26.4 Scale manager responsibilities

```text
output discovery mapping
logical -> physical conversion
scaled metrics cache
font cache keyed by role + scale + theme generation
icon cache request sizes
surface output selection
scale-change notifications
```

Every Flame-owned surface resolves metrics against its output:

- titlebar/frame;
- taskbar;
- Start;
- menus/popovers;
- desktop icons;
- selection rectangle border/hit metrics;
- calendar;
- settings;
- snap preview.

## 26.5 Critical X11 limitation

FlameWM can correctly scale **its own shell/chrome per output**.

A stacking X11 WM cannot universally force an arbitrary GTK/Qt/Xlib application's internal content to adopt a different per-monitor scale merely because its frame crossed a monitor.

Therefore the accurate product promise is:

> FlameWM shell and window chrome scale per monitor; client application content follows the client's toolkit/X11 scaling support.

Do not fake universal per-app scaling with output bitmap transforms by default; that can blur content and changes the semantics of the whole output rather than giving a clean toolkit-native scale.

---

# 27. Hotkeys

## 27.1 IceWM foundation

IceWM already has global key binding preferences and action dispatch.

### Classification

Core key handling: **KEEP**. Curated settings GUI/directional workspace abstraction/live conflict validation: **MODIFY/ADD.**

## 27.2 Current V4 user-facing set

Keep the editor simple and only expose current product actions, for example:

```text
Open Start
Workspace Left
Workspace Right
Workspace Up
Workspace Down
```

plus any currently approved window shortcuts already represented in V4.

Do not expose hundreds of raw IceWM key preferences as a generic binding editor.

## 27.3 Conflict validation

On capture:

- normalize modifiers;
- reject modifier-only combinations unless action intentionally supports one such as Super for Start;
- detect duplicate Flame-owned binding;
- require explicit replacement if conflict exists;
- atomic write;
- runtime update without restart where safe.

---

# 28. About

## V4 visible contract

Only:

```text
[FlameWM logo]
A lightweight desktop by ArkFlame Studios
```

### Classification

**ADD/replace the user-facing Settings page.**

Do not expose browser-prototype implementation text, Breeze asset notes or HTML/CSS/JS details.

The source package/documentation can still contain IceWM lineage, licenses and technical version metadata; the normal About page is intentionally minimal per current V4.

---

# 29. Breeze icons and missing-icon prevention

## 29.1 Existing IceWM foundation

`YIcon` already indexes icon themes and resolves common PNG/XPM/SVG candidates, then loads/scales to requested sizes.

### Classification

Icon search/loading engine: **KEEP**.

Flame fallback policy, semantic role sizing and packaging: **MODIFY/ADD.**

## 29.2 Required runtime lookup policy

```text
Flame-specific asset
-> Breeze / compatible installed theme
-> bundled audited critical fallback
-> hicolor/generic fallback
-> final built-in safe glyph for shell-critical action
```

Core UI must never render as an invisible dark-gray square because an SVG relies on unresolved `currentColor` or because the host does not have Breeze installed.

## 29.3 Semantic icon roles

Centralize sizes:

```text
TitleButton
TaskbarApp
TaskbarStatus
Menu
Launcher
SettingsCategory
Desktop
CalendarControl
```

Scale them through `FlameScaleManager`.

## 29.4 Critical fallback set

At minimum validate/ship fallbacks for:

- Start;
- Search;
- Internet/network;
- Media play/pause/prev/next;
- Volume/mute;
- Lock;
- Logout;
- Reboot;
- Shutdown;
- New Folder;
- Add Desktop;
- Desktop/Wallpaper;
- Home/Desktop/Network/System Disk;
- window minimize/maximize/restore/close;
- Trash.

Keep Breeze licensing/version metadata adjacent to any redistributed subset.

---

# 30. Dolphin/sidebar icons and third-party application theming

The V4 Dolphin window is a browser simulation of a file manager. Native FlameWM does not paint Dolphin's internal Qt widgets.

Therefore:

- FlameWM should set a coherent icon-theme/session environment so real Dolphin/Qt apps resolve Breeze appropriately;
- missing icon-theme fallback is Flame session/packaging policy;
- arbitrary client application internals remain the client toolkit's responsibility.

### Classification

IceWM window frame: **KEEP/MODIFY theme**. Client internal icon appearance: **session integration/packaging**, not a WM renderer rewrite.

Do not attempt to intercept every client icon paint call from the WM.

---

# 31. System settings persistence and live apply

## 31.1 Existing IceWM configuration

IceWM already loads preferences, theme and `prefoverride`.

### Problem

A modern Flame Settings app must not rewrite unrelated expert IceWM values or restart the WM for every harmless accent/font change.

### Classification

**ADD Flame-owned settings layer**, while preserving IceWM config as engine/expert layer.

## 31.2 Recommended config

```text
$XDG_CONFIG_HOME/flamewm/settings
```

Typed keys owned only by FlameWM.

Precedence:

```text
IceWM built-in defaults
-> FlameWM shipped product defaults/theme
-> FlameWM user settings
-> expert prefoverride
```

## 31.3 Atomic writes

```text
write temp
flush/close
rename temp -> settings
```

Unknown keys preserved across compatible version reads.

## 31.4 Runtime reload

Add narrow versioned control path:

```text
flamewmctl reload-settings
flamewmctl open-settings <page>
flamewmctl workspace-add <index>
...
```

An X11 ClientMessage/property protocol is a reasonable low-overhead X11-first implementation. Never expose arbitrary shell command execution.

---

# 32. Production process architecture

Recommended final process family:

```text
flamewm-session / existing IceWM-compatible session orchestration
├── icewmbg                  existing wallpaper process
├── flamewm                  IceWM-derived WM + taskbar + native status applets
├── flamewm-desktop          filesystem desktop/watermark, when enabled
├── lightweight notification daemon   distribution/session integration
└── lightweight polkit agent / locker distribution integrations

on demand only:
    flamewm-settings
```

Media/network/audio do **not** need standalone Flame daemons; integrate asynchronously in the WM/taskbar event loop.

---

# 33. Canonical Flame-owned source structure

Keep new product code physically isolated:

```text
src/flamewm/
├── core/
│   ├── runtime.*
│   ├── config.*
│   ├── control.*
│   └── scale.*
├── ui/
│   ├── palette.*
│   ├── metrics.*
│   ├── fonts.*
│   ├── iconroles.*
│   └── popover.*
├── snap/
│   ├── target.*
│   ├── state.*
│   └── overlay.*
├── workspace/
│   └── workspaceux.*
├── launcher/
│   ├── appmodel.*
│   ├── search.*
│   └── launcher.*
├── panel/
│   ├── composer.*
│   ├── taskidentity.*
│   ├── media.*
│   ├── audio.*
│   └── network.*
├── integrations/
│   ├── dbusdispatcher.*
│   ├── mpris.*
│   ├── networkmanager.*
│   └── pulse.*
└── display/
    └── randrmanager.*

src/flamewm-settings/
    ... pages + color picker + key capture + display controls

src/flamewm-desktop/
    ... model/view/layout/selection/trash/watcher/watermark
```

All new product code uses `namespace flamewm` where linked into the WM.

---

# 34. Upstream IceWM files that should be touched, and why

| Upstream file | Minimum justified Flame change |
|---|---|
| `src/wmapp.*` | construct/destroy Flame runtime; route narrow Flame actions/control messages |
| `src/movesize.cc` | active move -> Flame snap controller; drag-only workspace dwell hook |
| `src/wmframe.*` | minimal snap-state lifetime / region-application hook if required |
| `src/wmtitle.*` | true whole-titlebar center mode; scalable Flame chrome hooks |
| `src/wmbutton.*` | scalable button hit/glyph rendering if theme assets insufficient |
| `src/wmmgr.*` | indexed workspace transaction; generalized panel strut/orientation support; display/work-area notification hooks |
| `src/workspaces.h` | indexed insert/remove primitives |
| `src/aworkspaces.*` | V4 workspace context/pager topology hooks |
| `src/wmtaskbar.*` | Flame panel composition + vertical orientation/struts/docking hooks |
| `src/atasks.*` | Flame task presentation/context/identity integration |
| `src/objbar.*` | pinned launcher identity integration if reused |
| `src/aclock.*` | only if exact visible centered two-line date/time cannot be achieved by configuration |
| `src/wmprog.*` | only narrow compatibility/fallback if Flame Launcher becomes primary |
| `src/fdomenu.cc` | preferably extract reusable desktop-entry model, not duplicate parser |
| `src/yicon.*` | ideally no semantic rewrite; only repair proven theme/scaling defect |
| `src/ywindow.*` | expose output identity/geometry data needed by scale/display layer if current API insufficient |
| `src/icesm.cc` | launch/restart `flamewm-desktop` late in implementation |
| build files | register new libraries/executables/dependencies/tests/assets |

For every modified upstream file, keep a documented entry in `docs/UPSTREAM_TOUCHPOINTS.md`.

---

# 35. Things IceWM has poorly for the FlameWM target

This is the direct answer to “grab everything IceWM has poorly/not implemented.”

## 35.1 Global raw-pixel visual metrics

Problem:

- titlebar/icon/menu/taskbar metrics are largely global integers;
- no per-output logical UI scale;
- modern DPI/mixed-monitor shell behavior cannot be produced robustly by theme values alone.

Flame fix:

- semantic `FlameMetrics` + `FlameScaleManager`;
- per-surface output selection;
- scaled font/icon caches.

## 35.2 Legacy menu density and active-row rendering

Problem:

- hard-coded legacy padding;
- old flat/3D line semantics;
- not a modern popover/control surface.

Flame fix:

- explicit row height/padding/radius/state tokens;
- modern selection backplates;
- separate launcher/popover classes where `YMenu` assumptions conflict.

## 35.3 Classic Start architecture

Problem:

- shell-management menu rather than a focused app tree/search product.

Flame fix:

- reusable XDG app model;
- compact Flame category/menu/search UI;
- session actions isolated.

## 35.4 Hard-coded horizontal taskbar assumptions

Problem:

- top/bottom model;
- intrinsic applet sizes and legacy margins;
- not a first-class responsive panel composition.

Flame fix:

- orientation enum;
- Flame panel metrics;
- layout lanes;
- deterministic overflow;
- four-edge struts;
- direct docking.

## 35.5 Split launcher/task model

Problem:

- toolbar and running tasks are separate concepts.

Flame fix:

- stable desktop-app identity;
- one visual pinned/running application area;
- preserve IceWM task/window state as authority.

## 35.6 No filesystem desktop

Problem:

- MiniIcons are window miniatures, not files.

Flame fix:

- dedicated `flamewm-desktop` process with inotify/grid/trash/selection.

## 35.7 No native modern system controls

Problem:

- legacy throughput monitor is not NetworkManager;
- no MPRIS;
- no modern volume integration.

Flame fix:

- shared asynchronous D-Bus dispatcher;
- NetworkManager + MPRIS;
- libpulse client.

## 35.8 Theme icon resources can fail visually at modern SVG/symbolic expectations

Problem:

- legacy resource assumptions and host icon-theme variability can produce invisible or wrong-color symbols.

Flame fix:

- semantic roles;
- tested Breeze policy;
- critical local fallback subset;
- scale-aware explicit requested size;
- currentColor/symbolic handling tests.

## 35.9 No friendly Settings product

Problem:

- raw preferences are extremely capable but not a mainstream control center.

Flame fix:

- bounded on-demand `flamewm-settings`;
- live product-owned settings;
- expert config remains escape hatch.

---

# 36. Features that should explicitly NOT be implemented now

To prevent stale documents from expanding scope:

```text
NO Activities/Overview button/system
NO maximize-hover layout selector
NO Plasma widget framework
NO panel edit mode
NO arbitrary user applet system
NO compositor requirement
NO blur requirement
NO Wayland compositor
NO KRunner clone
NO online Start search
NO full NetworkManager replacement
NO audio server replacement
NO media daemon
NO permanent settings daemon
NO hard-coded production application catalog
NO attempt to scale arbitrary client application interiors from the WM
```

---

# 37. Native implementation order updated for V4

## Phase 0 — Baseline / fork discipline

- green CMake + CTest;
- green Autotools while supported;
- Xephyr dev command;
- upstream touchpoint ledger;
- baseline PSS/startup;
- fix center tile origin bug.

## Phase 1 — Scale/metrics + Flame visual foundation

Do this before pixel tuning:

- `FlamePalette`;
- `FlameMetrics`;
- `FlameScaleManager` skeleton;
- IBM Plex Sans default/font roles;
- Breeze critical fallback policy;
- dark/pitch-black + Flame red defaults;
- titlebar buttons/true title centering;
- intentional menu/task visual states.

## Phase 2 — Settings foundation

- Flame config store;
- IPC/reload;
- Appearance;
- Desktop;
- Fonts;
- Hotkeys;
- minimal About.

## Phase 3 — Window drag snap

- mouse targets;
- accent preview;
- restore geometry;
- drag-only edge conflict;
- **no hover chooser**.

## Phase 4 — Workspace UX

- indexed add/remove;
- minimum one;
- two-row topology;
- directional hotkeys;
- drag-window edge traversal.

## Phase 5 — Start + unified task area

- XDG app model;
- compact tree/search;
- session submenu;
- stable app identity;
- pinned/running tasks;
- exact V4 task context semantics.

## Phase 6 — Four-edge taskbar

- orientation enum;
- vertical struts/work area;
- child reflow;
- docking drag preview/commit;
- edge-aware popup positioning.

## Phase 7 — Displays

- XRandR output model/modes;
- per-output resolution setting with revert safety;
- per-output Flame shell scale;
- settings integration.

## Phase 8 — Status integrations

- D-Bus dispatcher;
- MPRIS;
- NetworkManager;
- libpulse;
- clock/calendar.

## Phase 9 — Filesystem desktop V4

- desktop process/window;
- XDG Desktop;
- inotify;
- grid;
- selection rectangle;
- multi-selection;
- group drag;
- shortcut/create-folder actions;
- freedesktop Trash;
- watermark;
- desktop context menu.

## Phase 10 — Release verification / memory surgery

- real resource measurements;
- 1,000-cycle menu/snap/settings stress tests;
- idle wakeup audit;
- output hotplug tests;
- process-isolation crash tests;
- final asset/license audit.

---

# 38. Required semantic verification matrix

## Windows

- move/resize unchanged with Flame snap disabled;
- minimize/maximize/restore/close unchanged;
- title double-click still maximizes;
- title center equals frame center where there is sufficient space;
- title never overlaps buttons on narrow frames;
- half/quarter/top drag targets exact;
- preview geometry equals committed geometry;
- snap respects taskbar strut on every panel edge;
- drag-away restores exact floating geometry;
- negative/non-zero monitor origins work.

## Workspaces

- add after first/middle/last;
- remove first/middle/last;
- cannot remove only workspace;
- every affected frame gets exact new desktop index;
- EWMH properties consistent;
- two-row directional navigation deterministic;
- passive edge pointer does not switch;
- dragged window can traverse side edge.

## Taskbar

- Bottom/Top/Left/Right all reserve correct work area;
- direct dock gesture only starts on unused bar region;
- popup anchoring correct on every edge;
- pinned menu = Unpin only;
- non-pinned running = Activate/Close;
- clock/date centered;
- no critical missing icon.

## Start

- installed XDG apps discovered;
- local category/search work;
- visible search icon;
- power/session icon and four action icons visible;
- keyboard navigation;
- no old IceWM configuration clutter in default view;
- no hard-coded dependency on test apps.

## Desktop

- actual filesystem create/delete/rename reflected;
- grid never places outside work area;
- taskbar move recomputes legal grid;
- selection rectangle intersect behavior exact;
- group drag preserves relative offsets;
- group cannot overlap unselected items;
- group clamps as a unit;
- drag to Trash creates spec-compliant Trash records;
- Empty Trash exact;
- output removal relocates safely;
- crash/restart desktop helper does not crash WM.

## Settings

- accent live applies;
- custom color valid/invalid behavior;
- font family live applies to Flame roles;
- global bold live applies;
- size offset preserves role-relative hierarchy;
- resolution change has safe revert;
- scale per monitor persists by durable identity;
- hotkey conflicts handled;
- About contains only approved visible content.

## Integrations

- NetworkManager absent -> graceful hidden/disabled network control;
- Pulse server restart -> reconnect without WM crash;
- MPRIS player appears/disappears -> media slot updates;
- playing/paused icon exact V4 rule;
- no recurring subprocess poll loop.

---

# 39. Performance gates

Measure owned processes with `/proc/<pid>/smaps_rollup` and record PSS, not just RSS.

At minimum:

```text
flamewm main WM/taskbar
icewmbg
flamewm-desktop
```

Idle scenario:

- 2 monitors where available;
- 20 desktop items;
- 4 workspaces;
- network/audio services connected;
- no menus open;
- no active media player;
- settings closed.

Gates:

- no high-frequency status polling;
- desktop uses inotify, not periodic scanning;
- no Overview resources because Overview does not exist in V4;
- no hidden snap chooser timers;
- no permanent settings process;
- no unbounded icon cache;
- no child/zombie accumulation;
- 40 MiB-class combined owned PSS remains the optimization target and must be reported honestly.

---

# 40. External standards/current public grounding

## IceWM

Current upstream identifies IceWM 4.1.0 as the 2026-08-06 release. The uploaded source matches 4.1.0.

## XRandR

X.Org documents RandR 1.2/libXrandr as providing automatic discovery of output modes (resolution/refresh) and dynamic output configuration. FlameWM should use that existing standard instead of shelling out to `xrandr` for persistent display controls.

## Trash

The freedesktop Trash Specification defines interoperable home trash at `$XDG_DATA_HOME/Trash`, with `files/` and `info/` plus `.trashinfo` metadata. Native FlameWM desktop trash must comply with that rather than implementing a private browser-like trash store.

---

# 41. Final decision table

## IceWM already has and is OK

Keep these engines:

```text
X11 WM core
focus/stacking
move/resize
minimize/maximize/fullscreen
titlebar double-click maximize
work-area authority
XRandR output geometry observation
workspace engine
window-to-workspace movement
runtime workspace count foundation
half/quarter tile geometry
edge-switch timing foundation
task-window state
task grouping foundation
system tray
battery foundation
clock/strftime engine
QuickSwitch/Alt+Tab
QuickSwitch previews
XDG application parsing/discovery
icon-theme lookup/SVG scaling foundation
icewmbg
session action backend
preferences/key-binding infrastructure
```

## IceWM has it but FlameWM must modify it

```text
whole-titlebar title centering
window-control visuals/hit sizes
mouse snap activation/preview/restore
workspace indexed insertion/removal
workspace two-row topology
Ctrl+Super directional desktop actions
drag-only workspace edge switching
taskbar composition and spacing
top/bottom panel model -> four-edge model
pinned/running task unification
task context menus
classic Start -> compact app tree/search
power/session menu presentation
clock/date visible centered layout
font roles -> simple global font settings
raw global UI metrics -> per-output logical metrics
icon lookup -> robust Breeze/fallback policy
wallpaper engine -> friendly settings live control
XRandR observer -> display-settings transaction model
raw key preferences -> curated Hotkeys UI
IceWM config -> Flame-owned safe live settings layer
```

## FlameWM must add

```text
Flame semantic palette
FlameScaleManager
left/right taskbar orientation
taskbar drag docking
drag snap preview overlay
snap floating restore state
Flame Launcher search surface where legacy YMenu is insufficient
MPRIS media integration
libpulse audio integration
NetworkManager Wi-Fi integration
calendar popup
flamewm-settings
font family/bold/size-offset GUI abstraction
per-output resolution/scale settings frontend and safe apply
flamewm-desktop
XDG Desktop filesystem model
inotify watcher
desktop grid persistence
selection rectangle
multi-selection
group drag
safe file actions/shortcut creation
freedesktop Trash
Flame watermark
critical Breeze fallback asset package
minimal About page
```

---

# 42. End-state architecture

The desired native system is not “replace IceWM.” It is:

```text
MATURE ICEWM CORE
    EWMH/ICCCM
    frames/focus/stacking
    work areas/XRandR observation
    workspaces
    task state
    tray
    QuickSwitch
    XDG/icon/wallpaper foundations

        +

FLAMEWM PRODUCT LAYER
    semantic visual/scale/font system
    exact V4 titlebar/taskbar/start UX
    drag snap
    workspace product operations
    event-driven status controls
    friendly settings
    four-edge panel

        +

FLAMEWM DESKTOP PROCESS
    filesystem desktop
    grid
    selection rectangle
    group drag
    trash
    watermark

        =

FLAMEWM V4 NATIVE EXPERIENCE
```

The implementation is complete only when an ordinary user cannot tell that the shell began as a classic IceWM UX, while the mature IceWM protocol/window-management engine remains underneath and upstream updates are still practical to merge.

---

# 43. Source anchors used for implementation handoff

```text
VERSION
    IceWM 4.1.0

src/wmtitle.cc:186+
    titlebar click/double-click maximize behavior

src/themable.h:28, 169, 181+
    title justification / TitleBarJustify and font/theme roles

src/wmframe.cc:1500+
    wmTile half/quarter/center geometry

src/default.h:504-512
    tile key actions

src/movesize.cc
    interactive move/resize and edge-switch hooks

src/wmmgr.cc:778+
    _NET_NUMBER_OF_DESKTOPS

src/wmmgr.cc:2854+
    extendWorkspaces

src/wmmgr.cc:2870+
    lessenWorkspaces

src/wmmgr.cc:2903+
    updateWorkspaces

src/default.h:315-318, 411
    edge workspace switching and EdgeSwitchDelay

src/aworkspaces.cc
    workspace button/pager interactions

src/wmtaskbar.cc:250+
    applet construction

src/wmtaskbar.cc:446+
    hard-coded taskbar layout model

src/default.h:321-363
    taskbar preferences

src/atasks.cc
    task state painting/click behavior

src/default.h:362
    task grouping preference

src/wmprog.cc:411+
src/wmapp.cc:542+, 998+
    logout/session actions and menus

src/default.h:447,453,455-457
    Lock/Logout/Shutdown/Reboot/Suspend commands

src/aclock.cc
src/default.h:466-468
    clock/date formats

src/apppstatus.cc
    legacy network throughput monitor, not Wi-Fi management

src/yicon.cc
    icon theme lookup + requested-size loading/scaling

src/fdomenu.cc
    XDG desktop application parsing/menu data

src/wmswitch.cc
src/preview.cc
    existing Alt+Tab and live previews

src/ywindow.cc:1873+
    XRandR output/CRTC discovery

src/wmmgr.cc:3904+
    RandR screen-change reaction

src/wmminiicon.cc
    useful draggable desktop child pattern, not filesystem desktop

src/icewmbg.cc
    wallpaper/background process
```

---

# 44. Public references

- IceWM upstream: https://github.com/ice-wm/icewm
- X.Org libXrandr: https://www.x.org/libraries/libxrandr/
- XRandR documentation: https://www.x.org/Projects/XRandR/
- Freedesktop Trash Specification: https://specifications.freedesktop.org/trash/latest/
- KDE Breeze Icons: https://github.com/KDE/breeze-icons

