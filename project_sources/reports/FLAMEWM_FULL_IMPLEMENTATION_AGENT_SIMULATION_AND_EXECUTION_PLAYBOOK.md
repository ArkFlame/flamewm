# FlameWM Full Implementation Agent Simulation & Execution Playbook

**Project:** FlameWM  
**Studio:** ArkFlame Studios  
**Repository:** `https://github.com/linsaftw/flamewm`  
**Base:** IceWM 4.1.0 / X11  
**Document role:** Pre-implementation execution simulator and coding-agent flight plan. This document deliberately describes how a future implementation agent must investigate, stage, implement, test, recover from failures, and complete FlameWM without damaging IceWM correctness or long-term upstream maintainability.  
**Status:** Planning-only. **No production source changes are authorized by this document itself.**  
**Audit date:** 2026-09-02

---

# 0. Assumptions, authority, and purpose

## 0.1 Assumptions

1. The FlameWM GitHub repository has been forked but is still effectively the IceWM 4.1.0 implementation baseline.
2. No feature code should be written until a clean build/test/development baseline is established on the actual development machine.
3. FlameWM remains X11-first for the first complete product generation.
4. FlameWM is not a replacement window-management engine. IceWM remains the authoritative engine for window, workspace, focus, stacking, EWMH/ICCCM, task, tray, and work-area state.
5. FlameWM is the user-facing product layer: defaults, visual identity, simple settings, modern interactions, taskbar composition, launcher, snap UX, workspaces UX, desktop, and system controls.
6. The current product description is canonical: **“IceWM made friendly by ArkFlame Studios.”**
7. The current visual direction uses a **Flame red default accent**. Older project-source wording that recommends Breeze blue as the default accent is superseded by the later prototype/product decision; blue remains a valid preset.
8. The project must stay lightweight by construction: no permanent Qt/GTK/QML/Electron/WebView shell dependency and no compositor requirement for core usability.
9. The goal is to make upstream IceWM updates easy to merge for years. New FlameWM logic therefore belongs overwhelmingly in Flame-owned source paths.
10. This playbook is not an instruction to perform all phases in one giant coding session. It is the **complete simulation of the end-state implementation path**, from which future bounded coding-agent handoffs should be derived.

## 0.2 Source authority order

When requirements disagree, the coding agent uses this order:

1. newest explicit product/user instruction;
2. verified current repository/source behavior;
3. this execution playbook;
4. `FLAMEWM_COMPLEMENTARY_ARCHITECTURE_MAINTAINABILITY_BLUEPRINT.md`;
5. `FLAMEWM_PRODUCT_SOUL_UX_CONSTITUTION.md`;
6. `ICEWM_FlameWM_Source_Grounded_Implementation_Report.md`;
7. `FlameWM_IceWM_4.1.0_Fork_Source_Report.md`;
8. old prototype behavior or historical IceWM defaults.

The agent must never silently reconcile conflicting requirements by guessing. If a newer decision supersedes an older project source, update the decision ledger/project source in the same bounded development phase.

## 0.3 What this document solves

The previous FlameWM reports define the product, the IceWM source architecture, the missing features, and the long-term fork strategy. This document fills the last operational gap:

> **What exactly does a coding agent do, in what order, with what source files and assets, under what invariants, and what mistakes should be simulated and prevented before real implementation begins?**

This is therefore a **simulated implementation runbook**.

---

# 1. Simulation success definition

The simulation is successful when a future coding agent can begin from the clean fork and answer all of these questions without improvising architecture:

- Which reports and assets must be read first?
- Which IceWM classes own each state transition?
- Which code is reused versus extended versus newly created?
- Which new source directories/classes are allowed?
- Which upstream IceWM files may be touched?
- Where does every integration hook go?
- Which runtime processes exist in the final architecture?
- How are FlameWM settings stored and applied?
- How do Breeze assets enter the product without creating licensing or maintenance debt?
- What does the theme implementation use as reference?
- How is the FlameWM logo used?
- Which final graphical assets are still missing?
- How are changes tested before the next phase begins?
- What failures should force the agent to stop rather than continue?
- How is each feature proven not to break upstream IceWM behavior?
- How do we ensure a future IceWM upstream merge remains tractable?
- How is low memory usage measured rather than assumed?

The implementation must **not** depend on the agent rediscovering these answers during coding.

---

# 2. Pre-coding project source package

Before feature implementation, create a repository-local **read-only engineering context package** named `project_sources/`.

This directory is not installed by FlameWM. It exists so every coding/planning agent has stable access to product decisions, visual references, asset provenance, upstream baseline information, and implementation doctrine.

## 2.1 Recommended `project_sources/` tree

```text
project_sources/
├── README.md
│
├── reports/
│   ├── FLAMEWM_PRODUCT_SOUL_UX_CONSTITUTION.md
│   ├── ICEWM_FlameWM_Source_Grounded_Implementation_Report.md
│   ├── FlameWM_IceWM_4.1.0_Fork_Source_Report.md
│   ├── FLAMEWM_COMPLEMENTARY_ARCHITECTURE_MAINTAINABILITY_BLUEPRINT.md
│   └── FLAMEWM_FULL_IMPLEMENTATION_AGENT_SIMULATION_AND_EXECUTION_PLAYBOOK.md
│
├── baseline/
│   ├── ICEWM_BASELINE.md
│   ├── SOURCE_ANCHORS.md
│   └── BUILD_BASELINE.md
│
├── visual/
│   ├── approved/
│   │   ├── flamewm-wordmark.png
│   │   ├── flamewm-wordmark.svg              # REQUIRED; currently not confirmed available
│   │   ├── flamewm-start.svg                 # REQUIRED; currently not confirmed available
│   │   ├── default-wallpaper.png             # REQUIRED if/when final wallpaper is approved
│   │   └── mockups/
│   │       ├── desktop.png
│   │       ├── taskbar.png
│   │       ├── start-menu.png
│   │       ├── settings.png
│   │       ├── snap-layouts.png
│   │       └── overview.png
│   │
│   └── references/
│       ├── arc-dark/
│       │   ├── default.theme
│       │   ├── README.md
│       │   └── screenshot-or-reference.png
│       └── visual-notes.md
│
├── vendor-reference/
│   └── breeze-icons/
│       ├── VERSION.txt
│       ├── SOURCE.txt
│       ├── COPYING-ICONS
│       ├── REQUIRED_ICONS.md
│       └── breeze-icons-6.29.0.tar.xz        # optional; Git LFS if committed
│
└── decisions/
    ├── ASSET_MANIFEST.md
    ├── VISUAL_TOKENS.md
    ├── DEFAULT_BEHAVIOR.md
    ├── SESSION_INTEGRATIONS.md
    └── ARCHITECTURE_DECISIONS.md
```

## 2.2 `project_sources/` is not runtime data

The agent must distinguish three domains:

```text
project_sources/      engineering/design reference only
lib/flamewm/          installed runtime assets/defaults
src/flamewm*/         production implementation
```

Never make production code load files directly from `project_sources/`.

When an approved asset becomes production-ready:

1. validate license/provenance;
2. copy/convert it into the appropriate `lib/flamewm/**` runtime path;
3. add it to the runtime asset manifest/build/install rules;
4. retain the original engineering reference under `project_sources/` if useful.

## 2.3 Assets currently available and how to classify them

### `flamewm.png`

Current supplied image is a transparent FlameWM wordmark/logo suitable as:

- desktop watermark reference;
- About-page wordmark reference;
- branding reference;
- wallpaper composition reference.

It should **not automatically become the Start icon**. Its aspect ratio and wordmark form are inappropriate for a small square taskbar button.

Recommended project-source path:

```text
project_sources/visual/approved/flamewm-wordmark.png
```

### Breeze Icons 6.29.0

Current archive is useful as the upstream icon source/reference.

Do not vendor the entire icon repository into FlameWM runtime by default.

Preferred production strategy:

1. use installed Breeze icon theme through IceWM's existing `YIcon` lookup when available;
2. bundle only a small audited fallback subset needed for FlameWM shell-critical icons;
3. retain `COPYING-ICONS` and exact Breeze version/source metadata;
4. use semantic icon roles instead of each UI surface choosing arbitrary SVG sizes.

### `icewm-extra-main.zip` / Arc-Dark

Arc-Dark is a **visual/reference starting point**, not the FlameWM runtime theme.

Use it to inspect:

- flat IceWM theme capabilities;
- modern-ish titlebar/button/resource conventions;
- useful dark-surface color relationships;
- task/menu/workspace assets that IceWM already supports.

Do not copy the theme wholesale. FlameWM must have original theme assets and its own red semantic accent, metrics, and branding.

### `icewm-master(2).zip`

Do **not** copy this archive into `project_sources/` as the ongoing source authority once the repository is cloned. The Git repository itself is authoritative.

Instead record:

```text
upstream URL
upstream tag/commit
fork base commit
IceWM version
source report date
```

in `project_sources/baseline/ICEWM_BASELINE.md`.

## 2.4 Required assets not yet confirmed complete

The simulated agent must flag these as **missing before final release**, even though early code work can proceed with temporary placeholders:

### A. Final square Start icon SVG

Required characteristics:

- square viewBox;
- flat monochrome/white-friendly form;
- FlameWM-specific, not generic Windows logo;
- clean at approximately 16-32 logical pixels;
- easy recoloring for hover/pressed/accent states;
- no embedded raster effects;
- no dependence on glow/gradient to remain recognizable.

Target project-source path:

```text
project_sources/visual/approved/flamewm-start.svg
```

### B. Vector FlameWM wordmark SVG

