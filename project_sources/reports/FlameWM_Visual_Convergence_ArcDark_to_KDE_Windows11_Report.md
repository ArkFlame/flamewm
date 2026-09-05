# FlameWM Visual Convergence Report

**Project:** FlameWM  
**Studio:** ArkFlame Studios  
**Base:** IceWM 4.1.0 fork  
**Report purpose:** source-grounded visual/UX audit of IceWM and the official `icewm-extra` Arc-Dark theme against the approved KDE Plasma / Windows 11-inspired FlameWM target.  
**Date:** 2026-09-02  
**Status:** complementary implementation specification; intended to be kept as a project source for future development.

---

## Assumptions

1. The uploaded IceWM 4.1.0 source snapshot is the engineering baseline. The public FlameWM repository is currently a fork of IceWM and the visual implementation has not yet been treated as complete.
2. The five supplied screenshots are visual evidence, not interchangeable specifications. The **last screenshot** is the strongest visual target. The existing FlameWM Product Soul & UX Constitution remains authoritative where a screenshot and the product contract differ.
3. `Arc-Dark` from `ice-wm/icewm-extra` is a **reference baseline**, not the final FlameWM theme. It is useful because it already removes much of the Win95/Motif visual language and moves IceWM toward a flat dark desktop.
4. FlameWM remains X11-first for this release. The design must not assume Wayland, KWin, Plasma Shell, Qt, GTK, QML, Electron, or a compositor is resident merely to draw the FlameWM shell.
5. “Perfect KDE Plasma / Windows 11 style” means the **FlameWM-owned shell surfaces** should reach that quality and interaction grammar. IceWM cannot directly restyle the internal widgets of arbitrary third-party GTK/Qt/Xlib applications; client-application theming is a separate session/toolkit-integration layer.
6. Low memory usage remains a product constraint. This report therefore distinguishes between visual features that are cheap in native IceWM rendering and effects that require a compositor or heavyweight dependency.
7. All pixel values in this report are **logical 100% scale reference values**, not hardcoded physical pixels. The first source-level visual prerequisite is a scale-aware metrics system.
8. The supplied Breeze icon archive is the intended icon source where a Breeze icon exists. FlameWM-specific product marks remain custom ArkFlame assets.
9. There is one project-source conflict to resolve explicitly: the earlier Product Soul document describes a cool-blue shell accent, while the later FlameWM v2 direction explicitly changes the default accent to red for ArkFlame/FlameWM branding. The later explicit product direction wins for this report. The Product Soul should be reconciled in a later documentation pass so both sources do not compete.

---

# 1. Executive conclusion

`Arc-Dark` is the correct **starting visual direction**, but it solves only the surface-color layer of the problem.

It already demonstrates that IceWM can become visually calm and flat without replacing the WM core. Its palette, thin borders, flat look, muted inactive text, and dark shell are substantially closer to KDE Plasma than the Win95, XP, Motif, or classic IceWM themes.

However, **Arc-Dark still looks unmistakably like IceWM** because most of the remaining visual gap is not stored in the theme file. It comes from IceWM's rendering geometry, fixed pixel assumptions, menu algorithms, taskbar model, popup model, interaction density, and start-menu architecture.

The correct FlameWM implementation is therefore:

```text
Arc-Dark visual lesson
        +
FlameWM design tokens
        +
per-output UI scaling
        +
modern native shell renderers
        +
new Flame Launcher
        +
modern taskbar/task model
        +
modern popup/menu geometry
        +
Breeze icons
        +
minimal client-toolkit session integration
        =
FlameWM visual identity
```

Do **not** attempt to obtain the final result by continuously adding larger XPM images and more values to `default.theme`. That would produce a higher-resolution IceWM theme, not the finished desktop defined by FlameWM.

The largest source-level visual blockers are:

1. **No coherent logical-pixel / per-monitor scale architecture.** Theme metrics are global raw integers.
2. **Legacy menu geometry.** Menu row height/padding and active-row rendering are hard-coded around classic IceWM looks.
3. **Legacy Start menu architecture.** `StartMenu` is a cascading `YMenu` that deliberately appends Run, Windows, Settings, Focus, Themes, Help, filesystem browsing, and Logout surfaces.
4. **Legacy taskbar layout.** The taskbar is laid out from applet intrinsic sizes with hard-coded lanes/margins instead of a modern panel metric and responsive shell layout.
5. **Legacy task-button rendering.** It is window-title/button oriented rather than a modern pinned/running icon model with simple active-state indicators.
6. **Chrome assets are fixed-size pixel resources.** Arc-Dark uses fixed XPM sprites and therefore cannot become genuinely scale-independent.
7. **Visual border and resize interaction are coupled too closely.** Modern desktops want a 1px visual border but a larger invisible/effective resize target.
8. **Rounded windows, rounded popovers and shadows are not a first-class design system.** IceWM has X Shape support and frame corner masks, but not a modern antialiased corner/shadow compositor path.
9. **Application interiors are outside IceWM themes.** The final screenshot's Dolphin content looks coherent because Qt/Breeze is styling the application, not merely the window manager.

The correct order is to fix the **metrics/scaling/rendering architecture first**, then build the Flame theme on top of it. Otherwise every visual element will be retuned twice.

---

# 2. Source and reference set audited

## 2.1 FlameWM project sources

This report complements the existing project sources:

- `FLAMEWM_PRODUCT_SOUL_UX_CONSTITUTION(1).md`
- `ICEWM_FlameWM_Source_Grounded_Implementation_Report(1).md`
- `FlameWM_IceWM_4.1.0_Fork_Source_Report(1).md`
- uploaded `icewm-master(2).zip`
- uploaded `breeze-icons-6.29.0.tar.xz`
- uploaded FlameWM visual/logo assets.

The product constitution already requires:

- a finished desktop instead of a construction kit;
- Breeze-informed flat visual language;
- moderate corner rounding;
- coherent spacing;
- a bottom taskbar by default;
- Start at the left;
- pinned/running apps;
- workspaces;
- system status controls;
- conventional window controls;
- bounded personalization;
- no raw IceWM settings exposed to normal users.

This report does not replace those requirements. It translates them into a **visual gap analysis and source-change map**.

## 2.2 IceWM source baseline

Verified in the uploaded archive:

```text
PACKAGE=icewm
VERSION=4.1.0
```

Relevant source owners audited directly:

```text
src/themable.h
src/default.h
src/wmprog.cc
src/ymenu.cc
src/ymenuitem.cc
src/wmbutton.cc
src/wmtitle.cc
src/wmframe.cc
src/decorate.cc
src/wmtaskbar.cc
src/atasks.cc
src/aworkspaces.cc
src/yxtray.cc
src/yicon.cc
src/fdomenu.cc
```

## 2.3 Current public upstream/fork state

Checked on 2026-09-02:

- IceWM upstream 4.1.0 remains the current 2026-08-06 release.
- `https://github.com/linsaftw/flamewm` is the public FlameWM fork.
- `https://github.com/ice-wm/icewm-extra` is the current IceWM extra-theme repository and includes `Arc-Dark` on `main`.

## 2.4 Arc-Dark reference

Current Arc-Dark source:

- Repository: `https://github.com/ice-wm/icewm-extra/tree/main/Arc-Dark`
- Theme: `https://github.com/ice-wm/icewm-extra/blob/main/Arc-Dark/default.theme`
- Screenshot: `https://github.com/ice-wm/icewm-extra/blob/main/Arc-Dark/screenshot.png`

Its current theme source explicitly defines:

```text
ThemeName="Arc-Dark"
Look=flat
TitleBarHeight=21
HideTitleBarWhenMaximized=1
BorderSizeX=1
BorderSizeY=1
DlgBorderSizeX=1
DlgBorderSizeY=1
SmallIconSize=16
TitleFontNameXft="verdana:size=10:bold"
ColorDefaultTaskBar=#2f343f
ColorActiveTitleBar=#2f343f
ColorNormalTitleBar=#2f343f
ColorActiveTitleBarText=#D3DAE3
ColorNormalTitleBarText=#7c818c
ColorListBoxSelectionText=#5294E2
ColorNetSend=#5294E2
```

The title-button XPM resources are fixed pixel sprites. For example `closeA.xpm` and `maximizeA.xpm` are declared as:

```text
18 42 3 1
```

which is effectively a fixed 18px-wide resource containing two 21px-high states for the 21px titlebar.

That single fact captures the central problem: Arc-Dark is **pixel-themed**, not scale-designed.

---

# 3. What the supplied screenshots reveal

The screenshots are useful because they show that changing the color palette is not enough.

## 3.1 Reference A — Win95 IceWM

The first screenshot shows IceWM's classic visual model clearly:

- extremely thin titlebars;
- bevel-driven controls;
- tiny title buttons;
- dense menu rows;
- text-heavy task buttons;
- tiny taskbar status areas;
- effectively no modern container padding;
- almost every region visually separated by 1px lines;
- cascaded menu hierarchy occupying many columns;
- tiny 16px icon grammar;
- very high information density.

