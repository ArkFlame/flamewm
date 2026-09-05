# FlameWM Complementary Architecture, Feature-Gap & Upstream-Maintainability Blueprint

**Project:** FlameWM  
**Studio:** ArkFlame Studios  
**Repository:** `https://github.com/linsaftw/flamewm`  
**Base:** IceWM 4.1.0 / X11  
**Document role:** Complementary canonical project-source document. This report does not replace the existing Product Soul, Fork Source Report, or Source-Grounded Implementation Report. It closes the remaining product, architecture, configuration, diagnostics, coding-rule, and long-term-upstream-maintenance gaps between them.  
**Audit date:** 2026-09-02  
**Status:** Normative for future FlameWM implementation unless a newer explicit product decision supersedes it.

---

## 0. Assumptions and authority order

1. FlameWM remains an X11-first IceWM fork for its first complete product generation.
2. The current public fork is still effectively the IceWM 4.1.0 baseline. The repository is registered as a direct fork of `ice-wm/icewm`, the current `VERSION` file still reports `PACKAGE=icewm` and `VERSION=4.1.0`, the README is still the IceWM README, and there are not yet Flame-specific source files in the current `src/` listing.
3. IceWM 4.1.0, released 2026-08-06, is the current upstream release as of this audit.
4. The product description **“IceWM made friendly by ArkFlame Studios.”** is the shortest canonical explanation of FlameWM.
5. The existing **FlameWM Product Soul & UX Constitution** remains the product-behavior authority.
6. The existing **FlameWM / IceWM 4.1.0 Source-Grounded Feature Implementation Report** remains the primary authority for source-proven IceWM capabilities.
7. The existing **FlameWM Fork Source Report** remains the architecture/source-map authority where it does not conflict with later source-grounded findings or this document.
8. Current explicit project direction wins over older report wording. In particular, the current visual prototype direction uses a **Flame red accent by default**. Older wording that calls for a blue default should be treated as superseded for the default accent; blue remains a valid preset.
9. Advanced IceWM behavior may remain reachable through expert configuration. It is not the mainstream FlameWM interface.
10. Upstream compatibility is a first-class engineering objective. We are building a product layer over IceWM, not trying to make every upstream file look like FlameWM source.
11. Low memory is measured, not assumed. PSS is the main memory metric for owned resident processes; idle CPU/wakeups and startup latency are co-equal performance gates.
12. No requirement in this document justifies Qt, GTK, QML, Electron, Chromium/WebView, or a new compositor as a permanent FlameWM shell dependency.

### Authority order when documents disagree

Use this order:

1. newest explicit user/product instruction;
2. verified current source/API behavior;
3. this complementary blueprint;
4. Product Soul & UX Constitution;
5. Source-Grounded Implementation Report;
6. Fork Source Report;
7. old prototype behavior or historical IceWM behavior.

Do not silently reconcile contradictions. Update the relevant project source when a product decision changes.

---

# 1. What FlameWM actually is

## 1.1 Canonical definition

> **FlameWM is the friendly product layer over IceWM: IceWM keeps doing the difficult, mature, low-overhead window-management work; FlameWM decides what users see, which choices are exposed, how normal desktop actions behave, how the desktop looks by default, and how the system stays understandable without configuration expertise.**

The GitHub description is therefore unusually precise:

> **IceWM made friendly by ArkFlame Studios.**

This wording should constrain implementation decisions.

FlameWM is not an alternative window-management engine placed beside IceWM. It is not a Plasma clone. It is not “IceWM with a theme.” It is not a generic desktop toolkit.

It is a productization layer.

## 1.2 The engine/product split

### IceWM owns the engine

IceWM remains authoritative for:

- X11 client management;
- ICCCM/EWMH behavior;
- focus and stacking;
- window frame/client lifecycle;
- move and resize;
- minimize/maximize/fullscreen;
- work-area calculation;
- XRandR monitor geometry;
- workspace identity and window workspace state;
- system tray protocol;
- existing task-window state;
- QuickSwitch/Alt+Tab mechanics;
- icon-theme lookup;
- existing theme/resource loading;
- `icewmbg` background publication;
- existing application launch/session mechanisms where sufficient.

### FlameWM owns the experience

FlameWM becomes authoritative for:

- default appearance;
- default accent and branding;
- taskbar composition and visual language;
- Start/launcher UX;
- workspace-management UX;
- snap-layout UX;
- desktop context menu;
- filesystem desktop UX if enabled;
- user-facing settings;
- live personalization behavior;
- system-control popovers;
- user-facing terminology;
- default shortcuts;
- diagnostics exposed to ordinary users;
- which upstream IceWM options are deliberately hidden from mainstream UX.

The strongest architectural rule follows directly:

> **Never create a second FlameWM state authority for something IceWM already owns correctly. FlameWM issues product-level intents; IceWM remains the state authority.**

Examples:

- FlameWM snap UI asks the active `YFrameWindow` to occupy a region; it does not maintain a second window registry.
- FlameWM workspace UI calls the manager's workspace transaction; it does not keep its own authoritative desktop count.
- FlameWM task UI derives running/focused/minimized state from IceWM frame/task state; it does not create a parallel process tracker.

---

# 2. Current repository state and why it matters

The public `linsaftw/flamewm` repository is currently an ideal fork point:

- GitHub identifies it as forked from `ice-wm/icewm`.
- The repository still carries the upstream source tree and 6,655-commit history.
- `VERSION` is still `PACKAGE=icewm`, `VERSION=4.1.0`.
- The repository README still describes IceWM 4.1.0.
- Current source files inspected from the fork match the source behaviors documented in the 4.1.0 reports.
- No broad FlameWM renaming/refactor has happened yet.

This means we can establish a **low-divergence architecture before divergence begins**.

Do not start development with a mass rename of:

- classes;
- atoms;
- preference keys;
- internal globals;
- source filenames;
- X11 protocol identifiers;
- every occurrence of `IceWM`.

Visible product branding and package/session identity can be changed at product boundaries without destroying source ancestry.

---

# 3. The FlameWM product soul, reduced to implementation laws

The existing constitution already defines the philosophy. The missing implementation translation is this set of laws.

## 3.1 Friendly means fewer decisions, not fewer capabilities

The engine may support 100 options. The settings application may expose 8.

Hidden capability is acceptable.

Bad default UX is not.

## 3.2 Beautiful by default is a functional requirement

A fresh session must not require the user to:

- choose a theme;
- locate an icon pack;
- edit a taskbar file;
- select a font;
- build a launcher menu;
- understand workspaces;
- tune titlebar metrics;
- disable legacy CPU/mail/network graphs;
- install Plasma merely to obtain familiar shell visuals.

FlameWM ships one coherent product default.

## 3.3 Configuration is an escape hatch, not the product

Normal users should never need to know:

- `TaskBarAtTop`;
- `QuickSwitchPreview`;
- `TitleBarMaximizeButton`;
- `WorkspaceNames` syntax;
- `prefoverride`;
- `WM_CLASS`;
- XRandR output coordinates;
- X11 root windows;
- raw pixel titlebar/button metrics.

Those remain implementation or expert concepts.

## 3.4 Lightweight means idle is almost free

A polished shell must not trade configuration complexity for permanent background cost.

Rules:

- no always-running settings process;
- no always-running profiler daemon;
- no polling loop for information that can be event-driven;
- no embedded web UI;
- no compositor dependency for core usability;
- no full file manager kept resident only to draw desktop icons;
- no repeated subprocess spawning every second for panel state.

## 3.5 Familiar behavior should be preserved when IceWM already has it

A feature is not “missing” merely because users do not know IceWM already supports it.

The correct FlameWM work may be:

- enable it by default;
- style it;
- expose it through a simple UI;
- rename it for users;
- remove legacy clutter around it;
- add only the missing interaction layer.

---

# 4. Source-proven feature gap matrix

