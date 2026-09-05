# FlameWM Reference Router — 0.0.5 / V8-authoritative contract over V7/V6/V5/V4

## Boot rule

Normal task:

1. root `AGENTS.md`;
2. this router;
3. only routed feature/reference packet(s);
4. exact current IceWM source owner(s).

First production coding session additionally reads:

- `implementation/READINESS.md`;
- assigned phase in `implementation/EXECUTION.md`.

Use `implementation/FAILURES.md` by trigger/high-risk subsystem. Never preload archives.

## V8 taskbar-ordering authority over non-conflicting V7/V6/V5/V4 (§1.0)

V8 is active authority over V7/V6/V5/V4, including taskbar ordering. Active evidence: `flamewm/prototype/v8/PORTING_CONTRACT.md`, present in this tree.

## V7 addendum — supersessions over V6/V5/V4 (§1.1; V8 remains newer authority)

V7 is an **addendum over non-conflicting V6/V5/V4**, not a replacement. V7 governs these points:

- Website: `https://wm.arkflame.com`.
- Start search stays inside Start and replaces normal Start rows while filtering.
- Desktop menu: Open Terminal / Create New Folder / optional New Sticky Note / Desktop and Wallpaper. Panel context creates virtual desktops.
- One workspace hides the workspace pager.
- Sticky notes are optional first-class desktop state; disabling removes all notes.
- Taskbar settings expose color, opacity, height, Start label and custom Start icon.
- Accent selection applies immediately; desktop selection paints desktop entries only.
- Appearance exposes icon-theme selection through IceWM/freedesktop infrastructure.

All non-conflicting V6/V5/V4 behavior remains authoritative.

## V5 addendum — retained non-conflicting supersessions over V4 (§1.2)

V5 is an **addendum over V4**, not a replacement. V4 remains locked (§1.2); the following supersede or extend V4 where noted:

- **Per-output panels**: one Flame panel per active RandR output (supersedes single primary-host taskbar only).
- **Toggle Start menu (not Open)**: Start button toggles the desktop/session menu owning the Start surface.
- **Active-output resolver order**: focused frame > pointer > primary > first output.
- **One Start globally**: exactly one Start button/menu system across all outputs.
- **Displays selector**: topology strip + selected-output controls (mode / resolution / scale / primary).
- **Hotkey UX**: shows `Not assigned`, `Escape` clears capture, per-row reset.
- **Opacity separation**: `DesktopSelectionFillOpacity` vs `WindowSnapPreviewFillOpacity` — distinct keys (not a shared opacity).
- **Semantic Settings icons**: per-page semantic icons (not generic placeholders).
- **Wordmark compact branding**: compact wordmark in header/About alongside full wordmark asset.
- **About 3 URLs**: Donate `https://paypal.me/LinsaFTW`, Source `https://github.com/arkflame/flamewm`, Website `https://wm.arkflame.com`.
- **Safe URI launch**: no shell interpolation; `Gio::AppInfo` / `g_app_info_launch_default_for_uri` / `xdg-open` via exec vector.

If V7, V5 and V4 conflict on the above points, newest contract governs. All other V4 rules remain in force.

## V4 lock — current product truth (remains locked per §1.3)

```text
NO Overview / Activities system or taskbar button
NO maximize-button hover snap-layout chooser; no Super+Z requirement
Default shell: dark / pitch-black
Default accent: Flame red
Window title: centered against complete titlebar when space permits
Default font: IBM Plex Sans; user can choose family, global bold, size offset
Desktop menu: Open Terminal / Create New Folder / optional New Sticky Note / Desktop and Wallpaper; panel context creates virtual desktops
Settings pages: Appearance / Desktop / Taskbar / Displays / Fonts / Hotkeys / About
Workspace UI: compact two-row topology; Ctrl+Super+Arrow directional navigation
Taskbar: Start / pinned+running / flexible drag area / desktops / media / audio / network / tray / centered clock+date
Taskbar host: primary display by default; primary-display change follows it unless later explicit placement owns otherwise
```

If historical material conflicts with this block, it is obsolete. Active V8/V7 rules govern conflicts; historical documents remain preserved as history.

## Implementation readiness

`0.0.5` is **V8-authoritative over V7/V6/V5/V4 — ready for phased code implementation** once coding environment establishes Phase-0 green baseline. No known product/architecture design blocker remains. Approved v8 HTML/CSS/JS and `PORTING_CONTRACT.md` are present.

## Workspace protocol

`.build/` contains generated output only. Agent coordination uses `.agents/inbox/`, `.agents/outbox/`, `.agents/claims/` and `.agents/locks/`; do not create status marker files. Default bounded Xephyr lab: 1600x900, auto display `:90..:99`, 10-second readiness timeout, isolated HOME/XDG, never host `DISPLAY`.

