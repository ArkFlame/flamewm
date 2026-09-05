# FlameWM Fork Source Report

**Upstream base:** IceWM 4.1.0  
**Working fork name:** FlameWM  
**Report date:** 2026-09-02  
**Purpose:** persistent project-source report for designing and implementing a lightweight, Plasma/Breeze-inspired IceWM fork without rediscovering the upstream architecture in each implementation session.

---

## 0. Assumptions and design contract

1. FlameWM is an **X11-first** fork of IceWM. This report does not pretend IceWM is a Wayland compositor or a complete KDE Plasma replacement.
2. The primary objective is **very low resident memory, fast startup, simple compilation, and maintainability**, while adding the specific desktop ergonomics requested for FlameWM.
3. Existing IceWM behavior should remain intact unless a FlameWM feature deliberately replaces a visible interaction. We should reuse upstream mechanisms instead of creating parallel state machines.
4. “Activities” means a **Plasma-like overview/launcher entry point** showing workspaces and windows. It does **not** mean implementing KDE Activities’ semantic activity profiles, separate activity-specific settings, or Plasma’s entire shell architecture.
5. “Desktop grid” means a lightweight desktop surface capable of background + grid-aligned desktop launchers/files. This should be a **separate FlameWM process** rather than filesystem/icon-management code inside the window manager process.
6. Breeze appearance means **original Breeze-inspired FlameWM assets and metrics**. Do not blindly copy KDE artwork into the fork without separately checking each asset’s license and attribution obligations.
7. System controls (audio, Wi-Fi, media) must not make PipeWire, NetworkManager, or a high-level desktop framework hard compile-time requirements for the WM. Runtime capability detection is preferred.
8. The report is authoritative for the uploaded `icewm-master.zip` snapshot. If upstream source changes, line numbers and some conclusions must be revalidated before implementation.

---

## 1. Executive decision

**Fork IceWM. Do not port XFCE, combine Openbox with another shell, or rewrite the window manager in another language.**

The uploaded IceWM source already contains most of the difficult low-level primitives FlameWM needs:

- X11 window management, decoration, focus, stacking, placement, EWMH/ICCCM integration.
- A taskbar/panel with start button, application tasks, toolbar, workspace pager, clock, system tray, network monitor, battery, keyboard, CPU and memory applets.
- Dynamic workspace count support.
- Live Alt+Tab window previews using XDamage/XRender.
- A window-list UI.
- Existing half/quarter/center tiling actions and geometry.
- Theme loading with colors, fonts, title buttons, panel resources and PNG/XPM assets.
- XRandR-aware work areas and per-window screen selection.
- XDG `.desktop` discovery via the `icewm-menu-fdo` helper.
- Session orchestration (`icewm-session`) and separate background process (`icewmbg`).

A rewrite would spend most of its effort reimplementing mature X11 protocol behavior that has nothing to do with the requested differentiating features. FlameWM should instead add a thin modern shell layer to the existing core.

### Recommended stack

| Layer | Choice | Reason |
|---|---|---|
| WM core | Existing IceWM C++/Xlib/XRender | Already mature, lightweight, protocol-compliant foundation |
| Language | Existing C++ style; use C++11-compatible constructs unless project baseline is deliberately raised | Lowest integration risk; no language bridge/runtime |
| Build | CMake as primary developer path; retain Autoconf compatibility initially | Upstream CI already tests both |
| UI | Existing `YWindow`/`YButton`/`YPopupWindow`/`Graphics` toolkit | No GTK/Qt runtime memory tax |
| Images/icons | Existing image abstraction; PNG theme assets; optional SVG icon loading | Native path already exists |
| Audio | Runtime `wpctl` backend when available | No libpipewire/libwireplumber compile dependency |
| Wi-Fi | Runtime `nmcli`/NetworkManager backend | No libnm compile dependency in v1 |
| Media | Runtime `playerctl` backend over MPRIS, optionally direct MPRIS later | Lightweight compile; standards-based behavior |
| Desktop icons | Separate `flamewm-desktop` process using the same small IceWM UI primitives | Isolates filesystem/DND complexity from WM reliability |
| Compositing | External compositor optional; not part of FlameWM v1 | Preserves low memory and scope |

---

## 2. Source snapshot identity and verification status

### 2.1 Snapshot

- `VERSION`: `PACKAGE=icewm`, `VERSION=4.1.0`.
- `README.md`: package identifies itself as IceWM 4.1.0, released **2026-08-06**.
- Official upstream web documentation checked on 2026-09-02 also identifies 4.1.0 as the current release.
- Uploaded archive SHA-256:

```
ba3741a41131bd4966154563a517d26de9c5ee6540bfc37d92111a0f298ce75c
```

- Source has **1,070 files** and **43 directories** in this snapshot.
- Approximate text/source inventory from the extracted tree:
  - 122 `.cc` files, ~77,542 lines.
  - 127 `.h` files, ~15,162 lines.
  - 624 `.xpm` files, mostly theme/image assets.
  - 47 `.po` translation files.
  - 19 `.theme` files.
  - Total counted newline-based text lines across all file types: ~330,661.

### 2.2 Build verification performed for this report

Command attempted:

```bash
cmake -S . -B /mnt/data/icewm-build-report -DBUILD_TESTING=ON
```

Result: **environment-blocked before generation**, not a source failure.

Observed missing development packages in the analysis sandbox:

```text
Package 'fontconfig', required by 'virtual:world', not found
Package 'xrender', required by 'virtual:world', not found
CMake Error: required package xrender was not found
```

Therefore this report does **not** claim that the uploaded snapshot was successfully compiled in this sandbox. Upstream CI supplies the dependency set and builds both CMake and Autoconf configurations.

### 2.3 Reproducible Ubuntu/Debian baseline

The upstream CI dependency line (`.github/workflows/cmake.yml:28-31`) is the best starting point:

```bash
sudo apt update
sudo apt install \
  libxrender-dev gettext autopoint libxft-dev libsndfile1-dev libao-dev \
  libsm-dev libx11-dev libxext-dev x11proto-core-dev markdown asciidoctor \
  libxpm-dev libimlib2-dev libgdk-pixbuf2.0-dev libglib2.0-dev \
  libfribidi-dev librsvg2-dev xorg-dev
```

Developer gate:

```bash
cmake -S . -B build -DCMAKE_BUILD_TYPE=RelWithDebInfo -DBUILD_TESTING=ON
cmake --build build -j"$(nproc)"
ctest --test-dir build --output-on-failure
```

Before merging FlameWM changes, also run the Autoconf path because upstream CI treats it as a supported build:

```bash
./autogen.sh
mkdir -p build-ac
cd build-ac
../configure
make -j"$(nproc)"
```

---

## 3. Repository organization

### 3.1 Top-level ownership

| Path | Responsibility |
|---|---|
| `src/` | Window manager, internal X11 UI toolkit, taskbar/applets, session/background/utilities, tests |
| `lib/` | Default configuration, icons, taskbar assets, themes, desktop entries |
| `man/` | POD manual sources for executables/config formats |
| `po/` | Native-language translations |
| `doc/` | User/developer documentation |
| `contrib/` | Auxiliary scripts/integrations |
| `utils/` | Developer/release helpers |
| `.github/` | CI/workflow metadata |
| `CMakeLists.txt`, `src/CMakeLists.txt` | CMake build graph |
| `configure.ac`, `Makefile.am`, `autogen.sh` | Autoconf/Automake build path |
| `COMPLIANCE` | EWMH/ICCCM/specification support notes |
| `TODO` | Upstream design debt and unfinished work |
| `COPYING` | GNU Library General Public License, Version 2, June 1991 |

### 3.2 `src/` is intentionally flat

There are no subsystem directories inside `src/`; major components are filename-based. This matters for future coding sessions: adding FlameWM features should **not** trigger a repository-wide directory migration. Add bounded new source pairs and wire them into `ICEWM_SRCS`.

Largest/most central implementation files include:

| File | Approx. LOC | Role |
|---|---:|---|
| `src/icesh.cc` | 6,270 | External window-control CLI |
| `src/wmmgr.cc` | 4,100 | Root window manager/state/workspaces/work areas/stacking |
| `src/wmframe.cc` | 4,075 | Managed frame behavior/actions/geometry |
| `src/wmapp.cc` | 2,233 | WM process/bootstrap/config/actions |
| `src/wmclient.cc` | 2,180 | Client X11 metadata/protocol handling |
| `src/ywindow.cc` | 2,031 | Fundamental internal X window abstraction |
| `src/yxapp.cc` | 1,592 | X application/extension/event support |
| `src/wmswitch.cc` | 1,346 | Alt+Tab switcher |
| `src/wmtaskbar.cc` | 1,230 | Taskbar/appbar creation and layout |
| `src/preview.cc` | 1,071 | Live switcher previews |
| `src/aworkspaces.cc` | 832 | Workspace pager/buttons/previews |
| `src/yxtray.cc` | 899 | XEmbed system tray |
| `src/wmtitle.cc` | 867 | Frame titlebar |
| `src/apppstatus.cc` | 846 | Network throughput/status applet |
| `src/wmwinlist.cc` | 811 | Window list UI |

### 3.3 Build targets

`src/CMakeLists.txt:435-486` builds two static internal libraries and the WM:

- `ice`: base X11/toolkit/platform/config primitives.
- `itk`: menus/widgets/config/theme-oriented UI pieces.
- `icewm`: the actual window manager + taskbar/applets.

Other executables:

- `icewm-session`: session orchestration.
- `icewmbg`: background manager.
- `icesh`: window manipulation tool.
- `icewmhint`: IceWM hint utility.
- `icewm-menu-fdo`: XDG desktop application menu generator, when enabled.
- `icesound`: event sound service when an audio backend is built.
- `icehelp`: help viewer.
- `icewmtray`: deprecated optional external tray build.
- Developer/build helpers such as `genpref`, `icecursor`.

---

## 4. Runtime architecture

### 4.1 Process model

Normal session architecture is intentionally small:

```text
icewm-session
├── icewmbg          background / _XROOTPMAP_ID
├── icewmtray        optional/deprecated external tray mode
├── icesound         optional event-sound process
├── startup script   user/session commands
└── icewm            actual window-manager process
    ├── YWMApp
    ├── YWindowManager
    ├── managed YFrameWindow/YFrameClient objects
    ├── TaskBar and applets
    ├── menus/popups
    ├── Alt+Tab switcher / preview objects
    └── X11 event loop + poll/timer integration
```

`icewm-session` explicitly launches background/tray/sound/WM components and runs the startup/shutdown scripts (`src/icesm.cc`). FlameWM should preserve this separation because it gives us a natural place to add optional shell processes without inflating the reliability-critical WM.

### 4.2 WM bootstrap

Source path:

```text
main()                                       src/wmapp.cc:1918+
  -> construct YWMApp                       src/wmapp.cc:1380+
     -> load preferences/theme/override     src/wmapp.cc:1403-1405
     -> register signals                    src/wmapp.cc:1428-1434
     -> create YWindowManager               src/wmapp.cc:1447
     -> initialize icons/pixmaps            src/wmapp.cc:1452-1454
  -> YWMApp::mainLoop()                     src/wmapp.cc:1583
     -> manager->manageClients()             src/wmapp.cc:1585
     -> inherited X/event main loop
```

Taskbar creation is lazy through `YWMApp::createTaskBar()` (`src/wmapp.cc:2015-2021`).

### 4.3 Core object responsibilities

#### `YWMApp` — process/application controller

`src/wmapp.h`, `src/wmapp.cc`

Owns application bootstrap, preferences/theme load, global actions, session actions, program launching, key reloads and taskbar creation. Global pointers `wmapp` and `manager` are used extensively.

#### `YWindowManager` — root WM authority

`src/wmmgr.h`, `src/wmmgr.cc`

Owns:

- Root-window protocol state.
- Managing/unmanaging client windows.
- Focus order and activation.
- Layers/stacking.
- Workspace state.
- Work area calculation and taskbar struts.
- Placement/arrangement/whole-screen tiling.
- EWMH desktop properties.
- Client list and window-list integration.

This is the only correct authority for operations that change global workspace identity or work-area semantics.

#### `YFrameClient` — client window protocol wrapper

`src/wmclient.h`, `src/wmclient.cc`

Tracks the application client window, hints, class/title/icon, state hints and protocol-facing metadata.

#### `YFrameWindow` — decorated managed window

`src/wmframe.h`, `src/wmframe.cc`

Owns decoration-level behavior, normal/maximized/fullscreen state, movement/resizing, actions, tabs, workspace assignment, titlebar/container and per-window tiling.

#### `YFrameTitleBar` / `YFrameButton`

`src/wmtitle.*`, `src/wmbutton.*`

Own frame chrome and title buttons. The maximize button already differentiates maximize vs. restore. This is the correct hover anchor for the FlameWM snap-layout chooser.

#### `TaskBar`

`src/wmtaskbar.*`

A `YFrameClient` that directly owns almost every panel applet. `TaskBar::initApplets()` creates the applets; `TaskBar::updateLayout()` hard-codes their placement. Upstream itself lists “taskbar rewrite / modularize taskbar objects” in `TODO:54-57`, and `wmtaskbar.h:196` explicitly calls out the global `taskBar` pointer as debt.

#### `WorkspacesPane` / `WorkspaceButton`

