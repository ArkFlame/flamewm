# FlameWM V5 — Settings, Multi-Display Start, Hotkeys, Overlay Opacity & About Native Implementation Quirks Report

**Project:** FlameWM  
**Studio:** ArkFlame Studios  
**Base:** IceWM 4.1.0 / X11  
**Reference implementation:** `breeze-desktop-prototype-v5`  
**Document role:** Current source-grounded addendum for implementing the V5 prototype behavior natively without repeating the visual, state-model, icon, shortcut, display-selection and link-handling mistakes discovered while iterating the browser prototype.  
**Date:** 2026-09-02  
**Status:** Normative where it reflects the newest explicit V5 direction. Older FlameWM reports remain useful for broader architecture and IceWM source ownership, but V5 decisions below supersede conflicting older UI details.

---

# 0. Assumptions and authority

1. FlameWM remains an X11-first fork of IceWM 4.1.0.
2. The browser prototype is specification code only. HTML/CSS/JavaScript is not part of the production desktop runtime.
3. IceWM remains authoritative for X11 window state, workspaces, focus, task windows, client geometry, EWMH/ICCCM and existing session actions.
4. FlameWM owns product-facing rendering, defaults, settings UX, multi-output panel presentation, display controls, shortcut presentation, branding and shell-specific state.
5. Newest explicit user requirements override older product-source wording where they conflict.
6. The phrase “current desktop/screen taskbar” is implemented natively as **the taskbar on the currently active physical XRandR output**. Virtual workspaces remain global IceWM workspaces unless a later product decision explicitly introduces independent per-monitor workspaces.
7. Every physical output should expose its own FlameWM panel/taskbar surface. That surface remains visible as the user switches virtual workspaces on that output. This does not mean a duplicated independent workspace engine per monitor.
8. FlameWM Settings remains an on-demand process; it should not consume idle memory when closed.
9. The current default accent remains Flame red.
10. Settings icons must be semantically correct, visually recognizable and contrast-tested. “An SVG loaded successfully” is not sufficient acceptance criteria.

---

# 1. Why this report exists

V4 had the correct broad functionality, but several Settings assets technically loaded while still failing the product visually:

- Desktop appeared as an indistinct square rather than a desktop/wallpaper concept.
- Fonts looked like an unrelated circular symbol rather than typography.
- Appearance, Hotkeys and About could render as effectively empty or unreadable glyphs.
- Several dark Breeze SVGs were structurally valid but near-black against FlameWM's dark sidebar, making them functionally invisible.
- The display page repeated all monitor configuration cards instead of treating the monitor topology as a selector.
- Hotkey reset was page-level instead of action-level, and Escape did not express “disable this binding.”
- `Open Start menu` described only half the intended action; the binding is a toggle and must target the active screen's panel.
- Overlay opacity was one visual concept but actually contains two independent product controls: desktop selection feedback and window-placement preview feedback.
- About had the product identity but lacked the useful project/donation navigation expected from a finished open-source desktop.

The engineering lesson is larger than the browser bugs:

> **FlameWM must test user-visible semantics, contrast, ownership and state transitions—not merely whether a resource path exists or a handler fires.**

---

# 2. IceWM source findings relevant to V5

The uploaded IceWM 4.1.0 source was re-audited for this iteration.

## 2.1 IceWM still has one global taskbar pointer

Source:

```text
src/wmtaskbar.cc:40
```

Current source:

```cpp
TaskBar *taskBar;
```

This is a major native boundary for the new “Start menu on the active screen” behavior.

Current IceWM is architected around a singleton taskbar. FlameWM cannot obtain correct independent per-output Start anchoring simply by changing a shortcut label. The panel ownership model must be generalized.

## 2.2 Current Start popup is owned directly by that singleton TaskBar

Source:

```text
src/wmtaskbar.cc:970+
```

Current behavior:

```cpp
void TaskBar::popupStartMenu() {
    if (fApplications) {
        popOut();
        fApplications->popupMenu();
    }
}
```

Therefore the native implementation must resolve **which TaskBar instance** owns the Start popup before calling the equivalent action.

## 2.3 IceWM already understands Super as Start-menu behavior

Source:

```text
src/default.h:293
```

Current `Win95Keys` description explicitly states that the left Super key toggles the Start menu.

This is useful because FlameWM does not need to invent the concept of a Super-based Start action. It needs to:

1. rename the product action to `Toggle Start menu`;
2. route it to the correct output-local taskbar;
3. preserve toggle semantics;
4. integrate it with the curated Hotkeys GUI.

## 2.4 Current `KeySysMenu` is position-specific and legacy-worded

Source:

```text
src/default.h:525
```

Current description:

```text
Activates the IceWM root menu in the lower left corner.
```

That product semantics is no longer sufficient for FlameWM because:

- taskbar can be Bottom/Top/Left/Right;
- multiple outputs can each have a taskbar;
- Start must anchor to the active output's Start button rather than a global lower-left coordinate.

FlameWM should preserve the existing IceWM action infrastructure but add a product-level `ToggleStartMenu` intent whose target is output-aware.