This is useful as the **negative baseline**. It exposes which IceWM assumptions were originally optimized for 1990s/early-2000s pixel densities.

## 3.2 Reference B — antiX/default IceWM

The second screenshot demonstrates that a cleaner theme and larger wallpaper do not solve the structural UX problem.

The Start menu is functionally capable but visually/semantically overloaded:

- Terminal;
- File Manager;
- Web Browser;
- Editor;
- App Select;
- Applications;
- Personal;
- Recent Files;
- Drives;
- Desktop;
- App Killer;
- Control Centre;
- Help;
- Run;
- Preferences;
- Themes;
- Logout.

The cascading Recent Files example spans multiple huge horizontal menus. This is exactly the opposite of FlameWM's product law: **show the user what they need, not every thing the shell knows how to expose**.

This clutter is partly distribution menu configuration, but the IceWM `StartMenu::refresh()` source itself also appends classic management surfaces. Therefore a clean FlameWM Start cannot be solved solely by editing `menu`.

## 3.3 Reference C — XP-style IceWM

The third screenshot shows an important scaling failure mode.

At 1920×1080:

- the taskbar is visually tiny relative to the screen;
- the Start control is tiny;
- titlebar controls are small targets;
- workspace buttons are miniature;
- iconography is low-resolution/pixel-like;
- the UI does not gain physical size with display density.

This screenshot is the strongest visual evidence for the user's scaling observation: **traditional IceWM metrics are physical pixels, not logical desktop units**.

## 3.4 Reference D — dark/translucent IceWM

The fourth screenshot proves that IceWM can become attractive through theming and compositing, but the shell still exposes old geometry:

- titlebars remain narrow;
- controls remain compact;
- status widgets look like separately assembled components;
- task buttons still read as classic rectangular buttons;
- workspace numbers are compressed;
- layout spacing is inconsistent between elements;
- no unified design-token system appears to govern radius, padding, icon size, or hit targets.

It looks like a well-themed window manager rather than one integrated desktop product.

## 3.5 Reference E — KDE Plasma target

The final 1920×1080 screenshot is qualitatively different.

Its most important characteristics are not merely “dark colors”:

### Hierarchy

- large, stable panel at the bottom;
- icon-first launcher/task area;
- distinct but quiet system controls;
- wide launcher surface with structured navigation;
- popovers anchored to their invoking control;
- selected states are obvious but restrained.

### Spacing

- content has air around it;
- menu/app rows are not 20px-high legacy rows;
- controls have predictable horizontal and vertical insets;
- icons sit inside consistently sized hit targets;
- titlebar controls have larger pointer targets than their glyphs.

### Typography

- regular-weight sans-serif text dominates;
- bold is used selectively;
- titles, labels, secondary labels and inactive text have deliberate contrast hierarchy.

### Geometry

- thin borders;
- mild corner rounding;
- popovers and selection backplates read as surfaces, not old-style borders;
- controls use modest radii and consistent padding.

### App launcher

- categories are a left navigation model;
- apps are presented as apps, not as shell-management commands;
- search is first-class;
- power/session actions are isolated at the bottom.

### System popover

The volume panel is a custom shell surface, not a generic `YMenu`:

- title/header;
- content grouping;
- device rows;
- slider controls;
- large touch/pointer targets;
- sufficient padding;
- no 3D line/bevel semantics.

### Panel

The target panel appears roughly in the modern ~48–56 physical-pixel class at 1080p. That is approximately two to three times the physical visual height of the Arc-Dark/legacy IceWM panel examples.

The correct FlameWM response is **not** to hardcode `56` everywhere. It is to define a logical default panel height and then scale it per output.

---

# 4. Arc-Dark: what is correct and should be retained conceptually

Arc-Dark is valuable because it already moves several IceWM decisions in the right direction.

## 4.1 Flat visual mode

`Look=flat` removes the worst Win95/Motif bevel language.

FlameWM should preserve a flat rendering grammar.

## 4.2 Thin visual frame

`BorderSizeX=1`, `BorderSizeY=1` is much closer to Breeze/Windows 11 than classic 4–6px decorative borders.

The **visual** border should remain approximately 1 logical pixel.

The problem is not the visual border value; the problem is that a modern desktop also needs a larger effective resize target.

## 4.3 Calm dark neutral

`#2f343f` and `#D3DAE3` form a workable dark neutral/background + primary-text pair.

FlameWM should use a more deliberate ArkFlame palette, but Arc-Dark proves the dark shell works cleanly inside IceWM's current renderer.

## 4.4 Muted inactive state

`#7c818c` for inactive title text is directionally correct.

The visual system needs clear hierarchy without turning every inactive surface into a 3D disabled button.

## 4.5 Minimal decoration

Arc-Dark is much less visually noisy than IceWM's Win95/XP themes. That aligns with FlameWM's “calm, focused, finished” identity.

## 4.6 Asset simplicity

The title glyphs are simple line-based minimize/maximize/close symbols. This is the correct icon vocabulary even though the current implementation is fixed-resolution XPM.

---

# 5. Arc-Dark mistakes relative to FlameWM

This section is intentionally strict. Arc-Dark is a useful reference, but it is not close enough to ship as FlameWM with branding changes.

## 5.1 Titlebar is too short

Arc-Dark:

```text
TitleBarHeight=21
```

This produces a compact legacy titlebar.

FlameWM target at 100% logical scale:

```text
Default titlebar height: 32 dp
Compact titlebar height: 28 dp
Large/accessibility titlebar height: 36 dp
```

The titlebar is both a visual surface and a drag target. 21px is not adequate as the flagship default for a Windows/KDE-familiar desktop.

### Classification

- A one-scale prototype can change this in the theme.
- Correct per-monitor behavior requires a source-level scale-aware titlebar metric.

## 5.2 Title buttons are too narrow and bitmap-coupled

Arc-Dark's button sprite width is 18px for a 21px bar.

Modern desktop behavior should separate:

```text
button hit target ≠ visible glyph size
```

Recommended 100% logical metrics:

```text
Title button hit width:   42 dp
Title button hit height:  32 dp
Glyph box:                12–14 dp
```

This makes the button easy to target without drawing an oversized icon.

### Required source direction

Draw the core window-control glyphs natively from scale-aware geometry or load vector resources into scale-keyed caches. Do not make the final product dependent on pre-rendered `18x42`, `27x63`, `36x84`, etc. sprite families.

## 5.3 `HideTitleBarWhenMaximized=1` conflicts with the target desktop grammar

Windows 11 and normal KDE Plasma workflows retain usable window controls when maximized. FlameWM also needs the maximize button as the snap-layout hover anchor.

FlameWM default should therefore be:

```text
HideTitleBarWhenMaximized=0
```

This is theme/config-level.

## 5.4 Verdana bold is the wrong typography

Arc-Dark:

```text
TitleFontNameXft="verdana:size=10:bold"
```

This reads like an older Windows/X11 skin.

FlameWM should use the host's modern sans family, preferably Noto Sans or a generic `sans-serif` fallback, with regular/medium weights.

Recommended logical typography:

```text
UI base:          10.5–11 pt equivalent
Title:            regular/medium, not globally bold
Menu/app labels:  regular
Section heading:  medium
Secondary text:   regular + muted color
Clock:            regular tabular-friendly sans/mono only where necessary
```

This is theme-level for one global scale, but true per-monitor font scale needs source support.

## 5.5 Arc-Dark still uses tiny 16px shell icons

Arc-Dark:

```text
SmallIconSize=16
```

16px remains correct for genuinely small glyph contexts, but it should not be the primary taskbar/start-menu icon size.

Recommended FlameWM scale:

```text
small:         16 dp
small-medium:  22 dp
medium:        32 dp
large:         48 dp
```

This mirrors the useful size progression found in modern KDE/Breeze design practice.

## 5.6 No consistent spacing scale

Arc-Dark changes colors and several dimensions but leaves old IceWM spacing algorithms intact.

FlameWM must introduce a small spacing token family, for example:

```text
space1 = 4 dp
space2 = 8 dp
space3 = 12 dp
space4 = 16 dp
space5 = 24 dp
```

Every shell component should derive padding/gaps from these tokens rather than scattered literals such as `1`, `2`, `4`, `6`, `8`, `10` throughout renderer code.

## 5.7 Active selection is too weak/inconsistent

Arc-Dark frequently uses the same background for active and normal taskbar/titlebar surfaces. It relies on text color and certain image resources to distinguish state.

FlameWM requires one coherent accent authority:

```text
accent
accentHover
accentPressed
selectionBackplate
focusRing
activeTaskIndicator
activeWorkspaceIndicator
```

The same semantic accent should flow through task selection, Start selection, workspace state, settings controls, snap preview and focused shell controls.

## 5.8 Arc-Dark's workspace accent is unrelated to its blue accent

It currently declares:

```text
ColorActiveWorkspaceButton="rgb:4b/46/f6"
```

while its other accent-like values use `#5294E2`.

FlameWM must have one accent source, not disconnected hardcoded blues/purples.

## 5.9 No modern hover/pressed system for all shell controls

Arc-Dark can provide rollover title images, but the product needs a general interaction-state system for:

- title buttons;
- Start button;
- task buttons;
- workspace buttons;
- tray/system controls;
- launcher categories;
- popup controls;
- settings controls;
- snap layouts.

This is more than theme pixmaps. It requires consistent source-side state rendering.

## 5.10 No rounded container language

Arc-Dark is flat but rectangular.

The FlameWM product constitution explicitly calls for moderate rounding.

Recommended baseline:

```text
control radius:      4 dp
selection radius:    4 dp
tooltip radius:      4 dp
menu/popover radius: 8 dp
window radius:       6–8 dp when floating
snapped/maximized:   0 dp at touching screen/snap edges
```

This is intentionally similar to the hierarchy used by Windows 11 while remaining compatible with Breeze-like restraint.

## 5.11 No real surface/elevation hierarchy

KDE/Windows distinguish:

- desktop;
- panel;
- window frame;
- menu;
- popover;
- selected row;
- tooltip.

Arc-Dark mostly distinguishes these by flat color substitutions. FlameWM should use controlled contrast differences and 1px separators, not heavy shadows or gradients.

## 5.12 Arc-Dark is still a theme for IceWM's legacy Start model

No amount of Arc-Dark color tuning converts a cascading root menu into the target launcher.

The Start surface must be a new FlameWM shell component.

---

# 6. The most important source discovery: IceWM has no true UI scale system

This is the first thing that should be corrected before visual implementation expands.

## 6.1 Global raw theme metrics

`src/themable.h` defines values such as:

```text
wsBorderX       = 6
wsBorderY       = 6
wsTitleBar      = 20
scrollBarWidth  = 16
scrollBarHeight = 16
menuIconSize    = 16
smallIconSize   = 16
largeIconSize   = 32
hugeIconSize    = 48
quickSwitchHMargin = 3
quickSwitchVMargin = 3
```

These are global integer values. They are not attached to an XRandR output and they are not expressed as logical device-independent units.

A source search finds image scaling functions, but no desktop UI `scaleFactor`, `devicePixelRatio`, or per-monitor DPI metrics system.

## 6.2 Title buttons directly consume the global titlebar height

`src/wmbutton.cc` does effectively:

```cpp
const unsigned height = wsTitleBar;
...
setSize(width, height);
```

The button therefore inherits one global physical-pixel titlebar metric.

## 6.3 Menus derive density from font/icon pixel sizes plus hard-coded padding

`src/ymenuitem.cc` computes ordinary item height from:

```cpp
fontHeight = max(16, menuFont->height() + 1)
ih = max(fontHeight, YIcon::menuSize())
```

For `lookFlat` it then uses:

```text
top = 1
bottom = 1
pad = 1
```

So a typical 16px icon/font menu row is only around the low-20px range.

That is a legacy density model, not a theme-defined modern row metric.

## 6.4 Taskbar height is emergent, not a first-class panel metric

`src/wmtaskbar.cc` constructs a list of applets, measures their current heights, and uses the tallest applet/task height to derive the panel height.

It contains rules such as:

```cpp
h[0] = max(h[0], max(YIcon::smallSize() + 8, fTasks->maxHeight()));
```

and many literal inter-applet margins such as `1`, `2`, `4`.

This explains why classic IceWM taskbars tend to look like a row of tightly fitted widgets instead of one deliberately sized panel surface.

## 6.5 Multi-monitor geometry exists, per-monitor UI scale does not

IceWM already knows monitor/work-area geometry through XRandR-aware code, which is excellent for window placement.

But geometry-awareness is not the same as scale-awareness.

The current model can know:

```text
monitor A = 1920x1080 at x=0
monitor B = 3840x2160 at x=1920
```

without knowing that FlameWM shell UI should be:

```text
monitor A = 100%
monitor B = 150%
```

## 6.6 Required FlameWM architecture

Introduce one authoritative service, conceptually:

```text
FlameScaleManager
```

Responsibilities:

```text
XRandR output discovery
output identity persistence
user-selected scale per output
logical -> physical conversion
scaled font creation/cache
scaled icon/resource cache keys
scaled shell metrics per output
notification when scale/output topology changes
```

Recommended supported user values:

```text
100%
125%
150%
175%
200%
```

Do not expose arbitrary 113.7% scaling in the normal UI. FlameWM is intentionally opinionated.

## 6.7 Output identity

Do not persist only a transient monitor index.

Prefer:

```text
EDID hash + connector name fallback
```

Example conceptual setting:

```ini
[DisplayScale]
eDP-1=125
HDMI-A-1@edid:ab12...=100
```

The exact config syntax can follow the final FlameWM settings architecture, but monitor index `0`, `1`, `2` is not durable enough.

## 6.8 Scaled metrics must be resolved per surface

Examples:

- frame titlebar: scale of the monitor containing the managed frame;
- panel: scale of the output hosting that panel;
- Start launcher: scale of its panel/output;
- context menu: scale of the output where it is opened;
- snap preview: scale of the target output;
- Overview: scale of its active output;
- tooltip: scale of the invoking control's output.

## 6.9 Do not mutate global theme values when a window crosses monitors

A naive implementation such as:

```text
wsTitleBar = 32 * currentMonitorScale
```

would change every frame globally.

Instead, keep the theme's base metrics logical and resolve a `ScaledMetrics` object per scale/output.

Conceptual model:

```cpp
struct FlameMetrics {
    int panelHeight;
    int titleBarHeight;
    int resizeGrab;
    int visualBorder;
    int menuRowHeight;
    int menuIcon;
    int taskIcon;
    int trayIcon;
    int controlRadius;
    int popupRadius;
    ...
};

const FlameMetrics& metricsFor(OutputId output);
```

## 6.10 Font scaling

Xft point-size behavior alone is not sufficient as a complete FlameWM scaling architecture because the surrounding hit targets and spacing remain raw pixels.

Font objects should be cached by:

```text
font role + scale bucket + theme generation
```

Examples:

```text
UiBody@100
UiBody@125
UiTitle@150
UiSecondary@200
```

## 6.11 Icon scaling

`src/yicon.cc` is already useful: it can resolve PNG, XPM, and SVG candidates and scales mismatched image data to the requested size.

This should be retained.

FlameWM should request output-scale-correct icon sizes rather than introducing a second icon engine.

For Breeze SVGs, the desired path is:

```text
semantic icon name
    -> existing YIcon/freedesktop theme lookup
    -> exact requested scaled size
    -> cache by size/theme
```

## 6.12 Critical limitation: IceWM cannot transparently scale arbitrary client application content per monitor

This must be stated clearly to prevent false expectations.

FlameWM can independently scale:

- its titlebars;
- taskbar;
- launcher;
- popovers;
- menus;
- desktop icons;
- overview;
- WM-owned text/icons.

It cannot automatically make an arbitrary X11 application's internal GTK/Qt/Xlib content adopt a different per-monitor scale merely by changing the WM frame.

Full application scaling depends on the client toolkit/application and X11 environment. X11 does not give a stacking WM a clean Wayland-like universal per-output client-scale protocol.

Therefore the first FlameWM scaling promise should be:

> **FlameWM shell and window chrome scale correctly per display; supported modern applications follow their own toolkit scaling behavior.**

Do not market WM chrome scaling as universal application scaling.

---

# 7. Proposed FlameWM 100% logical design tokens

These are a strong implementation baseline, not arbitrary pixel tweaks. They should be implemented as logical metrics and multiplied by the active output scale.

## 7.1 Spacing

| Token | 100% logical value | Use |
|---|---:|---|
| `space1` | 4 | micro gap, icon/glyph inset |
| `space2` | 8 | related controls, menu side padding base |
| `space3` | 12 | standard control gap |
| `space4` | 16 | container/page padding |
| `space5` | 24 | section separation |

## 7.2 Corners

| Token | Value | Use |
|---|---:|---|
| `radiusControl` | 4 | buttons, selections, small plates |
| `radiusTooltip` | 4 | tooltip |
| `radiusPopup` | 8 | Start, menus, quick-control popovers |
| `radiusWindow` | 6–8 | floating top-level frame |
| `radiusSnapped` | 0 at touching edges | snapped/maximized surfaces |

## 7.3 Core dimensions

| Surface | Compact | Default | Large |
|---|---:|---:|---:|
| Panel height | 40 | 48 | 56 |
| Titlebar height | 28 | 32 | 36 |
| Menu row | 30 | 34 | 38 |
| Launcher category row | 34 | 38 | 42 |
| Task hit target | 36 | 44 | 52 |
| Tray/status hit target | 32 | 40 | 48 |