This matrix is the primary answer to **what IceWM already has and what FlameWM still has to build**.

Legend:

- **KEEP** — existing IceWM behavior already satisfies the functional requirement.
- **WRAP** — existing capability exists; FlameWM needs a friendly surface/default.
- **EXTEND** — existing source is a strong base but misses part of the product interaction.
- **ADD** — genuine new FlameWM subsystem.
- **EXTERNAL INTEGRATION** — the desktop needs the capability, but it should rely on a standard host service rather than reimplement the service.

| Product requirement | Current IceWM 4.1.0 state | Decision | FlameWM work |
|---|---|---|---|
| Move windows | Existing mature behavior | KEEP | Theme/interaction regression tests only |
| Resize windows | Existing mature behavior | KEEP | Preserve forgiving resize hit area in Flame theme |
| Minimize/restore | Existing | KEEP | Restyle task/title controls |
| Maximize | Existing | KEEP | Preserve exact behavior |
| **Double-click titlebar to maximize** | **Already exists; `TitleBarMaximizeButton=1` is current default** | **KEEP** | Do not reimplement; test and keep enabled |
| Border/corner double-click directional maximize | Existing | KEEP | Leave as expert/familiar extra behavior |
| Fullscreen | Existing | KEEP | Preserve |
| Keyboard tiling halves/quarters | Existing `wmTile()` actions | KEEP/EXTEND | Reuse geometry engine; repair center-origin bug before generalization |
| Mouse edge half snap | Basic magnetic snapping exists, Windows-style target commit/preview does not | EXTEND | Add drag target detection + preview + commit |
| Corner quarter snap | Tiling geometry exists, drag gesture/preview missing | EXTEND | Add pointer target behavior |
| Top-edge maximize | Maximize exists; drag activation UX missing | EXTEND | Add drag target |
| Drag-away restore from snapped state | Not a complete product-level state | ADD | Save floating geometry and restore predictably |
| 1/3, 2/3, three-column layouts | Not in current basic tile actions | ADD on existing geometry foundation | Data-driven layout region engine |
| Maximize-button snap layout chooser | Missing | ADD | Flame snap popup |
| Work-area/strut awareness | Existing | KEEP | One geometry authority |
| Multi-monitor/XRandR geometry | Existing | KEEP | Test every Flame overlay/layout against non-zero/negative origins |
| Virtual desktops/workspaces | Existing | KEEP | Restyle/expose simply |
| Runtime workspace count | Existing append/shrink support | EXTEND | Friendly UI + indexed insert/remove |
| Remove clicked middle workspace | Not directly supported | ADD transaction on manager | Deterministic migration/reindex/EWMH update |
| Drag window to adjacent workspace | Existing basis | WRAP/EXTEND | Make drag-only behavior explicit; avoid passive edge switching default |
| Workspace pager | Existing | WRAP | Simpler visual and context menu |
| Taskbar | Existing in WM process | EXTEND | Curated composition, modern metrics, orientation work |
| Bottom taskbar | Existing | KEEP | Product default |
| Top taskbar | Existing | KEEP | Expose as simple setting |
| Left/right taskbar | **Not represented by current `TaskBarAtTop` boolean** | ADD | Real vertical layout + struts + rotated/reflowed child layout |
| Drag taskbar to edge without edit mode | Missing | ADD | Direct dock gesture and edge preview |
| Start button | Existing | WRAP | Flame icon, predictable Super behavior |
| Classic application/root menu | Existing | KEEP as fallback | Hide legacy clutter in product defaults |
| Searchable Start launcher | Missing as product launcher | ADD | `FlameLauncher` |
| XDG `.desktop` discovery | Existing via `icewm-menu-fdo`/FDO parser | WRAP | Extract/reuse data model; do not write a second parser unnecessarily |
| Pinned launchers | Existing toolbar concept | EXTEND | Unify identity/visual ordering with running tasks |
| Running task buttons | Existing | WRAP | Icon-first Flame rendering |
| Focused/minimized/running task states | Existing | WRAP | New indicator treatment, keep source state authority |
| Group multiple windows by application | IceWM has grouping concepts | EXTEND | Stable application identity + pinned/running unified model |
| Alt+Tab | Existing QuickSwitch | KEEP/WRAP | Ship curated configuration and Flame visuals |
| Alt+Tab previews | Existing `QuickSwitchPreview` and preview machinery | KEEP/WRAP | Enable/test; no second switcher engine |
| Overview/Activities | No product overview equivalent | ADD reusing preview primitives | New Flame overlay controller |
| System tray/XEmbed | Existing | KEEP | Keep compatibility |
| Clock | Existing | WRAP | Product format/time+date layout |
| Battery status | Existing basic APM/battery applet | WRAP/EXTEND | Modern visual; host-dependent |
| CPU/RAM graphs | Existing legacy taskbar monitors | KEEP but disable by default | Use only diagnostics/dev unless explicitly enabled |
| Network throughput graph | Existing | KEEP but disable by default | Not the Wi-Fi UI |
| Wi-Fi state/connection UI | Missing | ADD integration | NetworkManager D-Bus |
| Volume control | Missing as modern applet | ADD integration | libpulse async against PipeWire-Pulse/PulseAudio |
| Media controls | Missing | ADD integration | MPRIS over session D-Bus |
| Wallpaper | Existing `icewmbg` | KEEP | Flame settings surface + branded defaults |
| Desktop FlameWM watermark | Missing as independent product layer | ADD | Prefer `flamewm-desktop` overlay ownership |
| Files/folders on desktop | Missing; mini-icons are not filesystem icons | ADD | `flamewm-desktop` |
| Desktop icon grid/reorder/persistence | Missing | ADD | Dedicated model/view/layout |
| Desktop context menu | Existing root menus are generic IceWM menus | ADD/REPLACE UX | Flame-specific contextual menu |
| Breeze/icon-theme lookup | Existing generic icon-theme resolution | KEEP/WRAP | Put Breeze first, add licensed fallback subset |
| Breeze icons included reliably on non-KDE systems | Host-dependent | ADD packaging layer | Package dependency and/or curated fallback |
| Breeze-like window/task/menu appearance | Theme system exists | WRAP/EXTEND | Flame theme + targeted renderer changes |
| Light/dark product appearance | Theme mechanism exists, product switch UX missing | ADD product layer | Two coherent Flame variants + live switch |
| Flame red default accent | Not upstream concept | ADD product config/theme token | One accent token propagated through Flame surfaces |
| Simple settings GUI | **Missing** | **ADD** | Separate on-demand `flamewm-settings` process |
| Accent presets + custom picker | Missing product UI | ADD | Curated colors + Flame color picker |
| Wallpaper picker | Missing product GUI | ADD | Settings Desktop page |
| Taskbar semantic size | Raw IceWM settings exist, product abstraction missing | ADD mapping | Compact / Default / Large |
| Simple shortcut editor | No Flame settings UI | ADD | Curated shortcut set + capture/conflict validation |
| Live-safe settings apply | IceWM can restart/reload config, but product delta-apply layer missing | ADD | Atomic config + Flame runtime reload IPC |
| Real-time “top resource culprits” | Legacy aggregate CPU/MEM applets exist; per-process profiler missing | ADD on-demand diagnostics | Sample only while diagnostics visible |
| Notifications | IceWM is not a freedesktop notification daemon | EXTERNAL INTEGRATION | Distribution/session must provide a lightweight notification daemon; not a v1 WM-core subsystem |
| Polkit graphical authentication | Not a WM responsibility | EXTERNAL INTEGRATION | Session should provide a lightweight policy agent |
| Screen locking | IceWM can invoke `LockCommand`; locker is external | EXTERNAL INTEGRATION | Define default host integration, do not write a locker in WM core |
| Full display control center | Not IceWM core | EXTERNAL INTEGRATION / future | Flame Settings may link to a host display tool until a focused native page exists |
| Compositor blur/effects | No IceWM compositor | DELIBERATELY OMIT | Opaque/flat core UX; optional external compositor only |

