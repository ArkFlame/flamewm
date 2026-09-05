# FlameWM / IceWM 4.1.0 — Source-Grounded Feature Implementation Report

**Purpose:** define exactly what must be implemented in the uploaded IceWM source to produce the lightweight Plasma/XFCE-like desktop requested, while reusing existing IceWM behavior wherever the source already provides it.

**Source audited:** uploaded `icewm-master(1).zip`  
**Verified source version:** `icewm-master/VERSION` → `VERSION=4.1.0`  
**Upstream release verification:** IceWM 4.1.0, released 2026-08-06, is the current upstream release at the time of this report.

---

## Assumptions

1. The target is Linux/X11 for v1.
2. The memory target of approximately **40 MiB or less** refers to the resident graphical components we own, measured primarily by **PSS**, not the whole operating system including Xorg, PipeWire, NetworkManager, D-Bus, drivers, etc.
3. The product is intentionally opinionated: one fixed Breeze-like desktop workflow, not another infinitely configurable desktop-environment framework.
4. Existing IceWM behavior should be reused unless it materially conflicts with the requested UX.
5. Qt, GTK, Chromium/WebView, JavaScript runtimes, Electron, QML and a new compositor are not acceptable runtime dependencies for the main desktop shell.
6. The uploaded IceWM 4.1.0 source is the implementation baseline. Current upstream/manual documentation is supporting evidence, but source behavior wins where there is a conflict.
7. "Breeze style" means reproducing the visual/interaction language with licensed assets and IceWM-native rendering. It does **not** mean linking Plasma, KDE Frameworks, Qt or KWin into the runtime.
8. Passive workspace switching merely by touching the screen edge is not desired. Workspace switching at screen edges should occur specifically while a window is being dragged.

---

# 1. Final architecture decision

## Keep IceWM 4.1.0 as the native base

Do **not** port IceWM to Rust, Zig, JavaScript or another language.

Do **not** replace the existing WM with Openbox.

Do **not** create an XFCE fork.

Do **not** create a separate Qt/GTK panel.

Do **not** build an HTML/CSS/JS production shell.

The uploaded source already contains the expensive, mature parts of a desktop shell:

- X11 window management.
- Window move/resize/minimize/maximize.
- Work areas and monitor-aware geometry.
- Multiple workspaces.
- Runtime workspace-count changes.
- Window migration when workspace count shrinks.
- Window-to-workspace movement.
- Edge workspace switching infrastructure.
- Half/quarter tiling geometry.
- Taskbar.
- Start/menu button.
- Toolbar launchers.
- Running task buttons.
- Workspace pager.
- System tray.
- Clock.
- Alt+Tab QuickSwitch, including preview mode.
- XDG/Freedesktop application menu generation.
- System icon-theme lookup.
- Themeable taskbar/window decorations.
- Wallpaper utility.

The correct project is therefore:

> **A constrained IceWM fork with a new filesystem desktop layer, modern snap UX, a Plasma-like unified task area, and native event-driven audio/media/network applets.**

This preserves the low-memory native stack while minimizing the amount of mature WM code we have to invent.

---

# 2. Important corrections to the previous architectural assessment

The previous answer reached the correct high-level choice—IceWM—but overstated several missing features.

## 2.1 Half/quarter window tiling already exists

This is the largest correction.

The source already defines:

- `KeyWinTileLeft`
- `KeyWinTileRight`
- `KeyWinTileTop`
- `KeyWinTileBottom`
- `KeyWinTileTopLeft`
- `KeyWinTileTopRight`
- `KeyWinTileBottomLeft`
- `KeyWinTileBottomRight`
- `KeyWinTileCenter`

Evidence:

- `src/default.h:504-512`
- `src/wmaction.h:116-124`
- `src/wmframe.cc:1500+` → `YFrameWindow::wmTile(...)`
- `src/movesize.cc:757+` handles tiling key actions during move/size interaction.

`YFrameWindow::wmTile()` already computes geometry against `manager->getWorkArea(...)`, so it respects the usable work area rather than blindly using full screen coordinates.

### Consequence

We should **not** write a second tiling geometry engine.

We should add:

1. mouse-edge/corner detection,
2. live target preview,
3. state needed to restore the pre-snap floating geometry,
4. top-edge maximize behavior,
5. conflict resolution between left/right snapping and drag-to-workspace switching.

The existing `wmTile()` must remain the geometry authority for half/quarter placement unless a verified bug requires a local repair.

---

## 2.2 Runtime workspace creation/shrinking already exists

IceWM is not limited to startup-only `WorkspaceNames`.

The source handles `_NET_NUMBER_OF_DESKTOPS` client messages:

- `src/wmmgr.cc:778+`

It can grow the workspace set:

- `YWindowManager::extendWorkspaces(...)`
- `src/wmmgr.cc:2855+`

It can shrink it:

- `YWindowManager::lessenWorkspaces(...)`
- `src/wmmgr.cc:2871+`

It updates:

- desktop count,
- viewport,
- work area,
- taskbar workspace buttons,
- window-list workspace state,
- move menus.

Evidence:

- `src/wmmgr.cc:2898+` → `updateWorkspaces(...)`
- `src/aworkspaces.cc:405+` → dynamic pager button reconciliation.
- `src/icesh.cc:2990+` → `addWorkspace`.
- `src/icesh.cc:3110+` → workspace count control.

This behavior is also aligned with EWMH `_NET_NUMBER_OF_DESKTOPS`.

### Consequence

The requested add/remove UX is **not** a new workspace system.

What remains is:

- panel right-click UX,
- insertion after a selected workspace instead of append-only,
- arbitrary removal of the workspace that was clicked instead of only shrinking from the end,
- deterministic renumbering/migration when a middle workspace is removed,
- minimum workspace count of 1.

---

## 2.3 Dragging a window through an edge to another workspace is already mostly implemented

During opaque window movement:

- `src/movesize.cc:385` calls `checkEdgeSwitch(mouseX, mouseY)`.
- `src/movesize.cc:388+` arms the edge-switch timer.
- `src/wmframe.cc:1225+` switches workspace when the timer expires and keeps/takes the moving window across the transition.
- `src/wmmgr.cc:3415+` → `switchToWorkspace(int nw, bool takeCurrent)` already contains logic for carrying a window to the destination workspace.

### Consequence

Do not build drag-across-workspaces from zero.

We need to **split edge detection from passive pointer edge switching** so that:

- passive mouse-at-edge switching is disabled,
- the same edge destination logic remains available while actively dragging a window,
- snap target preview and workspace-dwell switching can coexist.