The current PNG is sufficient as visual reference, but final product scaling is cleaner with a real vector version.

Target:

```text
project_sources/visual/approved/flamewm-wordmark.svg
```

### C. Final default wallpaper

If no approved wallpaper exists yet, code must use a development placeholder. Final release is blocked until an approved wallpaper/license/provenance record exists.

### D. Approved UI mockups/screenshots

The agent must not reconstruct final metrics from memory. Put approved prototype screenshots in `project_sources/visual/approved/mockups/` as they are finalized.

---

# 3. Coding-agent operating protocol

Every implementation handoff derived from this document begins with this protocol.

## Step 1 — Identify exact repository state

The agent records:

```text
current branch
HEAD commit
origin URL
upstream URL
upstream merge-base
IceWM VERSION contents
working-tree status
submodule status if any
```

If the working tree is dirty, classify every pre-existing change before touching anything.

Do not overwrite unrelated work.

## Step 2 — Read project sources in authority order

Required reading before editing:

1. this playbook;
2. complementary architecture/maintainability blueprint;
3. Product Soul & UX Constitution;
4. source-grounded implementation report;
5. fork source report;
6. asset manifest and visual tokens once created.

The agent extracts a per-task contract:

```text
requested user-visible behavior
existing IceWM owner
FlameWM owner to create/use
allowed upstream touchpoints
runtime assets required
tests required
performance gate
fallback behavior
```

## Step 3 — Re-audit exact source before coding

Reports accelerate source reading; they do not replace source.

For every task:

- open the current implementation owner;
- trace callers and callbacks;
- verify target build/config APIs;
- verify line-level assumptions have not drifted;
- inspect both CMake and Autotools registration where relevant.

If source behavior changed since the reports, current source wins and the plan is updated.

## Step 4 — Establish a green baseline

Before Flame feature code:

```bash
cmake -S . -B build -DCMAKE_BUILD_TYPE=RelWithDebInfo -DBUILD_TESTING=ON
cmake --build build -j"$(nproc)"
ctest --test-dir build --output-on-failure
```

Also maintain the upstream-supported Autotools path while upstream supports it:

```bash
./autogen.sh
mkdir -p build-ac
cd build-ac
../configure
make -j"$(nproc)"
```

Record baseline failures exactly.

No feature implementation begins with a red baseline unless the failure is proven environmental and resolved/reproduced separately.

## Step 5 — Establish the nested development lab

Create a development script such as:

```text
./dev
```

Its responsibilities:

- incremental build;
- isolated `ICEWM_PRIVCFG` / Flame config;
- launch/reuse Xephyr;
- launch FlameWM/IceWM in nested display;
- deterministic test windows;
- convenient config/theme restart/reload;
- optional integration fixtures;
- easy log capture.

The agent must not use the user's real desktop session as the only test environment.

## Step 6 — Record baseline resource measurements

Before feature work, capture:

- WM PSS/RSS;
- `icewmbg` PSS/RSS;
- process/thread/fd counts;
- idle CPU;
- startup latency;
- baseline taskbar/menu/QuickSwitch behavior.

This becomes the comparison point for later phases.

## Step 7 — Create a bounded feature branch

Example:

```text
feature/flame-visual-foundation
feature/flame-runtime
feature/snap-layouts
feature/workspace-ux
```

One feature branch should not contain mass branding, unrelated refactoring, snap changes, taskbar changes, and settings simultaneously.

## Step 8 — Pure logic before integration hook

For state-heavy features, implement and test pure logic before touching high-risk IceWM files.

Examples:

- region partition math;
- workspace index transformation;
- app identity matching;
- settings parsing/migration;
- shortcut conflict detection;
- `/proc` parsing.

Only after pure logic is green should the agent add the tiny IceWM integration hook.

## Step 9 — Integrate through the narrowest owner

A Flame feature must route through the existing authoritative IceWM owner.

Never create a second state machine because it is easier locally.

## Step 10 — Verify immediately after each integration

The agent does not stack five unverified subsystems.

For each change:

```text
compile
unit tests
same failed test if repairing
Xephyr semantic verification
resource sanity
upstream-diff audit
```

Only continue when the current gate is green.

---

# 4. Mandatory source-reading order

This section is the coding agent's map through the IceWM repository.

## 4.1 Repository/build identity

Read first:

```text
VERSION
README.md
NEWS
ChangeLog
TODO
COMPLIANCE
COPYING
CMakeLists.txt
configure.ac
Makefile.am
src/CMakeLists.txt
src/Makefile.am
lib/CMakeLists.txt
lib/Makefile.am
```

Purpose:

- verify source version;
- understand supported build systems;
- detect upstream architecture debt;
- identify install/resource targets;
- preserve licensing and release behavior.

## 4.2 Defaults/config/theme resources

Read:

```text
src/default.h
src/themable.h
src/wmconfig.*
src/yprefs.*
lib/keys.in
lib/menu.in
lib/programs.in
lib/toolbar.in
lib/winoptions.in
lib/themes/*/default.theme
man/icewm-preferences.pod
man/icewm-theme.pod
```

Purpose:

- distinguish upstream preferences from Flame settings;
- understand theme-owned versus hard-coded geometry;
- avoid creating duplicate configuration authority.

## 4.3 Application lifecycle/event loop

Read:

```text
src/wmapp.*
src/yapp.*
src/yxapp.*
src/ywindow.*
src/ytimer.*
src/ypoll.* or relevant poll abstractions
```

Purpose:

- determine legal lifecycle for `flamewm::Runtime`;
- understand event-loop ownership;
- ensure D-Bus/libpulse integration never blocks X processing.

## 4.4 Global WM/workspace/work-area authority

Read:

```text
src/wmmgr.*
src/workspaces.h
src/wmframe.*
src/movesize.cc
```

Purpose:

- window/workspace ownership;
- work area/XRandR source of truth;
- existing tiling and move state;
- workspace update transactions;
- edge switching.

## 4.5 Frame chrome

Read:

```text
src/wmtitle.*
src/wmbutton.*
```

Purpose:

- maximize hover snap chooser;
- titlebar click/double-click preservation;
- button painting/assets.

## 4.6 Taskbar and applets

Read:

```text
src/wmtaskbar.*
src/atasks.*
src/aworkspaces.*
src/yxtray.*
src/aclock.*
src/aapm.*
src/apppstatus.*
src/objbar.*
```

Purpose:

- panel composition;
- task state;
- workspaces pager;
- existing clock/tray/battery;
- identify legacy monitors to disable by default rather than delete.

## 4.7 Launcher/application discovery

Read:

```text
src/wmprog.*
src/fdomenu.cc
src/yinputline.*
src/ylistbox.*
src/yscrollview.*
src/yicon.*
```

Purpose:

- reuse `.desktop` parsing/launch behavior;
- use existing input/list/icon primitives;
- avoid a second icon-theme engine.

## 4.8 Window previews / QuickSwitch

Read:

```text
src/wmswitch.*
src/preview.*
```

Purpose:

- preserve Alt+Tab behavior;
- extract/reuse lower-level thumbnail mechanics for Overview;
- understand XDamage/XRender resource lifetime.

## 4.9 Desktop/wallpaper patterns

Read:

```text
src/icewmbg.cc
src/wmminiicon.*
```

Purpose:

- preserve wallpaper ownership;
- reuse desktop-child drag/click patterns without confusing minimized-window MiniIcons with filesystem icons.

## 4.10 Session process orchestration

Read:

```text
src/icesm.cc
lib/icewm.desktop
lib/icewm-session.desktop
```

Purpose:

- later launch `flamewm-desktop`;
- add Flame session files without prematurely destroying upstream compatibility.

---

# 5. Canonical production source architecture

New Flame logic should be physically and semantically isolated.

## 5.1 WM-linked Flame code

```text
src/flamewm/
├── core/
│   ├── runtime.h
│   ├── runtime.cc
│   ├── config.h
│   ├── config.cc
│   ├── control.h
│   ├── control.cc
│   ├── version.h
│   └── version.cc
│
├── ui/
│   ├── palette.h
│   ├── palette.cc
│   ├── metrics.h
│   ├── metrics.cc
│   ├── iconroles.h
│   ├── iconroles.cc
│   ├── popover.h
│   └── popover.cc
│
├── snap/
│   ├── layout.h
│   ├── layout.cc
│   ├── state.h
│   ├── state.cc
│   ├── overlay.h
│   ├── overlay.cc
│   ├── chooser.h
│   └── chooser.cc
│
├── workspace/
│   ├── workspaceux.h
│   └── workspaceux.cc
│
├── launcher/
│   ├── appmodel.h
│   ├── appmodel.cc
│   ├── search.h
│   ├── search.cc
│   ├── launcher.h
│   └── launcher.cc
│
├── overview/
│   ├── thumbnails.h
│   ├── thumbnails.cc
│   ├── overview.h
│   └── overview.cc
│
├── panel/
│   ├── composer.h
│   ├── composer.cc
│   ├── taskidentity.h
│   ├── taskidentity.cc
│   ├── audio.h
│   ├── audio.cc
│   ├── network.h
│   ├── network.cc
│   ├── media.h
│   └── media.cc
│
├── integrations/
│   ├── dbusdispatcher.h
│   ├── dbusdispatcher.cc
│   ├── networkmanager.h
│   ├── networkmanager.cc
│   ├── mpris.h
│   ├── mpris.cc
│   ├── pulse.h
│   └── pulse.cc
│
└── diagnostics/
    ├── procstats.h
    ├── procstats.cc
    ├── profiler.h
    └── profiler.cc
```

All product code uses:

```cpp
namespace flamewm {
    ...
}
```