---

# 5. Important “already solved” findings that future agents must not reimplement

## 5.1 Double-click maximize is already correct product behavior

Current IceWM documentation and current fork source configuration expose:

```text
TitleBarMaximizeButton=1
```

A left double-click on the titlebar maximizes the window. Shift modifies vertical behavior and Alt+Shift modifies horizontal behavior.

**FlameWM action:** preserve this default. The implementation task is visual polish and regression coverage, not a new double-click maximize state machine.

## 5.2 Half/quarter tiling geometry already exists

Current `YFrameWindow::wmTile()` implements:

- left half;
- right half;
- top half;
- bottom half;
- four quarters;
- center.

**FlameWM action:** generalize the geometry into a data-driven region engine instead of replacing it.

Before extension, fix the source-proven center calculation bug for non-zero monitor origins.

## 5.3 QuickSwitch already does far more than a “legacy Alt+Tab” assumption suggests

Current IceWM supports:

- normal Alt+Tab;
- preview mode;
- horizontal/vertical layouts;
- minimized/hidden windows;
- workspace grouping;
- multiple icon modes;
- current selection styling.

**FlameWM action:** configure and restyle first. Only change source where the approved Flame mockup cannot be achieved through existing knobs.

## 5.4 Runtime workspaces already exist

IceWM already accepts runtime desktop count changes and updates EWMH state.

**FlameWM action:** add the missing friendly semantics—insert after selected desktop, remove arbitrary selected desktop, migrate windows deterministically, and expose this through the pager/Settings.

## 5.5 Breeze integration does not require a second icon lookup engine

The existing `YIcon` path already searches icon themes and can resolve common image formats depending on build configuration.

**FlameWM action:** use it. FlameWM should own icon preference/fallback policy, not duplicate icon discovery.

---

# 6. Product decisions missing or inconsistent in earlier reports — resolved here

## 6.1 Default accent

Earlier product text described a cool blue default. The current approved prototype direction asks for a **red default accent to establish FlameWM brand identity**.

Decision:

```text
Default accent: Flame red
Alternative presets: blue, purple, teal, green, amber, neutral
Custom color: optional via picker
```

Accent must be a semantic token, not hardcoded repeatedly.

Recommended internal model:

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
```

Components consume semantic tokens.

## 6.2 Start search is required

One source-grounded report correctly notes that search was not required by the earliest feature list. The later Product Soul makes the searchable Start experience part of FlameWM's finished desktop contract.

Decision: **searchable Start is required for the first complete FlameWM product**, even if the first technical milestone temporarily retains the classic menu.

## 6.3 Desktop implementation process boundary

Earlier reports disagreed between integrating desktop files into the WM and putting them in a separate process.

Decision: **filesystem desktop ownership belongs in `flamewm-desktop`, a separate FlameWM process.**

Reasons:

- filesystem watching is a different failure domain from window management;
- desktop DND/file operations should never destabilize focus/stacking;
- upstream IceWM already embraces process separation for `icewmbg`;
- it minimizes modifications to WM core;
- it makes future upstream merges easier;
- it can be restarted independently;
- its memory cost can be measured independently.

Memory caveat:

If measured PSS proves that the process boundary prevents the product from meeting its memory budget, integration may be reconsidered from evidence. Do not preemptively sacrifice fault isolation based on guessed process overhead.

## 6.4 Panel service backends

Earlier reports proposed both subprocess-based `wpctl`/`nmcli`/`playerctl` integrations and native asynchronous APIs.

Decision for the complete product:

```text
Network:  NetworkManager D-Bus via libdbus-1
Media:    MPRIS session D-Bus via the same libdbus-1 dispatcher
Audio:    libpulse asynchronous client API
```

Rationale:

- panel state is persistent and event-driven;
- repeated CLI execution is unnecessary churn;
- direct subscriptions provide real-time updates;
- PipeWire-Pulse intentionally supports PulseAudio clients;
- one small D-Bus event adapter can service both MPRIS and NetworkManager;
- no GLib/Qt main loop is required.

A command-based adapter can remain a development/fallback tool, not the primary panel architecture.

## 6.5 Desktop context menu

The canonical mainstream context menu should follow the Product Soul rather than preserving every prototype entry.

Default desktop menu when filesystem desktop is enabled:

```text
New Folder                 [only when desktop files are enabled]
────────────
Change Background
Desktop Settings
```

Do not put terminal, raw IceWM configuration, workspace internals, or unrelated utilities in the normal desktop menu.

Workspace add/remove belongs on the workspace control and in Settings.

## 6.6 Taskbar transparency without a compositor

FlameWM must not require a compositor merely for taskbar usability.

Decision:

1. Opaque taskbar is the guaranteed baseline.
2. If true alpha composition is available, the transparency slider can use it.
3. A future compositor-free pseudo-transparency implementation may sample/crop the root wallpaper pixmap (`_XROOTPMAP_ID`) behind the panel.
4. Blur is not a core FlameWM requirement.
5. Settings must not expose a broken transparency control when the runtime cannot produce a correct result.

---

# 7. The missing centerpiece: FlameWM Settings

The prior reports define what the settings application should expose but do not fully define how to implement it without harming upstream maintainability.

## 7.1 Process model

Create a separate executable:

```text
flamewm-settings
```

It is **on demand**, not session-resident.

Benefits:

- zero idle memory when closed;
- settings crashes cannot crash the WM;
- complex controls such as a color picker do not inflate reliability-critical WM state;
- independent testing;
- easier future redesign;
- Flame-specific code stays outside upstream classes.

Use the existing IceWM `ice`/`itk` primitives where they are practical. Do not introduce a heavyweight toolkit solely for Settings unless a measured/verified future requirement proves the in-tree toolkit inadequate.

## 7.2 Settings information architecture

Keep the navigation shallow.

### Appearance

Expose:

- Light / Dark / System where meaningful;
- Flame red and curated accent presets;
- custom accent picker;
- icon style policy where useful;
- Reset Appearance.

Do not expose:

- raw theme directory names;
- fontconfig/Xft strings;
- individual titlebar pixel values;
- every theme pixmap.

### Desktop

Expose:

- wallpaper preview;
- built-in wallpapers;
- Choose Image;
- Fill / Fit / Stretch / Center;
- Show FlameWM mark;
- Show desktop icons if desktop-file support is enabled.

### Taskbar

Expose:

- Bottom / Top / Left / Right;
- Compact / Default / Large;
- transparency only when supported correctly;
- Auto-hide;
- optionally Show window titles if both modes are product-quality.

Do not expose:

- taskbar raw pixel height;
- CPU graph sampling period;
- mailbox settings;
- network graph samples;
- toolbar config files;
- task button divisor;
- panel source internals.

### Start

Expose only user concepts:

- pinned/favorite apps;
- reset favorites;
- optional recent-app behavior once implemented.

Do not expose menu-file syntax.

### Windows

Expose:

- Snap windows;
- Snap layout chooser;
- optional reduced motion;
- optional Alt+Tab previews.

Do not turn this into a focus-policy laboratory.

### Workspaces

Expose:

- current count;
- Add Desktop;
- Remove Desktop;
- simple shortcut summary.

Minimum count is one.

### Shortcuts

This is a required complementary page because the prototype direction explicitly included simple hotkey customization.

Only expose a curated list:

- Open Start;
- Open Overview;
- Show Desktop;
- Previous Workspace;
- Next Workspace;
- Move Window to Previous Workspace;
- Move Window to Next Workspace;
- Snap Left;
- Snap Right;
- Maximize;
- Snap Layouts.

UI rules:

- click action;
- press desired chord;
- validate modifier/key combination;
- detect duplicate Flame-owned binding;
- warn before replacing a conflict;
- Restore Defaults.

Do not expose every IceWM key binding in the mainstream UI.

### About / Diagnostics

Expose:

- FlameWM version;
- ArkFlame Studios;
- IceWM base version;
- project homepage;
- license attribution;
- Copy diagnostic information;
- Open Performance Monitor.

## 7.3 Configuration ownership

Do not make `flamewm-settings` rewrite a giant upstream `preferences` file and accidentally clobber expert configuration.

Create one Flame-owned user config, for example:

```text
$XDG_CONFIG_HOME/flamewm/settings
```

Use a small, deterministic, typed configuration grammar. It may use IceWM-style `Key=Value` syntax to reuse parsing conventions, but it is a FlameWM-owned file with a documented schema.

Recommended precedence:

```text
upstream IceWM defaults
    -> FlameWM shipped defaults/theme
    -> FlameWM user settings
    -> expert prefoverride