The screenshot target may visually resemble a ~56px panel on a particular 1080p capture. With this system, a 48dp default panel at 125% becomes 60 physical pixels, which is exactly why scale must be solved before visual tuning.

## 7.4 Icon sizes

| Role | Logical size |
|---|---:|
| tiny/status emblem | 12 |
| small | 16 |
| small-medium | 22 |
| taskbar app | 32 |
| launcher app tile | 32 or 48 depending mode |
| desktop icon | 48 |
| large settings/overview | 48–64 |

## 7.5 Typography roles

Do not expose these as individual normal-user settings initially.

```text
UiBody
UiBodyMedium
UiTitle
UiSecondary
UiCaption
UiMonoStatus (only where numeric stability matters)
```

Recommended default family:

```text
Noto Sans
fallback: sans-serif
```

Do not use bold Verdana as the shell identity.

---

# 8. Window frame and titlebar gap analysis

## 8.1 Current IceWM/Arc model

Current strengths:

- frame/titlebar already isolated from client content;
- active/inactive colors themeable;
- titlebar height themeable;
- title button ordering themeable;
- button images themeable;
- X Shape-based frame masks exist;
- frame corner pixmap masks exist;
- active/inactive frame pixmaps exist.

Current weaknesses:

- dimensions are global physical pixels;
- title button hitbox is strongly coupled to titlebar/resource geometry;
- no explicit modern hover backplate system;
- no first-class corner-radius token;
- no first-class shadow/elevation token;
- no separate visual-border vs resize-grab metric;
- fixed bitmap chrome assets are scale-fragile.

## 8.2 Visual border vs resize hit area

Modern target:

```text
visual border: 1 dp
resize grab region: ~6 dp
```

These are not the same thing.

FlameWM should make a window look light and thin while remaining easy to resize.

Required source refactor:

```text
FrameVisualBorder
FrameResizeGrab
```

The resize hit-test should use the larger interaction metric while painting only the thin border.

This is one of the highest-value “friendly” improvements because users should not have to pixel-hunt a 1px edge.

## 8.3 Titlebar alignment

The KDE reference screenshot centers the title, while Windows typically uses left-oriented title identity.

The FlameWM Product Soul already specifies application icon/title at the left. That should win.

Recommended default:

```text
left app icon
8dp gap
left-aligned title
flexible empty drag region
minimize / maximize / close at right
```

Do not blindly copy the exact centered KDE screenshot if it conflicts with FlameWM's Windows-familiar interaction grammar.

## 8.4 Title button behavior

### Minimize

- large hit target;
- small horizontal glyph;
- subtle hover plate.

### Maximize/restore

- large hit target;
- simple square/overlap glyph;
- delayed hover opens FlameWM snap layouts;
- ordinary click remains maximize/restore.

### Close

- neutral at rest;
- red/destructive hover similar to familiar Windows behavior;
- white close glyph on destructive hover;
- no red permanent background.

Arc-Dark can provide an `O` rollover image, but the final system should render from semantic state rather than requiring manually authored sprite permutations at each scale.

## 8.5 Rounded window corners

Important source fact: `src/decorate.cc` already builds frame shape masks from corner pixmap masks and uses X Shape.

Therefore FlameWM has three implementation levels:

### Level 1 — theme-only binary rounding

Create frame-corner resources with transparent masks.

Pros:

- low cost;
- no compositor;
- uses existing machinery.

Cons:

- binary/1-bit edge;
- no true antialiasing;
- difficult to make excellent at several scales with fixed bitmap resources.

### Level 2 — source-generated X Shape rounding

Generate a scale-aware binary rounded mask in code from `radiusWindow`.

Pros:

- no asset variants;
- per-monitor scale-aware;
- consistent radius.

Cons:

- still binary X Shape edges.

### Level 3 — compositor-assisted antialiased rounding/shadow

Use ARGB/compositing for high-quality antialiased corners and real outer shadows.

Pros:

- closest to Windows/KDE polish.

Cons:

- compositor dependency/resource cost;
- outside core no-compositor guarantee.

### Decision

FlameWM core should support Level 2 cleanly and remain compositor-independent. If an optional lightweight compositor is later standardized, Level 3 can enhance it.

## 8.6 Snapped/maximized corner policy

Follow modern desktop geometry:

```text
floating window -> rounded
maximized window -> square at screen edges
snapped window -> square at touching snap/screen edges
```

This also avoids visual gaps between snap regions.

---

# 9. Menus and popovers: why a theme alone cannot reach Breeze quality

This is one of the most source-proven differences.

## 9.1 Current menu item height is algorithmic and legacy-dense

`src/ymenuitem.cc` hardcodes menu padding based on `wmLook`.

For `lookFlat`:

```text
top = 1
bottom = 1
pad = 1
```

The content height is based on a minimum 16px font/icon.

A modern FlameWM menu should instead have an explicit logical row height and side padding.

Recommended:

```text
MenuRowHeight = 34 dp
MenuHorizontalPadding = 10–12 dp
MenuIconSize = 22 dp
MenuIconTextGap = 10 dp
MenuSeparatorInset = 8–12 dp
```

## 9.2 `lookFlat` still paints legacy active-row lines

`src/ymenu.cc` paints a darker top line and brighter bottom line for flat/metal active rows.

That is a visual artifact of old button/bevel grammar.

Breeze/Windows-like selection should be:

```text
one flat selection backplate
4dp radius
no top/bottom 3D lines
accent or neutral selected fill
```

This requires a modern Flame render mode or a generalized new menu style, not another Arc-Dark `menusel.xpm` hack.

## 9.3 Menu outer geometry needs first-class radius and border

Recommended popup geometry:

```text
8dp radius
1dp subtle border/separator
12dp content inset where appropriate
no bevel
no heavy outline
```

## 9.4 Popup shadow

Without a compositor, use no fake large shadow.

A 1px edge/separator is preferable to a blocky simulated shadow.

With an optional compositor, add a restrained soft shadow externally.

## 9.5 Context-menu content must be contextual

This is a UX source change as much as a visual change.

Desktop context menu should be limited to FlameWM desktop actions, per the product constitution.

Taskbar context menu should be taskbar-specific.

Workspace context menu should be workspace-specific.

Do not reuse one generic legacy root menu as the answer to every surface.

---

# 10. Start menu: the current IceWM architecture is the wrong UI

This is the clearest example where source behavior—not the theme—is the problem.

## 10.1 Current source behavior

`StartMenu::refresh()` in `src/wmprog.cc` explicitly builds a classic shell menu. Depending on preferences, it can append:

- filesystem browsing for `/` and `$HOME`;
- Programs;
- Run;
- Windows;
- Settings;
- Help;
- Focus modes;
- Preferences;
- Themes;
- Logout.

The screenshot with Control Centre, Themes, Preferences, drives, recent files, etc. is a distribution-specific composition, but the upstream class itself is architected around this kind of classic management menu.

## 10.2 What can be hidden by configuration today

IceWM already exposes booleans such as:

```text
ShowProgramsMenu
ShowSettingsMenu
ShowFocusModeMenu
ShowThemesMenu
ShowLogoutMenu
```

Therefore FlameWM can immediately hide many legacy entries for an interim build.

The `menu` file can also be simplified and `icewm-menu-fdo` can generate XDG application categories.

### Interim configuration

For a temporary Arc-based development shell:

```text
ShowSettingsMenu=0
ShowFocusModeMenu=0
ShowThemesMenu=0
ShowWindowList=0
ShowRun=0
```

and use one clean XDG app menu source.

This is valuable during Phase 1, but it is not the final architecture.

## 10.3 Why `YMenu` is still wrong for FlameWM Start

The target requires:

```text
search field
category navigation
application presentation
favorites/pins
power/session footer
keyboard search
stable panel-anchored geometry
```

A cascading `YMenu` is the wrong abstraction for this.

## 10.4 Final Flame Launcher structure

The user's specific correction is important:

> show app categories and the apps; do not expose Control Centre/Preferences/Themes/implementation clutter as Start-menu structure.

Recommended Flame Launcher:

```text
┌──────────────────────────────────────────────┐
│ Search applications…                         │
├───────────────┬──────────────────────────────┤
│ Favorites     │                              │
│ All Apps      │   applications for selected │
│ Development   │   category / search results │
│ Education     │                              │
│ Games         │                              │
│ Graphics      │                              │
│ Internet      │                              │
│ Multimedia    │                              │
│ Office        │                              │
│ System*       │                              │
├───────────────┴──────────────────────────────┤
│ Sleep      Restart      Shut Down      Logout│
└──────────────────────────────────────────────┘
```

`System` is acceptable only as a normal XDG application category containing actual applications. It must not become an IceWM control/configuration dump.

FlameWM Settings itself can be a normal favorite/searchable application, not a special nested `Control Centre` tree.

## 10.5 Application data reuse

Do not reimplement Freedesktop `.desktop` parsing from scratch.

`src/fdomenu.cc` already handles fields including:

- `Name`;
- localized names;
- `Icon`;
- `Categories`;
- `Exec`;
- `Terminal`;
- `NoDisplay`.

Extract/reuse the parser/model for Flame Launcher.

## 10.6 Launcher visual target

Recommended 100% default:

```text
width:  620–680 dp
height: 500–560 dp
radius: 8 dp
outer padding: 12–16 dp
search height: 36–40 dp
category row: 38 dp
app icon: 32–48 dp depending list/grid mode
```

The target screenshot's large Kickoff-style launcher should be treated as a **structured panel**, not a giant cascading menu.

---

# 11. Taskbar/panel gap analysis

## 11.1 Current source model

`TaskBar::updateLayout()` builds a fixed list of IceWM applets with booleans for left/right row placement, intrinsic sizes and hard-coded pre/post margins.

The current model is excellent for a configurable classic WM panel, but it produces the visual characteristics visible in the screenshots:

- each applet has its own natural size;
- panel height emerges from content;
- tiny widgets sit beside tiny widgets;
- many 1–4px literal gaps;
- no single panel density authority;
- no modern semantic lane model.

## 11.2 FlameWM panel must have an explicit semantic height

The panel should own its height first; contained controls align within it.

Conceptually:

```text
panelHeight = metrics.panelHeight
controlHitBox <= panelHeight
icons centered in control hitboxes
```

Do not let the tallest legacy monitor widget silently define the FlameWM panel size.

## 11.3 Modern lane model

Keep the project architecture already identified:

```text
LEFT
    Flame Start
    Overview
    Workspaces
    pinned/running apps

FLEX
    task area / spacer

RIGHT
    media
    network
    volume
    battery
    tray
    clock/date
```

The exact order can follow the Product Soul, but all controls must be laid out by one spacing/metrics system.

## 11.4 Start button

Arc-Dark and classic themes use tiny taskbar images.

FlameWM:

```text
hit target: panel-height square or near-square
visible Flame icon: ~22–26 dp
no label by default
hover: subtle 4dp-radius backplate
pressed/open: accent/selected state
```

## 11.5 Running apps: icon-first, not rectangular title buttons

IceWM already supports:

```text
TaskBarShowWindowIcons
TaskBarShowWindowTitles
TaskBarTaskGrouping
```

An interim build can set:

```text
TaskBarShowWindowIcons=1
TaskBarShowWindowTitles=0
```

But the final visual needs source work.

Desired states:

```text
pinned, not running
running, inactive
focused
minimized
attention/urgent
multiple windows
```

Recommended rendering:

```text
icon centered in 44dp hit target
focused -> accent underline / pill indicator
running inactive -> muted underline/dot
minimized -> lower emphasis, still visibly running
hover -> neutral backplate
urgent -> controlled accent/attention pulse or marker
```

Do not repaint the entire button background with a different 1990s-style button skin for every state.

## 11.6 Pinned + running unification

The target panel should feel like one application area.

IceWM currently has separate concepts for toolbar launchers and task buttons.

The final FlameWM model should identify applications by desktop entry / WM_CLASS and unify pins with running tasks.

This is source-level product work, not theming.

## 11.7 Workspaces

Classic workspace buttons can remain functional, but the visual should be reduced to a modern compact state control.

Recommended:

- small numbered/dot/pill items;
- active desktop uses accent;
- inactive desktops use muted neutral;
- no bevel;
- 32–36dp hit area;
- context menu only for Add/Remove/appropriate workspace actions.

## 11.8 System tray

Keep XEmbed support.

Visual requirements:

- 22dp target icon size at 100%;
- consistent 36–40dp hit area;
- no bevel around the whole tray;
- consistent spacing;
- overflow only if implementation requires it later.

## 11.9 Clock/date

The target screenshot shows a clear two-line clock/date area.

Recommended default:

```text
12:37 AM
2/16/21
```

or locale-equivalent.

No LED clock sprites.

This may need a small source renderer change if one `strftime` line cannot achieve the desired stacked layout cleanly.

---

# 12. Native quick-control popovers

The KDE screenshot's audio panel illustrates another category of UI that should not be forced into `YMenu`.

FlameWM needs a reusable native **popover surface**, not only a popup menu.

## 12.1 Popover visual contract

```text
8dp outer radius
1dp subtle border
12–16dp container padding
36–40dp row/control heights
22dp status/action icons
header title
optional divider
keyboard navigation
anchored to taskbar control
```

## 12.2 Volume

Needs:

- current volume;
- mute;
- output devices where useful;
- slider;
- clear selected output state.

## 12.3 Network

Needs:

- current connection;
- Wi-Fi state;
- list of visible networks;
- signal strength;
- connect/disconnect path.

## 12.4 Media

Needs:

- track/artist when available;
- previous/play-pause/next;
- hidden when irrelevant.

## 12.5 Battery

Needs:

- battery percentage/state;
- optional basic power mode if dependable.

### Source classification

These are new FlameWM shell components. Arc-Dark cannot style them because they do not exist in IceWM.

---

# 13. Rounding and shadow implementation boundaries

The user explicitly called out margin sizes, rounding style and visual polish. The exact boundary matters.

## 13.1 Cheap and safe without a compositor

FlameWM can implement natively:

- rounded **binary-shaped** frame masks;
- rounded binary popup masks;
- rounded selection backplates drawn inside rectangular windows;
- flat 1px borders;
- hover surfaces;
- subtle inner separators;
- opaque dark/light surfaces.

These preserve the low-memory core.

## 13.2 Not equivalent to KDE/Windows compositor polish

Without a compositor, FlameWM cannot provide the same quality of:

- antialiased transparent outer corners;
- true external window shadows;
- background blur/acrylic;
- translucent overlapping popup shadows.

Do not fake these with large chunky bitmap edges.

## 13.3 Recommended product policy

Core FlameWM:

```text
opaque by default
clean binary/source-generated rounding
no blur
no compositor requirement
```

Optional enhanced mode later:

```text
lightweight compositor detected/enabled
-> antialiased shadows/transparency enhancements
```

This respects the project soul: modern appearance without importing a heavyweight desktop stack.

---

# 14. Theming arbitrary applications: a separate layer IceWM themes cannot solve

This is critical when comparing an Arc-Dark IceWM screenshot to the KDE screenshot.

The KDE target's Dolphin window is internally styled by Qt/Breeze:

- toolbar;
- sidebar;
- menu bar;
- file view;
- selection;
- scrollbars;
- dialogs.

IceWM only owns the outer frame/titlebar.

Therefore even a perfect FlameWM frame around an unthemed GTK/Qt application can still look inconsistent.

## 14.1 Session-level consistency target

FlameWM should provide a lightweight session integration policy for:

```text
icon theme
cursor theme
font defaults
GTK light/dark theme preference
Qt light/dark style preference where available
color-scheme hint where standardized
```

## 14.2 Do not make Plasma a dependency

If a Qt application already has Breeze available, FlameWM may set/use that style through normal Qt configuration.

But FlameWM must not start KDE settings daemons or Plasma services merely to make Qt apps look coherent.

## 14.3 GTK

A matching GTK theme/config can be shipped at distribution/session level without making GTK a dependency of the FlameWM process.

## 14.4 Icon theme

This is the easiest shared visual layer.

`YIcon` already supports system icon-theme lookup and SVG loading when supported.

Use Breeze as the default icon theme while preserving fallback behavior.

---

# 15. Exact theme/config/source boundary matrix

Legend:

- **THEME** — `default.theme` / theme resources can solve it correctly at one global scale.
- **CONFIG** — existing IceWM preference/menu configuration can solve behavior.
- **SOURCE** — FlameWM source change required.
- **SESSION** — outside WM process; toolkit/session configuration.
- **COMPOSITOR** — exact high-quality implementation requires compositing.