---

## 2.4 Plasma-like Alt+Tab is much closer than previously stated

IceWM 4.1.0 QuickSwitch already supports:

- vertical mode,
- horizontal mode,
- preview mode,
- all icons,
- minimized/hidden windows,
- grouping by workspace,
- selected-window preview,
- configurable colors/fonts/margins.

Evidence:

- `src/default.h:298-312`
- `src/wmswitch.cc`
- `man/icewm.pod` QuickSwitch section.
- `QuickSwitchPreview=1` exists.

The 4.1.0 `NEWS` also contains substantial QuickSwitch improvements.

### Consequence

A KDE/Plasma-like window selector is primarily:

- fixed configuration,
- Breeze styling,
- perhaps small renderer/layout adjustments.

It is **not** a new task-switcher subsystem.

---

## 2.5 IceWM already has system icon-theme discovery

The source can scan system icon paths and icon themes:

- `src/default.h:441-442` → `IconPath`, `IconThemes`.
- `src/yicon.cc:357+` builds the icon-theme search index.
- `src/yicon.cc:500+` resolves PNG/XPM/SVG candidates.

### Consequence

Do not embed a second icon lookup stack.

Use the existing `YIcon` machinery.

For Breeze, prefer:

1. the installed Breeze icon theme when present,
2. a small audited fallback subset for shell-specific icons only.

The KDE Breeze icon repository is Freedesktop-compatible. Its repository includes `COPYING-ICONS` and licensing metadata, so any bundled subset must retain applicable licensing information rather than blindly copying assets.

---

## 2.6 `icewmbg` is already a separate wallpaper process

Evidence:

- `src/icewmbg.cc`
- separate `icewmbg` target in `src/CMakeLists.txt`.

The previous answer proposed immediately integrating wallpaper into the WM. That is not justified before measurement.

### Decision

For the first implementation:

- keep `icewmbg`,
- add desktop filesystem icons as root/desktop children,
- measure its PSS,
- fold wallpaper handling into the main process **only if profiling proves the separate process materially harms the memory target**.

This reduces fork divergence and implementation risk.

---

## 2.7 IceWM's existing network applet is not a Wi-Fi control applet

IceWM does have `NetStatusControl`, but source audit shows that it is a network-interface/throughput monitor:

- `src/apppstatus.cc`
- it reads interface state and `/proc/net/dev`,
- maintains samples,
- uses a timer,
- renders traffic status.

There are **zero** source references to:

- D-Bus,
- NetworkManager,
- `nmcli`,
- MPRIS,
- libpulse/PulseAudio client APIs.

### Consequence

The requested Wi-Fi indicator/control, media control and volume control are genuine new integrations.

They should not be confused with the existing traffic graph.

---

## 2.8 IceWM's desktop "MiniIcons" are not filesystem desktop icons

IceWM supports minimized-window desktop mini-icons:

- `MinimizeToDesktop`
- `src/wmminiicon.cc`

Those icons are draggable and provide a useful implementation pattern, but they represent minimized application windows—not files or folders from the XDG Desktop directory.

### Consequence

The filesystem desktop grid is still the largest truly new UI subsystem.

However, `MiniIcon` gives us reusable source patterns for:

- child windows on the desktop,
- click handling,
- drag handling,
- screen/workarea clamping,
- stacking relative to managed windows.

---

# 3. Existing source capability matrix

| Requested behavior | Verified source state | Required implementation |
|---|---|---|
| Native lightweight X11 WM | Existing | Keep |
| Move windows | Existing | Keep |
| Resize windows | Existing | Keep |
| Minimize | Existing | Keep |
| Maximize | Existing | Keep |
| Restore | Existing | Keep |
| Half left/right layouts | Existing geometry | Add mouse activation + preview + restore state |
| Quarter corner layouts | Existing geometry | Add mouse activation + preview + restore state |
| Top-edge maximize | Existing maximize action | Add drag activation + preview |
| Bottom-edge layout | Existing bottom-half action exists | No bottom-center gesture required unless later desired |
| Work-area aware geometry | Existing | Reuse |
| Multi-monitor geometry/XRandR | Existing | Reuse and test |
| Virtual desktops/workspaces | Existing | Keep |
| Runtime workspace count change | Existing | Expose through panel UX |
| Add workspace | Existing append behavior | Add "insert after clicked" semantics |
| Remove workspace | Existing shrink-from-end | Add arbitrary-index removal |
| Minimum one workspace | Easy invariant | Enforce |
| Move windows between workspaces | Existing | Reuse |
| Drag moving window through edge | Existing base behavior | Restrict to active window drag and reconcile with snap |
| Workspace pager | Existing | Restyle + context menu |
| Start/menu button | Existing | Restyle |
| Application menu generation | Existing | Reuse |
| Fixed/pinned launcher toolbar | Existing separate toolbar | Merge visually/behaviorally with task model |
| Running task buttons | Existing | Restyle/unify |
| Focused task state | Existing | Render Breeze indicator |
| Minimized task state | Existing | Render Breeze indicator |
| Running/inactive task state | Existing | Render Breeze indicator |
| Click focused task → minimize | Existing | Keep |
| System tray | Existing | Keep unless a specific incompatibility appears |
| Clock | Existing | Configure/restyle; small code change only if exact two-line layout required |
| Date | Existing as format/tooltip | Make visibly present in panel |
| Alt+Tab selector | Existing | Configure/restyle |
| Alt+Tab live previews | Existing | Enable/test |
| Wallpaper | Existing through `icewmbg` | Keep initially |
| Files/folders on desktop | Missing | Implement |
| Desktop icon grid | Missing | Implement |
| Drag/reorder desktop icons | Missing for files; mini-icon pattern exists | Implement |
| Persist icon positions | Missing | Implement |
| Volume indicator/slider | Missing | Implement |
| Media playback control | Missing | Implement |
| Wi-Fi/network state/control | Missing; old traffic monitor is unrelated | Implement |
| Breeze shell appearance | Theme system exists | Theme + targeted renderer changes |
| Blur/translucent Plasma compositor effects | Not a goal | Deliberately omit |
| HTML/CSS/JS production UI | Not present | Do not add |

---

# 4. Product boundary

The target should be treated as a **small desktop shell**, not a desktop framework.

## Required product

1. excellent stacking WM,
2. excellent snap behavior,
3. filesystem desktop,
4. one bottom panel,
5. start menu,
6. virtual desktop pager,
7. application/task area,
8. volume,
9. media,
10. network,
11. clock/date,
12. Breeze-like visual design,
13. Plasma-like Alt+Tab/window selector.