`src/aworkspaces.*`

Workspace buttons, inline rename, scrolling, miniature workspace preview and pager painting. Right click currently opens the window-list menu, not a workspace-management menu.

#### `SwitchWindow` / `SwitchPreview` / `Preview`

`src/wmswitch.*`, `src/preview.*`

Alt+Tab implementations. `SwitchPreview` tracks windows and uses XDamage plus XRender pictures to maintain live previews. This is the source mechanism to reuse for Activities/Overview thumbnails.

#### `StartMenu` / `MenuLoader`

`src/wmprog.*`

Classic root/start menu using IceWM menu configuration and generated program menus. It is a cascading menu, not a Plasma-style searchable launcher.

#### `YXTray`

`src/yxtray.*`

XEmbed system tray. Preserve it; Wi-Fi/audio/media FlameWM controls are **native applets beside the tray**, not replacements for XEmbed.

#### `icewmbg`

`src/icewmbg.cc`

Separate background process; maintains root background and `_XROOTPMAP_ID`. It supports per-workspace and multihead background behavior. Do not merge this into WM core.

---

## 5. Internal toolkit and event model

IceWM does not depend on Qt or GTK for its WM UI. It implements a compact in-tree toolkit:

- `YWindow`: X window/event base.
- `YButton`, `ObjectButton`: buttons.
- `YMenu`, `YMenuItem`, `YPopupWindow`: menus/popups.
- `YInputLine`, `YListBox`, `YScrollView`: form/list primitives.
- `Graphics`, `GraphicsBuffer`, `YSurface`: drawing.
- `YImage`, `YPixmap`, `YIcon`: images/icons.
- `YFont`, `YColor`: text/color.
- `YTimer`: event-loop timers.
- `YPollBase`: FD polling integration.
- `YPipeReader`: fork/exec stdout reader integrated with polling.

The WM is fundamentally **event-driven and mostly single-threaded** around the X event loop. FlameWM should keep X object mutation on that event loop. Slow I/O must never occur in a click/expose/motion handler.

### Important `YPipeReader` caveat

`src/ypipereader.cc:27-54` can fork/exec an argument vector and register the child stdout pipe with the poll loop. However:

- It is a very small primitive, not a complete command-service abstraction.
- `read()` is one-shot; the listener must request the next read after each callback.
- It captures stdout only.
- The WM’s global SIGCHLD path reaps children (`src/yapp.cc`, `src/wmapp.cc:1432`), but command ownership/result correlation still needs to be implemented.

Therefore new system controls should use a dedicated `CommandRunner` wrapper rather than each applet manipulating `YPipeReader` directly.

---

## 6. Configuration architecture

Load order is visible in `src/wmapp.cc:1403-1405`:

1. Preferences/config file.
2. Theme configuration.
3. `prefoverride`.

`WMConfig::loadConfiguration()` and `loadThemeConfiguration()` are in `src/wmconfig.cc:24-49`.

Official IceWM config search priority is:

1. `$ICEWM_PRIVCFG/`
2. `$XDG_CONFIG_HOME/icewm/`
3. `$HOME/.icewm/`
4. system config directory (commonly `/etc/icewm/`)
5. installation data (commonly `/usr/share/icewm/`)

Important files include `preferences`, `prefoverride`, `menu`, `programs`, `toolbar`, `keys`, `theme`, `winoptions`, `startup`, `shutdown`.

### FlameWM configuration policy

Do **not** invent JSON/YAML for the fork. Extend the existing preference table and config grammar so the same parser, rewrite tooling and precedence continue to work.

Suggested new options, with conservative defaults:

```text
FlameActivitiesButton=1
FlameOverview=1
FlameLauncher=1
FlameSnapLayouts=1
FlameSnapHoverDelay=350
FlamePanelAudio=1
FlamePanelWifi=1
FlamePanelMedia=1
FlameDesktop=0
FlameDesktopGridSize=80
FlamePanelMediaMaxWidth=220
FlameSystemCommandTimeout=3000
```

Names are provisional until code begins, but the configuration *mechanism* should remain upstream-compatible.

---

## 7. Theme/rendering capabilities and boundaries

### 7.1 What native FlameWM can do

- Flat Breeze-like colors.
- Custom titlebar height and border metrics.
- Original minimize/maximize/close/menu button art.
- Hover/pressed title-button resources.
- Panel/task/workspace backgrounds.
- PNG resources even when the canonical theme resource is named `.xpm`: `src/wpixres.cc:374-408` scans both `.xpm` and `.png`, maps `.png` names to canonical `.xpm` resource names, and loads them.
- Fonts, text colors, focus colors, active/inactive chrome.
- Alpha-capable rendering inside FlameWM-owned windows when the X visual/rendering path permits it.

### 7.2 What IceWM is not

The source initializes XComposite/XDamage support (`src/yxapp.cc`) and uses composition primitives for specific functions such as tray embedding/previews, but it does **not** implement a root compositing manager. There is no `XCompositeRedirectSubwindows` compositor loop and no `_NET_WM_CM_Sn` ownership implementation in this snapshot.

Therefore FlameWM v1 must not promise native:

- Background blur/acrylic.
- Compositor-level window shadows.
- Universal transparent windows independent of an external compositor.
- Wayland effects/protocols.

Recommended visual target: **Breeze-inspired, flat, crisp, opaque by default**. If `picom` or another compositor is installed, FlameWM may ship an optional example config, but the WM itself remains compositor-independent.

---

## 8. Existing feature inventory relevant to FlameWM

| Requested/related capability | Upstream state | Source | FlameWM action |
|---|---|---|---|
| Start button | Exists | `wmtaskbar.cc:329-340` | Replace popup target with modern launcher; retain fallback root menu |
| Start/root menu | Exists, classic menu | `wmprog.*` | New searchable launcher UI; reuse app discovery/icon/program launch |
| Taskbar application buttons | Exists | `atasks.*`, `wmtaskbar.*` | Restyle, retain behavior |
| Workspace pager | Exists | `aworkspaces.*` | Restyle; add right-click management |
| Workspace live mini-preview | Exists | `aworkspaces.cc` + `PagerShowPreview` | Enable/use in Flame theme if desired |
| Dynamic workspace count | Exists for append/tail removal | `wmmgr.cc:2854-2921` | Reuse append path; add safe indexed removal |
| Clock | Exists | `aclock.*` | Theme/format to show time + date |
| System tray | Exists | `yxtray.*` | Keep |
| Network throughput applet | Exists | `apppstatus.*` | Do not confuse with Wi-Fi control |
| Audio mixer | Not present | — | New optional runtime backend/widget |
| Wi-Fi selector/control | Not present | — | New optional runtime backend/widget |
| Media/MPRIS control | Not present | — | New optional runtime backend/widget |
| Window list | Exists | `wmwinlist.*`, `wmwinmenu.*` | Retain; Overview is separate modern surface |
| Alt+Tab textual/icon switcher | Exists | `wmswitch.*` | Retain |
| Live Alt+Tab previews | Exists | `preview.*` | Factor/reuse thumbnail mechanism for Overview |
| Half/quarter snap actions | Exists | `wmframe.cc:1500-1578` | Generalize to layout-region engine |
| Tiling menu | Exists, textual | `wmapp.cc:634-655` | Keep fallback; visual layout popup becomes primary UX |
| Maximize titlebar button | Exists | `wmbutton.*` | Add delayed hover snap popup without changing click semantics |
| Desktop background | Exists | `icewmbg.cc` | Retain |
| General desktop file/icon manager | Not present | — | Separate `flamewm-desktop` process |
| Compositor | Not present | — | External/optional only |
| Wayland session | Not present | — | Out of scope for this fork architecture |

---

## 8A. Comprehensive upstream feature catalog

This catalog is broader than the FlameWM-specific mapping above and is intended to stop future sessions from re-implementing functionality IceWM already owns.

### Window management

- Reparenting/decorating X11 application windows.
- Minimize, maximize, vertical/horizontal maximize, restore, hide/show, roll-up/roll-down.
- Fullscreen handling.
- Raise/lower/depth operations and layered stacking.
- Interactive pointer and keyboard move/resize.
- Window arrangement and existing tile actions.
- Sticky/all-workspaces windows and per-window workspace assignment.
- Per-window/application options (`winoptions`) controlling decorations, functions, layers, workspaces, taskbar/window-list/pager visibility and related behavior.
- Window tabs inside a frame (`YFrameWindow` tab methods in `wmframe.h`).
- Transient/modal relationships and owner-related minimize/hide restoration behavior.
- Mini-icons on the desktop when `MinimizeToDesktop` is enabled.
- Focus ordering/history and last-focus tracking.
- Application/window icon and metadata handling.

### Focus/input models

`YWMApp` defines multiple focus modes (`wmapp.h:13-21`):

- custom,
- click-to-focus,
- sloppy mouse focus,
- explicit focus,
- strict mouse focus,
- quiet sloppy focus.

IceWM also supports extensive configurable key and mouse bindings, edge workspace switching, pointer-driven move/size and menu keyboard navigation.

### Workspaces/desktops

- Named virtual desktops.
- EWMH desktop count/current desktop/names/layout/work-area publication.
- Dynamic count increase/decrease at runtime.
- Workspace switching and moving windows between workspaces.
- Workspace pager buttons.
- Optional miniature graphical pager previews, including window outlines/icons.
- Inline workspace rename by double click.
- Mouse wheel workspace switching.
- Per-workspace background support through `icewmbg`.

### Taskbar/panel

The current taskbar can provide:

- Start/root-menu button.
- Show-desktop button.
- Window-list menu button.
- Configurable application toolbar.
- Workspace pager.
- Running-window task buttons.
- Window tray.
- XEmbed system tray.
- Clock.
- Battery/APM status.
- CPU monitor.
- Memory monitor.
- Network interface/throughput monitor.
- Mailbox monitor.
- Keyboard-layout indicator.
- Address bar/command entry.
- Collapse/autohide behavior.
- Top or bottom placement, size/width options and taskbar struts/work-area reservation.

### Menus and launch

- Configurable root/start menu.
- Program/application submenus.
- XDG application menu generation via `icewm-menu-fdo`.
- Toolbar launch entries.
- Run command integration.
- Window list menus.
- Focus/settings/themes/help/logout menus.
- User-defined key launch commands and run-once/restart semantics.

### Window switching and lists

- Standard Alt+Tab switcher.
- Preview-based switcher with live window thumbnails.
- Window list client/UI.
- Workspace-aware filtering and configurable quick-switch behavior.

### Appearance/theme

- Multiple built-in look styles and theme directories.
- Active/inactive frame colors and fonts.
- Titlebar/button images and rollover resources.
- Taskbar/workspace/button image resources.
- XPM and PNG theme resource lookup; optional broader image/SVG support through configured loaders.
- Theme overrides and user `prefoverride` precedence.
- Configurable titlebar/button/border metrics and ordering.

### Desktop/background/docks

- Separate `icewmbg` background process.
- Root pixmap publication via `_XROOTPMAP_ID`.
- Per-workspace and multi-monitor background logic.
- WindowMaker-style dock apps/dock support.
- Optional minimized-window desktop mini-icons.
- No general filesystem desktop-icon manager in upstream.

### Session/system integration

- `icewm-session` startup ordering and crash/restart handling.
- User `startup` and `shutdown` scripts.
- Optional X session management (SM/ICE).
- Optional `icesound` GUI event sounds.
- Logout/reboot/shutdown/suspend-oriented menu/action integration where configured.
- Ctrl+Alt+Delete/session UI paths.

### X11 standards/extensions

The source contains extensive ICCCM/EWMH behavior; `COMPLIANCE` documents supported standard atoms. Build/runtime support includes or can include:

- X11/Xext,
- XRender,
- XComposite,
- XDamage,
- XFixes,
- XCursor,
- XRandR,
- Xinerama,
- XRes,
- Xft/fontconfig,
- Shape,
- SM/ICE session management.

Presence of XComposite support does **not** make IceWM a compositing manager; see the rendering boundary section above.

### Utilities

- `icesh`: powerful external window/property/workspace manipulation CLI.
- `icewmbg`: background service.
- `icewm-session`: session supervisor.
- `icewm-menu-fdo`: XDG menu generation.
- `icewmhint`: apply IceWM-specific window hints.
- `icesound`: event-sound process when built.
- `icehelp`: integrated help viewer.
- Optional/deprecated `icewmtray` external tray.

### Localization and documentation

- gettext-based translations (`po/`, 47 `.po` files in this snapshot).
- POD man pages for commands and configuration files.
- Generated/default preferences and user-config rewrite support.

The rule for future work is: if a requested behavior appears in this catalog, locate and extend its existing owner before adding a new subsystem.

---

## 9. Requested FlameWM experience: exact target

### 9.1 Panel/taskbar

Default logical order:

```text
[ Flame/Start ] [ Activities ] [ Workspaces ] [ running tasks ... ]
                                            [ media ] [ audio ] [ wifi ] [ tray ] [ date/time ]
```

Rules:

- Start opens searchable Flame Launcher.
- Activities opens Overview.
- Workspaces support click switch and right-click `Add Desktop` / `Remove This Desktop`.
- At least one desktop must always exist.
- Running tasks reuse existing task buttons.
- Media shows compact current-track state only when a controllable player exists; otherwise it collapses completely.
- Audio is always compact when backend exists; click opens volume/output popup.
- Wi-Fi is compact when NetworkManager/nmcli backend exists; click opens state/network popup.
- XEmbed tray remains independent.
- Date/time are visible, not tooltip-only.

