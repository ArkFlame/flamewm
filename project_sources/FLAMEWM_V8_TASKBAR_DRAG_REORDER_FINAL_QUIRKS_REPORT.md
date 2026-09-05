# FlameWM V8 — Final Taskbar Drag/Reorder Quirks Report

**Project:** FlameWM  
**Base:** IceWM 4.1.0 / X11  
**Reference:** `breeze-desktop-prototype-v8`  
**Status:** Latest taskbar-ordering UX authority where it supersedes V7.

## 1. V7 failure mechanism

V7 correctly modeled one task entry per open window and a persistent pinned slot, but its browser drag implementation moved the **captured task button itself** through `insertBefore()` / `appendChild()` during `pointermove`.

That was an unstable browser interaction boundary. Pointer capture is meant to keep future pointer events targeted at the captured element while the pointer moves outside it, but moving the capture target through the DOM during the gesture can invalidate/lossily retarget that capture. The result was browser-dependent behavior where the task entry appeared impossible to drag or stopped receiving movement before a useful reorder occurred.

The V8 rule is therefore:

> **Never mutate the source task entry's DOM position while it owns the active pointer gesture.**

This is prototype-specific. Native FlameWM does not use DOM pointer capture, but the semantic acceptance criteria below still apply.

## 2. V8 interaction model

On primary-button press over a task entry:

1. Record pointer ID, initial pointer location and the entry identity.
2. Keep the real task entry stationary in the task-strip layout.
3. Capture the pointer on that stationary entry.
4. Movement below 6 logical pixels remains a click.
5. Crossing the threshold creates:
   - a floating visual clone (`task-drag-ghost`), and
   - an accent-colored insertion marker (`task-drop-indicator`).
6. Pointer motion is processed from a window-level capture listener, while element pointer capture guarantees the gesture remains owned if the pointer leaves the original button.
7. Determine insertion index by comparing the pointer to the midpoint of every sibling task entry on the taskbar's primary axis:
   - bottom/top -> X axis;
   - left/right -> Y axis.
8. The insertion marker may resolve to:
   - before the first entry;
   - between any two entries;
   - after the final entry.
9. Do **not** mutate persistent task order during pointer movement.
10. On release, splice the dragged key into the resolved insertion index, merge that visible order into the task-order authority, persist, then rerender once.
11. On pointer cancellation, remove ghost/marker and leave ordering unchanged.

## 3. Identity and state invariants

Task-strip identity remains V7's correct model:

```text
pin:<desktop-app-id>  persistent pinned slot
win:<window-id>       independent open-window entry
```

Pin state is application-level. Window actions and drag identity are entry/window-level.

A pinned application with open windows uses one anchored `pin:<app>` entry for its first represented window; additional windows remain independent `win:<id>` entries. No same-app grouping is permitted.

## 4. Exact context policy retained

```text
Pinned + no open window:
Open
Unpin

Pinned + open window:
Unpin
Maximize | Minimize
Close

Unpinned + open window:
Pin
Maximize | Minimize
Close
```

`Maximize | Minimize` is selected from the clicked window's state.

## 5. Native IceWM implementation consequence

IceWM 4.1.0 already contains the low-level task-button drag/reorder mechanism and can expose separate task buttons when grouping is disabled. Native FlameWM should reuse IceWM's frame/task identity and drag machinery rather than imitate the browser ghost implementation literally.

The native product contract is the semantic result:

- grouping disabled for Flame task entries;
- each open window independently reorderable;
- inactive pinned launchers occupy reorderable slots in the same strip;
- reorder insertion supports before/between/after, not only adjacent swaps that feel stuck;
- release commits the new order;
- cancellation preserves previous order;
- task click and task drag are separated by a small movement threshold;
- taskbar orientation selects horizontal or vertical ordering axis;
- order changes never alter X11 window identity, focus ownership or application pin identity.

If IceWM's existing adjacent-swap implementation produces equivalent smooth insertion semantics, retain it. If the pinned-slot integration introduces discontinuities, add the smallest Flame-owned ordering coordinator around the existing `TaskPane` mechanism rather than creating a second window/task registry.

## 6. Required regression proof

The implementation is not complete unless these cases pass:

1. Drag first pinned entry between entries 2 and 3; release; it stays there.
2. Drag final entry before the first; release; it becomes first.
3. Drag an inactive pinned launcher between two active window entries.
4. Open three windows of the same app and reorder the third between unrelated entries without grouping.
5. Reorder two same-app windows independently.
6. Movement below threshold behaves as click, not reorder.
7. Pointer/drag cancel preserves original order.
8. Horizontal bottom/top taskbars reorder on X.
9. Vertical left/right taskbars reorder on Y.
10. Close a reordered live window: only that ephemeral entry disappears; remaining order stays coherent.
11. Unpin an active app: the clicked application's pin state changes without losing/reordering unrelated live windows.
12. After all windows of a pinned app close, its pinned placeholder returns to its persisted strip slot.

## 7. V8 prototype proof

The final V8 browser harness executed Chromium interaction verification using an inline self-contained copy of the packaged HTML/CSS/JavaScript. It proved:

- drag ghost and insertion marker appear after threshold;
- first entry can be released between later entries;
- last entry can be released before the first;
- two additional Dolphin windows produce independent `win:<id>` entries;
- an extra Dolphin window can be dragged to the first taskbar position independently;
- no JavaScript/page-console error occurred in that test.