Nested namespaces are appropriate:

```cpp
flamewm::snap
flamewm::panel
flamewm::integrations
flamewm::diagnostics
```

## 5.2 Settings process

```text
src/flamewm-settings/
├── main.cc
├── settingsapp.h
├── settingsapp.cc
├── pages/
│   ├── appearance.h/.cc
│   ├── desktop.h/.cc
│   ├── taskbar.h/.cc
│   ├── start.h/.cc
│   ├── windows.h/.cc
│   ├── workspaces.h/.cc
│   ├── shortcuts.h/.cc
│   └── about.h/.cc
└── widgets/
    ├── colorpicker.h/.cc
    ├── keycapture.h/.cc
    └── previewcard.h/.cc
```

Properties:

- separate executable;
- starts only when user opens Settings;
- zero idle memory when closed;
- shares Flame config schema and visual tokens;
- failure cannot crash WM.

## 5.3 Desktop process

```text
src/flamewm-desktop/
├── main.cc
├── desktopapp.h/.cc
├── model.h/.cc
├── view.h/.cc
├── item.h/.cc
├── layout.h/.cc
├── watcher.h/.cc
└── watermark.h/.cc
```

Properties:

- filesystem Desktop model;
- inotify;
- desktop grid;
- drag/reorder;
- persistent positions;
- context menu;
- Flame watermark;
- separate crash domain from WM.

## 5.4 Tests

Do not move upstream IceWM tests merely to reorganize the repository.

Add Flame-owned tests in a bounded location, e.g.:

```text
src/flamewm/tests/
├── test_snap_layout.cc
├── test_snap_state.cc
├── test_workspace_mapping.cc
├── test_app_identity.cc
├── test_settings_config.cc
├── test_shortcut_conflicts.cc
├── test_procstats.cc
└── test_iconroles.cc
```

Register them in **both** supported build systems where required:

```text
src/CMakeLists.txt
src/Makefile.am
```

Add nested-X integration scripts/tests separately, for example:

```text
tests/flamewm-x11/
```

if introducing a top-level test directory is done cleanly without moving upstream tests.

## 5.5 Runtime assets

```text
lib/flamewm/
├── themes/
│   ├── FlameWM-Light/
│   │   ├── default.theme
│   │   └── ... original runtime assets
│   └── FlameWM-Dark/
│       ├── default.theme
│       └── ... original runtime assets
│
├── icons/
│   ├── COPYING-BREEZE-ICONS
│   ├── upstream-version.txt
│   ├── flamewm/
│   │   ├── start.svg
│   │   └── wordmark.svg
│   └── fallback/
│       └── ... audited Breeze subset
│
├── wallpapers/
│   └── default.*
│
└── defaults/
    ├── settings
    ├── theme
    └── shortcuts
```

---

# 6. `flamewm::Runtime` lifecycle simulation

The Flame runtime is the top-level coordinator inside the IceWM process.

It is not a second window manager.

## 6.1 Responsibilities

`flamewm::Runtime` owns lifecycle/coordinator references for:

- Flame settings snapshot;
- palette/metrics;
- launcher controller;
- snap controller;
- overview controller;
- workspace UX helper;
- panel services/controllers;
- D-Bus dispatcher;
- optional diagnostics hooks;
- Flame control/IPC command routing.

It does **not** own:

- authoritative window list;
- workspace count;
- frame geometry;
- focus order;
- task state;
- EWMH state.

Those remain IceWM-owned.

## 6.2 Construction order

The agent must trace exact current initialization, but the intended order is:

```text
X application initialized
IceWM config/theme loaded
YWindowManager created
Flame configuration loaded
flamewm::Runtime constructed with references to existing app/manager
Flame controllers become available
Taskbar constructed and asks Runtime/PanelComposer for Flame widgets
normal client management begins
```

Do not construct Flame controllers before the X/event-loop infrastructure and required IceWM authorities exist.

## 6.3 Destruction order

Destroy/unsubscribe Flame resources before the underlying app/manager/X objects disappear.

Particularly:

- remove D-Bus watches;
- disconnect libpulse callbacks;
- cancel timers;
- destroy snap/overview popups;
- release XDamage resources;
- clear frame references/listeners.

## 6.4 Dependency injection

Inside new Flame code, prefer constructor-injected references.

Example conceptual dependency:

```text
Runtime
 ├─ SnapController(YWindowManager&, settings, palette)
 ├─ Launcher(YWMApp&, appModel, settings)
 ├─ PanelServices(dbusDispatcher, pulseClient)
 └─ Overview(YWindowManager&, thumbnailFactory)
```

Do not proliferate new global variables merely because upstream IceWM historically uses some globals.

---

# 7. Upstream IceWM touchpoint plan

The agent may touch upstream source only in documented, minimal places.

Create `docs/UPSTREAM_TOUCHPOINTS.md` before feature hooks begin.

## 7.1 `src/wmapp.*`

Potential Flame changes:

- construct/destroy `flamewm::Runtime`;
- route Start/Overview/Flame control commands;
- expose the smallest safe runtime accessor if needed.

Do not paste launcher/settings/service business logic here.

## 7.2 `src/movesize.cc`

Potential Flame change:

- notify snap controller during active move pointer updates/release;
- notify/cancel drag-only workspace-edge dwell logic.

All geometry/policy lives under `src/flamewm/snap/**`.

## 7.3 `src/wmframe.*`

Potential changes:

- generic helper needed to apply a calculated outer region through IceWM size-hint/work-area constraints;
- minimal per-frame snap metadata if it truly must live with frame lifetime;
- callbacks when manual move/resize/maximize/fullscreen invalidates Flame snap state.

Do not duplicate frame state in Flame code.

## 7.4 `src/wmbutton.*` / `src/wmtitle.*`

Potential change:

- maximize-button enter/leave/hover hooks to Flame snap chooser;
- preserve normal maximize click and titlebar double-click semantics exactly.

## 7.5 `src/wmmgr.*`

Potential changes:

- indexed workspace mutation transaction;
- minimum generalization for vertical taskbar struts/work areas;
- work-area change notification for snapped-frame reflow;
- drag-only edge destination helper if required.

This is high-risk code. Flame policy/UI must not accumulate here.

## 7.6 `src/workspaces.h`

Potential change:

- indexed insert/remove model primitives.

## 7.7 `src/aworkspaces.*`

Potential change:

- route workspace context menu to Flame UX;
- Add Desktop / Remove This Desktop;
- preserve existing useful middle-click/window-list behavior unless product decision explicitly replaces it.

## 7.8 `src/wmtaskbar.*`

Potential change:

- allow Flame PanelComposer to contribute widgets;
- orientation/strut/layout generalization;
- taskbar direct dock movement hooks.

`TaskBar` remains layout/lifetime host. It should not gain NetworkManager, MPRIS, PulseAudio, or settings business logic.

## 7.9 `src/atasks.*` / `src/objbar.*`

Potential changes:

- provide/consume stable application identity;
- Flame task renderer state hooks;
- pin/unpin unification.

Do not replace IceWM task state authority.

## 7.10 `src/wmswitch.*` / `src/preview.*`

Default approach:

- configure/style first;
- only generalize lower-level preview primitive if Overview needs reuse.

Never turn the Alt+Tab controller itself into Overview.

## 7.11 `src/aclock.*`

Only modify if exact Flame time/date layout cannot be reached through existing format/theme configuration.

## 7.12 `src/icesm.cc`

Later phase only:

- start/restart `flamewm-desktop` as appropriate;
- do not entangle desktop process with WM critical lifecycle.

## 7.13 Build files

Expected:

```text
CMakeLists.txt / configure.ac        dependency discovery only if required
src/CMakeLists.txt
src/Makefile.am
lib/CMakeLists.txt
lib/Makefile.am
```

Every new executable/source/test/resource must remain correctly registered in supported build systems.

## 7.14 Session/desktop files

Late release phase:

Prefer **adding** FlameWM-specific files:

```text
lib/flamewm.desktop
lib/flamewm-session.desktop
```

rather than destructively replacing upstream `icewm.desktop` immediately.

This keeps compatibility and upstream merge clarity.

---

# 8. Hook marker and ledger rules

When Flame-specific code must be added inside an upstream file:

```cpp
// FLAMEWM-UPSTREAM-HOOK-BEGIN(<stable-id>): <why this hook exists>.
...
// FLAMEWM-UPSTREAM-HOOK-END(<stable-id>)
```

Example conceptual form:

```cpp
// FLAMEWM-UPSTREAM-HOOK-BEGIN(snap-drag): route active move gestures to Flame snap UX.
flamewmRuntime().snap().onMovePointer(*this, rootX, rootY);
// FLAMEWM-UPSTREAM-HOOK-END(snap-drag)
```

Rules:

- marker surrounds only Flame-specific fork delta;
- no nested markers;
- stable ID recorded in `UPSTREAM_TOUCHPOINTS.md`;
- if marked block grows large, extract logic into `src/flamewm/**`;
- generic upstream bug fixes should remain clean upstreamable commits, not be unnecessarily branded as product hooks.

Each touchpoint ledger entry contains:

```text
ID
upstream file(s)
reason
Flame owner
state authority preserved
behavioral invariant
unit/integration tests
merge conflict risk
upstreamable: yes/no
```

---

# 9. Theme and visual-foundation simulation

The first visible Flame milestone should make mostly-stock IceWM already feel intentional before major new functionality lands.

## 9.1 Starting reference

Use Arc-Dark as a source-reading/design reference because it demonstrates a flatter, more modern IceWM theme structure than legacy defaults.

Do not copy it wholesale.