## Explicitly not required

Do not introduce these unless a future request explicitly adds them:

- widget SDK,
- JavaScript extension engine,
- desktop effects framework,
- activities framework,
- online accounts,
- semantic search/indexer,
- global KDE-style settings daemon,
- full notification center/history framework,
- KRunner clone,
- Plasma widgets,
- blur,
- spring animations,
- compositing engine,
- custom file manager,
- custom network daemon,
- custom media daemon,
- custom audio server,
- Wayland compositor,
- plugin marketplace.

This narrow boundary is a core part of the memory objective.

---

# 5. Window-management implementation

## 5.1 Reuse `YFrameWindow::wmTile()`

Source:

- `src/wmframe.cc:1500+`

It already:

1. verifies movability/visibility,
2. clears incompatible maximized/rollup state,
3. obtains the work area for the current screen,
4. computes left/right/top/bottom/quadrant geometry,
5. commits geometry.

Do not duplicate this math.

---

## 5.2 Add drag snap target detection

New conceptual type:

```text
SnapTarget
    None
    LeftHalf
    RightHalf
    TopLeftQuarter
    TopRightQuarter
    BottomLeftQuarter
    BottomRightQuarter
    Maximize
```

The target is derived from:

- pointer root position,
- current output/monitor,
- output work area,
- configured edge-zone size.

Recommended activation:

```text
top-left corner       -> TopLeftQuarter
top-right corner      -> TopRightQuarter
bottom-left corner    -> BottomLeftQuarter
bottom-right corner   -> BottomRightQuarter
left edge             -> LeftHalf
right edge            -> RightHalf
top edge              -> Maximize
bottom center         -> None
```

The existing `TileTop`/`TileBottom` actions remain available by keyboard but do not need mouse gestures for the requested UX.

---

## 5.3 Add a snap preview overlay

A preview should appear while dragging before release.

Do **not** require a compositor for this.

For v1 use an IceWM-native override-redirect overlay with:

- Breeze accent border,
- lightly patterned/stippled fill or opaque low-intensity fill,
- no blur,
- no animation.

True alpha translucency on X11 generally depends on compositing. A compositor would undermine the low-memory design. The preview needs to be clear, not visually expensive.

New files should be small and isolated, for example:

```text
src/snapoverlay.h
src/snapoverlay.cc
```

Do not repurpose QuickSwitch window previews for snap overlays; they have different lifecycle/state semantics.

---

## 5.4 Preserve floating geometry

Existing `wmTile()` places the window but does not itself give us the exact KWin/Windows gesture of:

> drag a snapped window away → recover its previous floating size under the pointer.

Add explicit transient state to `YFrameWindow`:

```text
SnapState {
    SnapTarget target;
    YRect floatingOuterGeometry;
    bool hasFloatingGeometry;
}
```

Rules:

1. First transition from floating → snap stores the floating geometry.
2. Moving among snap targets does not overwrite the original floating geometry.
3. Dragging the titlebar away from a snapped position restores the saved floating size before continuing interactive movement.
4. Manual resize after snapping exits snap state and makes the resulting geometry the new normal geometry.
5. Maximize remains IceWM maximize state, but drag-away should follow the same restore UX.
6. Closing/destroying a frame clears state.
7. Monitor removal must not restore geometry onto a nonexistent output; clamp to a surviving work area.

---

## 5.5 Resolve snap vs workspace-edge switching

There is a genuine interaction conflict on left/right edges.

Use this deterministic rule:

### Corners

Corners are reserved for quarter snapping.

They never trigger workspace switching.

### Left/right center edge

While dragging a window:

1. entering the edge immediately shows the half-snap preview,
2. releasing quickly commits the half snap,
3. holding the pointer at the extreme edge for approximately `450 ms` cancels the snap preview and changes to previous/next workspace,
4. the window remains attached to the drag and follows to the destination workspace,
5. after the workspace changes, moving away from the edge resets the dwell timer.

This reproduces both requested behaviors without a modifier key.

The dwell time should be a named preference or fixed product constant, not scattered magic values.

---

## 5.6 Disable passive edge switching but keep drag edge switching

Current constructor logic in `src/wmmgr.cc` creates `EdgeSwitch` input-only edge windows only when `HorizontalEdgeSwitch`/`VerticalEdgeSwitch` is enabled. Those edge windows also react to ordinary pointer crossing.

The new architecture should decouple:

```text
edge geometry/destination calculation
```

from:

```text
passive pointer crossing behavior
```

Required product default:

```text
passive edge switch = off
dragging-window edge switch = on
```

Do not make users enable `EdgeSwitch=1` just to get window-drag workspace traversal.

---

# 6. Workspace implementation

## 6.1 Existing dynamic capability

`_NET_NUMBER_OF_DESKTOPS` handling is already standards-based.

The EWMH specification states that a pager may request a new number of desktops and requires workarea/viewport/current-desktop consistency when accepted.

IceWM already implements this.

Keep that external compatibility.

---

## 6.2 Add workspace insertion

Current `Workspaces` provides append and last-drop semantics:

- `src/workspaces.h`

Add explicit indexed operations:

```text
bool insert(int index, const char* name)
bool removeAt(int index)
```

Panel command:

```text
right-click desktop N
-> Add Desktop
```

Recommended result:

```text
insert new desktop directly after N
```

Then:

1. shift workspace metadata at indices `> N`,
2. shift every frame workspace index `> N` by +1,
3. preserve the active logical workspace,
4. update desktop names,
5. update `_NET_NUMBER_OF_DESKTOPS`,
6. update viewport/workarea,
7. rebuild pager labels/buttons,
8. update window-list and move menus.

---

## 6.3 Add arbitrary workspace removal

Panel command:

```text
right-click desktop N
-> Remove Desktop
```

Invariant:

```text
workspaceCount >= 1
```

If there is only one workspace, the remove item is disabled.

For removal of index `N`:

### Window migration rule

If `N > 0`:

```text
windows on N -> N - 1
```

If `N == 0`:

```text
windows on old 0 -> old 1
```

Then remove the workspace and shift all higher workspace indices down by one.

After renumbering, the destination for removed old workspace 0 naturally becomes new workspace 0.

This avoids losing windows and keeps migration deterministic.

### Other frames

```text
old workspace > N -> old workspace - 1
```

### Active workspace

If active workspace is removed:

- activate the migration target.

If active workspace is above the removed index:

- decrement active index.

### EWMH

Publish a normal reduced `_NET_NUMBER_OF_DESKTOPS` and corrected per-window `_NET_WM_DESKTOP` values after internal migration.

