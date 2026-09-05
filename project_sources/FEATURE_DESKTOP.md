# Filesystem Desktop — V4

## Process boundary

Add a separate `flamewm-desktop` process. Filesystem watching/file actions/selection/Trash must not run inside reliability-critical WM state paths. `icewmbg` keeps wallpaper ownership initially.

Suggested owners:

```text
desktopapp / model / view / item / layout / watcher
selection / trash / fileactions / watermark
```

A desktop-helper crash must not crash or wedge the WM.

## Filesystem model

Resolve the XDG Desktop directory; never hardcode `~/Desktop`. Minimum item kinds:

```text
Directory
RegularFile
DesktopLauncher
Symlink/Shortcut
TrashPseudoItem
```

Use inotify and coalesce create/delete/rename/relevant-attribute bursts. No periodic directory scan.

## Grid and persistence

Grid uses current usable work area, excluding whichever taskbar edge reserves space. Persist logical placement:

```text
outputIdentity + column + row
```

not raw pixels. Resolution/scale/taskbar/output changes recompute legal cells. Retain valid cells; deterministically reflow invalid cells; removed-output items move to first-free cells on a surviving primary output. Never lose metadata silently.

New item -> first deterministic free cell.

## Selection and group movement

Authoritative selected set is item identity, independent of filesystem order. Blank primary drag after threshold creates a pointer-transparent accent rectangle; select visual hit rectangles that intersect it. Blank click without meaningful drag clears selection.

Dragging any selected member moves the group as **one transaction**:

1. preserve relative cell offsets;
2. compute one candidate delta;
3. clamp the entire group bounding cell rectangle;
4. reject overlap with unselected items;
5. commit whole group or do not commit.

Never independently move members and repair collisions afterward.

## Trash

Use the freedesktop Trash specification, not a Flame-private trash store. Home trash uses `$XDG_DATA_HOME/Trash/files` + `info` `.trashinfo`; obey cross-filesystem/top-directory rules.

Group drop onto Trash excludes Trash itself, attempts every selected normal item, reports exact partial failures, then refreshes model from actual filesystem state.

`Empty Trash` must report failures and must not claim empty while entries remain.

Current V4 file context label `Delete` means permanent removal only with a clear destructive confirmation. Drag-to-Trash remains recoverable. Do not silently blur these semantics.

## File actions

Create Shortcut:

- application `.desktop` -> collision-safe Desktop launcher/copy preserving desktop-entry semantics;
- regular file/directory -> collision-safe symlink;
- never concatenate paths into shell strings.

Desktop context menu:

```text
Open Terminal
Create New Folder
Add Virtual Desktop
Desktop and Wallpaper
```

- Terminal should use XDG Desktop as working directory when safely supported.
- New Folder is collision-safe, refreshed/selected and assigned first-free cell.
- Add Virtual Desktop sends narrow intent to WM workspace authority; desktop process never mutates workspace truth.
- Desktop and Wallpaper opens the appropriate Settings route.

## Wallpaper and watermark

`icewmbg` remains wallpaper backend until measurement justifies consolidation. V4 default is pitch-black/no image until chosen.

Watermark: scalable Flame wordmark at bottom-right usable work area, taskbar-aware, pointer-transparent, behind normal windows, hidden during fullscreen, subtle, user-toggleable.

## Verification

- Real filesystem create/delete/rename reflected.
- Grid never exits work area; panel move/resolution/scale recomputes cells.
- Selection intersection and drag threshold exact.
- Group offsets/collision/clamp/commit semantics exact.
- Trash records spec-compliant; Empty Trash exact; partial failures visible.
- Output removal relocates safely.
- Killing/restarting desktop helper leaves WM usable.