## 9.2 Create original Flame themes

Runtime targets:

```text
lib/flamewm/themes/FlameWM-Light/
lib/flamewm/themes/FlameWM-Dark/
```

Use the IceWM theme system first for:

- titlebar height;
- borders;
- active/inactive frame surfaces;
- taskbar background;
- task button states;
- workspace button states;
- menu colors;
- fonts;
- title buttons;
- QuickSwitch surfaces where theme/config supports them.

Only change renderer source where the approved UI cannot be reached cleanly through theme resources.

## 9.3 Semantic design tokens

Flame-owned code must not scatter literal reds/grays/spacing constants.

Define a semantic palette and metrics source:

```text
FlamePalette
    accent
    accentHover
    accentPressed
    accentMuted
    surface
    surfaceElevated
    textPrimary
    textSecondary
    border
    danger

FlameMetrics
    titlebarHeight
    frameVisualBorder
    frameResizeHitArea
    taskbarCompact
    taskbarDefault
    taskbarLarge
    popupRadiusOrEquivalent
    menuRowHeight
    menuPadding
    iconRoleSizes
    desktopGridCell
```

The exact values come from approved mockups and actual IceWM rendering tests.

## 9.4 Icon roles

Centralize icon sizing:

```text
TitleButton
Taskbar
TaskbarStatus
Menu
Launcher
SettingsCategory
Desktop
Overview
```

The icon resolver requests the correct semantic role and runtime scale. Individual pages must not directly guess SVG dimensions.

## 9.5 Default product visibility

Initial Flame defaults should hide legacy clutter rather than delete upstream code:

```text
CPU graph              off
memory graph           off
legacy throughput net  off
mailbox                 off
address bar             off unless explicitly needed
Start                   on
workspaces              on
running tasks           on
system tray             on
clock/date              on
QuickSwitch previews    on
```

## 9.6 Simulation failure: copying Arc-Dark as FlameWM

**Wrong path:** agent copies Arc-Dark assets, changes blue to red, calls it FlameWM.

Why wrong:

- derivative visual identity rather than intentional product;
- unclear asset/license provenance;
- inherits Arc/IceWM quirks we specifically want to eliminate;
- creates a poor foundation for light/dark semantic tokens.

Correct response:

- use Arc-Dark only as a capability/reference audit;
- create original Flame theme resources and metrics.

## 9.7 Simulation failure: missing final Start SVG

The agent must not block all engineering because final branding art is missing.

Allowed development behavior:

- use a clearly marked temporary placeholder Flame glyph under development assets.

Not allowed:

- silently ship the placeholder as final;
- derive the start button from a generic Windows glyph;
- rasterize the wide wordmark into a 20px taskbar square.

Release gate remains blocked until final `flamewm-start.svg` is provided/approved.

---

# 10. Phase-by-phase simulated implementation

Each phase below describes what the future coding agent would do and what failures we simulate before real execution.

---

# Phase 0 — Fork governance, baseline, and development laboratory

## Objective

Create the conditions under which every later change is measurable, reversible, testable, and upstream-mergeable.

## Agent actions

1. Verify clean Git state and exact HEAD.
2. Configure remotes:

```text
origin   -> linsaftw/flamewm
upstream -> ice-wm/icewm
```

3. Enable:

```bash
git config rerere.enabled true
```

4. Add planning/governance files:

```text
docs/UPSTREAM_TOUCHPOINTS.md
tools/check-upstream-divergence.py
project_sources/**
```

5. Establish CMake build/test baseline.
6. Establish Autotools build baseline.
7. Create Xephyr `./dev` laboratory.
8. Record PSS/RSS/startup baseline.
9. Add CI before feature divergence.
10. Independently fix/test the source-proven center-tile non-zero-origin bug as an upstreamable generic correctness commit before using that geometry for Flame layouts.

## Simulated failure A — Missing build dependencies

Observed class:

```text
pkg-config cannot find XRender/fontconfig/etc.
```

Wrong response:

- modify source/build scripts to bypass a real required dependency;
- claim source is broken.

Correct response:

- classify as environment failure;
- install/document upstream dependency set;
- rerun same baseline command;
- no Flame feature coding until clean.

## Simulated failure B — CMake green, Autotools red

Wrong response:

- ignore Autotools because developer personally uses CMake.

Correct response:

- trace whether task caused breakage;
- repair registration in `Makefile.am`/`configure.ac` where required;
- preserve upstream-supported paths until explicit project decision drops one.

## Phase gate

```text
CMake build green
CTest baseline green
Autotools build green
nested WM boots
baseline metrics recorded
zero unexplained upstream diff
```

---

# Phase 1 — Flame visual/default foundation

## Objective

Make existing IceWM functionality already look and behave like the Flame product before new stateful systems are added.

## New/runtime assets

```text
lib/flamewm/themes/FlameWM-Light/**
lib/flamewm/themes/FlameWM-Dark/**
lib/flamewm/icons/**
lib/flamewm/wallpapers/**
lib/flamewm/defaults/**
```

## Agent actions

1. Stage approved wordmark, temporary/final Start icon, wallpaper.
2. Stage Breeze license/version + curated fallback subset.
3. Build semantic Flame palette/metrics/icon-role layer.
4. Create original light/dark theme resources.
5. Disable legacy applets in Flame defaults.
6. Configure QuickSwitch previews and intentional default layout.
7. Preserve existing titlebar double-click maximize.
8. Clean default menus of IceWM configuration clutter at the product-default layer without deleting upstream capability.
9. Verify at multiple DPIs/scales if supported by current Xft/rendering path.

## Simulated failure — Theme-only approach cannot produce a required visual

Correct decision tree:

```text
Can existing theme resource express it?
    yes -> use theme
    no -> can a small Flame renderer extension do it?
        yes -> add isolated Flame renderer/hook
        no -> re-evaluate whether visual requirement is worth invasive upstream change
```

No broad renderer rewrite merely for a few pixels of cosmetic parity.

## Phase gate

- normal window operations unchanged;
- double-click titlebar maximize still works;
- minimize/maximize/close clear and modern;
- taskbar/menu/workspace/QuickSwitch visually coherent;
- fallback icons work if Breeze theme unavailable;
- no compositor needed for baseline readability.

---

# Phase 2 — Flame runtime, settings schema, and control channel

## Objective

Build the architectural spine before feature business logic.

## Create

```text
src/flamewm/core/runtime.*
src/flamewm/core/config.*
src/flamewm/core/control.*
src/flamewm/core/version.*
src/flamewm/ui/palette.*
src/flamewm/ui/metrics.*
src/flamewm/ui/iconroles.*
```

## Settings storage

Use Flame-owned config, e.g.:

```text
$XDG_CONFIG_HOME/flamewm/settings
```

Suggested precedence:

```text
IceWM upstream defaults
-> Flame shipped defaults/theme
-> Flame user settings
-> expert IceWM prefoverride
```

`flamewm-settings` never rewrites an entire upstream preferences file.

## Persistence

Use atomic write:

```text
validate
write temp
flush/close
rename temp -> settings
```

Unknown keys are preserved where practical for forward/backward compatibility.

## Control protocol

Implement one narrow, versioned Flame control path, conceptually:

```text
flamewmctl reload-settings
flamewmctl open-settings <page>
flamewmctl show-launcher
flamewmctl show-overview
flamewmctl diagnostics
```

An X11 ClientMessage/property-based protocol is appropriate for X11-first FlameWM.

No arbitrary shell execution.

## Simulated failure — Agent puts every Flame setting in `src/default.h`

Wrong because:

- grows upstream diff permanently;
- mixes product UX schema with upstream expert preferences;
- makes Settings ownership ambiguous;
- increases merge conflict surface.

Correct:

- keep mainstream Flame preferences in Flame config;
- only generalize upstream preference machinery when a setting genuinely belongs to IceWM engine state.

## Phase gate

- Runtime initializes/destructs cleanly;
- config round-trip tests green;
- invalid values fall back safely;
- live reload command works for a harmless test token;
- no duplicate authority created.

---

# Phase 3 — `flamewm-settings` foundation

## Objective

Create the friendly configuration layer before advanced features depend on it.

## Initial pages

```text
Appearance
Desktop
Taskbar
Start
Windows
Workspaces
Shortcuts
About / Diagnostics
```

## Appearance

- Light / Dark / System where meaningful;
- Flame red + curated presets;
- custom color picker;
- reset appearance.

## Desktop

- wallpaper chooser;
- fit mode;
- watermark toggle;
- desktop icons toggle once available.

## Taskbar

Start with currently supported product-quality options:

- Bottom / Top initially;
- Compact / Default / Large;
- auto-hide if already robust;
- later Left / Right when vertical phase lands.

Do not show unsupported controls just because future architecture lists them.

## Windows

- snap enabled;
- snap chooser enabled;
- QuickSwitch preview toggle only if product needs it;
- no focus-policy laboratory.

## Workspaces

- count;
- Add Desktop;
- Remove Desktop;
- minimum one.

## Shortcuts

Expose only curated Flame actions.

Key-capture rules:

- capture chord;
- validate;
- detect duplicate Flame binding;
- warn before replacing;
- restore defaults.

## Simulated failure — Settings rewrites `preferences`

Wrong:

- destroys comments/expert settings;
- creates competing configuration authority;
- can silently reset unrelated IceWM behavior.

Correct:

- Flame-owned file + explicit translation/apply layer.

## Simulated failure — Apply button for every cosmetic setting

Product goal requires live safe personalization.

Correct:

- persist/apply immediately for safe appearance settings;
- only use restart/confirmation when technically necessary.

## Phase gate

