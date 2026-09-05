# Windows and Workspaces — V4

## Preserve IceWM authorities

KEEP IceWM frame/client lifecycle, focus/stacking, move/resize, minimize/maximize/fullscreen, work-area/XRandR geometry, workspace state/EWMH, existing tile actions, edge-switch foundation and QuickSwitch. Do not create parallel truth.

## Titlebar

IceWM `TitleBarJustify=50` centers in the free region between controls; V4 wants the title centered against the **complete titlebar** when space permits.

Flame layout:

```text
desiredX = (titlebarWidth - titleTextWidth) / 2
minimumX = leftOccupied + padding
maximumX = rightOccupied - padding - titleTextWidth
paintX = clamp(desiredX, minimumX, maximumX)
```

Keep upstream justify behavior available for expert/non-Flame mode. Preserve existing titlebar double-click maximize (`TitleBarMaximizeButton=1`). Separate glyph size, button hit target, visual border and resize hit region.

## Drag snap

Reuse `YFrameWindow::wmTile()` and current work-area authority for existing geometry. V4 pointer targets:

```text
left/right edge -> half
top-left/top-right/bottom-left/bottom-right -> quarter
top center -> maximize
bottom center -> none
```

No maximize-hover chooser and no Super+Z requirement.

Preview: small pointer-transparent override-redirect Flame surface using accent border + accent-muted fill; no blur/compositor dependency. Preview geometry must equal committed geometry.

Minimal per-frame snap state:

```text
SnapTarget currentTarget
YRect unsnappedOuterGeometry
bool hasUnsnappedGeometry
```

Rules: capture once on first floating->snap; snap->snap does not overwrite; manual resize exits snap state; explicit maximize remains normal IceWM state; drag-away restores exact floating geometry before continuing; output removal clamps restore target to a surviving work area.

Source-proven IceWM 4.1.0 center-tile defect must be isolated as a generic fix before relying on shared tiling math: current center formula adds non-zero monitor origin twice.

## Workspaces

Core workspace engine stays IceWM-owned. Add first-class indexed transaction primitives rather than abusing tail-only shrink:

```text
insertWorkspace(index)
removeWorkspace(index)
```

One transaction must update model count/names, every affected frame index, active/last workspace, visibility/focus, `_NET_NUMBER_OF_DESKTOPS`, `_NET_CURRENT_DESKTOP`, names/viewport/workarea and pager/window menus. Hard invariant: at least one workspace.

V4 pager is a compact **two-row topology**. One mapping function must drive paint and keyboard navigation:

```text
index <-> row,column
row,column + direction -> index
```

Expose product actions `WorkspaceLeft/Right/Up/Down`; default navigation is `Ctrl+Super+Arrow`. Do not implement Up/Down as magic workspace numbers.

## Snap vs dragged-window workspace edge

Passive pointer edge switching: off. While dragging a window:

1. corners always quarter-snap;
2. side edge immediately previews half snap;
3. quick release commits snap;
4. sustained extreme-side dwell cancels preview and moves to adjacent workspace while retaining the drag;
5. leaving edge resets dwell.

Use one named dwell setting/constant, not scattered timers.

## QuickSwitch

IceWM already provides Alt+Tab and live preview machinery. Configure/restyle first; do not create a second switcher engine. Verify minimized windows, correct activation, lifecycle cleanup, and no preview leak.

## Verification

- Flame snap disabled -> stock move/resize behavior unchanged.
- Minimize/maximize/restore/close and title double-click unchanged.
- Title center equals frame center where space permits and never overlaps controls.
- Half/quarter/top targets exact; preview == commit.
- Every taskbar edge work area respected.
- Negative/non-zero monitor origins.
- Exact drag-away restore.
- Workspace add/remove first/middle/last; min one; every frame index/EWMH property exact.
- Two-row directional navigation deterministic.
- Passive edge pointer does not switch; dragged window can traverse side edge.