```

`prefoverride` remains the expert escape hatch and therefore wins last.

The settings application writes only its own file.

Unknown keys are preserved when a newer/older Settings application reads the file.

Use atomic persistence:

```text
write temporary file
flush/close
rename temporary -> settings
```

Never partially overwrite live configuration.

## 7.4 Live application architecture

SIGHUP/restart already provides an upstream-compatible full reload, but restarting the WM for every cosmetic change is not the intended FlameWM UX.

Add a small Flame runtime settings service:

```text
FlameSettingsStore
FlameSettingsDelta
FlameSettingsApplier
```

Persistent flow:

```text
flamewm-settings
    -> validate typed value
    -> atomically write Flame config
    -> send ReloadFlameSettings command
    -> WM reads new Flame config
    -> computes delta
    -> applies only changed product-owned values
```

Safe live examples:

- accent;
- theme mode;
- wallpaper;
- taskbar size;
- taskbar edge;
- workspace count;
- snap enablement.

If a setting genuinely cannot be applied safely without restart, say so in the UI. Do not fake live application.

## 7.5 IPC

Prefer one tiny Flame-specific command protocol over many ad-hoc signals.

Recommended shape:

```text
flamewmctl
    reload-settings
    open-settings [page]
    set-workspace-count N
    show-overview
    show-launcher
    diagnostics
```

Implementation can use X11 ClientMessage/property mechanisms because FlameWM is X11-first and the WM already owns the X event loop.

Keep the protocol versioned and narrow.

Do not expose arbitrary shell execution through this control channel.

---

# 8. Performance architecture and the proposed real-time profiler

The idea of showing **what is actually consuming resources** fits FlameWM, but only if the profiler does not become one of the culprits.

## 8.1 Two separate performance products

### A. Development/release profiler — mandatory

Create a developer tool:

```text
tools/flamewm-profile
```

Purpose:

- establish baseline IceWM footprint;
- measure each Flame subsystem;
- compare commits/releases;
- identify regressions before shipping.

Record at minimum:

- PSS;
- RSS;
- private dirty memory;
- swap;
- thread count;
- file descriptor count;
- process count;
- idle CPU;
- wakeups/context switches where practical;
- startup time to usable taskbar;
- launcher cold/warm open latency;
- overview open latency.

Owned process grouping:

```text
flamewm / icewm core binary
icewmbg
flamewm-desktop
other Flame-owned persistent helper, if one ever exists
```

Do not hide a failure by measuring only the main WM PID.

### B. User-facing Performance Monitor — optional but recommended

Place it under **Settings -> About / Diagnostics -> Performance** rather than the default taskbar.

It samples **only while visible**.

Default view:

```text
FlameWM footprint
    FlameWM          memory / CPU
    Background       memory / CPU
    Desktop          memory / CPU
    Total            memory / CPU

Top applications now
    Process          Memory        CPU
    Firefox          ...           ...
    Code             ...           ...
    ...
```

The purpose is understandable:

> **“What is using my computer right now?”**

This is friendlier than exposing CPU/MEM graph internals.

## 8.2 Cheap sampling rules

Do not parse every process's full `smaps` every second.

Use cheap data for the list:

- `/proc/<pid>/stat` for CPU delta;
- `/proc/<pid>/status` for RSS/basic metadata;
- process start time to avoid PID-reuse mistakes.

Use `smaps_rollup`:

- for FlameWM-owned processes;
- for a selected process detail view;
- at a slower cadence if required.

Suggested visible cadence:

```text
CPU/RSS list:       1 second
Flame-owned PSS:    2-5 seconds
selected detail:    on selection + low-rate refresh
```

When the diagnostics page closes:

```text
sampling = off
proc scan timer = off
PSS scan timer = off
```

No daemon remains.

## 8.3 Release budget

The existing 40 MiB combined-PSS goal remains an optimization target, not a claim.

A release report should include a table such as:

| Component | Baseline PSS | Flame release PSS | Delta |
|---|---:|---:|---:|
| WM core | measured | measured | measured |
| `icewmbg` | measured | measured | measured |
| `flamewm-desktop` | 0 / disabled baseline | measured | measured |
| Combined | measured | measured | measured |

If the target is missed, publish the exact measured offenders and optimize from evidence.

---

# 9. Breeze icons strategy

The supplied Breeze Icons 6.29.0 archive is a freedesktop-compatible icon theme. Its `COPYING-ICONS` states that the icon artwork library is LGPL version 3 or later and explicitly discusses GUI/artwork use.

Do not create a second custom icon theme engine.

## 9.1 Runtime policy

Preferred lookup order:

```text
FlameWM-specific icon
Breeze
Breeze Dark / theme-compatible variant where relevant
configured system fallback theme
hicolor / generic fallback
```

Use the existing IceWM/YIcon theme resolver.

## 9.2 Packaging policy

Do not vendor the entire Breeze source tree into FlameWM merely because the archive is available.

Preferred packaging:

1. Depend on the distribution's Breeze icon package when available.
2. Ship a **small audited fallback subset** required for FlameWM shell controls so a non-KDE installation is never left with missing core icons.
3. Keep Breeze licensing notice beside the fallback asset directory.
4. Track exact upstream Breeze version used to refresh fallback assets.
5. Do not modify upstream Breeze SVGs casually; prefer FlameWM-owned icons for Flame-specific actions/branding.

Suggested layout:

```text
lib/flamewm/icons/
    COPYING-BREEZE-ICONS
    upstream-version.txt
    actions/
    status/
    places/
    flamewm/
```

## 9.3 Icon scaling contract

Every Flame-owned surface must request icons by semantic role, not by assuming a source SVG is already the correct pixel size.

Example roles:

```text
IconSize::TitleButton
IconSize::Taskbar
IconSize::TaskbarStatus
IconSize::Menu
IconSize::Launcher
IconSize::SettingsCategory
IconSize::Desktop
```

Centralize DPI/scale resolution.

Do not let each settings page compute its own SVG size; this directly prevents the prototype problem where Settings icons scale inconsistently.

---

# 10. Canonical FlameWM source architecture

The central maintainability goal is:

> **99% of new product logic should live in FlameWM-owned source. Upstream IceWM files should contain only the smallest possible integration hooks or unavoidable source fixes.**

C++ has namespaces, not Java-style packages. Use both a physical source boundary and a namespace boundary.

## 10.1 New source tree

Do not move existing IceWM source files.

Add:

```text
src/flamewm/
├── core/
│   ├── runtime.h/.cc
│   ├── config.h/.cc
│   ├── commands.h/.cc
│   ├── metrics.h/.cc
│   └── hooks.h/.cc
├── ui/
│   ├── palette.h/.cc
│   ├── metrics.h/.cc
│   ├── iconroles.h/.cc
│   └── popover.h/.cc
├── snap/
│   ├── layout.h/.cc
│   ├── state.h/.cc
│   ├── overlay.h/.cc
│   └── chooser.h/.cc
├── workspace/
│   └── workspaceux.h/.cc
├── launcher/
│   ├── appmodel.h/.cc
│   ├── search.h/.cc
│   └── launcher.h/.cc
├── overview/
│   ├── thumbnails.h/.cc
│   └── overview.h/.cc
├── panel/
│   ├── composer.h/.cc
│   ├── taskidentity.h/.cc
│   ├── media.h/.cc
│   ├── audio.h/.cc
│   └── network.h/.cc
├── integrations/
│   ├── dbusdispatcher.h/.cc
│   ├── mpris.h/.cc
│   ├── networkmanager.h/.cc
│   └── pulse.h/.cc
└── diagnostics/
    ├── procstats.h/.cc
    └── profiler.h/.cc