Normal user can change basic Flame appearance/taskbar/workspace/shortcuts without editing a text file.

---

# Phase 4 — Modern snap system

## Objective

Build Windows/KWin-style mouse snap UX while preserving existing IceWM geometry/state authority.

## Existing source to reuse

`YFrameWindow::wmTile()` already provides half/quarter/top/bottom/center geometry foundations.

Do not build a second WM geometry engine.

## New Flame files

```text
src/flamewm/snap/layout.*
src/flamewm/snap/state.*
src/flamewm/snap/overlay.*
src/flamewm/snap/chooser.*
```

## Layout representation

Use data-driven exact regions.

Conceptual region:

```text
denominatorX
denominatorY
startX
startY
spanX
spanY
```

For work width `W`, denominator `D`, start `i`, span `s`:

```text
left  = workLeft + floor(i       * W / D)
right = workLeft + floor((i + s) * W / D)
width = right - left
```

Same vertically.

This shared-edge rule prevents one-pixel gaps on odd resolutions.

## Drag targets

```text
left edge       -> LeftHalf
right edge      -> RightHalf
top-left        -> TopLeftQuarter
top-right       -> TopRightQuarter
bottom-left     -> BottomLeftQuarter
bottom-right    -> BottomRightQuarter
top center      -> Maximize
bottom center   -> None
```

## Rich chooser layouts

Maximize-button hover chooser may offer:

```text
1/2 + 1/2
2/3 + 1/3
1/3 + 2/3
1/3 + 1/3 + 1/3
four quarters
left half + two right quarters
```

## Snap state

First snap stores unsnapped outer geometry.

Rules:

- snapping among regions does not overwrite original floating geometry;
- manual resize exits snap state;
- drag-away restores floating geometry before continuing move;
- maximize remains IceWM maximize state;
- fullscreen wins over snap;
- monitor removal clamps restore target to a surviving work area.

## Hover chooser state machine

```text
IDLE
 -> maximize enter -> ARM_TIMER
ARM_TIMER
 -> leave before delay -> IDLE
 -> delay fires -> POPUP_OPEN
POPUP_OPEN
 -> pointer on button/popup -> stay open
 -> pointer leaves union -> CLOSE_GRACE
 -> region click -> apply + close
 -> normal maximize click -> maximize + close
 -> Esc/frame destroy/workspace switch/fullscreen -> close
```

## Workspace edge conflict

Corners belong to quarter snap.

For left/right center edges while dragging:

1. immediately show half-snap preview;
2. quick release commits snap;
3. sustained dwell at extreme edge (~450ms product constant/preference) changes workspace instead;
4. snap preview cancels while workspace traversal occurs;
5. dragged window follows to destination.

Passive pointer edge switching remains off by default.

## Simulated failures

### Failure A — 1px gaps in thirds

Cause: independently rounding widths.

Fix: shared-edge integer partition formula.

### Failure B — Correct on primary monitor, wrong on secondary

Cause: root-origin assumptions.

Fix: manager work area for frame's actual screen; tests with positive/negative output origins.

### Failure C — Snap chooser breaks maximize click

Cause: hover popup intercepts button lifecycle/grab.

Fix: chooser is delayed hover only; normal maximize click path unchanged.

### Failure D — Popup closes while pointer moves from button to popup

Fix: union-of-button-and-popup hover ownership + short close grace timer.

### Failure E — Restore size is wrong after re-snapping

Cause: overwrite saved floating geometry each snap.

Fix: capture only on first transition from floating -> snapped.

### Failure F — fixed/min-size application violates hints

Fix: route through existing IceWM size-hint constraint path; anchor constrained result within target region.

### Failure G — stale `YFrameWindow*` after app closes

Fix: lifecycle subscription/ownership; popup/controller removes references before destruction; integration test closes target while chooser visible.

## Phase gate

All snap geometry/state/X11 tests green, including existing double-click and maximize behavior.

---

# Phase 5 — Workspace UX and indexed mutation

## Objective

Turn existing IceWM runtime workspaces into simple Flame virtual desktops.

## Existing source

IceWM already supports runtime extend/shrink and EWMH desktop updates.

Missing product behavior:

- add after selected workspace;
- remove selected middle workspace;
- minimum one;
- deterministic migration/reindex.

## Upstream owner

`YWindowManager` remains transaction authority.

## New/generalized operations

Conceptually:

```cpp
bool insertWorkspace(int index);
bool removeWorkspace(int index);
```

Actual API should follow current source conventions.

## Remove transaction

For old count `N`, remove `R`:

1. reject if `N <= 1`;
2. validate `R`;
3. choose deterministic surviving neighbor;
4. move windows from removed workspace;
5. reindex windows above `R`;
6. remove model entry;
7. update active/last workspace indices;
8. publish EWMH desktop count/current/names/viewports/workarea;
9. refresh pager/window list/move menus;
10. restore valid focus.

One transaction owns the whole state change.

## Workspace context menu

```text
Add Desktop
Remove This Desktop
```

`Remove` disabled when count is one.

Custom naming is not core Flame UX; underlying IceWM capability may remain expert-accessible.

## Simulated failure — agent calls `lessenWorkspaces(count - 1)`

Result: right-clicking Desktop 2 removes the last desktop instead.

This is explicitly rejected.

## Simulated failure — windows disappear after middle deletion

Cause: model count changed but frame/EWMH indices not updated as one transaction.

Fix: manager-owned mutation with pure index-mapping tests before X integration.

## Phase gate

First/middle/last insertion/removal verified with real windows and EWMH properties.

---

# Phase 6 — Flame Launcher, pinned/running app identity, taskbar task UX

## Objective

Replace the user-facing classic Start experience with a searchable, familiar launcher while keeping upstream menu capability as fallback/expert functionality.

## Reuse

- existing Freedesktop `.desktop` parser/logic from `fdomenu.cc` where extractable;
- `YInputLine`;
- `YListBox` / native list/grid primitives;
- `YIcon`;
- existing launch/session actions.

## New Flame files

```text
src/flamewm/launcher/appmodel.*
src/flamewm/launcher/search.*
src/flamewm/launcher/launcher.*
src/flamewm/panel/taskidentity.*
```

## App model

Cache installed application metadata.

Do not rescan filesystem on every keystroke.

Store:

```text
desktop application ID
localized Name
GenericName
Icon
Categories
Exec parsed representation
Terminal
NoDisplay/Hidden
StartupWMClass
normalized search key
```

## Safe launching

Do not concatenate `.desktop` Exec or user search text into `/bin/sh -c`.

Preserve Freedesktop field-code semantics and argument-vector execution.

## Stable application identity

Preferred authority:

1. desktop application ID;
2. `.desktop` `StartupWMClass`;
3. X11 `WM_CLASS`;
4. normalized fallback.

Never identify an app by window title.

## Unified pinned/running behavior

States:

```text
Pinned/not running
Running/inactive
Focused
Minimized
Multiple windows
```

Existing IceWM task state remains authoritative; Flame renderer maps it to modern indicators.

Pinning stores desktop application IDs.

## Launcher keyboard UX

```text
Super -> open
Super again -> close
typing -> filter
arrows -> navigate
Enter -> launch
Esc -> close
```

## Simulated failure A — app identity by title

Breaks for dynamic document titles, localization, browser tabs.

Reject.

## Simulated failure B — directory scan on each keypress

Produces lag and I/O churn.

Fix: cached model + invalidation/watch/reload.

## Simulated failure C — raw shell interpolation

Creates injection/safety bugs.

Fix: parsed desktop entry + argv execution.

## Phase gate

Launcher warm-open is immediate, keyboard complete, and task identity survives title changes/multiple windows.

---

# Phase 7 — Four-edge taskbar and direct docking

## Objective

Extend IceWM's horizontal top/bottom taskbar into a Flame product taskbar that can live on all four edges without an edit mode.

This phase is deliberately separate because upstream code currently assumes a horizontal bar in many layout decisions.

## Product representation

Use a real enum in Flame/product logic:

```text
Bottom
Top
Left
Right
```

Do not encode left/right as magic values in an old boolean.

## Work required

- taskbar outer geometry;
- `_NET_WM_STRUT`/work-area reservation for vertical edges;
- child layout orientation;
- task buttons;
- workspace controls;
- tray;
- clock/date;
- media/audio/network widgets;
- popup anchoring;
- autohide if retained;
- direct drag docking.

## Child layout contract

Each Flame/taskbar component needs:

```text
horizontal preferred size
vertical preferred size / policy
collapse priority
```

## Vertical UX

Do not rotate ordinary text as a cheap hack.

Prefer:

- icon-first task buttons;
- compact workspace indicators;
- vertical-friendly status icons;
- clock/date layout designed for vertical width.

## Direct docking gesture

1. hold unused taskbar region;
2. drag toward screen edge;
3. show lightweight edge target;
4. release commits new edge;
5. layout/strut recomputes immediately;
6. no edit mode.

## Simulated failure A — modifying `TaskBarAtTop` into a magic four-state integer

Reject. That destroys semantic clarity and can make upstream merge/conflict behavior confusing.

Generalize boundary cleanly while keeping product orientation in Flame-owned code.

## Simulated failure B — vertical taskbar reserves wrong work area

Result: maximized/snapped windows overlap panel or lose too much screen.

Test exact struts on all edges and multiple monitors.

## Simulated failure C — popovers open off-screen

Every popup anchors based on current panel edge and monitor work area.

## Simulated failure D — horizontal-only child overflow

Use deterministic collapse policy; never hide critical clock/network/audio/tray unpredictably.

## Phase gate