---

## 6.4 Workspace pager UX

Current source:

- `src/aworkspaces.cc`

Already supports:

- current-state highlighting,
- workspace clicking,
- rename by double-click,
- mouse wheel switching,
- preview pager mode,
- dynamic button updates.

Change right click from the generic window-list menu to a workspace-specific popup:

```text
Desktop 4
────────────
Add Desktop
Remove Desktop
```

Optional rename can remain available by double-click; it does not conflict with the requested fixed/simple UI.

---

# 7. Desktop filesystem grid

This is the largest new subsystem.

## 7.1 Do not use a resident file manager for the desktop

Do not start:

- PCManFM desktop,
- Nemo desktop,
- Dolphin/Plasma shell,
- a Qt desktop process,
- a GTK desktop process.

That would undermine the memory target and split ownership across processes.

---

## 7.2 Reuse IceWM's native window primitives

Create desktop items using existing low-level infrastructure:

- `YWindow`
- `Graphics`
- `YIcon`
- existing drag/click patterns from `MiniIcon`.

Useful reference:

- `src/wmminiicon.cc:143-223`

Do not subclass `MiniIcon` directly because its domain is managed minimized windows. Reuse the patterns, not the semantic type.

---

## 7.3 Resolve the Desktop directory correctly

Do not hardcode `~/Desktop`.

Resolve the XDG user Desktop directory from the standard user-dirs configuration, with a safe fallback to `$HOME/Desktop`.

Conceptual model:

```text
DesktopModel
    desktopDirectory
    items[]
    filesystemWatcher
```

Each item:

```text
DesktopItemModel
    absolutePath
    displayName
    kind
    iconName/iconPath
    gridSlot
```

Kinds at minimum:

```text
Directory
RegularFile
DesktopLauncher
```

---

## 7.4 Watch filesystem changes with inotify

Linux v1 should use inotify.

React to:

- create,
- delete,
- rename/move,
- attribute changes that affect icon/launchability.

No periodic directory polling.

On a burst of filesystem events, coalesce them into one model refresh.

---

## 7.5 Grid behavior

Each monitor has a usable desktop rectangle:

```text
output rectangle
minus panel-reserved work area
```

Recommended fixed metrics:

```text
cell width
cell height
icon size
label width
label max lines
grid gap
```

They belong in one theme/metrics structure.

### Placement

New unpositioned item:

- first free slot in deterministic scan order.

### Drag

Dragging an item:

- tracks pointer,
- highlights destination grid cell,
- on release snaps to the target cell.

If the target cell is occupied:

- swap positions.

This is predictable and avoids complex push-chain behavior.

### Reorder persistence

Store only shell layout metadata, not a shadow copy of filesystem content.

Example:

```ini
[DesktopLayout]
version=1

[item "/home/user/Desktop/Projects"]
output=HDMI-1
column=2
row=1

[item "/home/user/Desktop/notes.txt"]
output=HDMI-1
column=3
row=1
```

Preferred location:

```text
$XDG_CONFIG_HOME/flamewm/desktop-layout
```

or the fork's equivalent private config directory.

Use atomic replace-on-save:

```text
write temp
fsync/close as appropriate
rename temp -> final
```

---

## 7.6 Monitor changes

When XRandR reports an output change:

1. recompute work areas,
2. retain item slots on outputs that still exist,
3. items assigned to a removed output move to the first free cells of the primary/surviving output,
4. do not silently lose layout records,
5. avoid placing desktop icons under the panel.

The source already has XRandR handling in the WM; reuse that notification path.

---

## 7.7 Opening items

Directory and regular file:

- use the system's default application via a transient `xdg-open`-style launch path.

`.desktop` file:

- reuse as much of the existing Freedesktop desktop-entry parsing/launch logic as possible from `icewm-menu-fdo`,
- do not build a second incompatible parser if the current internal code can be extracted cleanly,
- preserve field-code handling and quoting rules.

Avoid keeping a file-manager toolkit resident merely to open files.

---

## 7.8 Desktop scope for v1

Required:

- wallpaper visible,
- files/folders visible,
- icon+label,
- click/select,
- double-click/open,
- drag/reorder,
- persistent positions,
- filesystem refresh.

Not required for initial completion unless explicitly added later:

- desktop widgets,
- thumbnail generation,
- rich file previews,
- badges,
- cloud sync state,
- giant context-menu framework,
- inline file metadata panels.

---

# 8. Bottom panel

The taskbar should remain inside the IceWM process.

Source owner:

- `src/wmtaskbar.cc`
- `src/wmtaskbar.h`

Existing integrated objects include:

- start/applications button,
- window list button,
- show desktop button,
- toolbar,
- workspace pane,
- task pane,
- system tray,
- clock,
- CPU/memory/network monitors.

The product should disable/remove visual exposure of legacy monitors/features that are not part of the target experience.

---

## 8.1 Fixed panel order

Target order:

```text
Start
Activities/Show Desktop if retained by final product spec
Pinned + running applications
flex spacer
Workspace selector
Media
Volume
Network
System tray where needed
Clock + date
```

The original requirement explicitly needs:

```text
Start
active/running apps
workspace indicator
volume
media
wifi
time/date
```

Do not add extra monitors just because IceWM ships them.

---

# 9. Unified pinned/running application area

IceWM currently has two concepts:

1. `ObjectBar` toolbar launchers,
2. `TaskPane` running windows.

Plasma-style behavior needs a visually unified model.

## 9.1 Desired state model

For each application identity:

```text
Pinned, not running
    icon
    no activity indicator

Running, not focused
    icon
    subtle bottom indicator

Focused
    icon
    strong/accent bottom indicator

Minimized
    icon with reduced emphasis
    activity indicator still present
```

If multiple windows belong to one application:

- one icon,
- optional dots/count using the existing grouping model,
- click can activate the most recent or open the existing group popup.

---

## 9.2 Existing state reuse

`TaskButton::paint(...)` already distinguishes:

- active/focused,
- minimized,
- invisible,
- normal.

Evidence:

- `src/atasks.cc:395+`

`TaskButton::handleButton(...)` already supports the desired click behavior where clicking an already focused/visible task can minimize it.

Evidence:

- `src/atasks.cc:742+`.

Preserve these semantics.

Replace the old button background treatment with Breeze-style icon indicators.

---

## 9.3 Application identity

A unified launcher/task area needs a stable mapping from launcher to windows.

Recommended authority:

1. `.desktop` `StartupWMClass` when supplied,
2. current X11 `WM_CLASS`,
3. normalized desktop entry/application id fallback.