src/flamewm-settings/
├── main.cc
├── settingsapp.h/.cc
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

Exact filenames may change. Responsibility boundaries should not.

## 10.2 Namespace

All new product code uses:

```cpp
namespace flamewm {
    ...
}
```

Use narrower nested namespaces when useful:

```cpp
flamewm::snap
flamewm::panel
flamewm::integrations
flamewm::diagnostics
```

Do not rename upstream `YFrameWindow`, `YWindowManager`, `TaskBar`, etc. merely to make them look Flame-branded.

## 10.3 Runtime coordinator

Add one top-level Flame object owned by the WM application:

```text
flamewm::Runtime
```

Responsibilities:

- lifecycle of Flame subsystems;
- config snapshot;
- shared palette/metrics;
- shared D-Bus dispatcher;
- command/IPC routing;
- feature enablement;
- no window-manager state duplication.

`Runtime` delegates to specific controllers. It must not become a god object.

## 10.4 Hooks, not invasive rewrites

Examples of legitimate upstream integration hooks:

```text
YWMApp construction/destruction
    -> construct/destroy flamewm::Runtime

interactive move path
    -> notify Flame snap controller of pointer motion/release

title maximize button hover
    -> notify Flame snap chooser

workspace pager right click
    -> Flame workspace menu action

taskbar initialization/layout
    -> Flame panel composer contributes Flame widgets

frame/workarea changes
    -> notify snapped-state reflow controller
```

The implementation belongs behind the hook.

Bad:

```cpp
// 250 lines of Flame snap calculation pasted into movesize.cc
```

Good:

```cpp
// FLAMEWM-UPSTREAM-HOOK-BEGIN(snap-drag): product drag-snap UX.
flamewmRuntime().snap().onMovePointer(*this, rootX, rootY);
// FLAMEWM-UPSTREAM-HOOK-END(snap-drag)
```

The marker is not a substitute for good code. Its purpose is to make upstream-diff auditing and conflict resolution immediate.

---

# 11. Upstream-touchpoint policy

## 11.1 Allowed upstream changes

There are four valid categories.

### A. Small Flame integration hook

A few lines that delegate into `src/flamewm/**`.

### B. Source-proven upstream bug fix required by FlameWM

Example: center tiling math with a non-zero monitor origin.

For fixes that are useful to IceWM generally, prefer a patch that could be upstreamed independently.

### C. Minimal generalization of an IceWM owner

Example: expose indexed workspace removal in `YWindowManager` because workspace identity is legitimately owned there.

Keep policy/UI out of the upstream owner.

### D. Build registration

Add Flame source targets/files without reorganizing upstream sources.

Anything else needs explicit architectural justification.

## 11.2 Every upstream touch is documented

Create:

```text
docs/UPSTREAM_TOUCHPOINTS.md
```

Each entry records:

```text
ID
upstream file(s)
reason
Flame owner receiving the hook
behavioral invariant
tests covering it
expected merge-conflict risk
whether patch is upstreamable
```

Example:

```text
FW-UP-003 snap-drag-hook
Files: src/movesize.cc
Reason: send interactive move pointer updates to Flame snap controller
Flame owner: src/flamewm/snap/state.cc
Invariant: stock move behavior is unchanged while Flame snap is disabled
Tests: snap_drag_xephyr, move_without_snap_xephyr
Risk: medium; movesize.cc is active upstream code
Upstreamable: no, product-specific
```

## 11.3 Marker format

Use markers only around modifications inside an upstream file:

```cpp
// FLAMEWM-UPSTREAM-HOOK-BEGIN(<stable-id>): <why this hook exists>.
...
// FLAMEWM-UPSTREAM-HOOK-END(<stable-id>)
```

Rules:

- stable ID never changes merely because code moves;
- markers surround only the fork delta, not surrounding untouched code;
- explain why, not what the syntax does;
- no nested hook markers;
- no 200-line marked regions; if a region grows, extract it into Flame source.

For a genuine upstream bug fix, use a normal explanatory comment if needed and record it in the touchpoint ledger. Do not surround general correctness fixes with product-brand markers if that makes the patch harder to upstream.

## 11.4 Diff budget

Track fork divergence as an engineering metric.

A maintenance script should report:

```text
new Flame-owned files
number of modified upstream files
lines added/removed outside Flame-owned paths
touchpoints not documented in ledger
```

There is no magical maximum line count, but unexplained growth outside `src/flamewm/**`, `lib/flamewm/**`, `docs/flamewm/**`, tests, and packaging is a review failure.

---

# 12. Long-term upstream merge strategy

## 12.1 Git remotes

Canonical local setup:

```bash
git remote -v
# origin    https://github.com/linsaftw/flamewm.git
# upstream  https://github.com/ice-wm/icewm.git
```

Never repurpose `origin` as upstream.

## 12.2 Branch policy

Recommended:

```text
master/main                 FlameWM product branch
feature/<name>              normal feature development
fix/<name>                  Flame fixes
sync/icewm-<version>        temporary upstream integration branch
release/<flame-version>     stabilization only when needed
```

Do not maintain a permanently hand-edited “copy of upstream” branch. The upstream remote already provides that source of truth.

## 12.3 Merge upstream; do not rewrite public Flame history

For long-lived published FlameWM history, prefer a merge-based upstream sync.

Process:

```bash
git fetch upstream --tags
git switch -c sync/icewm-4.x master
git merge --no-ff upstream/master
# resolve only genuine conflicts
audit + build + tests + performance
# merge sync branch into master
```

Why merge rather than rebasing the entire public fork repeatedly:

- preserves public Flame commit identities;
- preserves clear ancestry to upstream;
- makes future blame/bisect understandable;
- allows Git to recognize already-merged upstream history;
- avoids force-pushing releases;
- keeps upstream update events explicit.

Feature branches may rebase on current Flame `master` before merge.

## 12.4 Enable `git rerere`

For a fork that repeatedly touches a small set of upstream files:

```bash
git config rerere.enabled true
```

This lets Git remember how recurring conflicts were resolved.

It is especially useful when FlameWM intentionally maintains small hooks in frequently changed upstream files.

## 12.5 Upstream sync gate

Every upstream sync must pass, in order:

1. identify upstream version/commit range;
2. read upstream `NEWS` and relevant `ChangeLog` entries;
3. inspect every conflict against `UPSTREAM_TOUCHPOINTS.md`;
4. verify whether upstream now implements any Flame feature better;
5. delete obsolete Flame code instead of preserving duplicate implementations;
6. CMake build;
7. registered tests;
8. Autoconf build while upstream continues supporting it;
9. Xephyr/Xvfb behavioral suite;
10. Flame settings/config migration tests;
11. PSS/idle CPU regression snapshot;
12. only then merge the sync branch.

## 12.6 Prefer upstreaming generic fixes

If FlameWM discovers a bug in generic IceWM behavior, isolate it into a minimal commit suitable for upstream.

Examples:

- monitor-origin geometry error;
- general EWMH correctness issue;
- generic memory leak;
- crash unrelated to Flame product semantics.

An accepted upstream fix reduces permanent fork divergence to zero for that bug.

---

# 13. Clean-code rules for Flame-owned code

These rules apply to new Flame code without forcing a style rewrite on untouched IceWM source.