### 9.2 Flame Launcher

Required v1 behavior:

- Search field focused immediately.
- Application list/grid sourced from XDG `.desktop` data.
- Keyboard navigation: arrows, Enter launch, Esc close.
- Application icon + display name.
- Optional categories/favorites can come after base search is correct.
- Power/session actions can be placed at the bottom using existing logout/restart/shutdown action paths.
- Opening launcher again toggles it closed.

Do not implement a webview/HTML renderer inside the production WM. The prior HTML/CSS/JS prototype is useful as a **visual specification**, not as the runtime shell technology.

### 9.3 Activities / Overview

Required v1 behavior:

- Full-screen or large per-active-monitor overlay.
- Workspace strip/grid.
- Live thumbnails of eligible windows on current/all workspaces.
- Click thumbnail activates window and closes Overview.
- Click workspace changes workspace.
- Esc closes.
- Activities button toggles.
- Existing Alt+Tab semantics remain untouched.

Later, window dragging between workspaces can be added once core state is stable.

### 9.4 Windows-style snap layouts

Required v1 behavior:

- Hover maximize button for a configurable short delay to open a **visual grid** of snap layouts.
- Normal left-click maximize continues to maximize exactly as before.
- Moving from maximize button to popup must not make the popup disappear during the pointer transition.
- Clicking a region places the active frame into that region of the frame’s current monitor work area.
- Keyboard equivalents and existing tiling actions remain functional.
- Layout geometry must be monitor/work-area aware and must not overlap reserved panel struts.

Suggested default layouts:

1. `1/2 + 1/2`
2. `2/3 + 1/3`
3. `1/3 + 2/3`
4. `1/3 + 1/3 + 1/3`
5. four quarters
6. left half + two right quarters

Do not create six separate geometry algorithms. They are data applied by one exact region engine.

### 9.5 Desktop grid

Required when enabled:

- Background remains owned by `icewmbg`.
- New `flamewm-desktop` creates a desktop-layer window and manages filesystem/launcher icons.
- Icons align to a deterministic grid.
- Separate process crash must not kill or destabilize window management.
- WM recognizes it as `_NET_WM_WINDOW_TYPE_DESKTOP`/desktop layer through existing EWMH behavior.

This is intentionally separated from the WM because directory monitoring, desktop-file parsing, drag/drop, file operations and thumbnail/icon layout are a different failure domain.

---

## 10. Fork architecture to implement

The safest architecture is **extension, not replacement**:

```text
Existing IceWM protocol/window core
│
├── Existing YWindowManager / YFrameWindow
│   └── Flame snap region API      [new, small geometry/state layer]
│
├── Existing TaskBar
│   ├── Existing TaskPane / Tray / Clock / Workspaces
│   └── Flame panel widgets        [new bounded widgets]
│       ├── ActivitiesButton
│       ├── MediaControl
│       ├── AudioControl
│       └── WifiControl
│
├── Existing menu/program launch mechanisms
│   └── FlameLauncher              [new popup surface]
│
├── Existing preview/window tracking mechanisms
│   └── FlameOverview              [new overlay surface]
│
├── Existing YPopupWindow/YTimer
│   └── FlameSnapPopup             [new titlebar hover surface]
│
├── Existing poll/SIGCHLD process support
│   └── FlameCommandRunner         [new safe async command service]
│
└── icewm-session
    └── flamewm-desktop optional   [new independent process]
```

### Proposed new source files

Names may be adjusted to upstream naming conventions, but responsibilities should stay separated:

```text
src/flamecommand.h
src/flamecommand.cc        safe async argv-based command execution + timeout/result

src/flamesnap.h
src/flamesnap.cc           layout definitions, exact region math, snap state/application
src/flamesnappopup.h
src/flamesnappopup.cc      visual hover chooser

src/flameoverview.h
src/flameoverview.cc       Activities/Overview overlay

src/flamelauncher.h
src/flamelauncher.cc       searchable launcher UI + app model/cache

src/flameaudio.h
src/flameaudio.cc          wpctl backend + panel widget/popup
src/flamewifi.h
src/flamewifi.cc           nmcli backend + panel widget/popup
src/flamemedia.h
src/flamemedia.cc          playerctl/MPRIS backend + panel widget

src/flamedesktop.cc        optional independent desktop process
src/flamedesktopmodel.h
src/flamedesktopmodel.cc   desktop entries/icon-grid model if needed
```

Avoid creating a generic framework before these features demand one. The only shared abstraction justified up front is safe asynchronous command execution because audio/Wi-Fi/media all require it.

---

## 11. Snap-layout design — pre-solved implementation constraints

This is the feature with the highest geometry/state risk.

### 11.1 Existing reusable actions

`src/wmaction.h` already defines:

- left/right/top/bottom half.
- top-left/top-right/bottom-left/bottom-right quarter.
- center.

`src/wmapp.cc:634-655` exposes them in a textual `TileMenu`.

`src/wmframe.cc:1500-1578` implements geometry in `YFrameWindow::wmTile()`.

Therefore FlameWM must **generalize this path** instead of creating an unrelated tiling manager.

### 11.2 Existing bug to fix before building layouts

Current center placement (`src/wmframe.cc:1564-1568`):

```cpp
w = (Mx - mx) / 2;
h = (My - my) / 2;
x = mx + (mx + Mx - w) / 2;
y = my + (my + My - h) / 2;
```

The `x`/`y` equations add the monitor/work-area origin twice. They are only correct when `mx == 0` and `my == 0`. On non-zero or negative monitor origins this can misplace the centered tile.

Correct conceptual equation:

```text
x = workLeft + (workWidth  - targetWidth)  / 2
y = workTop  + (workHeight - targetHeight) / 2
```

This should be repaired and regression-tested before the new layout engine is used.

### 11.3 Exact partition math

Never derive each region by independently rounding a percentage. That creates pixel gaps/overlaps at odd widths.

For total work-area width `W`, denominator `D`, start column `i`, span `s`:

```text
left  = workLeft + floor(i       * W / D)
right = workLeft + floor((i + s) * W / D)
width = right - left
```

Use the same edge-based rule vertically. Adjacent regions then share the exact same boundary.

Use integer arithmetic wide enough to avoid overflow (`int64_t` for multiplication before division), even though normal X11 desktop dimensions are much smaller.

### 11.4 Work-area authority

Always call the existing manager work-area path using the target frame’s screen:

```text
manager->getWorkArea(frame, ..., frame->getScreen())
```

Do not use global root dimensions. This preserves:

- panel struts,
- multi-monitor origin,
- XRandR geometry,
- top/bottom taskbar placement.

### 11.5 Fixed/minimum-size clients

Existing `wmTile()` moves only when `canSize()` is false. FlameWM must preserve that principle.

For resizable clients, ICCCM size hints can still make the requested exact region impossible. Policy:

1. Compute requested outer region.
2. Constrain through the existing size-hint path.
3. Anchor the resulting constrained window inside the selected region according to the region edge (left/right/top/bottom/center).
4. Never violate client minimum/fixed sizing just to make the visual grid exact.

Tests must include fixed-size and minimum-size windows.

### 11.6 Snap state vs. maximize state

Arbitrary snap regions are not EWMH maximize states. Do not set `_NET_WM_STATE_MAXIMIZED_*` for a 1/3 or quarter region.

Introduce minimal internal snap metadata only if needed for restore/reflow:

```text
unsnappedOuterGeometry
optional activeLayoutId
optional activeRegionId
```

Rules:

- First snap captures the unsnapped geometry.
- Re-snapping changes region without overwriting the original unsnapped geometry.
- Explicit maximize exits snap state and follows existing maximize bookkeeping.
- User drag/resize exits snap state once the move/resize is accepted.
- Restore from a snapped state restores the captured unsnapped geometry.
- Fullscreen always wins and hides/closes snap popup.

Do not persist arbitrary snap state into EWMH properties unless a future interop requirement proves it necessary.

### 11.7 XRandR changes

If a snapped frame’s work area changes because monitor geometry or panel struts change, reapply its region against the new work area **only if the frame is still marked snapped**. Normal windows must not be unexpectedly rearranged.

### 11.8 Hover popup state machine

The maximize button already inherits `YButton`, whose crossing state tracks pointer enter/leave (`src/ybutton.cc:316+`). `YFrameButton` can add a crossing hook for the `Maxi` kind.

Use this state machine:

```text
IDLE
  Enter maximize -> ARM_TIMER
ARM_TIMER
  leave button before delay -> IDLE
  delay fires and frame eligible -> POPUP_OPEN
POPUP_OPEN
  pointer enters popup -> keep open
  pointer returns button -> keep open
  pointer leaves union(button,popup) -> short CLOSE_GRACE timer
  click region -> apply snap -> close -> IDLE
  maximize click -> normal maximize -> close -> IDLE
  Esc / frame destroyed / workspace switch / fullscreen -> close -> IDLE
```

The popup must be owned/associated with the frame so destruction cannot leave a dangling frame pointer.

---

## 12. Workspace management — pre-solved semantics

### 12.1 Existing dynamic count

`YWindowManager::extendWorkspaces(int target)` and `lessenWorkspaces(int target)` already update desktop count/viewports/work area, taskbar buttons, window list and move menus (`src/wmmgr.cc:2854-2921`).

`Workspaces` caps the model at `NewMaxWorkspaces = 1000` and `lessenWorkspaces` hard-refuses target `< 1` (`src/workspaces.h`).

### 12.2 Why right-click removal needs new code

Upstream `lessenWorkspaces(target)` only truncates workspaces from the **end**. The requested interaction is “remove the desktop I right-clicked.” Calling `lessenWorkspaces(count - 1)` after right-clicking desktop 2 of 4 would incorrectly remove desktop 4.

Implement a first-class manager operation:

```cpp
bool YWindowManager::removeWorkspace(int index);
```

Also add an indexed remove to `Workspaces` using its existing `YObjectArray::remove(index)` support.

### 12.3 Required indexed-removal behavior

Given old count `N`, remove index `R`:

1. Reject if `N <= 1`.
2. Reject if `R` is outside `[0, N-1]`.
3. Choose a deterministic surviving destination:
   - if a workspace exists to the right, use old `R + 1`;
   - otherwise use old `R - 1`.
4. If active workspace is `R`, switch focus/visibility to that surviving destination before deleting model data.
5. Move windows on removed workspace to the destination.
6. Reindex all windows on workspace `> R` to `old - 1`.
7. Remove the workspace name/model entry at `R`.
8. Reindex `fActiveWorkspace`, `fLastWorkspace`, and any stored focus pointer/index semantics that refer to indices above `R`.
9. Publish updated:
   - `_NET_NUMBER_OF_DESKTOPS`
   - `_NET_CURRENT_DESKTOP`
   - `_NET_DESKTOP_VIEWPORT`
   - `_NET_DESKTOP_NAMES`
   - `_NET_WORKAREA`
10. Refresh taskbar workspace buttons, window list, move menu and previews.
11. Focus a valid last window on the resulting active workspace.

This operation needs dedicated tests because a mistake produces invisible windows or incorrect EWMH indices.

### 12.4 Right-click UI

Current right click on `WorkspaceButton` calls `manager->popupWindowListMenu(...)` (`src/aworkspaces.cc:91-93`). Replace with a dedicated small menu for FlameWM:

```text
Add Desktop
Remove This Desktop      [disabled when count == 1]
Rename Desktop
------------------
Windows on This Desktop  [submenu or existing window-list behavior]
```

Keep middle-click window-list behavior so no existing functionality disappears.

---

## 13. Activities / Overview — pre-solved architecture

### 13.1 Reuse live preview primitives, not Alt+Tab behavior

`Preview` creates an XDamage object for the client and an XRender picture for the frame; `SwitchPreview` maintains preview arrays and damage handling (`src/preview.cc`, `src/preview.h`).

Do **not** turn `SwitchPreview` itself into Overview. Alt+Tab has modifier-key lifecycle, ordering, selection and accept/cancel semantics that should remain stable.

Instead factor or reuse a lower-level thumbnail primitive:

```text
WindowThumbnail
- YFrameClient*
- YFrameWindow*
- XDamage handle
- render source picture
- target rectangle
- dirty flag
```

Then:

- `SwitchPreview` keeps its current switcher-specific controller.
- `FlameOverview` uses the same thumbnail renderer with overview-specific layout/input.

### 13.2 Damage/update throttling

Never repaint the whole overview for every damage event. Mark only the affected thumbnail dirty and coalesce redraws with a short timer (existing preview already uses timer-based update concepts).

When Overview is closed, destroy/unsubscribe its XDamage resources. There must be zero continual Overview thumbnail work when the feature is not visible.

### 13.3 Window eligibility

Use the same established skip semantics where appropriate:

- hidden/internal windows excluded,
- skip-pager/ignore-preview options respected,
- taskbar/dock/desktop windows excluded,
- normal visible/minimized application frames included according to chosen UX.

Do not invent a second definition of “real application window” if existing switcher/pager filtering can be shared.

### 13.4 Multi-monitor behavior