Bottom/top/left/right all pass work-area, popup, task, tray, workspace, and snap integration tests.

---

# Phase 8 — Native system controls

## Objective

Provide familiar network/audio/media taskbar surfaces without importing a heavy desktop stack or polling subprocesses continuously.

## Shared D-Bus dispatcher

Create:

```text
src/flamewm/integrations/dbusdispatcher.*
```

Integrate libdbus watches/timeouts into the existing IceWM event loop/poll/timer infrastructure.

Use for:

```text
session bus -> MPRIS
system bus  -> NetworkManager
```

No GLib event loop.

## NetworkManager integration

Create:

```text
networkmanager.*
panel/network.*
```

Model:

```text
unavailable
no network
wired
wifi connecting
wifi connected + SSID + strength
```

Popup scope:

- current connection;
- Wi-Fi toggle where appropriate;
- visible networks;
- signal;
- connect saved profile;
- disconnect;
- hand off complex/new-secret setup to host network editor until a secure SecretAgent exists.

### Security invariant

Never log/pass a new Wi-Fi secret through a shell string. Avoid exposing secrets in argv/process listings.

## Audio integration

Use async `libpulse` API; it works against PulseAudio and PipeWire-Pulse compatibility layer.

Create:

```text
pulse.*
panel/audio.*
```

State:

```text
default sink
volume
mute
sink changes
server reconnect
```

UI:

- muted/low/medium/high icon;
- click popup;
- slider;
- mute;
- scroll volume;
- optional current output.

No periodic `pactl` polling.

## MPRIS integration

Create:

```text
mpris.*
panel/media.*
```

Active-player rule:

1. currently Playing;
2. otherwise most recently active/changed;
3. otherwise first available;
4. none -> media widget collapses.

Popup:

```text
Track
Artist
Previous / Play-Pause / Next
```

No album-art downloader/cache required for first complete product.

## Re-entrancy simulation

D-Bus/libpulse callbacks may arrive while the WM is already in an event dispatch path.

Rule:

- trace callback call graph;
- do not recursively trigger a state mutation that routes back into the same service/UI callback;
- when needed, queue/defer a presentation update through the legal event loop rather than assuming a scheduled callback is always deferred.

## Simulated failure A — blocking D-Bus call in click handler

Result: frozen desktop when service stalls.

Reject. Use asynchronous requests/watchers.

## Simulated failure B — periodic `nmcli`/`playerctl`/`pactl` every second

Result: avoidable process churn and wakeups.

Reject as final architecture.

## Simulated failure C — missing NetworkManager/Pulse service crashes WM

Correct fallback:

- integration enters unavailable state;
- applet hides or shows concise unavailable state;
- WM remains fully functional;
- retries are event/backoff controlled, not log spam.

## Phase gate

Idle system controls are event-driven and cause no high-frequency wakeup loop.

---

# Phase 9 — Overview / Activities

## Objective

Provide one simple “show my windows and desktops” overview without duplicating QuickSwitch semantics.

## Reuse

`Preview`/`SwitchPreview` already demonstrate XDamage/XRender live thumbnails.

Do not make `SwitchPreview` itself the Overview controller.

Extract/share lower-level thumbnail functionality if needed.

## New Flame files

```text
src/flamewm/overview/thumbnails.*
src/flamewm/overview/overview.*
```

## Lifecycle

Resources exist only while Overview is open:

```text
open -> enumerate eligible frames -> create thumbnail resources
updates -> mark dirty/coalesce paint
close -> destroy XDamage/pictures/timers/references
```

Idle hidden Overview cost must be effectively zero.

## Eligibility

Reuse existing skip semantics where applicable:

- skip internal/taskbar/desktop/dock windows;
- respect relevant preview/pager exclusions;
- include ordinary visible/minimized apps according to product rule.

## UX

- Overview button toggles;
- keyboard shortcut;
- active monitor-aware overlay;
- window thumbnails;
- workspace strip;
- click window activates/closes Overview;
- click workspace switches;
- Esc closes.

Do not introduce KDE Activities semantics.

## Simulated failure A — stale frame pointer

Close an application while Overview is open in integration tests.

Controller must remove thumbnail safely before use.

## Simulated failure B — XDamage leak

Open/close Overview 1,000 times and verify resource/memory stability.

## Simulated failure C — repaint storm

Do not repaint entire overview for each damage event; mark affected thumbnail dirty and coalesce redraw.

## Phase gate

No hidden resource use, no leak, no Alt+Tab regression.

---

# Phase 10 — `flamewm-desktop`

## Objective

Provide filesystem desktop icons/grid and subtle Flame branding in a process isolated from WM correctness.

## Process ownership

`flamewm-desktop` owns:

- XDG Desktop directory resolution;
- filesystem model;
- inotify watcher;
- grid layout;
- icon selection/open;
- drag/reorder;
- position persistence;
- desktop context menu;
- FlameWM watermark.

`icewmbg` continues to own wallpaper initially.

## Desktop directory

Never hardcode `~/Desktop`.

Resolve XDG user Desktop directory with safe fallback.

## Model

```text
DesktopItem
    absolutePath
    displayName
    kind
    icon identity
    output
    grid slot
```

Kinds:

```text
Directory
RegularFile
DesktopLauncher
```

## Watcher

Use inotify; react to:

- create;
- delete;
- rename/move;
- relevant attribute change.

Coalesce bursts.

No periodic full directory polling.

## Grid

Per-output usable work rectangle excludes panel strut.

New item -> first free deterministic cell.

Drag -> target-cell preview -> release.

Occupied target -> deterministic swap.

## Persistence

Use Flame-owned atomic layout config, e.g.:

```text
$XDG_CONFIG_HOME/flamewm/desktop-layout
```

Persist layout metadata only; filesystem is the content authority.

## Open behavior

- normal file/directory -> standard default opener (`xdg-open` equivalent/host mechanism);
- `.desktop` -> reuse shared desktop-entry parsing/launch logic where possible;
- never keep a full file manager resident solely for desktop icons.

## Watermark

Use approved Flame wordmark/mark:

- bottom-right of usable desktop;
- safe taskbar margin;
- non-interactive;
- behind normal windows;
- hidden/covered in fullscreen naturally;
- adapts when taskbar moves.

## Desktop context menu

If filesystem desktop is enabled:

```text
New Folder
────────────
Change Background
Desktop Settings
```

Do not include terminal, random apps, raw IceWM settings, or workspace controls.

## Simulated failure A — implementing desktop files inside WM process

Rejected due to:

- filesystem/DND failure domain;
- crash impact;
- merge surface;
- responsiveness risk.

## Simulated failure B — desktop process owns wallpaper too

Creates competing background authority with `icewmbg`.

Correct: `icewmbg` owns wallpaper initially; desktop process owns icons/watermark/context surface.

## Simulated failure C — hardcoded `~/Desktop`

Breaks localized/custom XDG user directories.

Reject.

## Simulated failure D — item positions under moved vertical taskbar

On work-area/output change, recompute usable grid and relocate only invalid/out-of-bounds assignments deterministically.

## Process isolation test

Kill `flamewm-desktop` repeatedly.

Expected:

- WM remains fully usable;
- wallpaper remains;
- process can restart and restore grid safely.

## Phase gate

Filesystem operations cannot destabilize the WM.

---

# Phase 11 — Diagnostics and performance engineering

## Objective

Make “lightweight” measurable and make resource culprits understandable without turning diagnostics into a resident cost.

## Development/release profiler

Create:

```text
tools/flamewm-profile
```

Record:

- PSS;
- RSS;
- private dirty;
- swap;
- threads;
- fd count;
- process count;
- idle CPU;
- context switches/wakeups where practical;
- startup to usable taskbar;
- launcher cold/warm latency;
- Overview open latency.

Measure every Flame-owned persistent process, not only the main WM PID.

## User-facing Performance page

Location:

```text
Settings -> About / Diagnostics -> Performance
```

It samples only while visible.

View:

```text
FlameWM footprint
  WM
  background
  desktop
  total

Top applications now
  process | memory | CPU
```

## Efficient data sources

For process list:

```text
/proc/<pid>/stat
/proc/<pid>/status
```

For Flame-owned/selected accurate PSS:

```text
/proc/<pid>/smaps_rollup
```

Do not read full `smaps` for every process every second.

## Simulated failure — profiler becomes a daemon

Reject.

The user-facing sampler stops all timers/scans when page closes.

## Release metric

The 40 MiB combined-owned-PSS goal remains a target to prove, not a fact to assume.

Every release reports measured values and exact offenders if over budget.

---

# Phase 12 — Release branding, session integration, and packaging

## Objective

Finish visible product identity after behavior is stable, without mass-renaming the upstream engine.

## Branding tasks

- FlameWM version metadata;
- About page;
- product name;
- ArkFlame Studios attribution;
- IceWM base version;
- project homepage;
- license notices;
- Flame session desktop files;
- package metadata;
- final icons/wallpaper.

## Do not mass-rename

Do not rename every:

- `YWindow*` class;
- IceWM internal global;
- EWMH atom;
- source filename;
- expert preference key.

Visible product can be FlameWM while internal engine ancestry remains clear.

## Session integrations to package/document

Friendly desktop session needs:

- lightweight freedesktop notification daemon;
- graphical Polkit agent;
- secure screen locker referenced by `LockCommand`;
- display configuration path;
- default file opener/file manager.

These are host/session responsibilities, not reasons to implement those security/system subsystems inside WM core.

## Final upstream merge rehearsal

Before first serious release:

1. fetch latest upstream;
2. create `sync/icewm-<version>` branch;
3. merge upstream;
4. inspect every conflict against `UPSTREAM_TOUCHPOINTS.md`;
5. verify upstream has not superseded any Flame implementation;
6. remove redundant Flame code if upstream now solves it;
7. run complete gates;
8. record merge friction.

If upstream merge is unexpectedly painful, treat that as an architectural defect and reduce upstream divergence before release.

---

# 11. Simulated daily agent workflow

A coding agent should work like this for every bounded implementation session.

## Morning / task start

```text
1. git status
2. record HEAD/upstream merge-base
3. read exact task contract and project sources
4. inspect current source owners/call graph
5. run relevant baseline tests
6. create feature branch
```

## Before first source edit

The agent writes a small task ledger:

```text
USER BEHAVIOR
ICEWM AUTHORITY
FLAME OWNER
UPSTREAM FILES ALLOWED
NEW FILES
ASSETS
UNIT TESTS
X11 TESTS
PERFORMANCE CHECK
FALLBACK
```

If any row is unknown because source has not been inspected, source reading continues before editing.

## Implementation loop

```text
pure model/math first
-> unit test
-> compile
-> tiny upstream hook
-> integration test
-> nested visual verify
-> resource sanity
-> upstream divergence audit
-> commit
```

## End of session

The agent records:

```text
what changed
what upstream files were touched
which touchpoint IDs were added/updated
exact tests run
resource delta
known remaining limitations
whether asset placeholders remain
whether all gates are green
```

No “DONE” if a required gate is red.

---

# 12. Commit simulation

A clean history might evolve approximately like this. Exact commits should remain smaller than this list where helpful.

```text
0001 docs: add FlameWM project source manifest and upstream touchpoint ledger
0002 ci: preserve upstream build gates and add Flame divergence audit
0003 tooling: add nested Xephyr dev launcher and baseline profiler
0004 fix: correct center tile geometry on non-zero monitor origins
0005 feat(theme): add FlameWM visual foundation and runtime assets
0006 feat(core): add Flame runtime, config, palette, metrics and control channel
0007 feat(settings): add on-demand FlameWM Settings foundation
0008 feat(snap): add data-driven regions and drag snap overlay
0009 feat(snap): add restore state and maximize hover chooser
0010 feat(workspaces): add indexed workspace mutation and friendly pager actions
0011 feat(launcher): add cached searchable application launcher
0012 feat(taskbar): unify pinned/running application identity and rendering
0013 feat(taskbar): add four-edge orientation and direct docking
0014 feat(integrations): add D-Bus dispatcher and NetworkManager integration
0015 feat(integrations): add async PulseAudio/PipeWire-Pulse volume control
0016 feat(integrations): add MPRIS media controls
0017 feat(overview): add Flame overview using shared thumbnail primitives
0018 feat(desktop): add isolated filesystem desktop and watermark
0019 feat(diagnostics): add on-demand performance diagnostics
0020 release: add FlameWM session/package branding and final assets
```

Important:

- generic upstream bug fixes should be isolated from Flame-specific changes;
- do not combine mass formatting/refactor with feature commits;
- every commit should leave the branch buildable/testable whenever practical.

---

# 13. Failure/assertion matrix

This is the pre-mortem. Future agents should actively test these failure classes.

| Failure | Why it happens | Required assertion / prevention |
|---|---|---|
| Agent trusts old line numbers | upstream drift | open current file and trace current implementation before edit |
| Baseline already red | environment or pre-existing issue | classify and establish green baseline before feature work |
| CMake green, Autotools broken | only one build registration updated | maintain both while upstream supports both |
| Massive unrelated diff | formatter/refactor temptation | no drive-by formatting; divergence audit |
| Flame logic pasted into `wmmgr.cc` | easiest local insertion | extract product logic to `src/flamewm/**`; tiny hook only |
| Duplicate window/workspace state | Flame controller keeps its own authority | IceWM state owner remains authoritative |
| Theme copies Arc-Dark wholesale | reference mistaken for product asset | original Flame resources; provenance manifest |
| Breeze icons missing on non-KDE install | assume host theme installed | curated fallback subset + license/version metadata |
| SVG icons scale inconsistently | each widget chooses its own size | semantic `IconRole` resolver |
| Start icon is generic Windows glyph | placeholder becomes final | final Flame-specific square SVG release gate |
| Settings overwrites IceWM preferences | no config ownership boundary | Flame-owned config + explicit precedence |
| Cosmetic settings require restart | only full reload path used | delta apply for safe product-owned values |
| Snap thirds have 1px gaps | independent rounding | shared-edge integer partition |
| Snap wrong on second monitor | origin-zero geometry | use frame screen work area; negative-origin tests |
| Snap loses original floating geometry | overwritten on re-snap | capture once on floating->snap transition |
| Chooser breaks maximize | hover intercepts button semantics | maximize click path unchanged |
| Chooser UAF after window closes | stale frame pointer | lifecycle removal + destruction integration test |
| Edge snap prevents workspace traversal | both own same edge | quick snap + sustained dwell workspace rule |
| Passive pointer switches workspace | reuse old edge mode blindly | drag-only workspace traversal default |
| Remove workspace deletes last instead of clicked | tail shrink reused | indexed manager transaction |
| Windows disappear after workspace removal | partial index/EWMH update | one atomic manager operation + mapping tests |
| Launcher lags while typing | filesystem scan per keystroke | cached app model |
| Launcher can execute injected shell text | shell string construction | desktop-entry parser + argv execution |
| Pinned task mismatches app | title-based identity | desktop ID/StartupWMClass/WM_CLASS |
| Vertical taskbar overlaps windows | bad strut | four-edge strut/work-area tests |
| Vertical taskbar rotates unreadable text | horizontal UI reused literally | icon-first vertical-specific composition |
| Network popup freezes WM | sync D-Bus/CLI | asynchronous event-loop integration |
| Audio/media/network wakes CPU constantly | polling | event subscriptions |
| Wi-Fi password leaks | argv/log/shell | use saved profile/secret agent handoff; never log secrets |
| D-Bus callback recursively mutates state | re-entrancy | trace full callback graph; defer when necessary |
| Overview consumes CPU while closed | damage resources retained | construct/subscribe only while visible |
| Overview crashes after client closes | stale thumbnail reference | client-destroy lifecycle handling |
| Desktop icons crash WM | filesystem code in WM process | isolated `flamewm-desktop` |
| Desktop hardcodes `~/Desktop` | naive path assumption | XDG user-dir resolution |
| Desktop fights `icewmbg` | two wallpaper authorities | `icewmbg` remains wallpaper owner initially |
| Profiler is itself expensive | always-running full `/proc` scans | on-demand sampling; cheap stat/status; limited PSS |
| Upstream merge becomes painful | broad invasive changes | touchpoint ledger, markers, Flame-owned paths, sync rehearsal |

---

# 14. Conditions that force the coding agent to stop

The agent must stop the current implementation path—not continue guessing—when any of these occur:

## 14.1 Source baseline drift

If the repository version/architecture differs materially from the project-source reports:

```text
STOP feature coding
re-audit changed owners/APIs
update task plan
then continue
```

## 14.2 Required baseline gate is red

Do not build new features on unexplained build/test failures.

## 14.3 Required asset license/provenance is unresolved

Use temporary developer placeholder if feature logic can proceed, but final runtime inclusion/release remains blocked.

## 14.4 Proposed Flame feature requires hundreds of lines in a high-churn IceWM file

This is an architecture warning.

Redesign toward:

- general upstream helper;
- Flame-owned controller;
- narrow hook.

## 14.5 Two state authorities appear

Examples:

- both Flame and `YWindowManager` storing workspace truth;
- both Flame task model and IceWM task model deciding minimized/focused state;
- both `icewmbg` and desktop process changing wallpaper.

Stop and consolidate ownership.

## 14.6 Memory/idle CPU regression is unexplained

Do not handwave “still lightweight.”

Measure exact culprit before adding more persistent behavior.

## 14.7 User-facing requirement is genuinely contradictory

Use authority order and update project decision. Do not invent a third compromise setting simply to avoid deciding.

---

# 15. Required input checklist for the first real coding-agent handoff

Before the first implementation handoff begins, the handoff package should contain:

## Repository

- FlameWM repository URL;
- target branch;
- exact HEAD commit;
- upstream remote/merge-base;
- statement that workspace is clean or exact dirty-hunk classification.

## Project sources

- Product Soul & UX Constitution;
- Source-Grounded Implementation Report;
- Fork Source Report;
- Complementary Architecture/Maintainability Blueprint;
- this simulation/execution playbook.

## Visual/asset inputs

- current FlameWM wordmark PNG;
- **final FlameWM Start SVG — currently required/missing unless added before coding**;
- preferably vector wordmark SVG — currently required for final product-quality scaling;
- approved wallpaper or explicit temporary placeholder status;
- approved desktop/taskbar/settings/start/snap mockups;
- Arc-Dark reference theme snapshot;
- Breeze Icons source/version/license package or manifest.

## Environment

- target Linux distribution/version;
- compiler(s);
- X11/XRandR environment;
- required development packages;
- Xephyr/Xvfb availability;
- NetworkManager presence for integration phase;
- PipeWire-Pulse/PulseAudio presence for audio phase;
- D-Bus session/system availability.

## Verification

- baseline build commands;
- baseline tests;
- nested development command;
- baseline performance report;
- current upstream touchpoint ledger.

---

# 16. The first actual coding handoff should not implement FlameWM in one shot

This full simulation exists specifically to **avoid** a single massive implementation request.

The first coding handoff should likely cover only **Phase 0 + the smallest Phase 1 foundation**, for example:

```text
1. establish fork governance/touchpoint ledger/divergence tooling;
2. verify/build both supported build systems;
3. create nested Xephyr development lab;
4. record resource baseline;
5. import project-source references/assets safely;
6. create Flame theme skeleton and default asset installation path;
7. repair/test generic center-tile origin bug if still present;
8. no launcher/settings/snap/workspace/taskbar architecture yet unless the phase gate is green.
```

Why:

- every later handoff then inherits known-good build/test/measurement infrastructure;
- architecture can be adjusted from actual IceWM rendering behavior before heavy feature code exists;
- upstream divergence remains auditable from the first commit.

---

# 17. Final simulated architecture

At the end of the complete implementation, the intended runtime shape is:

```text
FLAMEWM SESSION
│
├── IceWM-derived WM process (FlameWM product binary/session identity)
│   │
│   ├── existing IceWM engine
│   │   ├── X11 client management
│   │   ├── focus/stacking
│   │   ├── EWMH/ICCCM
│   │   ├── frame lifecycle
│   │   ├── work areas/XRandR
│   │   ├── workspace state
│   │   ├── task state
│   │   ├── tray
│   │   ├── QuickSwitch
│   │   └── theme/icon infrastructure
│   │
│   └── flamewm::Runtime
│       ├── Flame settings snapshot/control
│       ├── semantic palette/metrics/icons
│       ├── snap controller/overlay/chooser
│       ├── workspace UX
│       ├── searchable launcher
│       ├── pinned/running task composition
│       ├── four-edge taskbar extensions
│       ├── Overview
│       ├── D-Bus dispatcher
│       │   ├── NetworkManager
│       │   └── MPRIS
│       └── async libpulse audio
│
├── icewmbg
│   └── wallpaper ownership
│
├── flamewm-desktop
│   ├── XDG desktop files/grid
│   ├── inotify
│   ├── persistent positions
│   ├── desktop context menu
│   └── FlameWM watermark
│
└── host/session services
    ├── notification daemon
    ├── Polkit agent
    ├── secure locker
    ├── NetworkManager
    ├── PipeWire-Pulse/PulseAudio
    └── MPRIS applications

ON DEMAND ONLY
├── flamewm-settings
└── Flame diagnostics/performance sampling while its page is visible
```

This architecture preserves the core proposition:

> **IceWM stays the mature, tiny engine. FlameWM is the carefully isolated layer that removes the friction.**

---

# 18. Final implementation principles for every future agent

1. **Read current source before editing.** Reports are maps, not substitutes for code.
2. **Reuse before replacing.** IceWM already solves much more than a superficial UI inspection suggests.
3. **One authority per state.** FlameWM presents intent; IceWM remains engine authority where it already owns the state.
4. **New product logic lives in Flame-owned paths.** Upstream files get minimal hooks/generalizations only.
5. **Every upstream touch is documented.** Stable marker + touchpoint ledger + tests.
6. **No mass rename/refactor.** Visible Flame branding does not require destroying IceWM ancestry.
7. **Theme first, source renderer second.** Use existing IceWM theming capabilities before adding paint code.
8. **No heavyweight shell dependency.** Native IceWM primitives + small direct integrations.
9. **No blocking work in the X event loop.** D-Bus/audio/filesystem/process operations are asynchronous/event-driven.
10. **No hidden polling tax.** Idle means idle.
11. **No raw shell interpolation.** User/external strings never become shell commands.
12. **Desktop filesystem work stays crash-isolated.** `flamewm-desktop` is separate.
13. **Settings are on-demand and curated.** No GUI exposing the IceWM configuration laboratory.
14. **Performance is a gate.** PSS, idle CPU, startup latency and leak behavior are measured at each major phase.
15. **Multi-monitor coordinates are never assumed to start at zero.** Test positive and negative output origins.
16. **Build-system parity matters.** Keep CMake and Autotools green while upstream supports both.
17. **Every feature has an unavailable/off path.** Optional integration failures cannot take down the WM.
18. **Do not call a task done with a red verification gate.** Diagnose -> bounded repair -> rerun the same gate -> continue only after PASS.
19. **Upstream merges are rehearsed, not feared.** If a merge is painful, reduce divergence before piling on more code.
20. **The user should see FlameWM concepts, not IceWM internals.** That is the product.

---

# 19. Simulation-derived mistakes prevented before first code edit

This simulation already resolves several mistakes that would otherwise be likely during implementation:

## Mistake 1 — Rebuilding things IceWM already has

Prevented by explicit reuse map:

- double-click titlebar maximize;
- half/quarter tiling geometry;
- runtime workspace foundations;
- task state;
- QuickSwitch previews;
- system tray;
- clock;
- wallpaper;
- icon-theme resolution.

## Mistake 2 — Building FlameWM inside `wmmgr.cc`/`wmtaskbar.cc`

Prevented by Flame-owned namespace/directories and mandatory hook policy.

## Mistake 3 — Treating a theme as the whole product

Prevented by separating visual foundation from launcher/settings/snap/workspaces/system controls/desktop architecture.

## Mistake 4 — Treating Arc-Dark as source material to copy

Prevented by classifying it as reference-only.

## Mistake 5 — Bundling all Breeze assets blindly

Prevented by dependency-first + audited fallback subset strategy.

## Mistake 6 — Using the wide Flame wordmark as Start icon

Prevented by asset contract requiring a separate square `flamewm-start.svg`.

## Mistake 7 — Creating another settings authority

Prevented by Flame-owned typed config with documented precedence and atomic persistence.

## Mistake 8 — Implementing system controls with permanent CLI polling

Prevented by final event-driven D-Bus/libpulse architecture.

## Mistake 9 — Putting filesystem desktop code in the WM

Prevented by `flamewm-desktop` process isolation.

## Mistake 10 — Making a profiler that costs resources all the time

Prevented by mandatory dev profiler + on-demand user diagnostics only.

## Mistake 11 — Attempting four-edge panel as a trivial theme flag

Prevented by giving vertical taskbar its own high-risk phase with geometry/strut/reflow tests.

## Mistake 12 — Doing all FlameWM in one giant handoff

Prevented by gated phases and bounded commits.

---

# 20. Definition of readiness for the first coding session

FlameWM is ready for the first real implementation handoff when:

- repository HEAD and upstream merge-base are recorded;
- all five project-source reports are under `project_sources/reports/`;
- current wordmark asset is staged;
- Breeze license/version reference is staged;
- Arc-Dark reference is staged without treating it as production asset;
- missing final Start SVG / vector wordmark / wallpaper status is explicitly recorded;
- build dependencies are documented;
- CMake baseline is green;
- CTest baseline is green;
- Autotools baseline is green;
- nested Xephyr dev command works;
- baseline resource report exists;
- `UPSTREAM_TOUCHPOINTS.md` exists;
- divergence audit exists;
- first bounded implementation phase is selected.

Only then should feature source editing begin.

---

# 21. Canonical handoff sentence

Future coding handoffs can prepend this compact contract:

> **Implement the assigned FlameWM phase as a product layer over the current IceWM source. Read the current source owner before editing; reuse IceWM state/behavior wherever it already exists; keep new logic under Flame-owned paths and `namespace flamewm`; make upstream-file edits only as minimal documented hooks/generalizations; preserve CMake and Autotools; verify with pure tests plus real X11/Xephyr behavior; measure performance; never continue past a required red gate; and never turn a user-facing FlameWM feature into a second authority for IceWM-owned state.**

---

# 22. Source basis

This playbook is complementary to and derived from the current FlameWM project-source set:

- `FLAMEWM_PRODUCT_SOUL_UX_CONSTITUTION.md`
- `ICEWM_FlameWM_Source_Grounded_Implementation_Report.md`
- `FlameWM_IceWM_4.1.0_Fork_Source_Report.md`
- `FLAMEWM_COMPLEMENTARY_ARCHITECTURE_MAINTAINABILITY_BLUEPRINT.md`

It also assumes the current fork remains based on IceWM 4.1.0 until current source inspection proves otherwise.

Primary external references that future implementation agents should re-verify against current versions when relevant:

- FlameWM fork: `https://github.com/linsaftw/flamewm`
- IceWM upstream: `https://github.com/ice-wm/icewm`
- IceWM documentation: `https://ice-wm.org/`
- EWMH: `https://specifications.freedesktop.org/wm/latest-single/`
- NetworkManager D-Bus API: `https://networkmanager.dev/docs/api/latest/spec.html`
- MPRIS: `https://specifications.freedesktop.org/mpris/latest/`
- PipeWire PulseAudio compatibility: `https://pipewire.pages.freedesktop.org/pipewire/page_module_protocol_pulse.html`
- Breeze Icons: `https://github.com/KDE/breeze-icons`

---

# 23. Final decision

The future coding agent should **not** approach FlameWM as “modify IceWM until it looks different.”

It should approach the fork as:

```text
PRESERVE
    IceWM's mature engine and standards behavior

ISOLATE
    FlameWM product logic into owned namespaces/directories/processes

HOOK
    into IceWM only at authoritative state boundaries

PRODUCTIZE
    theme, defaults, launcher, taskbar, settings, snap, workspaces,
    system controls, Overview and desktop into one coherent experience

MEASURE
    every permanent resource cost

VERIFY
    real X11 semantics, not screenshots alone

MERGE
    upstream regularly with a small documented conflict surface
```

The final implementation target is therefore not a large rewrite.

It is a disciplined, source-grounded transformation of IceWM into the desktop described by the FlameWM product soul:

> **Familiar. Calm. Fast. Finished. Focused. Lightweight.**

> **IceWM made friendly by ArkFlame Studios.**