## 2.5 Current taskbar dragging only expresses top vs bottom

Source:

```text
src/wmtaskbar.cc:952+
```

Current logic switches a boolean by comparing pointer Y against half the desktop height.

This reinforces the existing V4/V5 architecture requirement: FlameWM's four-edge panel needs a real edge enum and a per-output panel host. It cannot remain a global `TaskBarAtTop` boolean.

## 2.6 Current taskbar strut code only handles top and bottom

Source:

```text
src/wmtaskbar.cc:785+
```

Current `updateWMHints()` only fills `YStrut.top` or `YStrut.bottom`.

Per-output left/right panels therefore require real strut/work-area generalization. The Start-menu work must not be implemented as a visual-only duplicate panel that IceWM's work-area authority does not know about.

## 2.7 XRandR discovery already exists

Source:

```text
src/ywindow.cc:1873+
src/ywindow.cc:1880+
```

`YDesktop::updateXineramaInfo()` already:

- obtains `XRRScreenResources`;
- enumerates outputs;
- reads output information;
- reads CRTC geometry;
- tracks active screen rectangles;
- tracks a primary screen.

This is the correct low-level geometry foundation.

## 2.8 IceWM already reacts when XRandR screen geometry changes

Source:

```text
src/wmmgr.cc:3904+
src/wmmgr.cc:3913+
```

`YWindowManager::updateScreenSize()` already:

- calls `XRRUpdateConfiguration`;
- refreshes monitor information;
- updates root desktop geometry;
- relocates the taskbar;
- recomputes work areas;
- relayouts the taskbar/workspaces;
- constrains windows if configured.

FlameWM must extend this authority rather than create a separate monitor state machine that can drift from IceWM.

## 2.9 Current source is discovery/reactive, not a user display-control application

The current IceWM path consumes XRandR state. It does not provide the V5 Settings model:

```text
select output
-> inspect only that output
-> choose resolution
-> choose FlameWM shell scale
-> apply safely
```

That user-facing model is a genuine FlameWM addition.

## 2.10 Existing icon infrastructure is valuable and should be reused

`src/yicon.cc` already supports:

- freedesktop icon-theme lookup;
- scalable SVG candidates when SVG support is built;
- size-specific image resolution;
- scaled image acquisition.

The V5 icon problem is therefore **not** “IceWM cannot show Breeze icons.”

The problem is product policy:

- selecting the right semantic icon;
- handling dark/monochrome SVG source colors correctly;
- requesting the right semantic size;
- guaranteeing fallback assets;
- testing actual rendered contrast.

Do not create a second icon-theme engine.

---

# 3. V5 prototype changes — authoritative behavior inventory

The V5 reference prototype now implements:

1. semantic Settings icons for Appearance, Desktop, Displays, Fonts, Hotkeys and About;
2. compact FlameWM wordmark in the Settings navigation header;
3. `Toggle Start menu` action naming;
4. active-desktop Start targeting in the single-view browser model;
5. Hotkey Escape -> `Not assigned`;
6. per-row icon-only default restore;
7. selected-display-first Displays UX;
8. stateful selected-output resolution simulation;
9. stateful selected-output shell-scale simulation;
10. independent selection-rectangle fill opacity;
11. independent window-layout-preview fill opacity;
12. About actions for Donate, Source Code and Website using exact approved destinations;
13. actual PayPal and GitHub brand icons with their license retained;
14. V5-local persistence keys so prior prototype data does not pollute verification.

These behaviors are the reference for the native implementation.

---

# 4. Settings icon system — never repeat the “valid but wrong SVG” mistake

## 4.1 Failure mode

A resource can pass all of these checks and still be wrong:

```text
file exists
SVG parses
browser/native loader returns image
image has non-zero dimensions
```

Yet the UI can still fail because:

- the source symbol is semantically unrelated;
- it is visually just a geometric blob at 18–22 px;
- the SVG uses dark fixed fills and disappears on a dark surface;
- a full-color icon is incorrectly treated as a monochrome mask;
- a monochrome icon is rendered without theme foreground recoloring;
- the chosen symbolic icon has details too fine for sidebar size;
- the asset's intrinsic viewBox has excessive whitespace and appears tiny;
- fallback resolution returns a generic placeholder rather than a product-safe icon.

## 4.2 V5 semantic role mapping

The prototype uses this explicit conceptual mapping:

| Settings page | Intended semantic glyph | V5 reference asset |
|---|---|---|
| Appearance | appearance/global-theme artwork | `settings-appearance.svg` |
| Desktop | wallpaper/desktop monitor artwork | `settings-desktop.svg` |
| Displays | monitor/display | `settings-displays.svg` |
| Fonts | typography / clear `A` | `settings-fonts.svg` |
| Hotkeys | keyboard | `settings-hotkeys.svg` |
| About | information/about | `settings-about.svg` |

The exact V5 files are reference assets, not a mandate to hardcode those filenames into native code.

Native code should request semantic roles:

```cpp
enum class SettingsIconRole {
    Appearance,
    Desktop,
    Displays,
    Fonts,
    Hotkeys,
    About,
};
```