For v1, Overview opens on the **active pointer/focused-frame monitor** and can show all eligible windows with monitor badges/positioning, or one overlay per monitor if the implementation remains straightforward. Do not accidentally size a single overlay from root geometry and assume monitor origin zero.

---

## 14. Flame Launcher — pre-solved architecture

### 14.1 Why the classic `StartMenu` is not enough

`StartMenu` derives from the menu-file system and ultimately `YMenu`. It is designed for cascading menu items, not an always-focused search box with dynamic filtering.

Create a dedicated `YPopupWindow`-based launcher using existing:

- `YInputLine`
- `YListBox` or custom app grid
- `YIcon`
- `YScrollView`
- existing program launch service (`YSMListener`/`runProgram`)

### 14.2 Application data

Upstream `fdomenu.cc` already parses `.desktop` fields such as `Name`, localized name, `GenericName`, `Icon`, `Categories`, `Exec`, `Terminal`, and `NoDisplay`.

For FlameWM, do not make every launcher opening spawn and parse a full textual menu. Build/cache an application model at first use, then invalidate it when relevant application directories change or on explicit reload.

A safe implementation can extract the desktop-file parsing model into reusable source rather than duplicating spec parsing.

### 14.3 Desktop `Exec` safety

Do not launch search results by concatenating user-visible names into shell strings. Preserve `.desktop` field-code processing rules and existing argument-vector launching semantics. Search text must never become command text.

### 14.4 Performance

- Lazy-create launcher on first open.
- Cache normalized lowercase search keys.
- Do not load huge icons for every app in advance; use existing icon caching and requested display size.
- Filtering should be in-memory and synchronous; filesystem rescans should not run on every keystroke.

---

## 15. Panel architecture and overflow policy

### 15.1 Current taskbar debt

`TaskBar::initApplets()` directly allocates all existing components (`src/wmtaskbar.cc:254-415`). `TaskBar::updateLayout()` builds a hard-coded `LayoutInfo` list (`:446+`). This can accommodate a few more controls, but adding business logic there would make the known upstream coupling worse.

### 15.2 Required separation

Each Flame panel control owns:

- rendering/input state,
- its popup,
- a backend/service reference,
- no global workspace/window-management authority.

`TaskBar` only owns widget lifetime and layout.

### 15.3 Three logical lanes

Refactor only the layout description enough to express:

```text
LEFT    start, activities, workspaces, optional toolbar
CENTER  task buttons (flexible)
RIGHT   media, audio, wifi, system tray, clock
```

Do not rewrite all existing taskbar behavior at the same time.

### 15.4 Narrow-screen overflow priority

Prevent overlap deterministically:

1. Hide media track text but retain media icon/control.
2. Collapse optional toolbar items if configured.
3. Let task buttons shrink using existing behavior.
4. Let workspace pane use its existing constrained/scroll behavior.
5. Never hide audio/Wi-Fi status, tray, or clock without an explicit setting.

Every layout pass must be deterministic and independent of previous layout geometry.

---

## 16. Audio control

Official WirePlumber provides `wpctl`, including:

```text
wpctl get-volume @DEFAULT_SINK@
wpctl set-volume @DEFAULT_SINK@ 50%
wpctl set-mute @DEFAULT_SINK@ toggle
wpctl list audio sinks
wpctl set-default <ID>
```

### v1 design

- Detect `wpctl` at runtime once and cache availability.
- If unavailable, hide/disable native audio applet rather than failing WM startup.
- Read current default-sink volume on popup open and after FlameWM changes it.
- Volume mouse wheel can issue bounded increments.
- Clamp FlameWM’s default UX to 0–100% unless an explicit `AllowVolumeOver100` option is enabled.
- Backend process I/O is asynchronous.
- No periodic process spawning every second.

Later, an event-driven PipeWire library backend can be added behind the same widget interface if needed, but should not be the first implementation.

---

## 17. Wi-Fi control

`NetStatusControl` is **not** a Wi-Fi selector. It is a network status/throughput monitor. New functionality is required.

Official NetworkManager `nmcli` supports listing APs, device state, rescans and connecting networks.

### v1 design

Use machine-oriented fields and terse output rather than parsing pretty terminal tables. Commands must use argument vectors, not shell interpolation.

Required UX:

- current SSID/signal/state,
- list nearby networks on popup open,
- refresh/rescan action,
- activate an existing saved connection,
- disconnect/toggle Wi-Fi if supported by chosen commands,
- show a clear “NetworkManager unavailable” state when backend is missing.

### Password/security decision

Do **not** put a newly typed Wi-Fi password into a shell command string. Do not log it. Avoid passing secrets on argv if possible because process arguments may be observable.

For v1, prefer NetworkManager’s existing secret-agent flow/saved connection profiles. If a network requires a new secret and no secret agent can satisfy it, surface a controlled error/open the configured network editor rather than inventing insecure secret handling. A native FlameWM secret-agent implementation is a separate security-sensitive feature.

---

## 18. Media control

MPRIS 2.2 defines the standard `org.mpris.MediaPlayer2` and `.Player` D-Bus interfaces, including play/pause, next, previous, playback state and metadata.

For minimum compile complexity, v1 can use runtime `playerctl`, which is a command-line controller for MPRIS players.

Required behavior:

- Hide media widget if no supported player exists.
- Show icon + truncated title/artist when a player is active.
- Previous / play-pause / next.
- On player disappearance, cleanly collapse.
- Never block WM event loop waiting for a D-Bus/player timeout.

The backend should be replaceable with direct MPRIS later without changing panel UI.

---

## 19. Date/time

Existing clock support is sufficient. `src/default.h` exposes `TimeFormat`, `TimeFormatAlt`, and `DateFormat`; `src/aclock.cc` supports changing display formats.

FlameWM should ship a default time format that visibly includes both date and time, e.g. a compact single-line or two-line representation depending final panel height. No second clock implementation is needed.

---

## 20. Desktop process

### Why separate it

A real desktop surface adds:

- directory watching,
- file icons,
- launcher `.desktop` files,
- rename/delete/open actions,
- drag/drop,
- grid persistence,
- MIME/application launching.

None of that belongs in the core frame/focus/stacking event path.

### `flamewm-desktop` contract

- Launched by `icewm-session`/future `flamewm-session` only when enabled.
- Registers itself as desktop window type.
- Uses existing icon/image/font toolkit where feasible.
- Maintains icon positions in a small config file under FlameWM user config.
- Keeps file operations off the WM process.
- If it crashes, background and WM remain usable.

This mirrors the good upstream decision to keep `icewmbg` separate.

---

## 21. Branding and fork identity

Do **not** begin by mass-renaming every `IceWM` symbol, config key and atom. That creates a huge untestable diff before feature work.

Recommended sequence:

1. Create the Git fork and preserve upstream history.
2. Add FlameWM theme/artwork/visible branding.
3. Add features while binaries/config paths remain compatible during development.
4. After behavior is stable, introduce release branding deliberately:
   - package/display name,
   - optional `flamewm` binary/session aliases,
   - XDG desktop files,
   - config-directory migration/compatibility policy.
5. Preserve EWMH standard atoms unchanged and preserve compatibility for IceWM-specific behavior where useful.

This keeps `git bisect` and upstream cherry-picking practical.

---

## 22. Licensing constraints

The uploaded `COPYING` is **GNU Library General Public License, Version 2, June 1991**. README describes the release as LGPL.

Fork rules for engineering purposes:

- Preserve copyright/license notices.
- Keep `COPYING` in distributions.
- Track source changes cleanly.
- Do not assume KDE/Breeze artwork can be copied merely because both projects are open source; verify each external asset’s license and attribution compatibility before importing it.
- Prefer original FlameWM Breeze-inspired SVG/PNG artwork to avoid unnecessary asset-license coupling.

This section is engineering guidance, not legal advice.

---

## 23. Performance and memory budget rules

FlameWM exists because the requested product prioritizes low memory. Feature implementation must obey these rules:

1. **No Qt/GTK shell dependency** for panel/launcher/overview.
2. **No embedded browser/webview**.
3. No permanent high-frequency polling for Wi-Fi/audio/media.
4. No Overview thumbnail resources while Overview is closed.
5. Lazy-create launcher/overview/popups on first use where practical.
6. Cache icons at requested sizes; do not preload all full-resolution art.
7. Do not add background worker threads for ordinary X/UI work. Use the event loop/poll integration.
8. External command backends must have timeouts and bounded output buffers.
9. Hidden/missing backend means hidden/disabled applet, not retry spam.
10. Keep `flamewm-desktop` optional and separate.
11. Measure RSS/PSS before and after each major subsystem. Do not accept “lightweight” by assumption.

### Suggested acceptance budgets

These are project targets, not claims about current IceWM measurements:

- Idle WM-only feature overhead versus the same build with Flame features disabled: **< 10 MiB RSS target**, lower preferred.
- No continuously growing memory after 1,000 launcher/overview/snap popup open-close cycles.
- No child process accumulation/zombies after repeated system-control actions.
- No idle command execution loop faster than a human-visible status requirement.

Record real baseline/after numbers on target hardware before declaring success.

---

## 24. Testing strategy

### 24.1 Current upstream test coverage is insufficient for our new UI/state work

CMake currently registers only:

- `strtest`
- `testpointer`
- `testarray`

Other test sources exist, but are not registered by the current CMake `BUILD_TESTING` block. Therefore “ctest passes” alone will not verify snap/workspace/panel behavior.

### 24.2 Add pure deterministic tests first

Create tests for logic that does not need a live X server:

#### Snap geometry

- even work area.
- odd width/height.
- negative monitor origin.
- non-zero positive origin.
- 1/2, 1/3, 2/3, quarters.
- every layout’s regions remain inside work area.
- adjacent regions share exact edges.
- complete layouts cover expected area without gaps/overlap.
- center regression for non-zero origin.

#### Workspace index remapping

Model old workspace assignments and assert exact new assignments after removing first/middle/last workspace.

#### Launcher filtering

Normalized matching, localized names, hidden/no-display applications.

#### Command parsing

Machine-readable `wpctl`, `nmcli`, `playerctl` fixture outputs; malformed/empty/timeout cases.

### 24.3 X11 integration tests

Add an Xvfb/Xephyr CI job for:

- create several test windows,
- snap regions and assert frame geometry,
- panel strut-aware work area,
- multi-monitor layout where XRandR test environment supports it,
- arbitrary workspace removal and `_NET_*` property values,
- maximize hover popup lifecycle,
- overview open/close and activate window,
- no crash when target frame is destroyed while popup is open.

### 24.4 Sanitizers/debug

Use project debug builds plus ASan/UBSan in a dedicated CI configuration where possible. The source itself includes commented sanitizer flags in `src/CMakeLists.txt:104-105`, indicating this is compatible with the upstream development style.

---

## 25. Failure modes to prevent before implementation

| Failure | Root cause | Preventive design |
|---|---|---|
| Snap correct on primary, wrong on secondary monitor | origin-zero assumptions / existing center formula bug | region math based on `(left, top, width, height)` + regression tests |
| 1px gaps/overlaps in thirds | independently rounded widths | edge-based integer partition formula |
| Snap popup vanishes while pointer travels into it | naive LeaveNotify handling | button+popup hover union + grace timer |
| Maximize click stops working | hover popup steals button action/grab | maximize remains normal click; popup only delayed hover |
| Window cannot restore after repeated snap | normal geometry overwritten on every snap | explicit first-snap unsnapped geometry |
| Fixed-size dialogs explode or move off region | forcing requested width/height | respect `canSize` + existing size hints |
| Removing workspace 2 removes workspace 4 | reuse of tail-only `lessenWorkspaces` | indexed `removeWorkspace(index)` |
| Windows disappear after middle workspace removal | incomplete reindex/EWMH update | one manager transaction + mapping tests |
| Overview permanently increases CPU | damage tracking remains active hidden | allocate/subscribe only while open |
| Overview crashes after window close | raw thumbnail pointer lifetime | destruction notification removes thumbnail before use |
| Panel freezes on Wi-Fi scan | synchronous `nmcli` in X event handler | async command service + timeout |
| Shell injection from SSID/title | `/bin/sh -c` string construction | `execvp` argv only; never interpolate into shell |
| Wi-Fi secret appears in logs/process args | naive password CLI invocation | use saved/NM secret-agent flow; no logging |
| Media applet wakes CPU constantly | fixed rapid polling | event/on-demand backend; collapse when absent |
| Taskbar becomes unmaintainable | business logic added to `TaskBar` | widget/backend classes; `TaskBar` only lifecycle/layout |
| Breeze blur unexpectedly absent | IceWM mistaken for compositor | opaque native theme; external compositor optional |
| Huge fork diff blocks upstream updates | immediate mass rename/refactor | feature-first bounded files; branding migration later |
| “Desktop icons” crash WM | filesystem/DND code in WM process | separate `flamewm-desktop` process |

---

## 26. Implementation order

### Phase 0 — Baseline and fork hygiene

1. Create fork preserving upstream history.
2. Install full upstream dependencies.
3. Green CMake build + CTest.
4. Green Autoconf build.
5. Record baseline RSS/PSS and startup time.
6. Add CI before feature changes.
7. Fix/test the existing center-tile non-zero-origin bug.

**Gate:** no Flame feature work until baseline builds are green on a real development environment.