## 13.1 Preserve upstream style at boundaries

When touching existing IceWM files:

- match local formatting;
- do not reformat surrounding code;
- do not modernize unrelated raw pointers/containers;
- do not rename local variables for style reasons;
- do not reorder includes unless required;
- do not perform drive-by cleanup.

Minimal diff wins.

## 13.2 Modern ownership inside Flame code

Where supported by the repository's current compiler baseline:

- use RAII;
- prefer values and references;
- use `std::unique_ptr` for sole ownership;
- use explicit non-owning pointers/references where IceWM object lifetimes require them;
- never introduce a shared ownership graph without proving it is needed;
- unsubscribe listeners/timers before owner destruction.

Do not raise IceWM's required C++ language baseline simply to use a preferred construct. Both current build systems must remain green unless a deliberate project-wide baseline change is approved.

## 13.3 Single responsibility

Examples:

- geometry calculation does not draw the chooser;
- chooser does not mutate workspaces;
- NetworkManager transport does not paint the taskbar icon;
- settings parser does not render a page;
- process metrics reader does not sort/present rows;
- desktop filesystem watcher does not decide visual grid metrics.

## 13.4 One authority per state

Examples:

```text
workspace count        -> YWindowManager / Workspaces
window geometry        -> YFrameWindow + manager work area
running task state     -> IceWM frame/task model
Flame preferences      -> FlameSettingsStore
panel network model    -> NetworkManager adapter snapshot
media model            -> MPRIS adapter snapshot
```

UI may cache presentation data. It does not become authoritative.

## 13.5 Event-loop safety

X11/UI changes stay on the legal WM event-loop thread.

Never:

- block the X event loop on network/D-Bus/process I/O;
- sleep in a callback;
- synchronously scan the filesystem in pointer/expose handlers;
- read large `/proc/*/smaps` data in taskbar paint;
- spawn a helper and wait synchronously from a click handler.

Use IceWM poll/timer integration and asynchronous adapters.

## 13.6 No shell injection surface

External process invocation, when used, must be argv-based.

Never build `/bin/sh -c` strings from:

- SSIDs;
- application titles;
- filenames;
- user search input;
- desktop entry fields;
- media metadata.

## 13.7 Failure handling

- fail optional integrations closed/hidden, not by crashing WM startup;
- rate-limit repeated backend errors;
- do not log the same exception and then rethrow it through another logger;
- never log Wi-Fi secrets or private command content;
- preserve last valid user setting when a new value fails validation;
- provide deterministic fallback theme/icon/wallpaper.

## 13.8 Comments

Comments explain **why an invariant or workaround exists**.

Bad:

```cpp
// Set x to zero.
x = 0;
```

Good:

```cpp
// Keep region boundaries derived from shared edges so odd monitor widths cannot create 1px gaps.
```

## 13.9 No generic framework without two real consumers

Do not invent:

- plugin architecture;
- widget SDK;
- generalized service container;
- declarative UI language;
- theme scripting engine;
- extension marketplace.

Extract shared abstractions only after multiple real Flame components prove the duplication.

The first justified shared abstractions are already clear:

- palette/metrics;
- D-Bus dispatcher;
- settings store;
- command/IPC routing;
- icon role resolver.

---

# 14. Architecture-specific rules

## 14.1 Snap/layout engine

One exact region engine.

Represent layouts as data:

```text
Layout
    id
    minimumMonitorWidth
    regions[]

Region
    denominatorX
    denominatorY
    startX
    startY
    spanX
    spanY
```

Use shared-edge integer partitioning so thirds/quarters never leave one-pixel gaps.

Edge drag targets remain intentionally small:

- left half;
- right half;
- four corners;
- top maximize.

The maximize-button chooser may offer richer layouts:

- 1/2 + 1/2;
- 2/3 + 1/3;
- 1/3 + 2/3;
- three columns;
- four quarters;
- left half + two right quarters.

Do not put every possible tiling ratio in settings.

## 14.2 Workspace mutation

Indexed add/remove must be one manager transaction.

Never let separate UI callbacks independently mutate:

- model count;
- frame workspace index;
- active workspace;
- EWMH properties;
- pager buttons.

The manager commits all of them or none.

## 14.3 Panel

Keep taskbar ownership inside the existing WM process.

`TaskBar` should progressively become a layout host rather than receiving Flame business logic.

Flame panel components own:

- their local render state;
- their popup;
- their integration adapter reference.

They do not own global window/workspace state.

## 14.4 Four-edge taskbar

This is one of the largest actual panel extensions because upstream currently models top/bottom.

Implement a real orientation enum in Flame product terms:

```text
Bottom
Top
Left
Right
```

Do not mutate the existing boolean into magic values.

At the IceWM boundary, introduce the smallest generalization needed so strut/workarea and taskbar geometry understand vertical bars.

Every child applet must declare:

- horizontal preferred size;
- vertical preferred size or vertical rendering policy;
- collapse priority.

Do not rotate text bitmaps as a shortcut if it creates illegible UI. A vertical taskbar can use icon-first tasks and a compact vertical clock/status treatment.

## 14.5 `flamewm-desktop`

The desktop helper owns:

- filesystem Desktop directory model;
- inotify watcher;
- icon grid;
- desktop item selection;
- drag/reorder;
- persistent positions;
- desktop context menu;
- FlameWM watermark overlay.

`icewmbg` continues to own wallpaper initially.

The desktop helper must advertise correct desktop-window EWMH semantics and remain below normal application windows.

Kill/restart it during integration tests; the WM must remain unaffected.

## 14.6 Launcher application identity

Prefer stable identities in this order:

1. desktop file/application ID;
2. `.desktop` `StartupWMClass` mapping;
3. X11 `WM_CLASS`;
4. normalized fallback.

Never identify an application by current window title.

Pinned taskbar order stores application IDs, not raw titles.

---

# 15. System integrations that a friendly desktop needs but IceWM should not reinvent

A window manager can be friendly only if the session around it is coherent.

These are distribution/session contracts, not invitations to rebuild Linux subsystems inside FlameWM.

## 15.1 Notifications

Many desktop applications expect `org.freedesktop.Notifications`.

FlameWM v1 does **not** need a notification-center framework, but the shipped session should include or depend on a lightweight notification daemon.

Possible distribution choice: a lightweight standards-compliant daemon such as dunst or an equivalent distro component.

The specific daemon is packaging policy, not WM-core source.

## 15.2 Polkit agent

Administrative applications often need graphical policy authentication.

The session should ensure one lightweight graphical Polkit agent exists.

Do not implement privilege acquisition in the WM.

## 15.3 Screen lock

IceWM can invoke `LockCommand`; the actual secure locker remains an external security component.

FlameWM should ship a sane default integration for the target distribution rather than implementing authentication/locking itself.

## 15.4 Displays

The first FlameWM Settings version may expose a **Display Settings** entry that launches the distribution's lightweight display tool.

A full XRandR GUI can become a future Flame-owned page only if product scope requires it.

Do not delay the core FlameWM shell to reimplement a display control center.

## 15.5 File manager

Opening a desktop folder/file uses the user's/default application via standard mechanisms.

Do not keep a file manager resident merely to provide desktop icons.

---

# 16. Default product configuration

The exact numeric metrics belong to the approved visual implementation, but the behavior defaults should be explicit now.

Recommended canonical defaults:

```text
Theme mode:                 Light (with complete Dark variant available)
Accent:                     Flame red
Taskbar position:           Bottom
Taskbar size:               Default
Taskbar edit mode:          None / nonexistent
Taskbar CPU graph:          Off
Taskbar MEM graph:          Off
Taskbar legacy net graph:   Off
Taskbar mailbox:            Off
Taskbar Start:              On
Taskbar workspaces:         On
Taskbar running apps:       On
System tray:                On
Clock/date:                 On
Snap windows:               On
Snap layout chooser:        On
Titlebar double-click:      Maximize (`TitleBarMaximizeButton=1`)
Passive pointer edge switch: Off
Drag-window workspace edge: On
QuickSwitch:                On
QuickSwitch previews:       On
Desktop FlameWM mark:       On
Desktop files/icons:        Product decision; if shipped, On by default
Animations:                 Short/subtle or Off on constrained hardware
Blur:                       Off / unsupported by core
```