Then resolve each role through one Flame icon resolver.

## 4.3 Native resolution policy

Preferred order:

```text
FlameWM-owned icon override
-> Breeze semantic icon
-> compatible system theme icon
-> packaged FlameWM fallback
```

A Settings page must never disappear visually merely because the host lacks Breeze.

## 4.4 Monochrome vs full-color rendering

Do not treat all SVGs the same.

Native icon metadata should identify whether the selected asset is:

```text
Symbolic/monochrome -> recolor with current semantic foreground
Full-color           -> preserve source colors
Brand                 -> preserve official/approved brand treatment
```

The browser V5 had to make this distinction explicitly because some Breeze glyphs were dark by source design. Native FlameWM should solve it structurally.

## 4.5 Required visual verification

Every shell-critical icon must be captured/tested under at least:

```text
FlameWM dark @ 100%
FlameWM dark @ 150%
FlameWM light @ 100% when light theme lands
missing Breeze theme -> packaged fallback
```

Acceptance is visual recognizability, not only image-load success.

## 4.6 Semantic icon sizes

Use the existing project icon-role direction:

```text
SettingsCategory = one logical size resolved through output scale
```

Do not let each page or button pick arbitrary SVG CSS/native dimensions.

---

# 5. FlameWM Settings navigation branding

## 5.1 V5 decision

The navigation column no longer says:

```text
FlameWM Settings
```

It displays the compact FlameWM wordmark instead.

Why:

- window title already identifies the application as System Settings;
- repeating “FlameWM Settings” wastes vertical space;
- the wordmark supplies brand identity with less visual noise;
- it is consistent with FlameWM's quiet-branding rule.

## 5.2 Native implementation

`flamewm-settings` should load the approved scalable FlameWM wordmark and fit it inside a fixed semantic brand slot.

Rules:

- `object-fit/contain` equivalent behavior;
- preserve aspect ratio;
- no cropping;
- no giant logo;
- no separate redundant `FlameWM Settings` label beside/below it;
- image fallback may use a text `FlameWM` label only if the approved brand asset is genuinely unavailable, and such a packaging failure must be visible in diagnostics/tests.

---

# 6. Toggle Start menu — active-output architecture

This is the most important non-visual V5 change.

## 6.1 Product wording

Canonical action label:

```text
Toggle Start menu
```

Not:

```text
Open Start menu
```

The action is stateful:

```text
closed -> open
open -> close
```

## 6.2 Per-output taskbar requirement

Native FlameWM should expose one Flame taskbar/panel surface for each active physical XRandR output.

Conceptual ownership:

```text
PanelManager
    OutputPanel[eDP-1]
    OutputPanel[HDMI-1]
    ...
```

Each `OutputPanel` owns:

- its visual taskbar window;
- its Start button;
- its local panel popovers;
- its local clock/status presentation;
- task/workspace renderers backed by shared IceWM state.

Do not duplicate IceWM window/workspace authorities inside each panel.

## 6.3 Active-output resolution

When `Toggle Start menu` fires, resolve output deterministically:

```text
1. output containing the currently focused managed frame
2. if no usable focused frame: output containing pointer root coordinates
3. if no pointer/output match: configured/known primary output
4. final fallback: first active output
```

This makes keyboard behavior match where the user is working.

## 6.4 Toggle state rules

Pseudo-contract:

```cpp
void PanelManager::toggleStartOnActiveOutput() {
    OutputId target = activeOutputResolver.resolve();

    if (openStartOutput && *openStartOutput == target) {
        panel(target).closeStart();
        openStartOutput.reset();
        return;
    }

    closeAnyOpenStart();
    panel(target).openStart();
    openStartOutput = target;
}
```

There should be at most one keyboard-opened Start menu active at a time unless a later explicit product requirement says otherwise.

## 6.5 Virtual workspace interaction

Switching virtual desktops must not redirect Start to an unrelated physical screen.

Virtual workspace identity remains an IceWM manager concern.

Every panel renders the current workspace/task state appropriate to its output, but the panel instance is keyed to physical output, not workspace index.

In other words:

```text
screen/output owns panel placement
IceWM owns workspace state
```

Do not implement:

```text
workspace 1 has completely independent panel process/state
workspace 2 has another unrelated panel model
```

unless a future product decision explicitly requires it.

## 6.6 Current IceWM changes required

Because current source exposes:

```cpp
TaskBar *taskBar;
```

and `TaskBar::popupStartMenu()`, a multi-output Flame implementation needs to generalize panel ownership.

Recommended low-divergence direction:

```text
keep TaskBar as the low-level panel class where possible
add Flame PanelManager/PanelHost collection
replace direct global taskBar assumptions at Flame-required touchpoints
preserve compatibility accessor for primary/default taskbar where upstream code still expects one during transition
```

Every global-taskbar use must be audited before converting to a vector, because work area, XRandR resize, pager, tray and session behavior currently assume singleton semantics.

## 6.7 Verification cases

Native integration must prove:

1. one-monitor Super toggles local Start;
2. two-monitor focus on eDP opens eDP Start;
3. focus on HDMI opens HDMI Start;
4. no focused normal window + pointer on HDMI opens HDMI Start;
5. same key again closes it;
6. opening Start on output B closes output A's currently open Start;
7. moving taskbar to Top/Left/Right still anchors Start correctly;
8. output hot-unplug while its Start menu is open safely closes/reassigns state;
9. switching workspaces does not duplicate or orphan Start windows;
10. taskbar/work-area state remains correct on every output.

---

# 7. Displays page — selected output first

## 7.1 UX mistake being corrected

Repeating a complete settings card for every connected monitor creates unnecessary vertical scanning and makes it harder to understand which monitor a change targets.

V5 uses the topology itself as the selector.

## 7.2 Canonical page model

```text
Displays

[ eDP-1 ] [ HDMI-1 ] ...
  topology / monitor previews

Selected: HDMI-1
Resolution [2560x1440 v]
Scale      [100% v]
[large selected-output simulation]
```

Only the selected output's controls appear below.

## 7.3 Selection is transient UI state

Maintain:

```text
selectedOutputId
```

This controls which detail editor is visible.

It is not itself the authoritative system display configuration.

Selection may be persisted for convenience, but output configuration remains separate.

## 7.4 Durable output identity

Do not persist settings only by an integer screen array index.

Preferred identity:

```text
EDID-derived identity + connector name
```

Fallback sequence:

```text
EDID + connector
connector name
XRandR output XID only for current session, never durable persistence
```

A laptop dock/reconnect must not silently apply HDMI settings to a different monitor just because it became “screen 1.”

## 7.5 Native output model

Flame Settings needs richer data than current `DesktopScreenInfo` rectangles alone.

Conceptual model:

```cpp
struct OutputModel {
    OutputId id;
    std::string connector;
    std::optional<EdidFingerprint> edid;
    bool connected;
    bool enabled;
    bool primary;
    RROutput output;
    RRCrtc crtc;
    RRMode currentMode;
    std::vector<DisplayMode> modes;
    int x;
    int y;
    Rotation rotation;
    int shellScalePercent;
};
```

The Settings process may obtain this through a Flame display service/control protocol rather than directly duplicating IceWM's internal monitor snapshot.

## 7.6 Resolution change transaction

The prototype only simulates the visual result. Native FlameWM should actually apply supported XRandR modes.

Do not issue a blind mode change.

Required transaction:

```text
read and snapshot current output/CRTC configuration
validate requested mode belongs to output
apply requested mode through XRandR
wait for RandR events / refresh IceWM geometry
show Keep changes? confirmation with countdown
if Keep -> persist
if timeout / user Revert / application error -> restore snapshot
```

This is essential because a bad display mode can make the settings UI unreadable or move it off-screen.

## 7.7 Process-crash safety

The rollback mechanism must not depend exclusively on the Settings window surviving.

Preferred architecture:

- WM/runtime or small display transaction authority keeps the old configuration and deadline;
- Settings requests the transaction;
- confirmation commits it;
- if Settings exits unexpectedly, transaction authority reverts on timeout.

Do not leave recovery to a JavaScript-style page timer equivalent.

## 7.8 Per-output shell scale

V5 exposes:

```text
100%
125%
150%
175%
200%
```

This is FlameWM logical shell scale, not merely XRandR transform scaling.

It affects Flame-owned surfaces on that output:

- panel/taskbar metrics;
- Start menu;
- popup geometry;
- titlebar and title buttons of frames on that output;
- menu fonts/icons;
- Settings geometry when placed on that output;
- desktop icon/grid metrics;
- selection rectangle border thickness where scale-aware;
- snap preview geometry/stroke;
- clock/calendar;
- status icons.

## 7.9 Shell scale is not universal client scaling

X11 client contents belong to their own toolkit/application.

FlameWM must never claim:

```text
“150% on HDMI means every arbitrary GTK/Qt/Xlib application's content is now independently 150%.”
```

The correct promise is:

> FlameWM shell/chrome scales per output; compatible client applications follow their toolkit/X11 scaling behavior.

## 7.10 Live output movement

When a Flame-managed frame crosses outputs with different shell scale:

- frame/titlebar metrics update using the destination output's scale at a stable transition point;
- snap/work-area geometry uses destination output;
- icon/font caches resolve destination bucket;
- do not mutate global raw `wsTitleBar` style values and accidentally resize every frame.

This confirms the existing project requirement for an output-keyed scale manager.

## 7.11 Monitor-selector visual semantics

Selected display outline uses the **current semantic accent**, not a hardcoded green/red.

The user mentioned a green outline based on what they saw; the underlying product rule is more robust:

```text
selected output -> accent outline
```

Default FlameWM accent is red, so default selected outline is red. If user selects green, it becomes green.

## 7.12 Verification matrix

Test:

- eDP only;
- HDMI only;
- eDP + HDMI;
- outputs with same model/resolution;
- non-zero monitor origin;
- negative monitor origin;
- portrait output;
- output hotplug/unplug;
- selected output unplugged;
- mode switch accepted;
- mode switch timeout/revert;
- unsupported mode rejected before apply;
- 100/125/150/175/200 shell scales;
- different scale on adjacent outputs;
- taskbar on every edge at each scale;
- Start opening on correct output after mode/scale change.