### Phase 1 — Flame visual foundation

1. New original Breeze-inspired theme directory.
2. Title button PNG assets and hover/pressed states.
3. Panel metrics/colors/fonts.
4. Default date/time format.
5. Do not add compositor-only effects.

**Gate:** full existing WM behavior works under Flame theme.

### Phase 2 — Workspace controls

1. Implement indexed workspace removal transaction.
2. Add `Add Desktop`/`Remove This Desktop` context menu.
3. Preserve rename/window-list functions.
4. Add model + X11 integration tests.

**Gate:** first/middle/last removal correct; one-workspace minimum enforced.

### Phase 3 — Snap engine + layout chooser

1. Extract exact region geometry.
2. Route existing half/quarter actions through it.
3. Add new layout catalog.
4. Add hover popup.
5. Add snap restore/reflow state only as required.
6. Multi-monitor/XRandR tests.

**Gate:** no regression in maximize, restore, move/resize, fullscreen, fixed-size windows.

### Phase 4 — Activities/Overview

1. Factor thumbnail rendering primitive.
2. Build overview overlay.
3. Workspace/window interaction.
4. Damage coalescing and lifecycle tests.

**Gate:** zero idle Overview CPU/resource footprint while closed.

### Phase 5 — Flame Launcher

1. Reusable application model from `.desktop` data.
2. Search popup UI.
3. App launching and session actions.
4. Cache/invalidation.

**Gate:** search opens instantly after warm cache and does no filesystem scan per keypress.

### Phase 6 — Panel system controls

1. `FlameCommandRunner` with timeout/output bound/argv execution.
2. Audio backend/widget.
3. Wi-Fi backend/widget.
4. Media backend/widget.
5. Refine three-lane taskbar layout/overflow.

**Gate:** no X-event-loop blocking; absent tools degrade cleanly.

### Phase 7 — Optional desktop grid

1. Separate process.
2. Desktop window semantics.
3. Icon grid and persistence.
4. File/launcher interactions.

**Gate:** kill/restart desktop process without affecting WM.

### Phase 8 — Branding/release migration

Only after functional gates are green:

- package/binary/session naming,
- docs/man pages,
- configuration migration/aliases,
- distribution packaging,
- complete license/attribution review.

---

## 27. Source files most likely to change

### Existing files

| File | Expected change |
|---|---|
| `src/CMakeLists.txt` | Register new Flame sources/tests/executable |
| `src/default.h` / `src/themable.h` | New preferences/theme options where appropriate |
| `src/wmapp.*` | Construct/toggle launcher/overview; wire global actions |
| `src/wmaction.h` | New Activities/launcher/snap actions only if needed |
| `src/wmmgr.*` | Indexed workspace removal; work-area/reflow hooks |
| `src/workspaces.h` | Indexed workspace model removal |
| `src/aworkspaces.*` | Workspace context menu |
| `src/wmframe.*` | Route snap geometry/state; fix center bug |
| `src/wmbutton.*` | Maximize-hover entry point |
| `src/wmtitle.*` | Only if hover timer ownership belongs here |
| `src/wmtaskbar.*` | Add bounded widgets + three-lane placement |
| `src/preview.*` | Factor thumbnail primitive without changing switcher semantics |
| `src/wmprog.*` / `src/fdomenu.cc` | Reuse/extract app model, not wholesale rewrite |
| `src/icesm.cc` | Optional desktop process launch later |
| `lib/themes/FlameBreeze/` | New original theme resources |
| `lib/keys.in` | Defaults for Activities/layout actions if desired |
| `man/*` | New options/behavior documentation |
| `po/*` | New translatable strings after UI stabilizes |

### Files that should **not** be broadly refactored for these features

- `src/wmclient.cc`
- core linked-list/layer machinery in `wmmgr.cc`
- generic `YWindow` event dispatch
- EWMH/ICCCM protocol code unrelated to workspace index updates
- `icesh.cc`

Touch these only when a traced requirement proves it necessary.

---

## 28. Definition of done for the first FlameWM release

A first release implementing the requested product is done only when all are true:

### Build/reliability

- CMake build green.
- registered CTest suite green.
- Autoconf build green.
- new Xvfb/Xephyr integration suite green.
- no sanitizer finding in exercised Flame paths.

### Panel

- Start, Activities, workspace pager, task buttons, media, audio, Wi-Fi, tray, visible date/time all lay out without overlap.
- Missing optional runtime tools do not break startup.

### Workspaces

- Add works up to configured/model limit.
- Remove right-clicked workspace works for first/middle/last.
- Last remaining workspace cannot be removed.
- Windows and EWMH indices remain correct.

### Snap layouts

- Hover maximize popup stable.
- Existing maximize click unchanged.
- Layouts correct on odd/even dimensions and non-zero/negative monitor origins.
- Panel struts respected.
- fixed/min-size windows handled safely.
- restore behavior deterministic.

### Activities

- Overview shows correct eligible windows/workspaces.
- thumbnails update while open.
- click activation works.
- resources are released/idle while closed.

### Launcher

- searchable applications.
- keyboard navigation.
- safe launch.
- no per-keystroke disk scan.

### Performance

- baseline and final PSS/RSS/startup metrics recorded.
- no unbounded process spawning or polling.
- no leak across repeated popup/overview/launcher cycles.

### Visual

- original Flame Breeze-inspired theme is default for FlameWM builds/releases.
- no visual feature silently assumes a compositor.

---

## 29. External authoritative references used for design

These supplement, but do not override, the uploaded source snapshot.

- IceWM official repository / 4.1.0 release: `https://github.com/ice-wm/icewm`
- IceWM manual: `https://ice-wm.org/man/icewm.html`
- IceWM preferences manual: `https://ice-wm.org/man/icewm-preferences.html`
- WirePlumber `wpctl`: `https://pipewire.pages.freedesktop.org/wireplumber/man/wpctl.html`
- NetworkManager `nmcli` examples/manual: `https://www.networkmanager.dev/docs/api/latest/nmcli-examples.html`
- MPRIS 2.2: `https://specifications.freedesktop.org/mpris/latest/`
- Playerctl MPRIS controller: `https://github.com/altdesktop/playerctl`

---

## 30. Future-session source-reading protocol

Before implementing any FlameWM request based on this report:

1. Read the exact current source files named in the relevant section.
2. Verify the uploaded/repository version still matches this report or identify drift.
3. Trace existing ownership/state transitions before adding a callback.
4. Prefer an existing IceWM action/data path over a second state authority.
5. For window geometry, prove monitor origin/work-area/size-hint behavior with tests first.
6. For workspace changes, treat EWMH properties + frame indices + focus indices as one transaction.
7. For panel backend commands, never block the X event loop and never construct shell commands from external/user strings.
8. Re-run the exact failed test/build gate after every repair; do not package with a red gate.
9. Measure memory/CPU rather than assuming an implementation is lightweight.
10. Update this report when architecture or source ownership materially changes.

---

# Appendix A — Full repository tree

The following is the complete extracted tree of the uploaded `icewm-master.zip` snapshot used for this report.