Workspace initial count should stay small. Two is a reasonable product default if no later UX test proves one or four is better.

---

# 17. Testing rules required by the architecture

## 17.1 Never declare a feature complete with only a compile test

FlameWM modifies interactive state machines. We need semantic tests.

## 17.2 Pure tests

Required categories:

- layout edge partition math;
- snap target classification;
- snap state restoration;
- workspace index insertion/removal mapping;
- application identity matching;
- settings parsing/migration;
- shortcut conflict detection;
- `/proc` metrics parsing;
- icon-role size resolution;
- palette derivation/contrast constraints.

## 17.3 X11 integration tests

Use Xephyr/Xvfb with real windows and assert:

- double-click maximize still works;
- existing maximize button still works after hover chooser addition;
- left/right/corner drag snap exact geometry;
- top drag maximize;
- panel struts respected;
- negative/non-zero monitor origins;
- fixed-size windows not violated;
- snapped restore behavior;
- taskbar top/bottom/left/right work-area reservation;
- workspace add/remove first/middle/last;
- no remove below one;
- dragged window follows workspace edge behavior;
- passive pointer alone does not switch workspace;
- task active/minimized visuals reflect real state;
- Alt+Tab selection activates correct frame;
- overview resources disappear when closed;
- settings delta applies without destroying unrelated state.

## 17.4 Process-isolation tests

Kill:

- `flamewm-desktop`;
- settings while open;
- optional backend connection/service.

The window manager must remain alive and usable.

## 17.5 Upstream compatibility tests

Every Flame subsystem must have an **off** or unavailable path that proves stock source behavior is not accidentally broken where practical.

Example:

```text
Flame snap disabled -> normal IceWM move behavior
Flame launcher unavailable -> fallback menu still opens
NetworkManager unavailable -> network control hides/degrades; taskbar continues
Breeze missing -> fallback icons remain readable
```

---

# 18. CI and fork-maintenance automation

Add high-value automation early.

## 18.1 Build matrix

Preserve upstream-supported paths:

- CMake build + tests;
- Autoconf build while upstream maintains it;
- at least GCC and Clang on the primary Linux target where practical.

## 18.2 Flame architecture checks

Add a script such as:

```text
tools/check-upstream-divergence.py
```

It should:

- compare the Flame branch to the merge-base with `upstream/master`;
- list modified upstream files;
- allow all Flame-owned paths;
- verify every modified upstream source file is represented in `UPSTREAM_TOUCHPOINTS.md`;
- optionally detect unmarked Flame integration blocks;
- report divergence statistics in CI artifacts.

Do not make a simplistic line-count threshold the only gate. It is a diagnostic and review aid.

## 18.3 Asset checks

For Breeze fallback assets:

- verify license file exists;
- verify expected source version file exists;
- reject accidental duplicate/unused giant assets;
- validate SVGs can be loaded at required semantic sizes.

## 18.4 Performance checks

Noisy shared CI is not a trustworthy absolute PSS benchmark.

Use CI for:

- leak/stress tests;
- process-count checks;
- obvious idle polling regressions;
- profiler script validity.

Use a stable reference machine/VM for release PSS and startup numbers.

---

# 19. Recommended implementation phases after the fork was established

This sequence differs slightly from earlier reports because the fork is now real and maintainability infrastructure should precede feature divergence.

## Phase 0 — Fork governance and clean baseline

Deliver:

- add `upstream` remote documentation;
- `docs/UPSTREAM_TOUCHPOINTS.md`;
- divergence audit script;
- CMake green baseline;
- Autoconf green baseline;
- baseline PSS/startup report;
- Flame version metadata separate from upstream base version;
- fix/test generic center-tile origin bug.

Gate: zero unexplained source divergence.

## Phase 1 — Flame visual/default layer

Deliver:

- Flame theme assets;
- Flame red semantic accent;
- Breeze icon preference + fallback subset;
- default wallpaper;
- Flame Start icon;
- taskbar legacy monitor defaults disabled;
- intentional QuickSwitch default;
- preserve titlebar double-click maximize;
- product menu defaults stripped of IceWM configuration clutter.

Goal: before major new features, **stock capabilities already feel like FlameWM**.

## Phase 2 — `flamewm-settings` foundation

Deliver:

- settings process;
- Flame config store;
- Appearance;
- Desktop wallpaper/mark;
- Taskbar top/bottom and size first;
- Windows snap enablement;
- Workspaces count;
- Shortcuts;
- live reload command;
- About page.

Gate: normal personalization requires no text-file editing.

## Phase 3 — Modern snap UX

Deliver:

- region engine;
- drag overlay;
- half/corner/top gestures;
- restore state;
- maximize-hover chooser;
- third/two-third layouts;
- multi-monitor tests.

## Phase 4 — Workspace UX

Deliver:

- indexed insert/remove;
- context menu;
- deterministic window migration;
- drag-only edge switching;
- friendly workspace Settings.

## Phase 5 — Taskbar/product launcher

Deliver:

- searchable Flame Launcher;
- pinned/running application identity;
- modern task indicators;
- curated session/power actions;
- keyboard navigation.

## Phase 6 — Four-edge taskbar

Deliver:

- vertical geometry/struts;
- child reflow;
- vertical task/task-status design;
- direct taskbar drag docking;
- Settings position control.

This deserves its own phase because it changes a horizontal assumption present in upstream code.

## Phase 7 — System controls

Deliver:

- D-Bus dispatcher;
- MPRIS;
- NetworkManager;
- libpulse volume;
- battery modernization if required;
- deterministic narrow-screen overflow.

Gate: idle applets are event-driven.

## Phase 8 — Overview

Deliver:

- shared thumbnail primitive;
- Overview overlay;
- workspace/window activation;
- zero hidden resource cost.

## Phase 9 — `flamewm-desktop`

Deliver:

- desktop window;
- watermark;
- XDG Desktop folder;
- inotify;
- grid/reorder/persistence;
- New Folder/context menu;
- crash isolation tests.

## Phase 10 — Diagnostics and release optimization

Deliver:

- developer profiler;
- on-demand user performance monitor;
- final PSS/CPU/startup measurement;
- cache/timer audit;
- optional process consolidation only where measurements justify it;
- release branding/package/session metadata.

---

# 20. Definition of “friendly” for feature review

Before accepting any FlameWM feature, reviewers answer:

1. Does a normal Windows/KDE-familiar user know what this is without reading IceWM documentation?
2. Is the default already correct for most users?
3. Does the feature expose a semantic choice instead of implementation detail?
4. Does it reuse an existing IceWM state owner where one exists?
5. Is its Flame logic isolated from upstream source?
6. Is it idle when not in use?
7. If it needs a service, can it use a standard host service rather than reimplementing one?
8. Is the failure mode still a usable desktop?
9. Is it keyboard-operable where appropriate?
10. Is the setting reversible/resettable?
11. Does adding the option avoid making a product decision we should make ourselves?
12. Will an upstream IceWM update make this code easy to understand and either keep, adapt, or delete?

If the answers are weak, the feature is not ready.

---

# 21. Definition of “maintainable fork”

FlameWM is maintainable only if all of these remain true:

- upstream history is preserved;
- upstream sync is routine rather than a rewrite event;
- new features overwhelmingly live under Flame-owned paths;
- modified upstream files are explicitly documented;
- no broad renaming obscures ancestry;
- no duplicate window/workspace state engines exist;
- generic IceWM bug fixes can be upstreamed independently;
- build systems remain green;
- settings have one authority;
- optional helpers cannot crash the WM;
- performance is continuously measured;
- release notes state the IceWM base version;
- tests exercise merge-sensitive hooks;
- obsolete Flame code is deleted when upstream gains equivalent/better behavior.