---

# 8. Hotkeys — nullable assignments and per-row reset

## 8.1 Product state model

A hotkey is not always a string.

Native model must permit:

```text
assigned shortcut
or
unassigned
```

Do not encode “disabled” by inventing a fake key chord.

Recommended value type:

```cpp
std::optional<KeyBinding>
```

or a typed equivalent.

## 8.2 Escape during capture

V5 behavior:

```text
recording + Escape -> clear binding -> display Not assigned
```

Escape is a **capture command** in this context, not the shortcut being recorded.

This distinction prevents the classic key-editor bug where a user tries to cancel/disable and accidentally binds the action to Escape.

## 8.3 UI wording

Unassigned state displays exactly:

```text
Not assigned
```

Avoid cryptic `None`, blank cells or `Disabled` when the user is editing a key assignment.

## 8.4 Per-row revert

Each row has a left-side icon-only revert action.

Behavior:

```text
click revert on Toggle Start menu
-> restore shipped Toggle Start default only
-> preserve all other customized hotkeys
```

Do not make “Restore defaults” a page-wide destructive action for this common correction workflow.

## 8.5 Default snapshot authority

Do not derive reset defaults from current user config.

Keep immutable shipped defaults:

```text
DEFAULT_HOTKEYS / FlameDefaultBindings
```

and a separate mutable user override map.

A reset removes/replaces only that override with the canonical default.

## 8.6 Conflict detection

Before persisting a new assigned chord:

1. normalize modifiers/key;
2. detect collision with another Flame-owned action;
3. detect collision with critical IceWM binding where practical;
4. show explicit replacement/conflict UX;
5. do not silently make both actions fire.

## 8.7 Modifier-only bindings

The prototype still supports the special Start-on-Super behavior. Native key capture must explicitly distinguish:

- acceptable special modifier-only actions such as Super Start toggle;
- accidental modifier-only records for actions that require a normal key.

This should be action metadata, not an accidental parser side effect.

Example:

```cpp
BindingSpec ToggleStartMenu { allowModifierOnly = true };
BindingSpec DesktopLeft    { allowModifierOnly = false };
```

## 8.8 Persistence

Store only user overrides where possible. That keeps default evolution manageable.

Example conceptual file:

```text
ToggleStartMenu=Super
DesktopLeft=Ctrl+Super+Left
DesktopRight=Ctrl+Super+Right
DesktopUp=Ctrl+Super+Up
DesktopDown=Ctrl+Super+Down
```

Unassigned value should have one canonical syntax, e.g. empty value or explicit `None`, parsed into `std::nullopt`.

The settings serializer must round-trip unknown future keys without destructive rewriting if that is part of the Flame config contract.

## 8.9 Native key registration

After a live edit:

```text
validate
-> atomically persist
-> unregister old binding
-> register new binding if assigned
-> report grab conflict if X11 cannot claim it
-> update Settings row to effective state
```

Do not show a shortcut as successfully assigned if the X grab failed.

## 8.10 Verification

Test:

- assign normal chord;
- assign Ctrl+Super+Arrow;
- clear with Escape;
- restart settings and confirm Not assigned persists;
- disabled action does not fire;
- reset only one row;
- reset Start after clearing restores default Super behavior;
- conflict path;
- X grab failure path;
- modifier-only rule;
- output-aware Toggle Start behavior after custom binding.

---

# 9. Desktop feedback opacity — two independent controls

## 9.1 Product distinction

There are two visually related but functionally separate overlays:

### A. Desktop selection rectangle

Represents item selection on the desktop filesystem grid.

### B. Window layout/snap preview

Represents the target geometry for a window currently being dragged.

They share the Flame accent language but do not share state.

## 9.2 Separate native settings

Use independent typed properties, e.g.:

```text
DesktopSelectionFillOpacity
WindowSnapPreviewFillOpacity
```

Do not create one generic `OverlayOpacity` value.

## 9.3 Fill vs border

V5 changes only **fill** opacity.

Border remains high contrast.

Conceptual paint:

```text
fill   = accent with user opacity
border = accent at strong fixed/derived opacity
```

Why:

- user can reduce visual obstruction while retaining exact selection/placement geometry;
- 0% fill remains usable;
- a low fill setting does not make the overlay functionally disappear.

## 9.4 Range

The browser reference uses 0–60% for fill. Native Settings can retain that bounded range unless a later explicit product decision changes it.

Do not expose 0–100% merely because raw alpha permits it; 100% would turn a lightweight preview into an opaque obstruction.

## 9.5 Live application

Both settings are safe cosmetic properties and should apply immediately.

No WM restart.

Data flow:

```text
Settings slider
-> typed Flame setting
-> atomic persist
-> runtime delta/reload message
-> overlay renderer updates alpha
```

## 9.6 Process ownership

Selection rectangle belongs to `flamewm-desktop`.

Snap preview belongs to the WM-linked Flame snap subsystem.