| Surface / issue | Arc-Dark / IceWM today | FlameWM target | Boundary |
|---|---|---|---|
| Shell dark palette | Arc-Dark already good direction | ArkFlame neutral + accent tokens | THEME initially; SOURCE for dynamic accent tokens |
| Titlebar color | themeable | coherent active/inactive surfaces | THEME |
| Titlebar height | global `21` in Arc | 28/32/36 logical, per monitor | THEME prototype; SOURCE final |
| Title typography | Verdana 10 bold | system/Noto Sans regular/medium | THEME prototype; SOURCE for per-output font scale |
| Window border | 1px | 1dp visual | THEME |
| Resize hit target | effectively tied to frame geometry | ~6dp invisible/effective grab | SOURCE |
| Title button glyph | fixed XPM | scale-aware vector/procedural glyph | SOURCE |
| Title button hit width | fixed resource width | ~42dp | THEME prototype possible; SOURCE final |
| Close red hover | can use rollover asset | semantic destructive hover | THEME prototype; SOURCE preferred |
| Hide title when maximized | Arc enables | disabled by default | THEME/CONFIG |
| Frame rounded corners | not Arc default; masks possible | 6–8dp floating | THEME binary prototype; SOURCE scale-aware |
| Antialiased window corners | no | desirable optional | COMPOSITOR for exact polish |
| Window shadow | no WM compositor | restrained optional | COMPOSITOR |
| Menu background | themeable | surface token | THEME |
| Menu row height | hardcoded from font/icon + legacy padding | 34dp | SOURCE |
| Menu horizontal padding | legacy hardcoded | 10–12dp | SOURCE |
| Menu selection radius | none | 4dp | SOURCE |
| Flat menu legacy top/bottom active lines | hardcoded | none | SOURCE |
| Popup outer radius | none as design token | 8dp | SOURCE/X Shape |
| Tooltip radius/padding | legacy | 4dp modern | SOURCE |
| Start menu clutter | configurable + source-appended legacy items | apps/categories/search + power only | CONFIG can reduce interim; SOURCE final |
| Start search field | absent | required | SOURCE |
| Start categories panel | cascading menus | fixed navigation column | SOURCE; reuse `fdomenu` data |
| Start favorites | toolbar/menu concepts separate | first-class | SOURCE |
| Taskbar color | themeable | Flame surface | THEME |
| Taskbar explicit height | emergent from applets | 40/48/56 logical | SOURCE |
| Taskbar per-monitor scale | absent | required | SOURCE |
| Icon-only task buttons | existing option exists | default | CONFIG interim |
| Active app underline/pill | not native modern state | required | SOURCE |
| Pinned + running unified model | separate toolbar/task concepts | required | SOURCE |
| Taskbar consistent hit targets | intrinsic applet sizes | required | SOURCE |
| Workspace visuals | classic buttons | compact modern indicators | THEME partly; SOURCE final |
| System tray bevel | optional | no bevel | THEME/CONFIG |
| Tray icon max size | global | scale-aware 22dp | THEME prototype; SOURCE final |
| Clock single line | existing | two-line time/date preferred | CONFIG if sufficient; SOURCE if stacked renderer needed |
| Volume popover | absent | required | SOURCE |
| Network popover | absent | required | SOURCE |
| Media popover | absent | required | SOURCE |
| Breeze icons | YIcon supports theme/SVG lookup | default | CONFIG/THEME + packaging |
| Desktop context menu | root menu behavior | minimal desktop actions | SOURCE/config integration |
| Desktop files/grid | absent | required by project | SOURCE/new component per project architecture |
| Desktop watermark | absent | subtle Flame mark | SOURCE/new desktop layer |
| QuickSwitch colors/margins | themeable | Breeze-like | THEME + CONFIG first |
| QuickSwitch exact modern layout | existing preview mode | tune after prototype | SOURCE only if current renderer insufficient |
| Shell global 100/125/150/175/200 scale | absent | required | SOURCE |
| Independent monitor scale | absent | required for Flame shell | SOURCE |
| Arbitrary application internal scale | not WM-owned | toolkit-dependent | SESSION/application limitation |
| GTK/Qt widget appearance | not IceWM theme | coherent light/dark | SESSION |
| Blur/acrylic | absent | deliberately nonessential | COMPOSITOR optional |

---

# 16. Source-specific visual defects/quirks worth fixing in FlameWM

These are IceWM quirks exposed by source review that are relevant to modern polish.

## 16.1 `YMenuItem::queryHeight()` hardcodes look-based padding

Root issue:

```text
modern density cannot be expressed cleanly by theme metrics
```

Fix:

```text
introduce Flame menu metrics
```

Do not alter legacy IceWM themes globally if the fork wants compatibility. Gate modern geometry under the Flame shell/theme mode.

## 16.2 `YMenu::paintItem()` contains legacy flat/metal edge lines

Root issue:

```text
lookFlat still means legacy flat button drawing, not modern flat design
```

Fix:

Create a Flame menu paint path:

```text
opaque surface
rounded selection plate
no 3D/top-bottom highlight lines
semantic focus/disabled state
```

## 16.3 `TaskBar::updateLayout()` uses scattered literal margins and intrinsic sizes

Root issue:

```text
panel has no single visual geometry authority
```

Fix:

- explicit panel logical height;
- left/center/right lanes;
- scale-aware hit targets;
- shared spacing tokens;
- deterministic overflow priorities.

Do not rewrite the window manager core to accomplish this.

## 16.4 Title button sizing consumes fixed pixmap widths

Root issue:

```text
visual resource dimensions become interaction geometry
```

Fix:

Separate:

```text
ButtonRect
GlyphRect
```

and draw the glyph centered.

## 16.5 Classic Start source adds configuration concepts

Root issue:

```text
StartMenu is a WM administration menu, not a modern app launcher
```

Fix:

New `FlameLauncher`; keep classic menu available only as expert/fallback if desired.

## 16.6 Global theme metrics block independent monitor scale

Root issue:

```text
one global `wsTitleBar`, `smallIconSize`, etc.
```

Fix:

logical base theme + `ScaledMetrics` per output/scale.

## 16.7 Frame corner assets can shape windows but are not a radius system

Root issue:

```text
rounding is encoded in pixmap masks, not semantic geometry
```

Fix:

source-generated mask from logical radius; theme token selects radius/profile.

## 16.8 No modern surface abstraction for non-menu popovers

Root issue:

```text
volume/network/media cannot be expressed well as YMenu rows
```

Fix:

one lightweight `FlamePopover` primitive for anchored panels.

Do not build a generic widget framework; implement only what the product needs.

## 16.9 Hard-coded old visual assumptions appear in comments/TODOs

The taskbar source itself contains legacy layout notes such as `// !!! for now`, and the upstream TODO already acknowledges taskbar modularization debt.

FlameWM should treat the taskbar renderer/layout as a bounded modernization zone, not preserve every layout quirk merely for historical compatibility.

---

# 17. Proposed Flame-native rendering architecture

Do not convert IceWM into Qt or GTK.

Use the existing native toolkit, but introduce a tiny semantic visual layer.

## 17.1 `FlameDesignTokens`

Own:

```text
colors
spacing
radii
semantic sizes
font roles
animation durations
```

Example conceptual API:

```cpp
struct FlameDesignTokens {
    FlamePalette palette;
    FlameLogicalMetrics metrics;
    FlameTypography typography;
};
```

## 17.2 `FlameScaleManager`

Own:

```text
output -> scale
logical -> physical conversion
scale change notifications
```

## 17.3 `FlameSurfacePainter`

Provide bounded helpers:

```text
paintPanelSurface
paintPopupSurface
paintSelectionBackplate
paintHoverBackplate
paintFocusRing
paintSeparator
```

Do **not** create a CSS engine.

## 17.4 `FlameGlyphRenderer`

Own simple WM glyphs:

```text
minimize
maximize
restore
close
chevron
check
radio/dot
```

This avoids fixed XPM title-button scaling problems.

Application icons remain `YIcon`/Breeze.

## 17.5 `FlamePopover`

One reusable anchored surface for:

```text
volume
network
media details
calendar/date
possibly task group surfaces
```

It should support layout and basic controls, not become a plugin framework.

---

# 18. Arc-Dark-derived interim theme specification

Before all source work is complete, FlameWM can use a deliberately improved Arc-style theme to make development sessions visually closer to the final direction.

This should be treated as an **interim development theme**, not the final renderer.

Suggested 100%-scale baseline:

```ini
Look=flat

# Window
TitleBarHeight=32
BorderSizeX=1
BorderSizeY=1
DlgBorderSizeX=1
DlgBorderSizeY=1
HideTitleBarWhenMaximized=0
TitleButtonsLeft="s"
TitleButtonsRight="xmi"
ShowFrameIcon=1

# Icons
MenuIconSize=22
SmallIconSize=22
LargeIconSize=32
HugeIconSize=48
TrayIconMaxWidth=22
TrayIconMaxHeight=22

# Typography
TitleFontNameXft="Noto Sans:size=10.5"
MenuFontNameXft="Noto Sans:size=10.5"
NormalButtonFontNameXft="Noto Sans:size=10.5"
NormalTaskBarFontNameXft="Noto Sans:size=10.5"
ActiveTaskBarFontNameXft="Noto Sans:size=10.5:medium"
NormalWorkspaceFontNameXft="Noto Sans:size=10.5"
ActiveWorkspaceFontNameXft="Noto Sans:size=10.5:medium"
ClockFontNameXft="Noto Sans:size=10"

# Remove legacy tray bevel
TrayDrawBevel=0
RolloverButtonsSupported=1
```

The actual Flame palette should come from one shared palette source rather than copying Arc-Dark's exact values.

This interim theme will still have incorrect menu padding, panel semantics, per-monitor scaling and legacy Start structure. That is expected.

---

# 19. Proposed FlameWM dark palette direction

Do not hardcode these across many files. This is a semantic reference palette only.

A FlameWM dark shell should have approximately:

```text
surface0 / desktop shell darkest:   #1b1d22 class
surface1 / panel/menu:              #24272d class
surface2 / hover/raised:            #2d3138 class
surface3 / selected neutral:        #353a43 class
primary text:                       #e6e9ee class
secondary text:                     #aab0ba class
disabled text:                      #747b86 class
border/separator:                   white ~8–12% equivalent or #3a3f47 class
accent default:                     FlameWM red from brand system
accent hover:                       derived lighter red
accent pressed:                     derived darker red
close destructive hover:           familiar red/destructive semantic
```

The latest explicit FlameWM v2 direction chooses red as the default shell accent for brand identity. This intentionally supersedes the earlier Product Soul sentence that proposed a familiar cool-blue interaction accent. Therefore Arc-Dark's `#5294E2` blue should not survive as the default active-state identity merely because Arc uses it.

This is a documented product-source reconciliation, not an accidental palette drift: **latest explicit direction = red default accent**. Breeze supplies the icon grammar; FlameWM supplies the brand accent.

---

# 20. Light theme requirements

Do not design dark first and invert it later.

The light variant must have its own coherent surfaces:

```text
window/panel neutral near white/light gray
clear 1px separators
very dark primary text
muted secondary text
red accent
same spacing/radius metrics
same icon geometry
same hit targets
```

Geometry and spacing must be invariant between light/dark. Only semantic colors/assets should change.

---

# 21. Scaling behavior by component

## 21.1 Titlebar

When a window enters another monitor, resolve the new frame scale from the frame's target screen.

Do not rescale continuously from every pointer pixel if that causes geometry jitter. Use the same monitor ownership rule IceWM already uses for frame screen selection, or a deterministic center/majority rule.

When scale changes:

```text
rebuild frame metrics
rebuild glyph/resource sizes
preserve client content geometry as safely as possible
recompute outer frame
repaint
```

## 21.2 Panel

A panel belongs to one output and therefore has one unambiguous scale.

This is the easiest component to make truly per-monitor scale-correct.

## 21.3 Menus/popovers

Resolve scale from the target popup monitor at opening time.

If the popup is repositioned to another monitor to remain on-screen, use the final target monitor scale before final layout.

## 21.4 Desktop icons

Resolve grid/icon metrics per output.

A 48dp desktop icon becomes:

```text
48px @100%
60px @125%
72px @150%
96px @200%
```

Breeze SVGs should be rasterized/requested at the target size.

## 21.5 Cursor

Cursor scaling is a session/Xcursor concern and should track the monitor strategy as well as X11 realistically allows. Do not tie it to titlebar bitmap sizes.

---

# 22. What “perfect” should mean for FlameWM

It should **not** mean pixel-for-pixel cloning Plasma.

It should mean that a user accustomed to KDE Plasma or Windows 11 does not encounter the visual/interaction tells that normally announce “old lightweight WM”.

The following tells must disappear:

- 20px titlebars on a 1080p/HiDPI screen;
- tiny 16px taskbar controls as the default;
- bevels and 3D menu edges;
- text-heavy rectangular task buttons;
- cascading Start menus stretching across the screen;
- Control Centre/Focus/Themes/Run internals in Start;
- arbitrary raw pixel density;
- inconsistent 1/2/4px magic spacing;
- tiny resize edges;
- bitmap title glyphs that blur when scaled;
- panel height controlled by whichever legacy applet happens to be tallest;
- every popup being a classic menu;
- toolkit/application icon inconsistency;
- no per-monitor shell scaling.

At the end, FlameWM should still be recognizably its own product:

- Flame start icon;
- red default accent;
- subtle FlameWM desktop mark;
- simpler settings than Plasma;
- fewer shell concepts;
- far fewer resident services;
- deterministic fixed composition.

---

# 23. Development priority — visual conversion order

The order matters.

## P0 — Scale/metrics foundation

Implement before polishing assets:

1. `FlameDesignTokens` logical metrics.
2. `FlameScaleManager` per-output scale.
3. scale-keyed font/icon/resource caches.
4. output-scale selection for frames/panels/popups.
5. tests for 100/125/150/200 and mixed monitors.

**Reason:** every later visual component depends on this.

## P1 — Window chrome

1. 32dp default titlebar.
2. separate visual border/resize grab.
3. native/procedural title glyphs.
4. modern hover states.
5. close destructive hover.
6. scale-aware rounded shape mask.
7. retain titlebar when maximized.

## P2 — Panel visual architecture

1. explicit semantic panel height.
2. three-lane layout.
3. Flame Start button.
4. icon-only running apps.
5. active/running indicators.
6. modern workspaces.
7. consistent tray/status hitboxes.
8. date/time layout.

## P3 — Modern menu/popover painter

1. modern row metrics.
2. 22dp menu icons.
3. rounded selection plate.
4. rounded popup shape.
5. modern tooltip geometry.
6. reusable `FlamePopover`.

## P4 — Flame Launcher

1. app model from existing XDG parser.
2. search.
3. categories.
4. favorites.
5. power/session footer.
6. no legacy WM configuration entries.

## P5 — System popovers

1. audio.
2. network.
3. media.
4. battery/calendar as product scope requires.

## P6 — Application/session consistency

1. Breeze icons.
2. cursor.
3. GTK/Qt color/theme defaults.
4. font defaults.
5. light/dark propagation.

## P7 — Optional compositor polish

Only after the non-composited shell is excellent:

- soft shadows;
- antialiased outer rounding;
- controlled translucency.

Do not block FlameWM completion on blur.

---

# 24. Verification matrix

Visual work needs deterministic gates, not subjective “looks better” only.

## 24.1 Scaling tests

Test at:

```text
1920x1080 @100%
2560x1440 @100%
3840x2160 @150%
3840x2160 @200%
```

Mixed-monitor cases:

```text
1920x1080 @100% + 3840x2160 @150%
1920x1080 @125% + 2560x1440 @100%
left monitor with negative X origin
right monitor with positive X origin
```

Assert:

- panel physical size matches logical scale;
- titlebar matches scale;
- menus opened on each monitor match scale;
- icons select/rasterize correct target sizes;
- pointer hit targets scale;
- frame movement between outputs does not corrupt geometry;
- no global theme-size mutation affects other monitors.

## 24.2 Geometry visual tests

For each scale:

- title text vertically centered;
- title glyphs pixel-aligned where possible;
- close/maximize/minimize hitboxes equal;
- border is visually 1 logical pixel;
- resize target remains easy to hit;
- rounded mask has expected radius;
- maximized/snap edges become square where required.

## 24.3 Menu tests

Assert:

- all normal menu rows equal logical height;
- icon/text baseline alignment is stable;
- selection backplate remains inside row padding;
- submenu arrow aligns to trailing edge;
- separators have consistent inset;
- popup fits monitor work area;
- popup opened on 150% monitor does not use 100% geometry.

## 24.4 Start tests

Assert:

- only application categories/apps and approved footer/session actions appear;
- no IceWM Themes/Focus/Preferences/Control Centre tree leaks into the default launcher;
- search has focus immediately;
- keyboard operation works;
- app launch uses safe desktop-entry semantics;
- category/app ordering deterministic.

## 24.5 Panel tests

Assert:

- panel has explicit height independent of applet natural heights;
- icons centered;
- active app indicator visible;
- pinned/running state stable;
- tray does not change panel height;
- no overlap at narrow width;
- per-output scale correct;
- vertical panel mode later uses same logical system.

## 24.6 Screenshot baselines

Maintain reference screenshots in the FlameWM repository for:

```text
light 100%
dark 100%
dark 150%
mixed DPI
Start open
volume popover open
window active/inactive
snap preview
Alt+Tab
```

Screenshot comparison should detect major visual regressions, but semantic/X11 tests remain authoritative for behavior.

---

# 25. Concrete first-pass “Arc-Dark to FlameWM” delta checklist

This is the shortest operational checklist for a first visual implementation pass.

## Theme/config changes that can happen immediately

- [ ] Fork Arc-Dark concepts into `lib/themes/Flame/`, do not ship under Arc branding.
- [ ] Replace blue accent with FlameWM red accent tokens.
- [ ] Replace Verdana with Noto Sans / generic system sans.
- [ ] Increase development titlebar to ~32px at 100% until scale system lands.
- [ ] Disable `HideTitleBarWhenMaximized`.
- [ ] Remove tray bevel.
- [ ] Use Breeze icons through existing icon-theme lookup.
- [ ] Increase menu/small icon reference sizes toward 22px for the development theme.
- [ ] Enable icon-only taskbar (`TaskBarShowWindowTitles=0`) for the prototype.
- [ ] Hide CPU/MEM/net graph/mail/address-bar legacy applets from Flame defaults.
- [ ] Hide Themes/Focus/legacy Settings/Run/WindowList entries from default Start during transition.
- [ ] Keep logout/session actions only in an approved footer/fallback path.

## Source changes required before visual completion