The ideal future upstream update should look like:

```text
fetch upstream
merge
resolve a small known list of hooks
delete any now-redundant Flame workaround
run gates
ship
```

—not:

```text
manually port a rewritten desktop to a new IceWM version.
```

---

# 22. Final architecture decision

The correct long-term structure is:

```text
UPSTREAM ICEWM
    X11 WM core
    client/frame state
    focus/stacking
    work areas/XRandR
    workspace authority
    current taskbar primitives
    system tray
    QuickSwitch
    theme/icon infrastructure
    icewmbg

        | minimal documented hooks
        v

FLAMEWM CORE LAYER     [src/flamewm/**]
    Runtime
    typed settings snapshot
    semantic palette/metrics
    snap UX
    workspace UX
    launcher
    overview
    panel composition
    application identity
    system integrations
    diagnostics plumbing

FLAMEWM ON-DEMAND APP
    flamewm-settings
        simple semantic personalization
        shortcuts
        about/diagnostics
        no idle cost while closed

FLAMEWM ISOLATED DESKTOP
    flamewm-desktop
        desktop files/grid
        watermark
        desktop context menu
        crash-isolated filesystem work

HOST SERVICES
    NetworkManager
    PipeWire-Pulse/PulseAudio
    MPRIS players
    notification daemon
    polkit agent
    secure screen locker
```

The objective is not to make FlameWM source look maximally different from IceWM.

The objective is to make the **user experience maximally different in the ways that matter while keeping the engine maximally reusable**.

That is the core of “IceWM made friendly.”

---

# 23. First-release required feature set, consolidated

The following is the consolidated product target after auditing all current project documents.

## Must be present

- FlameWM branded session/product identity;
- polished light appearance;
- polished dark appearance;
- Flame red default accent;
- Breeze icons with reliable fallback;
- branded wallpaper and subtle FlameWM desktop mark;
- bottom taskbar by default;
- taskbar position control;
- no edit mode;
- simple taskbar size/transparency behavior where technically correct;
- Flame Start button;
- searchable application launcher;
- pinned/running applications;
- modern active/running/minimized indication;
- workspace controls;
- add/remove workspace with minimum one;
- normal double-click titlebar maximize preserved;
- drag edge/corner snap;
- snap preview;
- snap restore;
- maximize-button layout chooser;
- Alt+Tab/QuickSwitch with intentional Flame styling and previews;
- system tray;
- volume;
- Wi-Fi/network;
- battery when available;
- media when active;
- time/date;
- FlameWM Settings;
- wallpaper/accent/taskbar/workspace/shortcuts settings;
- no raw IceWM configuration in mainstream UI;
- live safe personalization;
- low-memory/performance release report;
- complete build/behavior gates.

## Required session integration, not necessarily FlameWM code

- notification daemon;
- Polkit agent;
- screen locker;
- host display configuration path;
- default file opener/file manager.

## May be staged after the first usable milestone but belongs to the complete vision

- Overview;
- filesystem desktop grid if not included in the first technical release;
- four-edge taskbar if vertical orientation needs additional stabilization;
- on-demand user performance monitor.

These may be milestone-staged, but should not be forgotten from the complete product roadmap.

---

# 24. Project-source gaps this document closes

The previous project sources were already strong. The missing information was mainly:

1. **No complete fork-governance/upstream-merge doctrine.**  
   This report defines remotes, branch/sync policy, merge strategy, rerere, touchpoint ledger, upstreamable fixes, and divergence audits.

2. **No exact physical/namespace rule for keeping 99% of new code outside upstream files.**  
   This report defines `src/flamewm/**`, `src/flamewm-settings/**`, `src/flamewm-desktop/**`, the `flamewm` namespace, and hook rules.

3. **No fully specified implementation architecture for FlameWM Settings.**  
   This report defines process isolation, pages, ownership, atomic persistence, live delta application, and IPC.

4. **No final resolution of desktop-process architecture.**  
   This report chooses an isolated `flamewm-desktop`, subject to measurement.

5. **No final resolution of panel integration backend architecture.**  
   This report selects direct event-driven D-Bus/libpulse integration for the complete product.

6. **No real-time resource-culprit design.**  
   This report defines a mandatory release profiler and a zero-idle-cost on-demand user performance monitor.

7. **No packaging-level Breeze icon strategy.**  
   This report defines system Breeze preference + small licensed fallback subset + semantic scaling roles.

8. **No explicit session-service gap analysis.**  
   Notifications, Polkit, locking, and display control are identified as session integration responsibilities instead of accidentally becoming WM features.

9. **Product-document inconsistencies were not resolved.**  
   This report resolves current red accent authority, searchable Start requirement, desktop process, context menu scope, and transparency behavior.

10. **No source-maintenance marker/ledger format.**  
    This report defines stable `FLAMEWM-UPSTREAM-HOOK` markers and the touchpoint registry.

11. **No explicit “do not reimplement double-click maximize” rule.**  
    Current source/documentation proves it already exists; it is now listed as a KEEP capability.

---

# 25. Source references

## Project sources audited

- `FLAMEWM_PRODUCT_SOUL_UX_CONSTITUTION(1).md`
- `FlameWM_IceWM_4.1.0_Fork_Source_Report(1).md`
- `ICEWM_FlameWM_Source_Grounded_Implementation_Report(1).md`
- supplied `breeze-icons-6.29.0.tar.xz`
- current FlameWM visual prototype context and repository description

## Current FlameWM repository

- `https://github.com/linsaftw/flamewm`
- current `VERSION`: `PACKAGE=icewm`, `VERSION=4.1.0`
- current README still describes IceWM 4.1.0

## Current upstream IceWM

- `https://github.com/ice-wm/icewm`
- `https://github.com/ice-wm/icewm/releases`
- `https://ice-wm.org/man/icewm.html`
- `https://ice-wm.org/man/icewm-preferences.html`

Current upstream release verified during this audit:

```text
IceWM 4.1.0
released 2026-08-06
```

Source behaviors reverified against the current FlameWM fork during this audit include:

- titlebar double-click maximize preference;
- half/quarter `wmTile()` actions;
- current top/bottom taskbar preference model;
- current package/version identity.

## Breeze Icons archive

The supplied `breeze-icons-6.29.0.tar.xz` identifies Breeze Icons as a freedesktop-compatible icon theme. `COPYING-ICONS` states LGPL-3.0-or-later terms for the artwork library and explains the GUI/artwork-library usage model. Preserve the relevant notice with any bundled subset.

---

# 26. Canonical one-paragraph context for future agents

**FlameWM is “IceWM made friendly by ArkFlame Studios.” IceWM remains the X11 engine and the authority for windows, workspaces, work areas, task state, tray, QuickSwitch and existing theme/icon infrastructure. FlameWM owns the finished product UX: Flame-red Breeze-informed defaults, simple Settings, Start/search, modern snap layouts, workspace UX, modern taskbar composition, system controls, desktop branding/files, and diagnostics. New implementation belongs under Flame-owned namespaces/directories; upstream files receive only minimal documented hooks or generic fixes. Every upstream touch is registered in `UPSTREAM_TOUCHPOINTS.md`, public history is synchronized by merging upstream rather than repeatedly rewriting the fork, generic fixes should be upstreamed, and low memory is measured by combined process-group PSS/idle CPU rather than assumed. Existing features—including titlebar double-click maximize, basic half/quarter tiling, dynamic workspace foundations, system tray, QuickSwitch previews, icon-theme lookup and wallpaper—must be reused instead of reimplemented. The product succeeds when users get a familiar, beautiful desktop without learning IceWM configuration, while future IceWM releases remain straightforward to merge.**