The same setting store can provide values to both processes, but each process owns its own renderer/state.

Do not centralize both overlays into one process simply because they share a color token.

## 9.7 Verification

Test:

- selection 0%, snap 60%;
- selection 60%, snap 0%;
- changing one leaves the other unchanged;
- accent color change updates both hue but preserves their separate alpha;
- borders remain visible at 0%;
- persistence across restart;
- output scale changes preserve perceived border geometry and alpha semantics.

---

# 10. About page external actions

## 10.1 Approved destinations

Exact product URLs:

```text
Donate
https://paypal.me/LinsaFTW

Source Code
https://github.com/arkflame/flamewm

Website
https://flamewm.arkflame.com
```

Do not substitute the older lowercase/alternate repository address from historical documents.

## 10.2 Layout

Buttons are horizontally centered under:

```text
FlameWM logo
A lightweight desktop by ArkFlame Studios
```

They may wrap on a very narrow Settings window, but centered horizontal composition is the normal desktop state.

## 10.3 Icon identity

- Donate -> PayPal brand icon;
- Source Code -> GitHub brand icon;
- Website -> recognizable web/browser/globe icon.

Do not use a generic Breeze action icon when the product specifically identifies a branded service and an appropriate licensed brand glyph is available.

## 10.4 License/provenance

V5 prototype uses Font Awesome Free brand SVGs for GitHub and PayPal and retains its license text.

Native packaging must continue to record:

- source project/version;
- applicable icon license;
- copied/modified asset status;
- attribution/license file required by that source.

This is separate from Breeze icon licensing.

## 10.5 Safe URI launching

Never execute:

```cpp
system(("xdg-open " + url).c_str());
```

Use an argv-based URI opener or portal abstraction where URL is one non-shell argument.

Conceptual:

```text
launchUri("https://github.com/arkflame/flamewm")
```

The common launch helper must reject or safely pass malformed values without shell interpretation.

## 10.6 No background web integration

These are links, not web widgets.

About must not:

- fetch GitHub metadata;
- load a PayPal page into an embedded webview;
- resolve website content at startup;
- keep network sockets alive;
- add a Chromium/WebKit runtime.

Click opens the user's normal browser and FlameWM returns to idle.

## 10.7 Failure UX

If no URI handler exists:

- display a bounded error/toast/dialog;
- allow copying the URL;
- do not silently do nothing.

---

# 11. Native source architecture additions implied by V5

The broader project architecture already recommends Flame-owned source isolation. V5 strengthens these specific types.

## 11.1 Panel/output ownership

Recommended:

```text
src/flamewm/panel/
    panelmanager.*
    outputpanel.*
    startcontroller.*
    popoveranchor.*
```

Responsibilities:

### `PanelManager`

- active output -> panel mapping;
- panel collection lifecycle;
- hotplug reconciliation;
- toggle Start routing;
- one-open-Start invariant;
- integration with global IceWM work-area authority.

### `OutputPanel`

- one physical output's panel window/surface;
- edge/orientation;
- Start button anchor;
- status/task/workspace view composition;
- no duplicated authoritative task/workspace state.

## 11.2 Display configuration model

Recommended:

```text
src/flamewm/integrations/displaymanager.*
src/flamewm/core/scalemanager.*
src/flamewm-settings/pages/displays.*
```

Separate concerns:

```text
DisplayManager -> XRandR output/mode/layout transaction
ScaleManager   -> Flame logical scale per durable output identity
DisplaysPage   -> selection-first user interface
```

Do not combine XRandR mode resolution with Flame UI scale into one ambiguous “zoom” property.

## 11.3 Shortcut model

Recommended:

```text
src/flamewm/core/shortcutregistry.*
src/flamewm-settings/widgets/keycapture.*
src/flamewm-settings/pages/hotkeys.*
```

`ShortcutRegistry` owns effective bindings and X grab results. Settings owns editing UX only.

## 11.4 Semantic icon roles

Recommended extension of existing Flame icon policy:

```text
src/flamewm/ui/iconroles.*
```

Add Settings-specific roles plus metadata:

```text
role
preferred Breeze names
fallback asset
render mode: symbolic/full-color/brand
logical size role
```

---

# 12. Configuration schema additions

Exact key names may change during implementation, but responsibility should be explicit.

Example Flame-owned settings:

```ini
[Desktop]
SelectionFillOpacity=20
SnapPreviewFillOpacity=20

[Displays "eDP-1@<edid>"]
Scale=100
PreferredMode=1920x1080@60

[Displays "HDMI-1@<edid>"]
Scale=100
PreferredMode=2560x1440@60

[Hotkeys]
ToggleStartMenu=Super
DesktopLeft=Ctrl+Super+Left
DesktopRight=Ctrl+Super+Right
DesktopUp=Ctrl+Super+Up
DesktopDown=Ctrl+Super+Down
```

An unassigned hotkey needs one canonical serialized representation.

Do not store current selected Settings monitor as if it were monitor configuration. If persisted, it belongs to lightweight UI state, not display-mode authority.

---

# 13. Do not repeat these prototype mistakes in native FlameWM