- [ ] Logical design tokens.
- [ ] Per-output scale manager.
- [ ] Per-output metrics cache.
- [ ] Scale-aware font cache.
- [ ] Scale-aware title glyph renderer.
- [ ] Separate visual border from resize hit target.
- [ ] Scale-aware rounded frame shape.
- [ ] Modern menu row geometry.
- [ ] Modern menu selection painting.
- [ ] Popup radius/shape.
- [ ] Explicit panel semantic height.
- [ ] Three-lane panel layout.
- [ ] Modern task app indicators.
- [ ] Unified pinned/running app identity.
- [ ] Flame Launcher.
- [ ] Flame Popover base.
- [ ] Audio/network/media surfaces.
- [ ] Desktop context menu reduction.
- [ ] Session/application appearance integration.

---

# 26. Source ownership map for the visual work

| Concern | Primary source owner |
|---|---|
| logical theme metrics | `src/themable.h` + new Flame metrics layer |
| global preferences | `src/default.h` |
| per-output geometry/monitor mapping | existing XRandR/desktop/window manager code + new scale manager |
| title button hitbox/glyph | `src/wmbutton.cc/.h` |
| title layout/paint | `src/wmtitle.cc/.h` |
| frame border/shape | `src/decorate.cc`, `src/wmframe.cc/.h` |
| menu row geometry | `src/ymenuitem.cc/.h` |
| menu popup layout/paint | `src/ymenu.cc/.h` |
| Start legacy behavior | `src/wmprog.cc/.h` |
| app category parsing | `src/fdomenu.cc` |
| taskbar layout | `src/wmtaskbar.cc/.h` |
| task app rendering/state | `src/atasks.cc/.h` |
| workspaces | `src/aworkspaces.cc/.h` |
| XEmbed tray | `src/yxtray.cc/.h` |
| system icon theme/SVG lookup | `src/yicon.cc/.h` |
| theme pixmaps | `src/wpixres.cc`, `src/wpixmaps.*` |
| desktop layer | project-defined desktop component |
| Flame Launcher | new bounded Flame source pair(s) |
| Flame popovers | new bounded Flame source pair(s) |
| scale manager | new bounded Flame source pair(s) |

---

# 27. What should remain unchanged

Do not confuse visual modernization with a full IceWM rewrite.

The following should remain the mature engine underneath:

- EWMH/ICCCM behavior;
- focus/stacking;
- frame/client ownership;
- work-area calculations;
- XRandR monitor geometry;
- workspace engine;
- existing snap geometry authority where already correct;
- XEmbed tray protocol;
- YIcon system icon lookup;
- XDG application parsing logic where reusable;
- event loop;
- low-level X11 drawing primitives.

The visual project is successful if the old engine becomes invisible to ordinary users—not if it is deleted.

---

# 28. Final target definition

The final FlameWM desktop should look approximately like the supplied KDE Plasma reference **in visual maturity**, but with FlameWM's own identity and simpler UX.

A successful result has:

```text
modern logical scaling
large enough titlebar hit targets
thin visual frames
gentle rounding
clean sans typography
Breeze icon grammar
red FlameWM accent
48dp-class default panel
icon-first pinned/running task area
structured Start launcher
application categories instead of WM internals
clean system popovers
consistent spacing
consistent hover/pressed states
per-monitor FlameWM shell scale
subtle FlameWM branding
no legacy CPU/RAM graphs or config clutter by default
no heavy desktop framework dependency
```

The visual outcome should communicate:

> **This is a modern desktop that happens to use IceWM internally.**

It should no longer communicate:

> “This is IceWM with a nice theme.”

That distinction is the entire FlameWM product opportunity.

---

# 29. Final implementation decision

Use Arc-Dark only as evidence that IceWM's current theme engine can already supply the right **flat dark baseline**.

Do not base the final UI architecture on Arc-Dark's fixed XPM sizes or legacy IceWM menu/taskbar geometry.

The visual transformation should be implemented as:

```text
KEEP
    IceWM 4.1.0 WM core
    XRandR geometry
    YWindow/Graphics native toolkit
    YIcon + SVG/icon-theme lookup
    XDG desktop-entry parsing
    task/window/workspace state engines
    system tray protocol
    existing frame shape capability

REUSE FROM ARC-DARK CONCEPTUALLY
    flat look
    thin visual borders
    quiet inactive states
    dark neutral palette structure
    simple window-control glyph vocabulary

REPLACE / EXTEND IN SOURCE
    raw pixel geometry -> logical scale-aware metrics
    global-only shell metrics -> per-output resolved metrics
    fixed XPM title controls -> scale-aware semantic glyphs
    1px resize target -> thin visual border + larger interaction edge
    legacy menu density -> modern row/padding/radius system
    classic Start YMenu -> Flame Launcher
    emergent taskbar height -> explicit semantic panel height
    text-button task model -> modern icon/pinned/running presentation
    generic menu surfaces -> dedicated popovers where needed
    bitmap corner behavior -> scale-aware radius/mask behavior

ADD OUTSIDE WM THEME
    Breeze icon packaging/default
    client GTK/Qt appearance defaults
    cursor/font integration
    optional compositor polish only after core passes memory gates
```

This is the shortest credible path from IceWM/Arc-Dark to the visual quality represented by the KDE screenshot while preserving FlameWM's actual purpose: **IceWM made friendly by ArkFlame Studios.**

---

# 30. External references

## IceWM / Arc-Dark

- IceWM upstream: https://github.com/ice-wm/icewm
- FlameWM fork: https://github.com/linsaftw/flamewm
- IceWM extra themes: https://github.com/ice-wm/icewm-extra
- Arc-Dark directory: https://github.com/ice-wm/icewm-extra/tree/main/Arc-Dark
- Arc-Dark theme file: https://github.com/ice-wm/icewm-extra/blob/main/Arc-Dark/default.theme
- IceWM manual: https://ice-wm.org/man/icewm.html
- IceWM preferences: https://ice-wm.org/man/icewm-preferences.html

## KDE/Breeze design grounding

- KDE HIG layout/navigation: https://develop.kde.org/hig/layout_and_nav/
- KDE HIG icons: https://develop.kde.org/hig/icons/
- KDE Breeze icons: https://develop.kde.org/frameworks/breeze-icons/
- Plasma units reference: https://develop.kde.org/docs/plasma/widget/plasma-qml-api/

Relevant KDE HIG concepts used in this report include:

- `gridUnit`-based consistent sizing;
- standard spacing roles;
- DPI-scaled icon sizes;
- small/medium icon-size roles;
- one shared `cornerRadius` concept rather than arbitrary per-widget radii.

## Windows 11 geometry grounding

- Windows 11 geometry: https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/geometry
- Rounded desktop windows: https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-rounded-corners

Windows 11 uses a useful modern hierarchy of approximately:

```text
4px control radius
8px top-level/flyout radius
0px where snapped/maximized edges must meet
```

FlameWM does not need to clone Windows, but this hierarchy is a strong sanity check for its own moderate rounding system.

---

# 31. Local source anchors verified in IceWM 4.1.0

```text
src/themable.h:21-39
    global border/titlebar/icon sizes

src/themable.h:41-68
    QuickSwitch margins and global font roles

src/themable.h:70-151
    global semantic colors

src/ymenuitem.cc:60+
    menu item height/padding logic

src/ymenu.cc:901+
    popup width/height algorithm

src/ymenu.cc:1061+
    active-row paint behavior, including flat/metal legacy lines

src/wmbutton.cc:54+
    title button height directly derived from `wsTitleBar`

src/wmprog.cc:330+
    Start menu legacy entries appended by source

src/wmtaskbar.cc:446+
    hard-coded applet layout and margins

src/decorate.cc:250+
    frame corner masks / X Shape composition

src/yicon.cc:357+
    icon-theme path indexing

src/yicon.cc:422-427
    PNG/XPM/SVG candidate resolution

src/yicon.cc:511+
    icon load and requested-size scaling
```

---

# 32. Canonical one-paragraph summary for future sessions

**Arc-Dark is the right reference baseline but not the final FlameWM solution. It proves IceWM can support a flat dark shell, thin borders and modern colors, yet its 21px titlebar, 16px icon grammar, fixed XPM title controls, legacy YMenu density, classic Start-menu composition, emergent taskbar height and global raw-pixel theme metrics are the main reasons it still looks like IceWM rather than KDE Plasma/Windows 11. FlameWM should retain IceWM's WM/XRandR/YIcon/XDG engines, introduce a logical design-token and per-output scaling layer first, then modernize window chrome, menu/popover geometry, taskbar/task rendering, and replace the classic Start YMenu with a dedicated app-category/search launcher. A theme can solve palette, one-scale sizes, fonts, border thickness and basic assets; source changes are required for per-monitor scale, modern padding/radii/hit targets, unified pinned/running tasks, launcher structure, popovers and semantic state rendering. Client application interiors require separate GTK/Qt/session theming, and exact antialiased shadows/outer rounding require optional compositing. The goal is not Plasma code or a Plasma clone: it is a modern FlameWM shell whose IceWM ancestry is invisible in ordinary use.**