Do not identify applications purely by window title.

Pinned launchers should store a desktop-entry id, not a raw shell command where avoidable.

---

# 10. Start menu

IceWM already has:

- root/start menu,
- nested menus,
- `icewm-menu-fdo`,
- Freedesktop `.desktop` discovery,
- automatic menu reload support.

Evidence:

- `src/fdomenu.cc`
- `src/CMakeLists.txt:509+` builds `icewm-menu-fdo` when `CONFIG_FDO_MENUS` is enabled.
- `CONFIG_FDO_MENUS` defaults to `on`.

## Required work

Primarily visual and structural:

- Breeze-like dark popup.
- Consistent icon size.
- Consistent row height.
- Fixed useful width.
- Application categories.
- power/session section.
- no old/irrelevant IceWM configuration clutter in the default menu.

A search field is **not required by the original feature request**. Do not add one to the critical path unless the product prototype explicitly standardizes it later.

---

# 11. Alt+Tab / window selector

Use IceWM QuickSwitch.

Recommended fixed product configuration:

```text
QuickSwitch=1
QuickSwitchPreview=1
QuickSwitchToMinimized=1
QuickSwitchToHidden=1
QuickSwitchToAllWorkspaces=0
QuickSwitchFillSelection=1
```

Then style:

- dark Breeze popup,
- accent selection,
- modern margins,
- application icon/title,
- horizontal preview grid if it matches the target screenshots best.

Only change `src/wmswitch.cc` if configuration/theme knobs cannot reach the intended layout.

Do not replace QuickSwitch with a new window enumeration/thumbnail implementation.

---

# 12. Clock and date

IceWM already uses `strftime`-style formatting:

- `TimeFormat`
- `TimeFormatAlt`
- `DateFormat`

Evidence:

- `src/default.h:466-468`
- `src/aclock.cc`.

If a one-line compact representation satisfies the final UI, this can be configuration-only.

If exact Plasma-like stacked text is required:

```text
16:23
02/09/2026
```

make a small `YClock` rendering adjustment for a fixed two-line shell mode.

Do not introduce a separate clock process.

---

# 13. Volume applet

This is new.

## 13.1 Backend decision

Use **libpulse's asynchronous client API**.

Reason:

PipeWire's PulseAudio protocol module is explicitly designed so existing PulseAudio client libraries continue to work against PipeWire-Pulse. Therefore a libpulse client gives compatibility with the normal Ubuntu/PipeWire desktop stack without linking the much larger desktop frameworks.

Use:

- default sink query,
- volume query/set,
- mute query/set,
- subscription events for sink/server changes.

Do not poll `pactl`.

Do not spawn `pactl` every second.

---

## 13.2 Event loop integration

For minimum idle cost, do not create a GLib main loop.

Preferred architecture:

```text
PulseMainloopAdapter
    translates pa_mainloop_api IO events -> IceWM poll/watch infrastructure
    translates timer events -> IceWM timers
    translates deferred events -> queued callbacks
```

This keeps:

- one UI process,
- no GLib dependency,
- no periodic volume polling,
- no dedicated helper daemon.

If this adapter proves disproportionately complex, a libpulse threaded main loop is an acceptable fallback only after measuring its real PSS/thread cost.

---

## 13.3 UI

Panel icon states:

```text
muted
low
medium
high
```

Click popup:

```text
Output          64%
[----------●------]

Mute
```

Required interactions:

- click icon -> popup,
- wheel over icon -> volume step,
- mute toggle,
- slider,
- external volume changes update immediately.

No full audio settings framework.

---

# 14. Media applet

This is new.

## 14.1 Backend

Use MPRIS directly on the session D-Bus.

MPRIS provides:

- player discovery through `org.mpris.MediaPlayer2.*`,
- playback status,
- metadata,
- previous,
- next,
- play/pause.

Do not create a media daemon.

---

## 14.2 Shared D-Bus dispatcher

The source currently has zero D-Bus integration.

Add one small IceWM-native D-Bus event adapter.

Recommended files:

```text
src/dbusdispatcher.h
src/dbusdispatcher.cc
```

Integrate D-Bus watches/timeouts into the same IceWM poll/timer infrastructure.

Use it for:

- session bus → MPRIS,
- system bus → NetworkManager.

This avoids:

- GLib,
- QtDBus,
- duplicated event-loop adapters.

---

## 14.3 Active-player selection

Deterministic rule:

1. currently `Playing` player wins,
2. otherwise most recently active/changed player,
3. otherwise first available player,
4. no player → hide or render disabled media icon according to final panel spacing choice.

Popup:

```text
Track title
Artist

Previous   Play/Pause   Next
```

No artwork download/cache is required for v1.

---

# 15. Wi-Fi/network applet

This is new and distinct from IceWM's traffic monitor.

## 15.1 Backend

Use NetworkManager's system D-Bus API.

Required state:

- no network,
- wired,
- Wi-Fi connecting,
- Wi-Fi connected,
- signal strength,
- active SSID when applicable.

NetworkManager exposes device, wireless, active connection and access-point objects over D-Bus.

---

## 15.2 Scope

Original requirement is an indicator. Keep connection management intentionally small.

Required popup:

- current connection,
- visible Wi-Fi networks,
- signal strength,
- connect known connection,
- disconnect.

For a secured SSID that does not already have a configured connection profile, do not immediately build a full SecretAgent/settings framework unless necessary. A transient handoff to the system's connection setup mechanism is acceptable because it has no idle memory cost.

The shell must not become a complete NetworkManager settings clone.

---

## 15.3 Remove/disable the legacy traffic monitor from the default shell

`NetStatusControl` may stay in source for upstream compatibility, but the product default should not show it.

Do not mutate the throughput monitor into the Wi-Fi applet. Create a separate responsibility, for example:

```text
src/anetwork.h
src/anetwork.cc
```

---

# 16. Breeze visual system

## 16.1 Use IceWM theme machinery first

IceWM themes already support:

- fonts,
- colors,
- borders,
- titlebar dimensions,
- frame pixmaps,
- taskbar background,
- normal/active/minimized task button visuals,
- workspace button visuals,
- start button,
- gradients.

Upstream theme documentation explicitly documents taskbar pixmaps and active/minimized states.

Therefore Phase 1 should get most of the visual result using a theme before changing renderers.

---

## 16.2 Product visual constants

Recommended baseline:

```text
Panel height        ~36-40 px
Titlebar height     ~28-30 px
Window borders      thin but resizeable
Font                Noto Sans / system sans
Panel background    opaque dark charcoal
Menu background     opaque dark charcoal
Accent              Breeze-like blue
Animations          none by default
Blur                none
Transparency        none by default
```

The exact values should be based on the approved HTML prototype/screenshots, but they must live in one coherent theme/metrics layer.

---

## 16.3 Icons

Prefer the installed `Breeze` icon theme through existing `IconThemes` lookup.

Bundle only shell-specific fallback icons that are actually required.

If Breeze assets are bundled:

- audit each included asset's license,
- retain attribution/license files,
- do not assume the entire repository uses one identical license because `COPYING-ICONS` is separate from the library license.

---

# 17. Wallpaper

Keep `icewmbg` initially.

Reasons:

- it already handles root background changes,
- current QuickSwitch preview documentation explicitly works with root background managers that update `_XROOTPMAP_ID`,
- it is isolated mature code,
- integrating it before profiling creates unnecessary fork divergence.

After the full shell is functional, measure:

```text
icewm PSS
icewmbg PSS
combined process-group PSS
```

Only integrate background logic if the measured benefit justifies the maintenance cost.

---

# 18. Build/dependency plan

## Existing build

Uploaded source:

- root `CMakeLists.txt`,
- `src/CMakeLists.txt`,
- IceWM executable uses native C++,
- current CMake attempts C++20 when the CMake version supports it,
- existing core uses X11/Xrender/Xcomposite/Xcursor/Xdamage/Xfixes/Xext and optional XRandR/Xinerama/Xft/image loaders.

Use:

```text
CMake + Ninja
GCC or Clang
```

Do not create another language/toolchain layer.

---

## New dependencies

For the requested Linux shell:

```text
libdbus-1
libpulse
```

Do **not** add:

```text
Qt
GTK
GLib main loop
Electron
WebKit
Chromium
QML
```

A direct D-Bus dependency is much smaller than pulling a desktop toolkit only for IPC.

---

# 19. Development loop

Create a repository command:

```bash
./dev
```

Responsibilities:

1. incremental CMake/Ninja build,
2. create/use isolated `ICEWM_PRIVCFG`,
3. start or reuse Xephyr,
4. start the fork in that nested display,
5. start several deterministic test windows,
6. optionally launch MPRIS/audio/network test fixtures,
7. make theme/config reload easy.

IceWM already documents:

- `ICEWM_PRIVCFG` for isolated user configuration,
- SIGHUP/restart for configuration reload.

This gives the desired:

```text
edit -> build -> restart nested shell -> inspect
```

without logging out of the real desktop.

---

# 20. Testing strategy

The current CMake `BUILD_TESTING` block only wires a small set of low-level tests (`strtest`, `testpointer`, `testarray`). The source contains additional X11 hint test programs in the autotools setup, but there is no complete automated nested-WM integration suite for the requested behaviors.

We need one.

## 20.1 Unit tests

### Snap target geometry

One assertion per test where practical:

```text
left edge -> LeftHalf
right edge -> RightHalf
top-left -> TopLeftQuarter
...
top center -> Maximize
bottom center -> None
```

Test:

- monitor offset,
- nonzero panel strut,
- odd screen dimensions,
- minimum-size constrained windows.

### Workspace index transformation

Test:

- insert at first,
- insert middle,
- insert last,
- remove first,
- remove middle,
- remove last,
- cannot remove only workspace,
- active workspace migration,
- every window receives expected new index.

### Desktop grid

Test:

- first-free placement,
- occupied-cell swap,
- monitor removal relocation,
- persisted positions round-trip,
- rename event keeps one item without duplication.

### App identity

Test `.desktop`/WM_CLASS matching without live X server where possible.

---

## 20.2 Xephyr integration tests

Start the real fork under a nested X server and use actual X/EWMH state.

Verify:

1. move/resize,
2. snap left/right/corners,
3. snap restore,
4. maximize from top edge,
5. workspace edge dwell while dragging,
6. no workspace switch from passive pointer edge,
7. add workspace,
8. remove clicked middle workspace,
9. no removal below one workspace,
10. window migration after workspace deletion,
11. task focus indicator,
12. click focused task -> minimized state,
13. QuickSwitch enumerates expected windows,
14. desktop file create/delete/rename appears correctly,
15. desktop drag persists after shell restart.

Do not assert only screenshots. Assert X11 geometry/properties/state.

---

## 20.3 IPC integration tests

### MPRIS

Use a small fake D-Bus service implementing the required MPRIS interface.

Verify:

- discovery,
- play/pause call,
- metadata update,
- player disappearance.

### NetworkManager

Prefer an interface abstraction and mock D-Bus message boundary for deterministic tests. Real NetworkManager integration can be a separate manual/system test.

### PulseAudio

Abstract the minimal sink backend from the widget and test UI/model transitions. A real PipeWire-Pulse test can run only where a server is available.

---

# 21. Memory/performance gates

The 40 MiB goal must be a measured release gate, not marketing text.

## 21.1 Measurement

Use:

```text
/proc/<pid>/smaps_rollup
```

Record:

- PSS,
- RSS,
- Private_Clean,
- Private_Dirty,
- Swap,
- thread count,
- fd count.

Measure each owned resident process and combined process group.

---

## 21.2 Idle benchmark

After at least 60 seconds settled:

```text
1-2 monitors
wallpaper
desktop grid
20 desktop items
panel
multiple workspaces
network connected
audio backend connected
no menu open
no app windows
```

Target:

```text
combined owned PSS <= 40 MiB
idle CPU effectively 0
no periodic high-frequency wakeups
no monotonic memory growth
```

---

## 21.3 Loaded benchmark

```text
browser
terminal
editor
multiple workspaces
several minimized windows
MPRIS player active
volume changed repeatedly
workspace/menu cycles
desktop icon drag cycles
```

Verify:

- no leak,
- no persistent preview buffers after QuickSwitch closes,
- no unbounded icon cache growth,
- no polling loops for D-Bus/audio state.

---

## 21.4 Do not optimize dormant source before profiling

Legacy IceWM code that is not instantiated costs little/no resident heap.

First disable unused applets in product defaults.

Only delete old code when it:

- reduces actual binary/runtime cost,
- removes meaningful maintenance burden,
- or conflicts with the fixed product UX.

Prematurely deleting stable upstream code increases rebasing cost without guaranteed memory savings.

---

# 22. Exact source ownership map

## Existing files to modify

### `src/movesize.cc`

Owns interactive move flow.

Changes:

- calculate drag snap target,
- update/hide snap overlay,
- manage edge dwell state,
- trigger snap on release,
- restore floating geometry on drag-away,
- route drag-only workspace edge behavior.

### `src/wmframe.h`
### `src/wmframe.cc`

Changes:

- snap state,
- saved floating geometry,
- helper to enter/leave snapped state,
- continue using `wmTile()` as placement authority.

### `src/wmmgr.h`
### `src/wmmgr.cc`

Changes:

- drag-only edge destination helper independent of passive edge windows,
- indexed workspace insertion/removal,
- frame workspace renumbering,
- workarea/desktop property refresh after arbitrary mutation.

### `src/workspaces.h`

Changes:

- indexed insert/remove primitives with clear ownership semantics.

### `src/aworkspaces.cc`
### `src/aworkspaces.h`

Changes:

- workspace-specific context menu,
- Add Desktop,
- Remove Desktop,
- disable removal at count 1,
- refresh labels/buttons after indexed mutations.

### `src/wmtaskbar.cc`
### `src/wmtaskbar.h`

Changes:

- fixed panel order,
- new media/audio/network applet ownership,
- remove old monitors from default layout,
- fixed Breeze sizing.

### `src/atasks.cc`
### `src/atasks.h`

Changes:

- icon-only Breeze task rendering,
- focused/running/minimized indicator treatment,
- unified app grouping hooks.

### `src/objbar.cc`
### `src/objbar.h`

Changes:

- pinned-launcher side of unified task area or shared launcher model.

### `src/wmswitch.cc`

Only if theme/config is insufficient for exact approved switcher layout.

### `src/aclock.cc`

Only if exact two-line visible date/time requires renderer support.

### `src/yicon.cc`

Prefer no semantic changes. Reuse existing icon search. Only repair if a specific Breeze path/scale behavior is source-proven wrong.

### `src/CMakeLists.txt`

Add:

- D-Bus dependency,
- libpulse dependency,
- new source files,
- new tests.

---

## New files recommended

Names can be adjusted to existing IceWM naming conventions after implementation begins.

```text
src/snapoverlay.h
src/snapoverlay.cc

src/desktopmodel.h
src/desktopmodel.cc
src/desktopview.h
src/desktopview.cc
src/desktopitem.h
src/desktopitem.cc
src/desktoplayout.h
src/desktoplayout.cc

src/dbusdispatcher.h
src/dbusdispatcher.cc
src/amedia.h
src/amedia.cc
src/anetwork.h
src/anetwork.cc
src/aaudio.h
src/aaudio.cc
```

Avoid creating dozens of tiny abstractions. Each file should correspond to a real state owner.

---

# 23. Implementation phases

## Phase 0 — Baseline and dev laboratory

Deliver:

- untouched IceWM 4.1.0 build on target machine,
- `./dev`,
- Xephyr nested session,
- isolated config,
- baseline PSS/RSS/thread/fd data,
- screenshot/prototype theme reference directory,
- baseline behavior tests.

No feature code before this baseline exists.

---

## Phase 1 — Breeze shell without architectural changes

Deliver:

- Breeze-like window decoration,
- bottom panel height/colors,
- start button,
- workspace styling,
- task styling,
- system icon theme,
- QuickSwitch styling,
- clock/date appearance.

Goal: visually recognizable target while behavior remains mostly stock IceWM.

---

## Phase 2 — KWin/Windows-style mouse snap UX

Deliver:

- left/right half,
- four corners,
- top maximize,
- preview overlay,
- drag-away restore,
- monitor-aware target geometry,
- snap/workspace-edge conflict handling.

Reuse existing `wmTile()`.

---

## Phase 3 — Workspace UX

Deliver:

- add via workspace context menu,
- remove clicked workspace,
- minimum one,
- arbitrary index insertion/removal,
- deterministic window migration,
- window-drag edge traversal only,
- no passive pointer edge switching.

---

## Phase 4 — Filesystem desktop

Deliver:

- XDG Desktop discovery,
- wallpaper coexistence,
- native file/folder icons,
- grid,
- drag/reorder,
- persistence,
- inotify refresh,
- multi-monitor relocation.

---

## Phase 5 — Unified Plasma-like task area

Deliver:

- pinned launchers,
- grouped running apps,
- focused/running/minimized indicators,
- click behavior,
- desktop-entry/WM_CLASS identity.

---

## Phase 6 — Native panel integrations

Deliver:

- D-Bus dispatcher,
- MPRIS media applet,
- NetworkManager indicator/menu,
- libpulse volume applet,
- external-state event subscriptions,
- no periodic shell polling.

---

## Phase 7 — Memory surgery and release gate

Only after behavior is complete:

- measure every owned process,
- audit timers,
- audit wakeups,
- inspect caches,
- inspect preview buffers,
- trim unused default applets,
- evaluate `icewmbg` integration based on measured PSS,
- enable suitable release optimization/LTO only after correctness gates remain green.

---

# 24. Completion criteria

The fork is functionally complete for the requested v1 only when all of the following are true:

## Desktop

- wallpaper renders,
- Desktop directory files/folders appear,
- items align to a deterministic grid,
- items can be dragged/reorganized,
- positions survive WM restart,
- filesystem create/delete/rename is reflected without restart.

## Windows

- windows move and resize normally,
- left/right edges snap to halves,
- four corners snap to quarters,
- top edge maximizes,
- preview appears before commit,
- dragging away restores floating geometry,
- panel workarea is respected,
- multi-monitor geometry is correct.

## Workspaces

- at least one workspace always exists,
- workspace buttons switch desktops,
- Add Desktop works at runtime,
- Remove Desktop removes the clicked desktop,
- removed-workspace windows migrate deterministically,
- indices/names update correctly,
- dragging a window through left/right edge switches workspace,
- ordinary pointer edge contact does not switch workspace.

## Panel

- start menu,
- pinned/running app icons,
- focused state,
- inactive running state,
- minimized state,
- workspace selector,
- media control,
- volume,
- network/Wi-Fi,
- system tray if required,
- time and date.

## Window selector

- Alt+Tab works,
- Breeze-like presentation,
- minimized windows handled,
- preview mode works without leak.

## Resource gate

- no JavaScript/WebView/Qt/GTK shell process,
- no periodic polling for MPRIS/NetworkManager/volume state,
- no unbounded memory growth,
- combined owned resident PSS measured and reported,
- 40 MiB goal either passes or is accompanied by exact measured offenders—not guessed explanations.

---

# 25. Build audit note from this report environment

The uploaded archive was readable and extracted successfully. `VERSION=4.1.0` was verified directly.

