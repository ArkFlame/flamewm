# Engineering, Build and Fork-Maintenance Contract — V4

## Production boundaries

Future new product logic belongs primarily in:

```text
src/flamewm/              WM-linked product controllers/services/UI
src/flamewm-settings/     on-demand Settings executable
src/flamewm-desktop/      filesystem desktop executable
lib/flamewm/              runtime themes/icons/defaults/optional wallpapers
```

All new product C++ uses `namespace flamewm`. Do not reorganize the existing IceWM flat source tree.

**Language floor:** CMake currently tries newer C++ modes, while Autotools can force `-std=c++11`. New Flame code must remain C++11-compatible until both build systems are deliberately raised together and verified. CMake-only success is not closure.

## Runtime shape

A small future `flamewm::Runtime` may coordinate config/palette/metrics/scale/snap/workspace/launcher/panel/integrations/control objects after IceWM/X authorities exist. It never owns a duplicate window/workspace/focus/task/EWMH registry.

V4 WM-linked shape conceptually:

```text
core/          runtime, config, control/version
ui/            palette, metrics/scale, icon roles, popover/anchoring
snap/          drag target, state, overlay
workspace/     product workspace intents/topology helpers
launcher/      reusable FDO app model/search/launcher
panel/         composition, task identity, controls
integrations/  D-Bus, NetworkManager, MPRIS, Pulse
diagnostics/   release/on-demand measurement helpers
```

No Overview directory and no snap-chooser subsystem in current V4.

Desktop process additionally owns `selection`, `trash`, `fileactions`, `watermark`; Settings owns Appearance/Desktop/Displays/Fonts/Hotkeys/About.

Prefer constructor injection in new Flame code. Destroy timers/watches/D-Bus/Pulse/X resources before underlying owners disappear. Exact construction/destruction, callback-generation and re-entrancy rules are in `implementation/EXECUTION.md` / `FAILURES.md`.

## Config/control ownership

Use one small typed Flame-owned config (conceptually `$XDG_CONFIG_HOME/flamewm/settings`), atomic persistence, and a narrow versioned X11-first control/reload channel. Never overwrite unrelated expert IceWM preferences and never expose arbitrary command execution.

## Runtime dependencies

Planned/allowed where feature requires them:

- existing IceWM X11/XRender/XRandR/etc.;
- `libdbus-1` for NetworkManager + MPRIS;
- async `libpulse` for PulseAudio/PipeWire-Pulse;
- existing YIcon/image stack;
- Linux inotify.

No permanent Qt/KDE Frameworks/Plasma, GTK/GLib main loop, QML, Electron/WebView/JS runtime, resident file manager or compositor for core UX. No blocking I/O/wait inside X motion/paint/click handlers; no periodic `nmcli`/`playerctl`/`pactl` polling. Product-profile CI must not silently compile required V4 integrations out: compile-time feature availability is explicit; runtime service absence degrades gracefully.

## Implementation session contract

Before first edit record:

```text
GOAL / user-visible behavior
CURRENT ICEWM AUTHORITY
FLAME OWNER
ALLOWED UPSTREAM TOUCHPOINTS
INPUTS / BLOCKERS
UNIT/SEMANTIC/X11 TESTS
PERFORMANCE/FALLBACK GATE
```

Then: source trace -> pure/state logic first -> tests -> tiny upstream hook -> same semantic gate -> nested X verify -> resource/diff audit. No drive-by rename/refactor.

## Build bootstrap

Canonical gates:

```bash
cmake -S . -B build -DCMAKE_BUILD_TYPE=RelWithDebInfo -DBUILD_TESTING=ON
cmake --build build -j"$(nproc)"
ctest --test-dir build --output-on-failure
./autogen.sh
mkdir -p build-ac && cd build-ac && ../configure && make -j"$(nproc)"
```

The source report derives the upstream Debian/Ubuntu baseline from IceWM CI: `libxrender-dev gettext autopoint libxft-dev libsndfile1-dev libao-dev libsm-dev libx11-dev libxext-dev x11proto-core-dev markdown asciidoctor libxpm-dev libimlib2-dev libgdk-pixbuf2.0-dev libglib2.0-dev libfribidi-dev librsvg2-dev xorg-dev`. Re-check exact package availability on the target distribution rather than weakening source requirements. V4 integration phases additionally need the development headers for `libdbus-1` and `libpulse` in the product build profile.

## Baseline before production work

1. record branch/HEAD/remotes/upstream merge-base/dirty hunks;
2. stage/hash raw V4 prototype when available;
3. revalidate source assumptions;
4. green CMake + CTest;
5. green Autotools while upstream supports it;
6. isolated Xephyr/Xvfb dev lab;
7. baseline PSS/CPU/startup/process/thread/fd data;
8. create `docs/UPSTREAM_TOUCHPOINTS.md`;
9. verify/isolate center-tile non-zero-origin generic fix.

A red required baseline is diagnosed before feature work unless proven environmental and separately resolved.

## Current V4 phases

```text
0 baseline/fork discipline + center-tile generic fix
1 scale/metrics + Flame visual foundation
2 Settings/config/IPC + Appearance/Desktop/Fonts/Hotkeys/minimal About
3 window drag snap (NO hover chooser)
4 indexed/two-row workspace UX
5 Start + unified pinned/running task area
6 four-edge taskbar
7 Displays: XRandR modes, safe resolution revert, per-output shell scale
8 event-driven status integrations + clock/calendar
9 filesystem desktop: inotify/grid/selection/group drag/file actions/Trash/watermark
10 release verification/memory surgery
```

## Verification closure

A compile-only check never proves a desktop feature. Required gates by affected domain include:

- exact title/snap/workspace/taskbar geometry including negative/non-zero origins;
- fixed/min-size window behavior and exact restore;
- workspace first/middle/last mapping + EWMH;
- pinned/running identity/context policy;
- launcher XDG discovery/search/keyboard/Exec safety;
- every panel edge strut/workarea/popover anchor;
- XRandR mode safe revert + durable scale identity/hotplug;
- D-Bus/Pulse disconnect/reconnect/missing-service behavior;
- desktop inotify/grid/selection/group/Trash/output-removal/restart;
- Settings atomic persistence/live delta/crash isolation;
- asset fallback/contrast;
- same failed gate rerun after bounded repair.

No phase DONE with a required red gate or task-created skipped test.

## Performance gate

Measure PSS via `/proc/<pid>/smaps_rollup` for at least WM/taskbar, `icewmbg`, `flamewm-desktop`. V4 release idle scenario targets 2 outputs where available, 20 desktop items, 4 workspaces, network/audio connected, no menus/media, Settings closed.

Required: no high-frequency status polling, no periodic desktop scan, no Overview resources (feature absent), no hidden snap chooser timer (feature absent), no permanent Settings process, no unbounded cache/zombies. **40 MiB-class combined owned PSS is a target to measure and report, never a claim.**

Final stress: 1,000-cycle menu/snap/settings runs, idle wakeup audit, output hotplug, process-isolation crashes, asset/license audit.

## Upstream maintenance

Use `origin` Flame fork + `upstream` IceWM, bounded branches, merges rather than public-history rewriting, `git rerere`, isolated upstreamable generic fixes, and a touchpoint ledger. Every upstream sync re-runs build/test/X11/integrity gates and re-audits changed owners.

Version progression: `0.0.1 ... 0.0.9 -> 0.1.0 ... 0.1.9 -> 0.2.0 ...`.