```text
icewm-master/
├── .github/
│   ├── workflows/
│   │   └── cmake.yml
│   ├── dependabot.yml
│   └── FUNDING.yml
├── contrib/
│   ├── Additional_Categories.csv
│   ├── config.cmd
│   ├── conv_cat.py
│   ├── fd-leak-patch.txt
│   ├── icewm-menu-xrandr
│   ├── icewm_cpp_style.xml
│   ├── Main_Categories.csv
│   ├── sysdep.os2
│   ├── tbf-gnomevfs.diff
│   ├── tbf-movesize-fx.diff
│   ├── tbf-wm-session.diff
│   └── xrandr_menu
├── doc/
│   ├── CMakeLists.txt
│   ├── icewm.adoc
│   ├── icewm.md
│   └── Makefile.am
├── lib/
│   ├── icewm/
│   │   ├── close.xpm
│   │   ├── maximize.xpm
│   │   ├── menu.xpm
│   │   └── restore.xpm
│   ├── icons/
│   │   ├── about_16x16.xpm
│   │   ├── about_32x32.xpm
│   │   ├── app_16x16.xpm
│   │   ├── app_32x32.xpm
│   │   ├── bomb_16x16.xpm
│   │   ├── bomb_32x32.xpm
│   │   ├── cancel-logout_16x16.xpm
│   │   ├── emacs_16x16.xpm
│   │   ├── emacs_32x32.xpm
│   │   ├── file_16x16.xpm
│   │   ├── file_32x32.xpm
│   │   ├── focus_16x16.png
│   │   ├── focus_16x16.xpm
│   │   ├── focus_32x32.png
│   │   ├── focus_32x32.xpm
│   │   ├── folder_16x16.xpm
│   │   ├── folder_32x32.xpm
│   │   ├── gimp_16x16.xpm
│   │   ├── gimp_32x32.xpm
│   │   ├── gnome_16x16.xpm
│   │   ├── help_16x16.xpm
│   │   ├── help_32x32.xpm
│   │   ├── hibernate_16x16.xpm
│   │   ├── hibernate_32x32.xpm
│   │   ├── icewm_16x16.png
│   │   ├── icewm_32x32.png
│   │   ├── java_16x16.xpm
│   │   ├── java_32x32.xpm
│   │   ├── kde_16x16.xpm
│   │   ├── key_16x16.png
│   │   ├── key_32x32.png
│   │   ├── lock_16x16.xpm
│   │   ├── lock_32x32.xpm
│   │   ├── lock_48x48.xpm
│   │   ├── logout_16x16.xpm
│   │   ├── logout_32x32.xpm
│   │   ├── pdf_16x16.xpm
│   │   ├── pdf_32x32.xpm
│   │   ├── pref_16x16.png
│   │   ├── pref_32x32.png
│   │   ├── programs_16x16.xpm
│   │   ├── programs_32x32.xpm
│   │   ├── reboot_16x16.xpm
│   │   ├── reboot_32x32.xpm
│   │   ├── restart_16x16.xpm
│   │   ├── restart_32x32.xpm
│   │   ├── run_16x16.xpm
│   │   ├── run_32x32.xpm
│   │   ├── save_16x16.png
│   │   ├── save_32x32.png
│   │   ├── setscreen12_32x32.png
│   │   ├── setscreen12x_32x32.png
│   │   ├── setscreen1_32x32.png
│   │   ├── setscreen21_32x32.png
│   │   ├── setscreen21x_32x32.png
│   │   ├── setscreen2_32x32.png
│   │   ├── settings_16x16.xpm
│   │   ├── settings_32x32.xpm
│   │   ├── shutdown_16x16.xpm
│   │   ├── shutdown_32x32.xpm
│   │   ├── suspend_16x16.xpm
│   │   ├── suspend_32x32.xpm
│   │   ├── themes_16x16.xpm
│   │   ├── themes_32x32.xpm
│   │   ├── tilebottom_48x48.png
│   │   ├── tilebottomleft_48x48.png
│   │   ├── tilebottomright_48x48.png
│   │   ├── tilecenter_48x48.png
│   │   ├── tileleft_48x48.png
│   │   ├── tileright_48x48.png
│   │   ├── tiletop_48x48.png
│   │   ├── tiletopleft_48x48.png
│   │   ├── tiletopright_48x48.png
│   │   ├── vim_16x16.xpm
│   │   ├── vim_32x32.xpm
│   │   ├── vim_48x48.xpm
│   │   ├── windows_16x16.xpm
│   │   ├── windows_32x32.xpm
│   │   ├── xload_16x16.xpm
│   │   ├── xload_32x32.xpm
│   │   ├── xterm_16x16.xpm
│   │   ├── xterm_32x32.xpm
│   │   ├── xv_16x16.xpm
│   │   └── xv_32x32.xpm
│   ├── ledclock/
│   │   ├── a.xpm
│   │   ├── colon.xpm
│   │   ├── dot.xpm
│   │   ├── m.xpm
│   │   ├── n0.xpm
│   │   ├── n1.xpm
│   │   ├── n2.xpm
│   │   ├── n3.xpm
│   │   ├── n4.xpm
│   │   ├── n5.xpm
│   │   ├── n6.xpm
│   │   ├── n7.xpm
│   │   ├── n8.xpm
│   │   ├── n9.xpm
│   │   ├── p.xpm
│   │   ├── percent.xpm
│   │   ├── slash.xpm
│   │   └── space.xpm
│   ├── mailbox/
│   │   ├── errmail.xpm
│   │   ├── mail.xpm
│   │   ├── newmail.xpm
│   │   ├── nomail.xpm
│   │   └── unreadmail.xpm
│   ├── taskbar/
│   │   ├── collapse.xpm
│   │   ├── debian.xpm
│   │   ├── desktop.xpm
│   │   ├── expand.xpm
│   │   ├── icewm.xpm
│   │   ├── linux.xpm
│   │   ├── linux1.xpm
│   │   ├── linux2.xpm
│   │   ├── linux20.xpm
│   │   ├── start.xpm
│   │   ├── windows.xpm
│   │   └── xfreeos2.xpm
│   ├── themes/
│   │   ├── CrystalBlue/
│   │   │   ├── ledclock/
│   │   │   │   ├── a.xpm
│   │   │   │   ├── colon.xpm
│   │   │   │   ├── dot.xpm
│   │   │   │   ├── m.xpm
│   │   │   │   ├── n0.xpm
│   │   │   │   ├── n1.xpm
│   │   │   │   ├── n2.xpm
│   │   │   │   ├── n3.xpm
│   │   │   │   ├── n4.xpm
│   │   │   │   ├── n5.xpm
│   │   │   │   ├── n6.xpm
│   │   │   │   ├── n7.xpm
│   │   │   │   ├── n8.xpm
│   │   │   │   ├── n9.xpm
│   │   │   │   ├── p.xpm
│   │   │   │   ├── slash.xpm
│   │   │   │   └── space.xpm
│   │   │   ├── taskbar/
│   │   │   │   ├── collapse.xpm
│   │   │   │   ├── desktop.xpm
│   │   │   │   ├── expand.xpm
│   │   │   │   ├── icewm.xpm
│   │   │   │   ├── taskbarbg.xpm
│   │   │   │   ├── taskbuttonactive.xpm
│   │   │   │   ├── taskbuttonbg.xpm
│   │   │   │   ├── taskbuttonminimized.xpm
│   │   │   │   ├── toolbuttonbg.xpm
│   │   │   │   ├── windows.xpm
│   │   │   │   ├── workspacebuttonactive.xpm
│   │   │   │   └── workspacebuttonbg.xpm
│   │   │   ├── buttonA.xpm
│   │   │   ├── buttonI.xpm
│   │   │   ├── close.xpm
│   │   │   ├── closeA.xpm
│   │   │   ├── closeO.xpm
│   │   │   ├── default.theme
│   │   │   ├── dframeAB.xpm
│   │   │   ├── dframeABL.xpm
│   │   │   ├── dframeABR.xpm
│   │   │   ├── dframeAL.xpm
│   │   │   ├── dframeAR.xpm
│   │   │   ├── dframeAT.xpm
│   │   │   ├── dframeATL.xpm
│   │   │   ├── dframeATR.xpm
│   │   │   ├── dframeIB.xpm
│   │   │   ├── dframeIBL.xpm
│   │   │   ├── dframeIBR.xpm
│   │   │   ├── dframeIL.xpm
│   │   │   ├── dframeIR.xpm
│   │   │   ├── dframeIT.xpm
│   │   │   ├── dframeITL.xpm
│   │   │   ├── dframeITR.xpm
│   │   │   ├── frameAB.xpm
│   │   │   ├── frameABL.xpm
│   │   │   ├── frameABR.xpm
│   │   │   ├── frameAL.xpm
│   │   │   ├── frameAR.xpm
│   │   │   ├── frameAT.xpm
│   │   │   ├── frameATL.xpm
│   │   │   ├── frameATR.xpm
│   │   │   ├── frameIB.xpm
│   │   │   ├── frameIBL.xpm
│   │   │   ├── frameIBR.xpm
│   │   │   ├── frameIL.xpm
│   │   │   ├── frameIR.xpm
│   │   │   ├── frameIT.xpm
│   │   │   ├── frameITL.xpm
│   │   │   ├── frameITR.xpm
│   │   │   ├── maximize.xpm
│   │   │   ├── maximizeA.xpm
│   │   │   ├── maximizeO.xpm
│   │   │   ├── menubg.xpm
│   │   │   ├── menuButtonA.xpm
│   │   │   ├── menuButtonI.xpm
│   │   │   ├── menusel.xpm
│   │   │   ├── minimize.xpm
│   │   │   ├── minimizeA.xpm
│   │   │   ├── minimizeO.xpm
│   │   │   ├── restore.xpm
│   │   │   ├── restoreA.xpm
│   │   │   ├── restoreO.xpm
│   │   │   ├── rolldown.xpm
│   │   │   ├── rolldownA.xpm
│   │   │   ├── rolldownO.xpm
│   │   │   ├── rollup.xpm
│   │   │   ├── rollupA.xpm
│   │   │   ├── rollupO.xpm
│   │   │   ├── titleAB.xpm
│   │   │   ├── titleAJ.xpm
│   │   │   ├── titleAM.xpm
│   │   │   ├── titleAP.xpm
│   │   │   ├── titleAQ.xpm
│   │   │   ├── titleAS.xpm
│   │   │   ├── titleAT.xpm
│   │   │   ├── titleIB.xpm
│   │   │   ├── titleIJ.xpm
│   │   │   ├── titleIM.xpm
│   │   │   ├── titleIP.xpm
│   │   │   ├── titleIQ.xpm
│   │   │   ├── titleIS.xpm
│   │   │   └── titleIT.xpm
│   │   ├── default/
│   │   │   ├── closeA.xpm
│   │   │   ├── closeI.xpm
│   │   │   ├── default.png
│   │   │   ├── default.theme
│   │   │   ├── depthA.xpm
│   │   │   ├── depthI.xpm
│   │   │   ├── hideA.xpm
│   │   │   ├── hideI.xpm
│   │   │   ├── maximizeA.xpm
│   │   │   ├── maximizeI.xpm
│   │   │   ├── menuButtonA.xpm
│   │   │   ├── menuButtonI.xpm
│   │   │   ├── minimizeA.xpm
│   │   │   ├── minimizeI.xpm
│   │   │   ├── restoreA.xpm
│   │   │   ├── restoreI.xpm
│   │   │   ├── rolldownA.xpm
│   │   │   ├── rolldownI.xpm
│   │   │   ├── rollupA.xpm
│   │   │   └── rollupI.xpm
│   │   ├── gtk2/
│   │   │   ├── closeA.xpm
│   │   │   ├── closeI.xpm
│   │   │   ├── default.theme
│   │   │   ├── maximizeA.xpm
│   │   │   ├── maximizeI.xpm
│   │   │   ├── menuButtonA.xpm
│   │   │   ├── menuButtonI.xpm
│   │   │   ├── minimizeA.xpm
│   │   │   ├── minimizeI.xpm
│   │   │   ├── restoreA.xpm
│   │   │   └── restoreI.xpm
│   │   ├── Helix/
│   │   │   ├── closeA.xpm
│   │   │   ├── closeI.xpm
│   │   │   ├── default.theme
│   │   │   ├── maximizeA.xpm
│   │   │   ├── maximizeI.xpm
│   │   │   ├── menuButtonA.xpm
│   │   │   ├── menuButtonI.xpm
│   │   │   ├── minimizeA.xpm
│   │   │   ├── minimizeI.xpm
│   │   │   ├── restoreA.xpm
│   │   │   ├── restoreI.xpm
│   │   │   ├── titleAB.xpm
│   │   │   ├── titleAM.xpm
│   │   │   ├── titleAP.xpm
│   │   │   ├── titleAS.xpm
│   │   │   ├── titleAT.xpm
│   │   │   ├── titleIB.xpm
│   │   │   ├── titleIP.xpm
│   │   │   ├── titleIS.xpm
│   │   │   └── titleIT.xpm
│   │   ├── icedesert/
│   │   │   ├── closeA.xpm
│   │   │   ├── closeI.xpm
│   │   │   ├── closeO.xpm
│   │   │   ├── default.theme
│   │   │   ├── maximizeA.xpm
│   │   │   ├── maximizeI.xpm
│   │   │   ├── maximizeO.xpm
│   │   │   ├── menuButtonA.xpm
│   │   │   ├── menuButtonI.xpm
│   │   │   ├── menuButtonO.xpm
│   │   │   ├── minimizeA.xpm
│   │   │   ├── minimizeI.xpm
│   │   │   ├── minimizeO.xpm
│   │   │   ├── restoreA.xpm
│   │   │   ├── restoreI.xpm
│   │   │   ├── restoreO.xpm
│   │   │   ├── titleAB.xpm
│   │   │   ├── titleAT.xpm
│   │   │   ├── titleIB.xpm
│   │   │   └── titleIT.xpm
│   │   ├── Infadel2/
│   │   │   ├── cursors/
│   │   │   │   ├── left.xpm
│   │   │   │   ├── move.xpm
│   │   │   │   ├── right.xpm
│   │   │   │   ├── sizeB.xpm
│   │   │   │   ├── sizeBL.xpm
│   │   │   │   ├── sizeBR.xpm
│   │   │   │   ├── sizeL.xpm
│   │   │   │   ├── sizeR.xpm
│   │   │   │   ├── sizeT.xpm
│   │   │   │   ├── sizeTL.xpm
│   │   │   │   └── sizeTR.xpm
│   │   │   ├── icons/
│   │   │   │   ├── app_16x16.xpm
│   │   │   │   ├── folder_16x16.xpm
│   │   │   │   └── folder_32x32.xpm
│   │   │   ├── mailbox/
│   │   │   │   ├── errmail.xpm
│   │   │   │   ├── mail.xpm
│   │   │   │   ├── newmail.xpm
│   │   │   │   ├── nomail.xpm
│   │   │   │   └── unreadmail.xpm
│   │   │   ├── taskbar/
│   │   │   │   ├── linux.xpm
│   │   │   │   ├── start.xpm
│   │   │   │   └── windows.xpm
│   │   │   ├── closeA.xpm
│   │   │   ├── closeI.xpm
│   │   │   ├── default.theme
│   │   │   ├── depthA.xpm
│   │   │   ├── depthI.xpm
│   │   │   ├── Ergonomic.theme
│   │   │   ├── fonts.dir.default
│   │   │   ├── hideA.xpm
│   │   │   ├── hideI.xpm
│   │   │   ├── maximizeA.xpm
│   │   │   ├── maximizeI.xpm
│   │   │   ├── menuButtonA.xpm
│   │   │   ├── menuButtonI.xpm
│   │   │   ├── minimizeA.xpm
│   │   │   ├── minimizeI.xpm
│   │   │   ├── Overloaded.theme
│   │   │   ├── restoreA.xpm
│   │   │   ├── restoreI.xpm
│   │   │   ├── rolldownA.xpm
│   │   │   ├── rolldownI.xpm
│   │   │   ├── rollupA.xpm
│   │   │   ├── rollupI.xpm
│   │   │   ├── snap.pcf
│   │   │   ├── titleAB.xpm
│   │   │   ├── titleAJ.xpm
│   │   │   ├── titleAM.xpm
│   │   │   ├── titleAP.xpm
│   │   │   ├── titleAQ.xpm
│   │   │   ├── titleAR.xpm
│   │   │   ├── titleAS.xpm
│   │   │   ├── titleAT.xpm
│   │   │   ├── titleIB.xpm
│   │   │   ├── titleIJ.xpm
│   │   │   ├── titleIM.xpm
│   │   │   ├── titleIP.xpm
│   │   │   ├── titleIQ.xpm
│   │   │   ├── titleIR.xpm
│   │   │   ├── titleIS.xpm
│   │   │   └── titleIT.xpm
│   │   ├── metal2/
│   │   │   ├── closeA.xpm
│   │   │   ├── closeI.xpm
│   │   │   ├── default.theme
│   │   │   ├── depthA.xpm
│   │   │   ├── depthI.xpm
│   │   │   ├── dframeAB.xpm
│   │   │   ├── dframeABL.xpm
│   │   │   ├── dframeABR.xpm
│   │   │   ├── dframeAL.xpm
│   │   │   ├── dframeAR.xpm
│   │   │   ├── dframeAT.xpm
│   │   │   ├── dframeATL.xpm
│   │   │   ├── dframeATR.xpm
│   │   │   ├── dframeIB.xpm
│   │   │   ├── dframeIBL.xpm
│   │   │   ├── dframeIBR.xpm
│   │   │   ├── dframeIL.xpm
│   │   │   ├── dframeIR.xpm
│   │   │   ├── dframeIT.xpm
│   │   │   ├── dframeITL.xpm
│   │   │   ├── dframeITR.xpm
│   │   │   ├── frameAB.xpm
│   │   │   ├── frameABL.xpm
│   │   │   ├── frameABR.xpm
│   │   │   ├── frameAL.xpm
│   │   │   ├── frameAR.xpm
│   │   │   ├── frameAT.xpm
│   │   │   ├── frameATL.xpm
│   │   │   ├── frameATR.xpm
│   │   │   ├── frameIB.xpm
│   │   │   ├── frameIBL.xpm
│   │   │   ├── frameIBR.xpm
│   │   │   ├── frameIL.xpm
│   │   │   ├── frameIR.xpm
│   │   │   ├── frameIT.xpm
│   │   │   ├── frameITL.xpm
│   │   │   ├── frameITR.xpm
│   │   │   ├── hideA.xpm
│   │   │   ├── hideI.xpm
│   │   │   ├── maximizeA.xpm
│   │   │   ├── maximizeI.xpm
│   │   │   ├── menuButtonA.xpm
│   │   │   ├── menuButtonI.xpm
│   │   │   ├── minimizeA.xpm
│   │   │   ├── minimizeI.xpm
│   │   │   ├── restoreA.xpm
│   │   │   ├── restoreI.xpm
│   │   │   ├── rolldownA.xpm
│   │   │   ├── rolldownI.xpm
│   │   │   ├── rollupA.xpm
│   │   │   ├── rollupI.xpm
│   │   │   ├── titleAB.xpm
│   │   │   ├── titleAL.xpm
│   │   │   ├── titleAM.xpm
│   │   │   ├── titleAP.xpm
│   │   │   ├── titleAR.xpm
│   │   │   ├── titleAS.xpm
│   │   │   ├── titleAT.xpm
│   │   │   ├── titleIB.xpm
│   │   │   ├── titleIL.xpm
│   │   │   ├── titleIM.xpm
│   │   │   ├── titleIP.xpm
│   │   │   ├── titleIR.xpm
│   │   │   ├── titleIS.xpm
│   │   │   └── titleIT.xpm
│   │   ├── motif/
│   │   │   ├── close.xpm
│   │   │   ├── default.theme
│   │   │   ├── maximize.xpm
│   │   │   ├── menu.xpm
│   │   │   ├── minimize.xpm
│   │   │   └── restore.xpm
│   │   ├── NanoBlue/
│   │   │   ├── icons/
│   │   │   │   ├── app_16x16.xpm
│   │   │   │   ├── app_32x32.xpm
│   │   │   │   ├── firefox_16x16.xpm
│   │   │   │   ├── firefox_32x32.xpm
│   │   │   │   ├── folder_16x16.xpm
│   │   │   │   ├── folder_32x32.xpm
│   │   │   │   ├── gimp_16x16.xpm
│   │   │   │   ├── gimp_32x32.xpm
│   │   │   │   ├── IceWM_16x16.xpm
│   │   │   │   ├── IceWM_32x32.xpm
│   │   │   │   ├── mplayer_16x16.xpm
│   │   │   │   ├── mplayer_32x32.xpm
│   │   │   │   ├── thunderbird_16x16.xpm
│   │   │   │   ├── thunderbird_32x32.xpm
│   │   │   │   ├── xterm_16x16.xpm
│   │   │   │   └── xterm_32x32.xpm
│   │   │   ├── ledclock/
│   │   │   │   ├── a.xpm
│   │   │   │   ├── colon.xpm
│   │   │   │   ├── dot.xpm
│   │   │   │   ├── m.xpm
│   │   │   │   ├── n0.xpm
│   │   │   │   ├── n1.xpm
│   │   │   │   ├── n2.xpm
│   │   │   │   ├── n3.xpm
│   │   │   │   ├── n4.xpm
│   │   │   │   ├── n5.xpm
│   │   │   │   ├── n6.xpm
│   │   │   │   ├── n7.xpm
│   │   │   │   ├── n8.xpm
│   │   │   │   ├── n9.xpm
│   │   │   │   ├── p.xpm
│   │   │   │   ├── slash.xpm
│   │   │   │   └── space.xpm
│   │   │   ├── taskbar/
│   │   │   │   ├── collapse.xpm
│   │   │   │   ├── desktop.xpm
│   │   │   │   ├── expand.xpm
│   │   │   │   ├── icewm.xpm
│   │   │   │   ├── taskbarbg.xpm
│   │   │   │   ├── taskbuttonactive.xpm
│   │   │   │   ├── taskbuttonbg.xpm
│   │   │   │   ├── taskbuttonminimized.xpm
│   │   │   │   ├── toolbuttonbg.xpm
│   │   │   │   ├── windows.xpm
│   │   │   │   ├── workspacebuttonactive.xpm
│   │   │   │   └── workspacebuttonbg.xpm
│   │   │   ├── closeA.xpm
│   │   │   ├── closeI.xpm
│   │   │   ├── closeO.xpm
│   │   │   ├── default.theme
│   │   │   ├── dframeAB.xpm
│   │   │   ├── dframeABL.xpm
│   │   │   ├── dframeABR.xpm
│   │   │   ├── dframeAL.xpm
│   │   │   ├── dframeAR.xpm
│   │   │   ├── dframeAT.xpm
│   │   │   ├── dframeATL.xpm
│   │   │   ├── dframeATR.xpm
│   │   │   ├── dframeIB.xpm
│   │   │   ├── dframeIBL.xpm
│   │   │   ├── dframeIBR.xpm
│   │   │   ├── dframeIL.xpm
│   │   │   ├── dframeIR.xpm
│   │   │   ├── dframeIT.xpm
│   │   │   ├── dframeITL.xpm
│   │   │   ├── dframeITR.xpm
│   │   │   ├── eos.jpg
│   │   │   ├── expandA.xpm
│   │   │   ├── expandI.xpm
│   │   │   ├── expandO.xpm
│   │   │   ├── frameAB.xpm
│   │   │   ├── frameABL.xpm
│   │   │   ├── frameABR.xpm
│   │   │   ├── frameAL.xpm
│   │   │   ├── frameAR.xpm
│   │   │   ├── frameAT.xpm
│   │   │   ├── frameATL.xpm
│   │   │   ├── frameATR.xpm
│   │   │   ├── frameIB.xpm
│   │   │   ├── frameIBL.xpm
│   │   │   ├── frameIBR.xpm
│   │   │   ├── frameIL.xpm
│   │   │   ├── frameIR.xpm
│   │   │   ├── frameIT.xpm
│   │   │   ├── frameITL.xpm
│   │   │   ├── frameITR.xpm
│   │   │   ├── maximizeA.xpm
│   │   │   ├── maximizeI.xpm
│   │   │   ├── maximizeO.xpm
│   │   │   ├── menuButtonA.xpm
│   │   │   ├── menuButtonI.xpm
│   │   │   ├── menuButtonO.xpm
│   │   │   ├── menusel.xpm
│   │   │   ├── menusep.xpm
│   │   │   ├── minimizeA.xpm
│   │   │   ├── minimizeI.xpm
│   │   │   ├── minimizeO.xpm
│   │   │   ├── restoreA.xpm
│   │   │   ├── restoreI.xpm
│   │   │   ├── restoreO.xpm
│   │   │   ├── titleAB.xpm
│   │   │   ├── titleAL.xpm
│   │   │   ├── titleAM.xpm
│   │   │   ├── titleAP.xpm
│   │   │   ├── titleAR.xpm
│   │   │   ├── titleAS.xpm
│   │   │   ├── titleAT.xpm
│   │   │   ├── titleIB.xpm
│   │   │   ├── titleIL.xpm
│   │   │   ├── titleIM.xpm
│   │   │   ├── titleIP.xpm
│   │   │   ├── titleIR.xpm
│   │   │   ├── titleIS.xpm
│   │   │   └── titleIT.xpm
│   │   ├── Natural/
│   │   │   ├── ledclock/
│   │   │   │   ├── a.xpm
│   │   │   │   ├── colon.xpm
│   │   │   │   ├── dot.xpm
│   │   │   │   ├── m.xpm
│   │   │   │   ├── n0.xpm
│   │   │   │   ├── n1.xpm
│   │   │   │   ├── n2.xpm
│   │   │   │   ├── n3.xpm
│   │   │   │   ├── n4.xpm
│   │   │   │   ├── n5.xpm
│   │   │   │   ├── n6.xpm
│   │   │   │   ├── n7.xpm
│   │   │   │   ├── n8.xpm
│   │   │   │   ├── n9.xpm
│   │   │   │   ├── p.xpm
│   │   │   │   ├── slash.xpm
│   │   │   │   └── space.xpm
│   │   │   ├── taskbar/
│   │   │   │   ├── taskbarbg.xpm
│   │   │   │   ├── taskbuttonactive.xpm
│   │   │   │   ├── taskbuttonbg.xpm
│   │   │   │   ├── taskbuttonminimized.xpm
│   │   │   │   └── windows.xpm
│   │   │   ├── closeA.xpm
│   │   │   ├── closeI.xpm
│   │   │   ├── default.theme
│   │   │   ├── dframeAB.xpm
│   │   │   ├── dframeABL.xpm
│   │   │   ├── dframeABR.xpm
│   │   │   ├── dframeAL.xpm
│   │   │   ├── dframeAR.xpm
│   │   │   ├── dframeAT.xpm
│   │   │   ├── dframeATL.xpm
│   │   │   ├── dframeATR.xpm
│   │   │   ├── dframeIB.xpm
│   │   │   ├── dframeIBL.xpm
│   │   │   ├── dframeIBR.xpm
│   │   │   ├── dframeIL.xpm
│   │   │   ├── dframeIR.xpm
│   │   │   ├── dframeIT.xpm
│   │   │   ├── dframeITL.xpm
│   │   │   ├── dframeITR.xpm
│   │   │   ├── frameAB.xpm
│   │   │   ├── frameABL.xpm
│   │   │   ├── frameABR.xpm
│   │   │   ├── frameAL.xpm
│   │   │   ├── frameAR.xpm
│   │   │   ├── frameAT.xpm
│   │   │   ├── frameATL.xpm
│   │   │   ├── frameATR.xpm
│   │   │   ├── frameIB.xpm
│   │   │   ├── frameIBL.xpm
│   │   │   ├── frameIBR.xpm
│   │   │   ├── frameIL.xpm
│   │   │   ├── frameIR.xpm
│   │   │   ├── frameIT.xpm
│   │   │   ├── frameITL.xpm
│   │   │   ├── frameITR.xpm
│   │   │   ├── logoutbutton.xpm
│   │   │   ├── mail.xpm
│   │   │   ├── maximizeA.xpm
│   │   │   ├── maximizeI.xpm
│   │   │   ├── menubg.xpm
│   │   │   ├── menuButtonA.xpm
│   │   │   ├── menuButtonI.xpm
│   │   │   ├── minimizeA.xpm
│   │   │   ├── minimizeI.xpm
│   │   │   ├── newmail.xpm
│   │   │   ├── nomail.xpm
│   │   │   ├── restoreA.xpm
│   │   │   ├── restoreI.xpm
│   │   │   ├── rolldownA.xpm
│   │   │   ├── rolldownI.xpm
│   │   │   ├── rollupA.xpm
│   │   │   ├── rollupI.xpm
│   │   │   ├── switchbg.xpm
│   │   │   ├── titleAB.xpm
│   │   │   ├── titleAL.xpm
│   │   │   ├── titleAM.xpm
│   │   │   ├── titleAT.xpm
│   │   │   ├── titleIB.xpm
│   │   │   ├── titleIL.xpm
│   │   │   ├── titleIM.xpm
│   │   │   ├── titleIT.xpm
│   │   │   └── unreadmail.xpm
│   │   ├── nice/
│   │   │   ├── blue.theme
│   │   │   ├── close.xpm
│   │   │   ├── default.theme
│   │   │   ├── hide.xpm
│   │   │   ├── maximize.xpm
│   │   │   ├── minimize.xpm
│   │   │   ├── restore.xpm
│   │   │   ├── rolldown.xpm
│   │   │   └── rollup.xpm
│   │   ├── nice2/
│   │   │   ├── closeA.xpm
│   │   │   ├── closeI.xpm
│   │   │   ├── default.theme
│   │   │   ├── maximizeA.xpm
│   │   │   ├── maximizeI.xpm
│   │   │   ├── menuButtonA.xpm
│   │   │   ├── menuButtonI.xpm
│   │   │   ├── minimizeA.xpm
│   │   │   ├── minimizeI.xpm
│   │   │   ├── restoreA.xpm
│   │   │   ├── restoreI.xpm
│   │   │   ├── titleAB.xpm
│   │   │   ├── titleAT.xpm
│   │   │   ├── titleIB.xpm
│   │   │   └── titleIT.xpm
│   │   ├── warp3/
│   │   │   ├── close.xpm
│   │   │   ├── default.theme
│   │   │   ├── hide.xpm
│   │   │   ├── maximize.xpm
│   │   │   ├── minimize.xpm
│   │   │   ├── restore.xpm
│   │   │   ├── rolldown.xpm
│   │   │   └── rollup.xpm
│   │   ├── warp4/
│   │   │   ├── close.xpm
│   │   │   ├── default.theme
│   │   │   ├── hide.xpm
│   │   │   ├── maximize.xpm
│   │   │   ├── minimize.xpm
│   │   │   └── restore.xpm
│   │   ├── win95/
│   │   │   ├── close.xpm
│   │   │   ├── default.theme
│   │   │   ├── maximize.xpm
│   │   │   ├── minimize.xpm
│   │   │   └── restore.xpm
│   │   └── yellowmotif/
│   │       ├── close.xpm
│   │       ├── default.theme
│   │       ├── maximize.xpm
│   │       ├── menu.xpm
│   │       ├── minimize.xpm
│   │       └── restore.xpm
│   ├── CMakeLists.txt
│   ├── icewm-session.desktop
│   ├── icewm.desktop
│   ├── IceWM.jpg
│   ├── keys.in
│   ├── Makefile.am
│   ├── menu.in
│   ├── programs.in
│   ├── toolbar.in
│   └── winoptions.in
├── man/
│   ├── .gitignore
│   ├── CMakeLists.txt
│   ├── icehelp.pod
│   ├── icesh.pod
│   ├── icesound.pod
│   ├── icewm-env.pod
│   ├── icewm-focus_mode.pod
│   ├── icewm-keys.pod
│   ├── icewm-menu-fdo.pod
│   ├── icewm-menu-xrandr.pod
│   ├── icewm-menu.pod
│   ├── icewm-preferences.pod
│   ├── icewm-prefoverride.pod
│   ├── icewm-programs.pod
│   ├── icewm-session.pod
│   ├── icewm-set-gnomewm.pod
│   ├── icewm-shutdown.pod
│   ├── icewm-startup.pod
│   ├── icewm-theme.pod
│   ├── icewm-toolbar.pod
│   ├── icewm-winoptions.pod
│   ├── icewm.pod
│   ├── icewmbg.pod
│   ├── icewmhint.pod
│   ├── icewmtray.pod
│   └── Makefile.am
├── po/
│   ├── ar.po
│   ├── ast.po
│   ├── be.po
│   ├── bg.po
│   ├── ca.po
│   ├── CMakeLists.txt
│   ├── cs.po
│   ├── da.po
│   ├── de.po
│   ├── el.po
│   ├── en.po
│   ├── eo.po
│   ├── es.po
│   ├── fa.po
│   ├── fi.po
│   ├── fr.po
│   ├── he.po
│   ├── hi.po
│   ├── hr.po
│   ├── hu.po
│   ├── icewm.pot
│   ├── id.po
│   ├── ie.po
│   ├── it.po
│   ├── ja.po
│   ├── ka.po
│   ├── kk.po
│   ├── ko.po
│   ├── LINGUAS
│   ├── lt.po
│   ├── lv.po
│   ├── Makevars
│   ├── mk.po
│   ├── nb.po
│   ├── nl.po
│   ├── nn.po
│   ├── no.po
│   ├── pl.po
│   ├── POTFILES.in
│   ├── pt.po
│   ├── pt_BR.po
│   ├── ro.po
│   ├── ru.po
│   ├── sk.po
│   ├── sl.po
│   ├── sr.po
│   ├── sv.po
│   ├── tr.po
│   ├── uk.po
│   ├── vi.po
│   ├── zh_CN.po
│   └── zh_TW.po
├── src/
│   ├── aaddressbar.cc
│   ├── aaddressbar.h
│   ├── aapm.cc
│   ├── aapm.h
│   ├── aclock.cc
│   ├── aclock.h
│   ├── acpustatus.cc
│   ├── acpustatus.h
│   ├── akeyboard.cc
│   ├── akeyboard.h
│   ├── amailbox.cc
│   ├── amailbox.h
│   ├── amemstatus.cc
│   ├── amemstatus.h
│   ├── applet.cc
│   ├── applet.h
│   ├── appnames.h
│   ├── apppstatus.cc
│   ├── apppstatus.h
│   ├── argument.h
│   ├── ascii.h
│   ├── atasks.cc
│   ├── atasks.h
│   ├── atray.cc
│   ├── atray.h
│   ├── aworkspaces.cc
│   ├── aworkspaces.h
│   ├── base.h
│   ├── bindkey.cc
│   ├── bindkey.h
│   ├── browse.cc
│   ├── browse.h
│   ├── CMakeLists.txt
│   ├── config.cmake.h.in
│   ├── debug.h
│   ├── decorate.cc
│   ├── default.h
│   ├── fdomenu.cc
│   ├── fdospecgen.h
│   ├── fontmacro.h
│   ├── genpref.cc
│   ├── globit.c
│   ├── globit.cc
│   ├── globit.h
│   ├── guievent.h
│   ├── iceclock.cc
│   ├── icecursor.cc
│   ├── icehelp.cc
│   ├── iceicon.cc
│   ├── icelist.cc
│   ├── icerun.cc
│   ├── icesame.cc
│   ├── icesh.cc
│   ├── iceskt.cc
│   ├── icesm.cc
│   ├── icesound.cc
│   ├── icetray.cc
│   ├── iceview.cc
│   ├── icewmbg.cc
│   ├── icewmbg_prefs.h
│   ├── icewmhint.cc
│   ├── intl.h
│   ├── keysyms.cc
│   ├── keysyms.h
│   ├── logevent.cc
│   ├── logevent.h
│   ├── Makefile.am
│   ├── misc.cc
│   ├── movesize.cc
│   ├── mstring.cc
│   ├── mstring.h
│   ├── MwmUtil.h
│   ├── obj.h
│   ├── objbar.cc
│   ├── objbar.h
│   ├── objbutton.cc
│   ├── objbutton.h
│   ├── objmenu.cc
│   ├── objmenu.h
│   ├── prefs.h
│   ├── preview.cc
│   ├── preview.h
│   ├── ref.cc
│   ├── ref.h
│   ├── strtest.cc
│   ├── switcher.h
│   ├── sysdep.h
│   ├── testarray.cc
│   ├── testlocale.cc
│   ├── testmap.cc
│   ├── testmenus.cc
│   ├── testnetwmhints.cc
│   ├── testpointer.cc
│   ├── testwinhints.cc
│   ├── themable.h
│   ├── themes.cc
│   ├── themes.h
│   ├── theminst.cc
│   ├── theminst.h
│   ├── udir.cc
│   ├── udir.h
│   ├── upath.cc
│   ├── upath.h
│   ├── WinMgr.h
│   ├── wmabout.cc
│   ├── wmabout.h
│   ├── wmaction.cc
│   ├── wmaction.h
│   ├── wmapp.cc
│   ├── wmapp.h
│   ├── wmbutton.cc
│   ├── wmbutton.h
│   ├── wmclient.cc
│   ├── wmclient.h
│   ├── wmconfig.cc
│   ├── wmconfig.h
│   ├── wmcontainer.cc
│   ├── wmcontainer.h
│   ├── wmdialog.cc
│   ├── wmdialog.h
│   ├── wmdock.cc
│   ├── wmdock.h
│   ├── wmframe.cc
│   ├── wmframe.h
│   ├── wmkey.cc
│   ├── wmkey.h
│   ├── wmmenu.cc
│   ├── wmmgr.cc
│   ├── wmmgr.h
│   ├── wmminiicon.cc
│   ├── wmminiicon.h
│   ├── wmoption.cc
│   ├── wmoption.h
│   ├── wmpref.cc
│   ├── wmpref.h
│   ├── wmprog.cc
│   ├── wmprog.h
│   ├── wmsave.cc
│   ├── wmsave.h
│   ├── wmsession.cc
│   ├── wmsession.h
│   ├── wmstatus.cc
│   ├── wmstatus.h
│   ├── wmswitch.cc
│   ├── wmswitch.h
│   ├── wmtaskbar.cc
│   ├── wmtaskbar.h
│   ├── wmtitle.cc
│   ├── wmtitle.h
│   ├── wmwinlist.cc
│   ├── wmwinlist.h
│   ├── wmwinmenu.cc
│   ├── wmwinmenu.h
│   ├── workspaces.h
│   ├── wpixmaps.h
│   ├── wpixres.cc
│   ├── wpixres.h
│   ├── yaction.h
│   ├── yactionbutton.h
│   ├── yapp.cc
│   ├── yapp.h
│   ├── yarray.cc
│   ├── yarray.h
│   ├── ybidi.h
│   ├── ybutton.cc
│   ├── ybutton.h
│   ├── ycolor.cc
│   ├── ycolor.h
│   ├── yconfig.cc
│   ├── yconfig.h
│   ├── ycursor.cc
│   ├── ycursor.h
│   ├── ydialog.cc
│   ├── ydialog.h
│   ├── yfileio.cc
│   ├── yfileio.h
│   ├── yfont.cc
│   ├── yfontbase.h
│   ├── yfontcache.h
│   ├── yfontcore.cc
│   ├── yfontname.h
│   ├── yfontxft.cc
│   ├── yfull.h
│   ├── yicon.cc
│   ├── yicon.h
│   ├── yimage.h
│   ├── yimage2.cc
│   ├── yimage2.h
│   ├── yimage_gdk.cc
│   ├── yimage_gdk.h
│   ├── yinputline.cc
│   ├── yinputline.h
│   ├── ykey.h
│   ├── ylabel.cc
│   ├── ylabel.h
│   ├── ylayout.h
│   ├── ylib.h
│   ├── ylist.h
│   ├── ylistbox.cc
│   ├── ylistbox.h
│   ├── ylocale.cc
│   ├── ylocale.h
│   ├── ymenu.cc
│   ├── ymenu.h
│   ├── ymenuitem.cc
│   ├── ymenuitem.h
│   ├── ymsgbox.cc
│   ├── ymsgbox.h
│   ├── ypaint.cc
│   ├── ypaint.h
│   ├── ypipereader.cc
│   ├── ypipereader.h
│   ├── ypixmap.cc
│   ├── ypixmap.h
│   ├── ypoint.h
│   ├── ypointer.h
│   ├── ypoll.h
│   ├── ypopup.cc
│   ├── ypopup.h
│   ├── yprefs.cc
│   ├── yprefs.h
│   ├── yrect.h
│   ├── yscrollbar.cc
│   ├── yscrollbar.h
│   ├── yscrollview.cc
│   ├── yscrollview.h
│   ├── ysmapp.cc
│   ├── ysmapp.h
│   ├── ysocket.cc
│   ├── ysocket.h
│   ├── ystring.cc
│   ├── ystring.h
│   ├── ysvg.cc
│   ├── ytime.cc
│   ├── ytime.h
│   ├── ytimer.cc
│   ├── ytimer.h
│   ├── ytooltip.cc
│   ├── ytooltip.h
│   ├── ytrace.h
│   ├── yurl.cc
│   ├── yurl.h
│   ├── ywindow.cc
│   ├── ywindow.h
│   ├── ywordexp.h
│   ├── yxapp.cc
│   ├── yxapp.h
│   ├── yxcontext.h
│   ├── yxembed.cc
│   ├── yxembed.h
│   ├── yximage.cc
│   ├── yxtray.cc
│   └── yxtray.h
├── utils/
│   ├── install-theme.sh
│   ├── mkbuild.sh
│   ├── prefs2cxx.xsl
│   ├── release.sh
│   └── search_strings
├── .clang-format
├── .gitignore
├── acinclude.m4
├── AUTHORS
├── autogen.sh
├── BUGS
├── ChangeLog
├── CHANGES
├── CMakeLists.txt
├── CODE_OF_CONDUCT.md
├── COMPLIANCE
├── configure.ac
├── configure.sh
├── CONTRIBUTING.md
├── COPYING
├── dist.sh
├── gennews.sh
├── icewm-set-gnomewm
├── icewm.spec.in
├── INSTALL
├── INSTALL-cmakebuild.md
├── INSTALL.cmakebuild
├── install.in
├── Makefile.am
├── NEWS
├── PLATFORMS
├── README.md
├── README.md.in
├── rebuild.sh
├── RELEASE.md
├── THANKS
├── TODO
└── VERSION
```