## Canonical packets

| Packet | Read for |
|---|---|
| `reference/PRODUCT.md` | mission, V4 defaults, boundaries, current UX laws |
| `reference/ENGINE.md` | IceWM capabilities, authorities, source-owner map |
| `reference/UI.md` | V4 visual/scaling/title/menu/icon rules |
| `reference/FEATURES.md` | feature-domain router only |
| `reference/features/WINDOWS.md` | titlebar, snap, workspaces, QuickSwitch |
| `reference/features/SHELL.md` | taskbar, pinned/running tasks, Start, clock/tray/battery |
| `reference/features/DESKTOP.md` | filesystem desktop, grid, selection, Trash, file actions, watermark |
| `reference/features/SETTINGS.md` | Settings, config/live apply, Displays, Fonts, Hotkeys |
| `reference/features/INTEGRATIONS.md` | MPRIS, audio, NetworkManager, calendar/status behavior |
| `reference/ENGINEERING.md` | architecture, build/tests, IPC, upstream/performance discipline |
| `reference/ASSETS.md` | prototype/branding/Breeze/Arc provenance |
| `implementation/READINESS.md` | whether coding can start and what remains environment/pixel-lock work |
| `implementation/EXECUTION.md` | phased source-grounded implementation simulation/gates |
| `implementation/FAILURES.md` | root-cause/recovery rules for coding failures |

## Task router

| Assigned work | Read | Inspect current source |
|---|---|---|
| First production edit / phase setup | `READINESS`, assigned `EXECUTION` phase, `ENGINEERING` | build/event-loop/owner files for that phase |
| Window chrome/title/scaling | `UI`, `WINDOWS`, Phase 1 | `themable.h`, `wmtitle.*`, `wmbutton.*`, `wmframe.*`, `decorate.cc` |
| Snap/drag-edge | `WINDOWS`, Phase 3 | `wmframe.*`, `movesize.cc`, `wmmgr.*` |
| Workspaces/pager/topology | `WINDOWS`, Phase 4 | `wmmgr.*`, `workspaces.h`, `aworkspaces.*`, keys owners |
| Taskbar/four-edge docking | `SHELL`, `UI`, Phase 6 | `wmtaskbar.*`, `wmmgr.*`, applet owners |
| Pinned/running tasks | `SHELL`, Phase 5 | `atasks.*`, `objbar.*`, `wmclient.*`, desktop-entry owners |
| Start/search/session menu | `SHELL`, `PRODUCT`, Phase 5 | `wmprog.*`, `fdomenu.cc`, `yicon.*`, input/menu widgets |
| Filesystem desktop/Trash | `DESKTOP`, Phase 9 | desktop EWMH owners, `wmminiicon.*`, `icesm.cc`, `yicon.*` |
| Settings/config/live apply | `SETTINGS`, Phase 2 | config parsers, `wmapp.*`, key owners |
| Displays/scaling | `SETTINGS`, `UI`, Phase 7 | XRandR/output code, work-area/taskbar owners |
| Theme/icons/visual polish | `UI`, `ASSETS`, Phase 1 | `themable.h`, `ymenu*`, `wmtaskbar.*`, `yicon.*`, theme resources |
| Build/CI/upstream merge | `ENGINEERING`, Phase 0/10 | CMake + Autotools + install/session files |
| Failure/re-entrancy/reconnect/race | relevant packet + `FAILURES` | full directed callback/state path before patch |
| Prototype/asset ingestion | `ASSETS`, `UI` | no production mutation during ingestion |

## Implementation order

0. Baseline/fork discipline + generic center-tile origin fix.
1. Runtime/scale/metrics + visual foundation.
2. Settings config/IPC + Appearance/Desktop/Taskbar/Displays/Fonts/Hotkeys/About.
3. Window drag snap — no hover chooser.
4. Workspace indexed mutation + two-row directional UX.
5. Start + unified pinned/running task area.
6. Four-edge taskbar.
7. Displays: XRandR modes + WM-owned safe revert + per-output shell scale.
8. Event-driven status integrations + clock/calendar.
9. Filesystem desktop V4 including selection/group drag/Trash/watermark.
10. Release closure/memory surgery/upstream merge rehearsal.

Do not bypass an earlier state/metrics/config authority required by a later phase.

## Deep evidence

- Current complete V4 native-gap report: `archive/CURRENT_V4_REPORT.tar.xz`.
- Older reports: `archive/HISTORICAL_REPORTS.tar.xz`.

Both are compressed intentionally so broad repository search cannot accidentally turn deep/obsolete prose into active authority. Extract only when active packets + current source cannot resolve a material ambiguity, verify the associated content checksum, and apply V7-over-non-conflicting-V6/V5/V4 authority order.