A CMake configure attempt was also made to validate the build entry point. It reached dependency discovery but could not complete in this sandbox because development packages such as `xrender`/`fontconfig` were not installed here.

The failure was environmental dependency absence, not a source compile diagnostic.

Therefore:

- source inspection is valid,
- no claim is made that this sandbox produced a successful IceWM binary,
- the first implementation session on the actual target system must establish the clean build baseline before changes.

---

# 26. Source-backed technical references

## Uploaded source references

Primary audited paths:

```text
icewm-master/VERSION
icewm-master/CMakeLists.txt
icewm-master/src/CMakeLists.txt
icewm-master/src/default.h
icewm-master/src/wmframe.cc
icewm-master/src/wmframe.h
icewm-master/src/movesize.cc
icewm-master/src/wmmgr.cc
icewm-master/src/wmmgr.h
icewm-master/src/workspaces.h
icewm-master/src/aworkspaces.cc
icewm-master/src/atasks.cc
icewm-master/src/wmtaskbar.cc
icewm-master/src/wmswitch.cc
icewm-master/src/aclock.cc
icewm-master/src/apppstatus.cc
icewm-master/src/icewmbg.cc
icewm-master/src/wmminiicon.cc
icewm-master/src/yicon.cc
icewm-master/src/fdomenu.cc
icewm-master/src/icesh.cc
icewm-master/man/icewm.pod
icewm-master/man/icewm-preferences.pod
```

Important source anchors:

```text
VERSION:2                         IceWM 4.1.0
src/default.h:298-312            QuickSwitch capabilities
src/default.h:314-318            SnapMove / edge switching preferences
src/default.h:321-369            taskbar/workspace/task applet preferences
src/default.h:441-442            icon path/theme support
src/default.h:466-468            time/date formats
src/default.h:504-512            half/quarter tile actions

src/wmframe.cc:1500+             YFrameWindow::wmTile
src/movesize.cc:385+             moving-window edge switch check
src/wmframe.cc:1225+             edge timer moving window across workspace

src/wmmgr.cc:778+                _NET_NUMBER_OF_DESKTOPS message
src/wmmgr.cc:2855+               extendWorkspaces
src/wmmgr.cc:2871+               lessenWorkspaces
src/wmmgr.cc:2898+               updateWorkspaces
src/wmmgr.cc:3415+               switchToWorkspace(..., takeCurrent)
src/wmmgr.cc:3746+               edgeWorkspace / EdgeSwitch

src/aworkspaces.cc:72+           workspace button click behavior
src/aworkspaces.cc:405+          dynamic pager-button update

src/atasks.cc:395+               focused/minimized/invisible/normal task rendering
src/atasks.cc:742+               task click/minimize behavior

src/wmtaskbar.cc:250+            integrated taskbar applets
src/wmtaskbar.cc:333+            start/window-list/show-desktop controls
src/wmtaskbar.cc:371+            workspace pane
src/wmtaskbar.cc:381+            task pane
src/wmtaskbar.cc:418+            toolbar loading

src/wmminiicon.cc:143-223        desktop child drag/click implementation pattern
src/yicon.cc:357+                icon-theme path scan
src/yicon.cc:500+                icon lookup/loading

src/CMakeLists.txt:470-486       IceWM source target
src/CMakeLists.txt:494-507       current CMake test registrations
src/CMakeLists.txt:509-520       icewm-menu-fdo target
```

---

## Current upstream IceWM

- IceWM upstream repository: https://github.com/ice-wm/icewm
- IceWM project site: https://ice-wm.org/
- IceWM 4.1.0 release listing: https://github.com/ice-wm/icewm/releases
- IceWM manual: https://ice-wm.org/man/icewm.html
- IceWM preferences: https://ice-wm.org/man/icewm-preferences.html
- IceWM theme documentation: https://ice-wm.org/man/icewm-theme.html
- IceWM Theme Creation Howto: https://ice-wm.org/themes/
- `icesh`: https://ice-wm.org/man/icesh.html

---

## Standards/backends

- Extended Window Manager Hints / `_NET_NUMBER_OF_DESKTOPS`:  
  https://specifications.freedesktop.org/wm/latest-single/

- MPRIS v2.2:  
  https://specifications.freedesktop.org/mpris/latest/

- MPRIS Player interface:  
  https://specifications.freedesktop.org/mpris/latest/Player_Interface.html

- NetworkManager D-Bus API:  
  https://networkmanager.dev/docs/api/latest/spec.html

- PipeWire PulseAudio-compatible protocol:  
  https://pipewire.pages.freedesktop.org/pipewire/page_module_protocol_pulse.html

- PulseAudio client API documentation:  
  https://freedesktop.org/software/pulseaudio/doxygen/

- KDE Breeze Icons:  
  https://github.com/KDE/breeze-icons

- KDE Breeze visual style repository:  
  https://github.com/KDE/breeze

---

# 27. Final implementation decision

The project should proceed as an IceWM 4.1.0 fork with the following exact strategy:

```text
KEEP
    IceWM X11 WM core
    move/resize/minimize/maximize
    work-area/XRandR logic
    wmTile half/quarter geometry
    workspace core
    dynamic desktop-count protocol handling
    move-window-between-workspaces logic
    taskbar process
    task state model
    start/root menu
    Freedesktop menu generator
    QuickSwitch
    system tray
    clock
    icon-theme lookup
    icewmbg initially

ADD
    mouse snap target detector
    snap preview overlay
    snap restore state
    drag-only workspace edge switching
    arbitrary workspace insertion/removal
    workspace context menu
    filesystem desktop model/view/grid
    inotify desktop watcher
    persistent desktop layout
    Plasma-like unified pinned/running task renderer
    D-Bus dispatcher
    MPRIS applet
    NetworkManager applet
    libpulse volume applet
    Breeze fixed product theme

DO NOT ADD
    Qt
    GTK shell components
    HTML/JS/WebView
    Electron
    QML
    compositor
    new WM core
    new Alt+Tab engine
    new workspace protocol
    new icon-theme engine
    resident file manager
```

This is materially smaller than the previous estimate because IceWM 4.1.0 already implements several features that were previously classified as new work.

The real heavy work is concentrated in four areas:

1. **filesystem desktop grid,**
2. **mouse snap preview/restore UX,**
3. **arbitrary workspace mutation UX/semantics,**
4. **modern audio/media/network panel integrations.**

Everything else should be treated as reuse, theming or a narrow modification of existing IceWM code.

That is the shortest credible path to the requested "Plasma feel without Plasma weight" while preserving a realistic chance of meeting the low-memory objective.