## 13.1 Do not equate “icon file exists” with “icon works”

Verify actual pixels/contrast and semantic recognizability.

## 13.2 Do not use generic geometric placeholders for Settings categories

A square/circle is not a Desktop/Fonts/About icon merely because it occupies the right slot.

## 13.3 Do not recolor every SVG blindly

Preserve full-color artwork and brand colors; recolor only symbolic assets.

## 13.4 Do not implement Start keyboard routing as a global lower-left popup

Resolve active output and anchor to that panel's Start button.

## 13.5 Do not multiply workspace authority per taskbar

Per-output presentation is not per-output workspace state.

## 13.6 Do not show every monitor's full settings simultaneously

Monitor topology is the selector; edit one selected output at a time.

## 13.7 Do not persist monitor config by array index

Use connector/EDID identity.

## 13.8 Do not apply risky display modes without rollback

Keep a timed recovery transaction outside the fragile page instance.

## 13.9 Do not call XRandR resolution and Flame shell scale the same thing

They are separate concepts and separate authorities.

## 13.10 Do not store Escape as a hotkey when user is trying to disable capture

Escape during recording means clear binding.

## 13.11 Do not make shortcut reset global when user clicked one row

Per-row revert restores only that action.

## 13.12 Do not fake disabled shortcut with an impossible magic string

Use a nullable typed binding and render `Not assigned`.

## 13.13 Do not couple desktop selection opacity to snap-preview opacity

Separate properties, shared accent only.

## 13.14 Do not reduce overlay border visibility with fill opacity

Fill is configurable; outline remains functional.

## 13.15 Do not hardcode social/project URLs in multiple UI files

Keep canonical project links in one product metadata/config source consumed by About.

## 13.16 Do not open URLs through a shell string

Use safe argv/portal URI launch.

## 13.17 Do not add a web runtime for About links

External browser only.

---

# 14. Upstream IceWM touchpoint ledger for this V5 increment

## V5-PANEL-01 — multi-output panel ownership

**Current upstream owner:**

```text
src/wmtaskbar.cc/.h
src/wmapp.cc/.h
src/wmmgr.cc/.h
```

**Reason:** current global `TaskBar *taskBar` cannot express one taskbar per active output.

**Flame owner:**

```text
flamewm::panel::PanelManager
flamewm::panel::OutputPanel
```

**Invariant:** IceWM remains authoritative for window/workspace state and work area.

**Risk:** high; current singleton references exist in multiple core paths.

**Required proof:** panel hotplug + work-area + task/pager regressions.

## V5-START-01 — active-output Start toggle

**Current upstream owner:**

```text
TaskBar::popupStartMenu()
Win95Keys / KeySysMenu action routing
```

**Flame owner:**

```text
PanelManager::toggleStartOnActiveOutput()
```

**Invariant:** one Start surface open globally unless later product direction changes.

**Risk:** medium once panel collection exists.

## V5-DISPLAY-01 — selected-output configuration

**Current upstream owner:**

```text
YDesktop::updateXineramaInfo()
YWindowManager::updateScreenSize()
```

**Flame owner:**

```text
DisplayManager
ScaleManager
DisplaysPage
```

**Invariant:** source of effective geometry remains XRandR/IceWM after mode changes.

**Risk:** high for mode apply/rollback; medium for Settings UI.

## V5-HOTKEY-01 — nullable bindings and per-action reset

**Current upstream owner:**

```text
existing key parsing/grab infrastructure in wmapp/default/preferences paths
```

**Flame owner:**

```text
ShortcutRegistry
KeyCapture widget
Hotkeys page
```

**Invariant:** UI never claims a binding that failed its real X grab.

**Risk:** medium.

## V5-ICON-01 — semantic Settings icon resolver

**Current upstream owner:**

```text
src/yicon.cc/.h
```

**Flame owner:**

```text
flamewm::ui::IconRoles
```

**Invariant:** no duplicate icon-theme scanner.

**Risk:** low.

## V5-OVERLAY-01 — independent opacity settings

**Current native owners:**

```text
flamewm-desktop selection renderer
Flame snap preview renderer
```

**Flame shared owner:**

```text
Flame settings store/palette
```

**Invariant:** opacity values independent; accent shared.

**Risk:** low.

## V5-ABOUT-01 — project URI actions

**Flame owner:**

```text
flamewm-settings About page
safe URI launch helper
```

**Invariant:** no shell interpolation and no resident web runtime.

**Risk:** low.

---

# 15. Test plan for the real IceWM implementation

## 15.1 Settings icon tests

Automated resource contract:

- every semantic Settings role resolves;
- packaged fallback exists;
- requested scaled image has non-zero dimensions.

Visual integration:

- screenshot dark sidebar at 100% and 150%;
- compare every category against expected semantic shape;
- no role appears as a blank/dark box.

## 15.2 Multi-output Start tests

Use Xephyr/XRandR virtual output configuration where practical.

Cases:

- one output;
- two outputs;
- output B focused;
- output B pointer-only fallback;
- panel Bottom/Top/Left/Right;
- open Start then hot-unplug output;
- open Start then change mode;
- workspace switch while Start open;
- taskbar instance creation/destruction leak check.