---

# Appendix B — Critical source anchors

| Concern | Source anchor |
|---|---|
| WM version | `VERSION`; `README.md` |
| Core dependencies | `configure.ac:201-202`; `src/CMakeLists.txt:183-190` |
| CMake core targets | `src/CMakeLists.txt:435-486` |
| CMake registered tests | `src/CMakeLists.txt:494-507` |
| CI dependencies/build paths | `.github/workflows/cmake.yml:28-100` |
| Config load order | `src/wmapp.cc:1403-1405` |
| Manager construction | `src/wmapp.cc:1447` |
| Main manage loop | `src/wmapp.cc:1583-1600` |
| Taskbar creation | `src/wmapp.cc:2015-2021` |
| Taskbar applet construction | `src/wmtaskbar.cc:254-415` |
| Taskbar hard-coded layout | `src/wmtaskbar.cc:446+` |
| Global taskbar debt note | `src/wmtaskbar.h:196` |
| Upstream taskbar modularization TODO | `TODO:54-57` |
| Workspace right-click current behavior | `src/aworkspaces.cc:72-104` |
| Dynamic workspace append/tail removal | `src/wmmgr.cc:2854-2921` |
| Workspace limits/model | `src/workspaces.h` |
| Frame workspace setter | `src/wmframe.cc:2959+` |
| EWMH desktop count/viewport | `src/wmmgr.cc:3098-3109` |
| Tile menu | `src/wmapp.cc:634-655` |
| Tile action geometry | `src/wmframe.cc:1500-1578` |
| Existing center-origin bug | `src/wmframe.cc:1564-1568` |
| Tile key routing | `src/movesize.cc:757-774` |
| Maximize button action | `src/wmbutton.cc` `YFrameButton::setKind` |
| Generic button hover crossing | `src/ybutton.cc:316+` |
| Popup primitive | `src/ypopup.h` |
| Live preview damage/render | `src/preview.cc:18-208`; `src/preview.h` |
| XComposite usage | `src/yxapp.cc:1134`; `src/yxtray.cc:455` |
| Theme PNG/XPM mapping | `src/wpixres.cc:374-408` |
| Start menu | `src/wmprog.cc:310-418`; `src/wmprog.h` |
| XDG desktop parsing | `src/fdomenu.cc` |
| Async pipe primitive | `src/ypipereader.*` |
| SIGCHLD handling | `src/yapp.cc`; `src/wmapp.cc:1432` |
| Background/root pixmap | `src/icewmbg.cc` |
| Session orchestration | `src/icesm.cc` |
| License | `COPYING` |

---

# Appendix C — Decision summary

**Use IceWM 4.1.0 as the base.**  
**Keep C++/Xlib and the in-tree toolkit.**  
**Do not embed Qt/GTK/web technology into the WM.**  
**Build a Flame Breeze theme, indexed workspaces, generalized snap layouts, Overview, Launcher, and optional system-control widgets on the existing primitives.**  
**Keep desktop filesystem/icon management and compositor functionality outside the WM process.**  
**Treat multi-monitor geometry, workspace reindexing, popup lifetime, child-process I/O, and secret handling as correctness boundaries with tests before feature polish.**