Assertions should inspect:

- correct panel/menu X window geometry;
- work area/strut properties;
- only intended menu visible;
- current workspace unchanged unless user action explicitly changes it.

## 15.3 Displays tests

Pure model:

- output identity stability;
- mode enumeration;
- selected-output state;
- scale validation;
- mode snapshot/restore plan.

Integration:

- apply supported mode;
- reject unsupported mode;
- auto-revert timeout;
- Keep persists;
- monitor removed during pending transaction;
- Settings process killed during pending transaction -> still reverts safely.

## 15.4 Hotkeys tests

One assertion per test target where possible:

- Escape capture -> null;
- null -> `Not assigned` presentation;
- reset one action -> shipped default;
- reset leaves unrelated override unchanged;
- Toggle Start label correct;
- conflict rejected/resolved explicitly;
- failed X grab not reported as active;
- Super modifier-only allowed for Start only if configured by spec.

## 15.5 Opacity tests

- selection alpha parser bounds;
- snap alpha parser bounds;
- changing selection leaves snap unchanged;
- changing snap leaves selection unchanged;
- 0% fill still paints border;
- accent change updates both RGB but not alpha values.

## 15.6 About tests

- exact three URLs;
- correct labels;
- safe launcher gets URL as one argv/URI parameter;
- no About-page network activity before click;
- missing URL handler surfaces error;
- license manifest includes brand icon provenance.

---

# 16. Performance constraints

V5 features must not create permanent background costs merely because the Settings prototype has richer UI.

Rules:

- `flamewm-settings` exits completely when closed;
- display enumeration is event/request driven, not busy polled;
- panel count scales with active outputs only;
- per-output taskbars share application/workspace models rather than duplicating expensive state;
- icon resources are cached by semantic role + scale/theme generation;
- URI links spawn external browser only on click;
- Hotkeys have no polling loop;
- overlay opacity is a paint property, not an animation timer;
- XRandR listeners reuse the existing event path;
- no embedded browser, Qt, GTK or QML dependency is justified by these V5 features.

Memory measurement must count all Flame-owned resident panel/desktop/background processes and not hide cost by measuring only one PID.

---

# 17. Implementation order for this V5 increment

Recommended order after the broader Flame foundation is green:

1. **Semantic icon roles/fallbacks** — lowest risk, immediately prevents broken Settings identity.
2. **Settings navigation wordmark** — bounded visual change.
3. **Typed shortcut model** — nullable bindings, defaults, per-row reset.
4. **Toggle Start action abstraction** — first preserve one-panel behavior behind the new intent.
5. **PanelManager/output panel collection** — generalize singleton carefully with tests.
6. **Route Toggle Start to active output**.
7. **Display model + selection-first Settings UI** without applying modes initially.
8. **ScaleManager** and per-output Flame metrics.
9. **XRandR mode transaction + timed rollback**.
10. **Independent overlay opacity settings** and live process updates.
11. **About metadata/link actions** and brand asset licensing.
12. **Full two-output integration verification** under Xephyr/nested X.
13. **PSS/startup regression measurement**.

Do not begin with a giant `TaskBar` rewrite plus Settings plus XRandR mutation in one commit.

---

# 18. Definition of done

This V5 native slice is complete only when all of these are true:

- Settings category icons are semantically recognizable and visible in the supported dark theme and scale buckets.
- FlameWM wordmark replaces redundant Settings brand text cleanly.
- `Toggle Start menu` is the canonical user-facing action.
- Start toggles on the taskbar belonging to the active physical output.
- Every active output can own its taskbar without duplicating window/workspace authority.
- Hotkeys can be genuinely unassigned.
- Escape while capturing produces `Not assigned` rather than an Escape binding.
- Per-row revert restores only that action.
- Displays topology selects one output and only that output's controls appear below.
- Resolution mode changes use a recoverable XRandR transaction.
- Per-output Flame shell scale works independently from resolution.
- Desktop selection fill opacity and snap preview fill opacity are independent and live-applied.
- About exposes exactly the approved Donate, Source Code and Website destinations with correct icon semantics.
- External URLs use safe URI launching and create no background web dependency.
- CMake/test gate remains green.
- supported upstream build path remains green according to project policy.
- nested-X semantic tests pass.
- no taskbar/work-area/workspace regression appears on multi-monitor layouts.
- resource/PSS measurement shows no unexplained permanent regression.

---

# 19. Final engineering rule

The recurring failure pattern in prototype polish is simple:

> **A technically present control is not a finished control.**

For FlameWM, every user-facing component must be verified at all three levels:

```text
SEMANTICS
Does the icon/action/name represent the concept users expect?

STATE AUTHORITY
Does it mutate/read the correct IceWM or FlameWM owner without duplicate state?

VISIBLE RESULT
Can the user actually see, understand and recover from the result on the real dark multi-monitor desktop?
```

Following that rule is how the native IceWM fork avoids rediscovering the same mistakes after implementation becomes substantially more expensive than editing the HTML prototype.
