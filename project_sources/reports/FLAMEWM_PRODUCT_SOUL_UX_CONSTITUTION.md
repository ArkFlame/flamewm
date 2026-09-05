# FlameWM Product Soul & UX Constitution

**Project:** FlameWM  
**Studio:** ArkFlame Studios  
**Base:** IceWM fork  
**Document role:** Canonical product-source specification for FlameWM identity, UX behavior, visual direction, defaults, personalization boundaries, and implementation decision-making.  
**Status:** Foundational / normative  
**Last updated:** 2026-09-02

---

## 0. Assumptions and scope

This document defines **what FlameWM is supposed to feel like and how users are supposed to interact with it**. It is intentionally not a low-level implementation plan and not a pixel-perfect UI specification.

Assumptions:

- FlameWM is a real IceWM fork, not merely an IceWM theme pack.
- IceWM remains the underlying lightweight X11 window-management engine unless a future project decision explicitly changes that architecture.
- FlameWM may extend or replace user-facing IceWM surfaces where necessary, while retaining IceWM internals when they provide the required behavior efficiently.
- FlameWM is designed first for desktop and laptop users who already understand the common Windows/KDE desktop model.
- The default experience is the product. FlameWM does not ask the user to assemble their own desktop.
- The visual reference is the clarity and familiarity of modern Windows and KDE Plasma/Breeze, but FlameWM must have its own coherent identity and should not become a visual clone dependent on Plasma.
- Advanced IceWM configuration may remain possible through configuration files for expert users, but it is **not part of the normal FlameWM UX**.
- Performance, low memory use, fast startup, and low background-service count are product constraints. Visual familiarity must not be achieved by importing an entire heavyweight desktop stack.

---

# 1. One-sentence definition

> **FlameWM is IceWM transformed from a configurable lightweight window manager into a complete, familiar, opinionated desktop experience that feels immediately understandable to Windows and KDE users while preserving IceWM-class efficiency.**

That sentence is the north star.

FlameWM is not trying to win a customization contest.

FlameWM is trying to make the lightweight desktop feel **finished**.

---

# 2. The problem FlameWM exists to solve

Traditional lightweight window managers often achieve low resource usage by transferring complexity to the user.

They expose configuration files, theme internals, applet choices, toolbar construction, exact dimensions, implementation terminology, manual menu generation, or a collection of loosely integrated programs.

Large desktop environments solve much of that usability problem, but they introduce a much larger software surface, more services, more components, more memory consumption, and more system behavior than many users actually need.

FlameWM exists between those extremes.

The product thesis is:

> **A lightweight desktop does not need to look unfinished, behave like an enthusiast toolkit, or require configuration knowledge.**

A user should be able to install a system using FlameWM and immediately understand:

- where applications are;
- where open windows are;
- how to switch windows;
- how to change the wallpaper;
- how to change volume or Wi-Fi;
- how to see the time;
- how to move and tile windows;
- how to use multiple desktops;
- how to shut down;
- how to make the desktop slightly more personal.

They should not need to understand IceWM to use FlameWM.

---

# 3. Product promise

FlameWM promises five things.

## 3.1 Familiar immediately

The first session must have the visual and behavioral grammar people already know from Windows and KDE-like desktops:

- desktop background;
- bottom taskbar by default;
- Start button at the left;
- running applications in the taskbar;
- virtual desktops / overview;
- system status on the right;
- clock and date at the far end;
- conventional window titlebar controls;
- familiar right-click behavior;
- familiar window snapping;
- a normal settings application.

A new user should not need a tutorial to perform ordinary desktop tasks.

## 3.2 Fast by construction

FlameWM must preserve the reason IceWM exists: low overhead.

The UX layer may add polish, but it must not recreate Plasma by launching a collection of Plasma-class daemons.

Every permanent process must justify its existence.

Every visual effect must have a cheap path.

Every new system surface should prefer event-driven behavior over polling.

Idle means idle.

## 3.3 Finished by default

The user installs FlameWM and receives a desktop, not a construction kit.

The taskbar is already arranged.

The colors already work.

The menus already have sensible spacing.

Window buttons already look conventional.

The wallpaper already looks intentional.

The system tray is already usable.

The default layout should be good enough that many users never open FlameWM Settings.

## 3.4 Personal, not infinitely customizable

FlameWM lets users personalize the parts normal desktop users expect to personalize.

They can change the wallpaper, accent, appearance, taskbar size and transparency, taskbar edge, icon preference, and a limited number of obvious behavior choices.

They do not design their own shell.

## 3.5 Predictable

The same action should have the same result every time.

No hidden edit mode.

No movable widgets accidentally detaching.

No panel entering a configuration state because of a strange context-menu option.

No application of a setting only after logout unless technically unavoidable.

No settings whose meaning requires knowledge of X11, EWMH, IceWM preference names, pixel geometry, or compositor internals.

---

# 4. The soul of FlameWM

The soul of FlameWM can be described with six words:

> **Familiar. Calm. Fast. Finished. Focused. Lightweight.**

Everything should reinforce those characteristics.

FlameWM should feel like somebody took the desktop model millions of people already understand and removed everything that did not need to be there.

It should not feel experimental.

It should not feel retro merely because it is lightweight.

It should not feel like a collection of Linux utilities.

It should not feel like “IceWM with a blue theme.”

It should feel like a small, deliberate desktop operating environment whose window manager happens to be extremely efficient.

---

# 5. Product model: engine versus experience

The architecture must maintain a conceptual boundary:

## IceWM is the engine

IceWM provides low-level capabilities such as:

- window management;
- focus;
- workspaces;
- task tracking;
- taskbar foundations;
- system tray support;
- application menus;
- window decorations;
- themes;
- keyboard shortcuts;
- Alt+Tab / QuickSwitch;
- wallpaper support through `icewmbg`;
- existing configuration machinery.

The official IceWM documentation confirms that it already supports configurable workspaces, a taskbar, start menu, system tray, window/task buttons, QuickSwitch including preview mode, themes, desktop backgrounds, and edge/window snapping. [S1][S2][S3]

## FlameWM is the experience

FlameWM owns:

- the visual system;
- the default theme;
- the default wallpaper;
- the FlameWM brand;
- the taskbar composition;
- the Start experience;
- the desktop context menu;
- the workspace UX;
- window layout UX;
- the user-facing settings application;
- what is configurable through the GUI;
- terminology;
- default shortcuts;
- system popovers;
- live-setting behavior;
- first-run expectations;
- interaction quality.

Users interact with FlameWM concepts.

IceWM concepts are implementation details unless exposing one directly produces a better mainstream experience.

---

# 6. Canonical design laws

These laws are normative. Future features should be evaluated against them.

## 6.1 Simple by default is mandatory

The default view should show what the user is likely to need now, not everything the software knows how to do.

KDE's own HIG explicitly recommends a “simple by default” approach, grouping related features on separate pages and avoiding unnecessary controls. GNOME's HIG similarly recommends progressive disclosure and putting common actions close at hand. FlameWM adopts that principle aggressively. [S4][S5]

## 6.2 The desktop is not an editor

There is no Plasma-style panel edit mode.

There is no widget-placement mode.

There is no “unlock widgets.”

There is no drag-anything-anywhere desktop construction model.

Direct manipulation is allowed only where the action is obvious and bounded, such as dragging the taskbar to another screen edge.

## 6.3 Prefer semantic choices over implementation values

The user should normally choose:

- Compact / Comfortable / Large;
- Bottom / Top / Left / Right;
- Fill / Fit / Stretch / Center;
- Light / Dark / System;
- Solid / Translucent.

The user should not normally choose:

- 43 px;
- 7 px border;
- 0.71 opacity;
- 11 px icon margin;
- 4 px title padding.

Internally FlameWM can use exact numbers. The settings surface should use concepts.

A bounded slider is acceptable when continuous control has obvious value, such as taskbar transparency, but the user should still see a meaningful preview rather than being forced to reason in raw implementation units.

## 6.4 Familiar beats novel

For common desktop behavior, established conventions are preferred unless they create a concrete problem.

Examples:

- Start at lower-left by default;
- minimize, maximize, close at upper-right;
- right-click opens relevant actions;
- Alt+Tab switches windows;
- Super opens Start;
- Super+arrow performs snapping;
- clock opens date/calendar;
- volume icon controls audio;
- Wi-Fi icon controls networking.

FlameWM should spend innovation budget on making lightweight computing better, not on relocating controls for aesthetic novelty.

## 6.5 One obvious path for ordinary tasks

A user should not have to choose between five different configuration tools.

Examples:

- Wallpaper → FlameWM Settings → Desktop.
- Taskbar → FlameWM Settings → Taskbar.
- Appearance → FlameWM Settings → Appearance.
- Window layout behavior → FlameWM Settings → Windows.

Context menus may provide shortcuts into those pages, but they should not create separate configuration systems.

## 6.6 Context is the menu

Menus should contain actions relevant to what was clicked.

The desktop menu is about the desktop.

The taskbar menu is about the taskbar.

A workspace button menu is about that workspace.

A window menu is about that window.

Do not place unrelated system utilities into every right-click menu merely because they are available.

## 6.7 Apply immediately

Safe personalization settings should update as soon as the user changes them.

The KDE HIG recommends immediately applying newly saved settings instead of requiring application relaunch. FlameWM should go further where safe: most appearance controls should preview and persist directly. [S6]

Examples:

- accent changes live;
- wallpaper changes live;
- taskbar transparency changes live;
- taskbar size changes live;
- taskbar edge changes live;
- icons update live where technically safe.

Do not make users click Apply after obvious cosmetic changes unless a transaction genuinely requires it.

## 6.8 Advanced capability may exist without becoming interface

IceWM's configuration system can remain available.

FlameWM should not delete power merely to hide complexity.

Instead:

> **Expose the common 90% as product UX; leave specialist configuration as an expert/config-file capability.**

The GUI is curated.

The engine can remain capable.

## 6.9 Settings are not a substitute for product decisions

If the team cannot decide how a feature should behave, “add an option” is not automatically acceptable.

KDE's own “Powerful when needed” guidance explicitly warns against using customizability to avoid design decisions. FlameWM adopts that as a core rule. [S6]

## 6.10 No heavyweight dependency for cosmetic imitation

FlameWM may take visual and interaction inspiration from Breeze and Windows.

It must not require a full KDE Plasma runtime merely to look familiar.

If an integration can be achieved through lightweight standards, freedesktop interfaces, X11/EWMH mechanisms, D-Bus, NetworkManager, PipeWire/PulseAudio compatibility, or a small dedicated FlameWM component, prefer that path.

## 6.11 Brand quietly

FlameWM branding should feel like product identity, not advertising.

The desktop should remind the user what environment they are using without putting logos on every surface.

## 6.12 User error must be difficult

The desktop should resist accidental destruction of its own layout.

There is no accidental panel deletion.

There is no ability to remove the last virtual desktop.

Dangerous actions must be bounded.

Settings should either be reversible or have a clear Reset to defaults path.

---

# 7. Desired emotional experience

FlameWM should feel:

### On first boot

Clean, modern, obvious, lightweight.

The user should immediately recognize a desktop rather than needing to discover how the window manager works.

### After one hour

Predictable.

The user should have stopped thinking about FlameWM and started thinking about their applications.

### After one week

Fast and dependable.

The desktop should feel like it wastes neither memory nor attention.

### After months

Personal enough to belong to the user without having turned into a fragile custom configuration.

The user may have changed a wallpaper, accent, taskbar edge, size, transparency, and several favorites. The basic FlameWM identity remains intact.

---

# 8. Visual identity

## 8.1 Overall visual language

FlameWM uses a **Breeze-informed modern desktop grammar**:

- flat surfaces;
- restrained depth;
- clean geometric icons;
- cool neutral backgrounds;
- thin separators;
- clear active/inactive states;
- generous enough spacing to feel modern;
- moderate corner rounding;
- no glossy gradients;
- no fake chrome;
- no skeuomorphic frames;
- no heavy blur dependency;
- no giant mobile-style controls on desktop.

Breeze itself is KDE Plasma's default visual style and includes color schemes, cursors, window decorations, application styling, and wallpapers. FlameWM may use compatible/openly licensed assets where legally and technically appropriate, but FlameWM-specific components should have a coherent native identity rather than appearing as copied Plasma fragments. [S7][S8]

## 8.2 Brand hierarchy

The brand hierarchy is:

1. **FlameWM** — desktop/window manager product.
2. **ArkFlame Studios** — creator/publisher.
3. **IceWM heritage** — technical ancestry, visible in About/documentation rather than normal daily UI.

Normal desktop surfaces say FlameWM.

The About page says:

> FlameWM  
> A lightweight desktop by ArkFlame Studios  
> Built on IceWM

Do not label everyday controls with IceWM terminology unless necessary.

## 8.3 Logo usage

The FlameWM logo is the primary shell mark.

Preferred placements:

- Start button;
- desktop watermark;
- Settings/About;
- session/greeter branding when FlameWM controls that surface;
- default wallpaper composition;
- package/about metadata.

Do not place the logo:

- inside every settings page;
- next to every menu;
- on every titlebar;
- inside normal application chrome;
- repeatedly in the system tray.

## 8.4 Desktop watermark

A subtle FlameWM mark appears in the bottom-right region of the desktop.

Canonical behavior:

- visually separate from the wallpaper itself when possible, implemented as a desktop-layer overlay;
- does not move desktop files/icons;
- ignores pointer input;
- never behaves like a widget;
- disappears behind normal windows;
- disappears in fullscreen applications;
- remains aligned to the desktop work area, not underneath the taskbar;
- adapts to taskbar placement;
- remains subtle enough not to compete with wallpaper content.

Recommended visual target:

- monochrome FlameWM symbol or compact wordmark;
- low opacity;
- no glow;
- no drop shadow unless required for basic contrast;
- restrained maximum size;
- safe margin from screen edges and taskbar.

A default-enabled **Show FlameWM mark** option may exist under Desktop settings. This preserves the identity by default without turning branding into forced wallpaper modification.

If implementation cost favors embedding the mark in the bundled default wallpaper for the first milestone, that is acceptable as an interim solution, but the long-term UX target is an independent desktop overlay so user wallpapers are not destructively modified.

## 8.5 Color philosophy

The shell default is a familiar cool-blue accent.

The interaction accent is not the brand logo itself.

A subtle warm “flame” hue may appear in FlameWM brand artwork, but normal selection/focus states should use a calm high-legibility accent.

The chosen accent propagates coherently to:

- selected menu rows;
- active workspace indicator;
- active task indicator;
- selected settings controls;
- snap-layout selection;
- toggles/sliders;
- focus/selection highlights.

The accent does **not** recolor arbitrary third-party application content.

## 8.6 Light and dark appearance

FlameWM supports a simple appearance choice:

- System / Automatic when a usable system preference exists;
- Light;
- Dark.

The default may be Light unless the surrounding distribution defines another system default.

A high-quality light theme is mandatory.

A high-quality dark theme is part of the same visual system, not an inverted afterthought.

## 8.7 Typography

Use a conventional highly legible sans-serif desktop font from the host distribution.

Do not make typography a branding stunt.

Hierarchy comes from weight, size, spacing, and context—not decorative fonts.

User-facing FlameWM settings should not expose low-level font selection in the initial product unless accessibility requirements justify it.

## 8.8 Icons

Icons should be:

- flat;
- vector where possible;
- readable at taskbar scale;
- consistent in optical weight;
- recognizable before being clever;
- accompanied by text where meaning is not universally obvious.

FlameWM-specific icons should be designed in the same visual grammar.

Breeze Icons are an open-source KDE icon theme, but any reused assets must preserve their license requirements. [S8]

---

# 9. Desktop UX

The desktop is deliberately quiet.

Its responsibilities are:

- show the wallpaper;
- host desktop files/icons if FlameWM chooses to support them;
- expose the subtle FlameWM mark;
- provide a minimal desktop context menu;
- act as the spatial background for windows.

It is not a widget canvas.

## 9.1 Desktop context menu

Default right-click menu:

**Change background**  
Opens the Desktop settings page directly at wallpaper selection.

**Desktop settings**  
Opens the Desktop settings page.

Optionally, **Display settings** may be included if FlameWM owns or can reliably open a coherent display configuration surface. It should not be included merely as a generic launcher.

Do not include:

- terminal;
- file manager;
- random applications;
- themes submenu;
- workspace internals;
- refresh desktop unless there is a real user-visible need;
- create launcher;
- panel edit;
- window-manager configuration;
- implementation/debug actions.

If desktop file management is later provided, file-specific creation actions may be added only when the user clicks the desktop's file area and they remain ordinary filesystem actions.

## 9.2 Wallpaper behavior

The Desktop settings page provides:

- current wallpaper preview;
- Choose image;
- built-in wallpapers;
- recent wallpapers;
- fit mode.

Fit mode uses familiar words:

- Fill;
- Fit;
- Stretch;
- Center;
- Tile, only if supported and considered useful.

IceWM's `icewmbg` already provides image, color, centering, scaling, cycling, per-workspace, and multihead-related background functionality. FlameWM should translate useful capabilities into simple vocabulary rather than exposing the underlying preference names. [S2]

Default behavior should favor one coherent wallpaper across workspaces unless a user explicitly chooses per-workspace behavior in a later product revision.

## 9.3 Desktop icon behavior

If desktop icons are included:

- align to a clean grid;
- snap automatically;
- use conventional labels;
- support selection rectangle;
- conventional double-click open;
- conventional right-click file actions;
- never expose arbitrary widget placement.

Grid spacing is a design token, not a user-entered pixel value.

---

# 10. Taskbar UX

The taskbar is the persistent command surface of FlameWM.

Default position: **bottom**.

Default composition, left to right:

**Start → Overview/Activities → Virtual desktops → pinned/running applications → flexible space → media/status → network → volume → battery when present → clock/date**

The exact presence of status icons is hardware/state dependent, but the ordering remains stable.

## 10.1 No edit mode

There is no taskbar edit mode.

There are no draggable widgets.

There are no arbitrary spacers.

There are no user-added plugin applets in the default UX.

The taskbar is a product component.

## 10.2 Moving the taskbar

The taskbar can be moved to:

- Bottom;
- Top;
- Left;
- Right.

The primary interaction is direct:

1. Press and hold on an unused area of the taskbar.
2. Drag toward a screen edge.
3. FlameWM shows a lightweight edge preview.
4. Releasing near the edge docks the taskbar there.
5. The taskbar reflows for the new orientation.

No edit mode is entered.

No permanent overlay appears.

The same position is also available in FlameWM Settings → Taskbar for accessibility and discoverability.

### Source-backed boundary

Current IceWM preferences document `TaskBarAtTop`, which provides the normal top/bottom distinction, but do not expose left/right orientation. Therefore four-edge taskbar placement is a real FlameWM extension, not merely a theme preference. [S1][S3]

## 10.3 Taskbar sizing

User-facing size uses semantic control.

Preferred model:

**Taskbar size:** Compact — Default — Large

A slider may be used if it snaps to sensible presets and displays semantic labels.

Internally FlameWM may map those modes to exact logical dimensions.

The user should not need to type a pixel height.

For a vertical taskbar, the same setting controls width.

## 10.4 Transparency

Taskbar transparency is a bounded personalization option.

UX:

- Off / Solid;
- a simple transparency slider, or several semantic steps;
- readable preview;
- text/icon contrast protected automatically.

Transparency must never make the bar illegible.

True blur is optional and must not be required for the core appearance.

## 10.5 Running applications

Application task buttons should follow conventional desktop behavior:

- clicking a non-active app activates it;
- clicking a minimized app restores it;
- active state is visually obvious;
- multiple windows of one application may group when the grouping UX is ready;
- pinned apps maintain a stable location;
- application icons remain recognizable;
- titles may appear when space allows, depending on the chosen taskbar mode.

IceWM already exposes task buttons, window icons, titles, task grouping controls, task reordering, and system-tray behavior. FlameWM should normalize these into one intentional default. [S1][S3]

## 10.6 Pinned applications

Normal users understand pinned apps.

Minimum behavior:

- right-click running application → Pin to taskbar;
- right-click pinned application → Unpin from taskbar;
- pinned item remains in stable order;
- dragging pinned icons within the application area may reorder them if this can be implemented cleanly without creating an edit mode.

## 10.7 Start button

The Start button uses the FlameWM mark.

Behavior:

- click toggles Start;
- Super opens Start;
- pressing Super again closes Start;
- when Start is open, typing begins application search;
- Escape closes it;
- it remains anchored to the taskbar edge.

The FlameWM logo should be recognizable at small scale and not visually dominate the taskbar.

## 10.8 Overview / Activities button

FlameWM should provide one familiar overview surface.

For user-facing language, **Overview** is preferred over implementing KDE's deeper “Activities” concept, because FlameWM's feature is fundamentally window/workspace overview.

If the existing prototype or product branding retains the label “Activities,” its semantics must still be simple:

> Show my open windows and desktops.

It should not introduce a second parallel concept beside virtual desktops.

Overview displays:

- windows on the current desktop;
- a compact strip/list of virtual desktops;
- ability to activate a window;
- ability to switch desktop;
- ability to close a window through an obvious secondary action.

It is not a full-screen application launcher. Start remains the application launcher.

---

# 11. Start menu UX

The Start menu is a compact application launcher designed around recognition and search.

It should feel familiar to Windows/KDE users without copying their exact layouts.

## 11.1 Structure

Recommended structure:

**Search field** at the top.

**Pinned / Favorites** as the default primary application area.

**All applications** available from a clear control.

**Recent** may exist only if it proves useful and does not create clutter.

**Session controls** at the bottom.

**User identity** may appear subtly at the bottom if the system reliably provides it.

Power actions:

- Lock;
- Log out;
- Restart;
- Shut down.

Suspend appears when supported.

## 11.2 Search

Start search is local-first and immediate.

Primary searchable things:

- installed applications;
- settings pages;
- common FlameWM actions.

Examples:

Typing `wallpaper` should be able to return Desktop settings.

Typing `taskbar` should return Taskbar settings.

Typing `wifi` may return the network control surface.

Do not add web search by default.

Do not add news, ads, online content, or cloud suggestions.

## 11.3 Application categories

“All applications” can organize applications by standard desktop categories, but search and favorites remain the primary paths.

Do not expose raw `.desktop` metadata or package-system terminology.

## 11.4 Keyboard behavior

Start must be fully usable without a mouse.

Expected flow:

- Super → open.
- Type → search.
- Arrow keys → move selection.
- Enter → launch.
- Escape → close.

---

# 12. Window management UX

Window management is where FlameWM should visibly improve on stock lightweight-WM expectations.

The goal is not a tiling window manager.

The goal is **excellent conventional window management**.

## 12.1 Window decorations

Default titlebar behavior:

- application icon/title at the left;
- minimize;
- maximize/restore;
- close at the right.

The close button gets a distinct hover/pressed treatment without being permanently aggressive.

The titlebar is easy to grab.

Resize borders are thin visually but have a forgiving interaction hit area.

## 12.2 Snap behavior

IceWM already provides basic snapping to screen edges/windows through `SnapMove` and `SnapDistance`. FlameWM should build a higher-level Windows-style snap-layout experience on top of or alongside that behavior. [S3]

Required common snap targets:

- left half;
- right half;
- top half when appropriate;
- four quarters;
- one-third / two-thirds combinations;
- three equal columns on sufficiently wide screens.

Do not expose arbitrary tiling trees in the mainstream UX.

## 12.3 Snap layouts

Hovering the maximize button, or invoking the dedicated shortcut, opens a compact layout chooser.

Recommended shortcut:

**Super+Z** → Snap layouts.

A layout chooser visually displays candidate regions.

Hover previews placement.

Click places the active window.

The chooser contains only arrangements appropriate for the available monitor geometry.

Small screens should show fewer choices than ultrawide monitors.

## 12.4 Edge drag

Dragging a window to:

- left edge → left half preview;
- right edge → right half preview;
- top edge → maximize or top-zone layout behavior;
- corners → quarter placement where enabled.

A translucent placement preview appears before release.

After a snap, FlameWM may offer other visible windows to fill the remaining region if this can be implemented without noticeable latency or complexity.

## 12.5 Alt+Tab

Alt+Tab should feel modern and visual.

IceWM already supports QuickSwitch and can use application preview mode. FlameWM should ship a curated horizontal or compact preview presentation rather than exposing the many underlying QuickSwitch tuning variables. [S3]

Expected behavior:

- Alt+Tab shows current-desktop applications by default;
- current selection is strongly visible;
- title is readable;
- icon is present;
- optional live/last preview where technically stable;
- release activates;
- Escape cancels.

Do not make users configure margins, icon sizes, traversal internals, or QuickSwitch mode flags.

## 12.6 Window menu

Right-clicking a titlebar may expose conventional actions:

- Restore;
- Move;
- Resize;
- Minimize;
- Maximize;
- Always on top, if supported;
- Move to desktop;
- Close.

Advanced IceWM-specific window states should remain hidden unless they have clear mainstream value.

---

# 13. Virtual desktops

Virtual desktops are a first-class capability but should remain easy.

IceWM already supports configurable workspaces and taskbar workspace controls. FlameWM turns them into a modern desktop interaction. [S1][S3]

## 13.1 Default

FlameWM starts with a small sensible number of virtual desktops.

A product decision may choose 2 as the initial FlameWM default even though stock IceWM historically creates four. The correct default is the one that minimizes clutter while making the feature discoverable.

The UX must work with one or many.

## 13.2 Minimum invariant

There must always be at least one virtual desktop.

The final desktop cannot be removed.

## 13.3 Taskbar interaction

Workspace controls are compact and immediately understandable.

Right-clicking a desktop:

- Add desktop;
- Remove this desktop.

`Remove this desktop` is disabled when only one remains.

When removing a desktop containing windows, FlameWM moves those windows predictably to a neighboring remaining desktop rather than losing or hiding them.

## 13.4 Adding

A `+` action may appear adjacent to workspace controls when appropriate.

Adding a desktop should be instant.

No naming dialog.

No creation wizard.

## 13.5 Naming

Custom workspace names are not a core FlameWM personalization feature.

Numbered or visually ordered desktops are simpler.

The underlying IceWM capability may remain available to experts.

## 13.6 Overview integration

Overview shows all desktops in one predictable strip.

The current desktop is visually obvious.

Dragging windows between desktops may be added when reliable.

---

# 14. System status and quick controls

A finished desktop cannot depend on users discovering separate tiny legacy utilities for daily laptop controls.

FlameWM's right-side taskbar status area should present system state coherently while remaining lightweight.

Core status surfaces:

- network;
- audio;
- battery/power when present;
- media when active;
- time/date;
- system tray for application indicators.

## 14.1 Network

The network icon represents current state.

Click opens a compact network popover:

- Wi-Fi on/off;
- connected network;
- available networks;
- connection action;
- link to full network settings when needed.

The surface should integrate with the host's normal network service rather than inventing a new networking stack.

## 14.2 Audio

Clicking volume opens:

- output volume;
- mute;
- current output device when multiple devices exist;
- link to fuller audio settings if available.

Scroll over the volume icon may adjust volume if this behavior proves reliable and discoverable through tooltip/help.

## 14.3 Battery

On laptops:

- battery percentage/status;
- charging state;
- estimated state when reliable;
- basic power mode only if the operating system exposes a dependable standard interface.

Do not fabricate inaccurate remaining-time estimates.

## 14.4 Media

When an MPRIS-compatible player is active, FlameWM may expose a compact media section:

- track/application identity;
- previous;
- play/pause;
- next.

Media controls should appear only when meaningful.

No permanent empty media widget.

## 14.5 Clock and date

The taskbar clock shows a simple familiar time.

Date may appear beside/below depending on taskbar size and orientation.

Click opens:

- current date;
- compact month calendar;
- no oversized organizer unless calendar integration becomes a separate product requirement.

---

# 15. FlameWM Settings

FlameWM Settings is the accessibility layer between ordinary users and IceWM/system configuration.

It must look and behave like a small modern settings application, not like a graphical editor for config files.

## 15.1 Navigation model

Top-level sections should be shallow.

Recommended initial sections:

1. Appearance
2. Desktop
3. Taskbar
4. Start
5. Windows
6. Workspaces
7. About

If system integration later expands, Displays, Sound, Network, Input, and Power can be launched or integrated deliberately, but the FlameWM product settings should not become an uncontrolled replacement for the distribution's entire control center unless that becomes an explicit future scope.

KDE and GNOME guidelines both favor shallow, comprehensible navigation over deeply nested configuration trees. [S5][S9]

## 15.2 Appearance

Controls:

**Mode**  
Light / Dark / System when supported.

**Accent color**  
A curated palette plus optional custom color if implementing it does not compromise contrast.

**Icons**  
A small curated set or a single recommended icon style plus compatible alternatives. Do not expose raw icon search paths.

**Reset appearance**  
Restores FlameWM appearance defaults for this page.

## 15.3 Desktop

Controls:

**Wallpaper**  
Preview, built-ins, Choose image, recent choices.

**Image fit**  
Fill / Fit / Stretch / Center / Tile if retained.

**Show FlameWM mark**  
Default on.

**Desktop icons**  
Only if the feature exists; simple On/Off or similarly bounded behavior.

No wallpaper-daemon command lines.

No raw background paths displayed as the primary UI.

## 15.4 Taskbar

Controls:

**Position**  
Bottom / Top / Left / Right.

**Size**  
Compact / Default / Large.

**Transparency**  
Simple bounded control.

**Auto-hide**  
Off by default; simple switch if supported to FlameWM quality.

**Show window titles**  
Only if FlameWM supports distinct icon-only versus labeled taskbar modes cleanly.

**Show desktops**  
Potential toggle, but default on if virtual desktops are a core product feature.

Avoid exposing:

- graph sample counts;
- mailbox applet internals;
- address bar;
- raw tray dimensions;
- task grouping integer modes;
- toolbar files;
- taskbar width percentages;
- font Xft strings;
- raw IceWM preference keys.

## 15.5 Start

Keep this page intentionally small.

Possible controls:

- favorites/pinned applications management;
- show recent apps on/off if the feature exists;
- reset favorites.

Do not let users redesign the Start menu layout.

## 15.6 Windows

Controls:

**Snap windows**  
On by default.

**Show snap layouts**  
On by default.

**Animations**  
On / Reduced / Off, if animations are implemented.

**Alt+Tab previews**  
Simple on/off only if needed.

Avoid exposing focus-policy laboratories.

Click-to-focus should be the mainstream default.

Special IceWM focus modes remain expert configuration.

## 15.7 Workspaces

Controls:

- visual count;
- Add desktop;
- Remove desktop;
- optional keyboard-shortcut summary.

No final desktop removal.

No complex workspace topology.

## 15.8 About

Show:

- FlameWM logo;
- FlameWM version;
- “A lightweight desktop by ArkFlame Studios”;
- IceWM base/version;
- license information;
- project homepage;
- diagnostic/version-copy action.

This is where technical lineage belongs.

## 15.9 Settings mechanics

Personalization changes apply live when safe.

Every page has a clear local reset if needed.

The application should not display Apply/Cancel for harmless live personalization.

For changes that can temporarily make the system difficult to use, provide automatic recovery or a reversible preview.

The settings app should never overwrite unrelated expert IceWM configuration when changing one FlameWM-owned preference.

---

# 16. Configuration ownership model

This is a critical UX/architecture contract.

FlameWM needs a clear distinction between **FlameWM-owned user settings** and **expert IceWM configuration**.

## 16.1 FlameWM-owned preferences

The FlameWM Settings application owns the curated keys it exposes.

Examples:

- theme mode;
- accent;
- taskbar edge;
- taskbar semantic size;
- transparency;
- wallpaper;
- wallpaper fit;
- watermark visibility;
- snapping enablement;
- workspace count;
- selected FlameWM UX toggles.

## 16.2 Expert compatibility

Advanced users may still edit IceWM configuration files.

FlameWM should avoid gratuitously deleting or reformatting unrelated values.

Where a FlameWM setting must override a lower-level IceWM setting, that ownership should be explicit and documented.

IceWM already separates preferences, themes, and `prefoverride` configuration concepts. FlameWM can use or extend that layering rather than rewriting arbitrary user files. [S1][S2]

## 16.3 No duplicate authority

For every user-facing option, one layer must be authoritative.

If FlameWM owns Taskbar Position, there should not simultaneously be three competing GUI surfaces that write different files.

If an expert manually changes an underlying FlameWM-owned key, the next settings read should either reflect the effective value or deliberately document the override behavior.

Silent configuration fights are forbidden.

---

# 17. Motion and feedback

Animations exist to explain state change, not to prove that animations exist.

Target character:

- fast;
- short;
- interruptible;
- subtle;
- low-overhead.

Examples:

- Start menu appears with a short fade/translate;
- snap target preview fades in quickly;
- settings page transitions are restrained;
- popovers anchor clearly to their taskbar item;
- workspace switch feedback is immediate.

Reduced-motion mode should eliminate unnecessary transforms while keeping state changes understandable.

No elastic bouncing.

No long easing.

No desktop animation should delay input readiness.

---

# 18. Interaction targets and accessibility

A lightweight desktop is not allowed to become difficult to click merely to save pixels.

Core controls need forgiving hit areas.

Important requirements:

- visible keyboard focus;
- full keyboard navigation for Start and Settings;
- meaningful tooltips for icon-only controls;
- no essential information conveyed by color alone;
- sufficient contrast in both light and dark modes;
- semantic labels for accessibility APIs where the toolkit permits them;
- no essential operation available only through a hidden right-click path.

KDE's accessibility guidance explicitly requires testing keyboard-only and pointing-device-only operation and warns against placing essential information only in hover tooltips. FlameWM adopts that baseline. [S10]

---

# 19. Language and terminology

FlameWM speaks in user concepts, not window-manager concepts.

Preferred:

- Desktop
- Taskbar
- Start
- Apps
- Windows
- Workspaces / Desktops
- Wallpaper
- Accent color
- Taskbar size
- Taskbar position
- Snap layouts
- Appearance
- System tray
- Show desktop

Avoid in normal UX:

- X11 root window
- EWMH
- Xft
- pager internals
- `TaskBarAtTop`
- `prefoverride`
- workspace preference syntax
- theme pixmap
- strut
- client window
- WM_CLASS
- Xinerama

Technical terminology belongs in developer documentation and diagnostics.

GNOME's writing guidance similarly recommends using concepts familiar to the audience rather than terms from the underlying system. [S11]

---

# 20. Default shortcut philosophy

Keyboard shortcuts are accelerators, not prerequisites.

Core defaults should follow common conventions:

| Action | Default |
|---|---|
| Start | Super |
| Switch windows | Alt+Tab |
| Close window | Alt+F4 |
| Snap left | Super+Left |
| Snap right | Super+Right |
| Maximize | Super+Up |
| Restore/minimize depending on state | Super+Down |
| Snap layouts | Super+Z |
| Show desktop | Super+D |
| Overview | Super+Tab or another conflict-free conventional binding |
| Switch workspace | Ctrl+Alt+Left / Right where compatible |
| Move window to workspace | Shift plus workspace-switch chord where compatible |

Exact bindings must be audited against IceWM's existing defaults and the target distribution before implementation.

The UX rule is more important than the literal initial binding:

> common actions receive conventional shortcuts; obscure actions do not consume valuable global keys.

---

# 21. What FlameWM deliberately does not become

FlameWM is **not**:

- a Plasma clone;
- an XFCE clone;
- a theme manager;
- a widget framework;
- a tiling-WM configuration laboratory;
- an IceWM preference editor;
- a desktop scripting platform exposed to ordinary users;
- a compositor-effects showcase;
- a collection of status monitors;
- a distro-specific control-center replacement by accident;
- a panel construction kit;
- a “rice” generator.

Those may be valid products. They are not FlameWM's purpose.

---

# 22. Explicit anti-features for the default UX

These should not be added without revisiting this constitution:

- arbitrary panel widgets;
- panel edit mode;
- arbitrary panel item ordering through a layout editor;
- multiple independent taskbars by default;
- freely draggable desktop widgets;
- user-entered panel pixel dimensions;
- user-entered window-decoration pixel dimensions;
- exposed IceWM preference files inside Settings;
- dozens of focus modes;
- dozens of Alt+Tab tunables;
- CPU/RAM/network graphs in the default taskbar;
- legacy mailbox applet in the default taskbar;
- shell command/address bar in the default taskbar;
- theme-marketplace UI;
- animated wallpaper by default;
- online Start-menu content;
- advertisements;
- forced cloud account;
- telemetry-dependent personalization;
- background services whose only job is cosmetic when the same result can be rendered directly.

---

# 23. Source-backed reuse map

The following capabilities already exist in IceWM and should be reused before reimplementation:

| FlameWM need | IceWM foundation |
|---|---|
| Window management | Core IceWM |
| Stacking windows | Core IceWM |
| Workspaces | Configurable workspaces |
| Taskbar | Built-in taskbar |
| Start menu foundation | Built-in start/program menu machinery |
| Running application buttons | Built-in task buttons |
| System tray | Built-in tray support |
| Clock | Built-in clock applet |
| Window switching | QuickSwitch / Alt+Tab |
| Window previews | QuickSwitch preview support |
| Basic window snapping | `SnapMove`, `SnapDistance` |
| Themes | IceWM theme system |
| Wallpaper | `icewmbg` |
| Per-workspace wallpaper capability | `icewmbg` |
| Background scaling/centering | `icewmbg` |
| Keyboard shortcuts | IceWM keys configuration |
| Show desktop | Existing taskbar/window-manager behavior |

Sources: [S1][S2][S3]

---

# 24. FlameWM-specific extension map

The following are not merely “make a theme” tasks; they define FlameWM's added product value.

## 24.1 Curated shell defaults

Ship one coherent desktop composition instead of exposing the raw IceWM default surface.

## 24.2 FlameWM visual system

Create:

- FlameWM theme;
- light/dark variants;
- titlebar assets;
- taskbar assets;
- state styling;
- FlameWM icons/branding;
- default wallpaper family.

## 24.3 Four-edge taskbar

Implement actual left/right taskbar orientations in addition to IceWM's documented top/bottom behavior.

All contained controls must reflow correctly.

## 24.4 Drag-to-edge taskbar positioning

Direct manipulation without edit mode.

## 24.5 FlameWM Settings

A purpose-built configuration frontend that presents product concepts and writes only owned settings.

## 24.6 Modern snap layouts

Provide modern layout previews and region selection beyond simple magnetic edge snapping.

## 24.7 Overview

Provide a coherent window/workspace overview rather than exposing multiple legacy window-list surfaces.

## 24.8 Desktop brand layer

Provide the subtle FlameWM watermark independent of user wallpaper when feasible.

## 24.9 Modern Start experience

Provide application search, favorites, system actions, and settings search in a compact familiar launcher.

## 24.10 System quick controls

Provide coherent lightweight access to network/audio/battery/media state through existing system services.

---

# 25. Performance is part of UX

A desktop that looks clean but stalls is not user friendly.

Performance requirements therefore belong in the product soul.

FlameWM should optimize for:

- rapid session startup;
- immediate Start opening;
- immediate task switching;
- no perceptible delay when opening context menus;
- no background CPU churn while idle;
- modest resident memory;
- bounded image caching;
- no continuously redrawing transparent effects when nothing changes;
- event-driven status updates;
- no unnecessary indexer;
- no online Start search;
- no animation blocking input;
- graceful degradation on low-end GPUs.

A feature that materially harms idle efficiency needs stronger justification than a feature in a heavyweight desktop environment.

---

# 26. Multi-monitor behavior

FlameWM must remain predictable with multiple displays.

Principles:

- taskbar lives on the primary display by default;
- changing primary display moves the primary taskbar unless the user explicitly chose another behavior;
- snap layouts operate within the active monitor;
- desktop watermark appears once per relevant desktop surface or according to the final multi-monitor brand rule, never awkwardly split;
- wallpaper fit is evaluated per monitor by default unless the user chooses span;
- popovers open on the monitor containing their taskbar;
- window placement must never make dialogs inaccessible.

IceWM already documents multi-head support for desktop backgrounds and primary-screen/taskbar-related behavior. FlameWM should preserve that capability while making the choices understandable. [S2][S12]

---

# 27. Failure behavior

A polished desktop handles incomplete integrations without looking broken.

Examples:

If Wi-Fi management is unavailable:
- do not show a dead Wi-Fi toggle;
- show a meaningful state or omit the control.

If no battery exists:
- omit battery UI.

If no media session exists:
- omit media controls.

If wallpaper loading fails:
- retain the previous valid wallpaper;
- show a concise non-blocking error.

If a theme asset is missing:
- fall back to a safe built-in visual;
- do not render unreadable controls.

If a saved taskbar position cannot be restored:
- fall back to Bottom.

If workspace configuration is invalid:
- recover to at least one workspace.

The desktop must always have a usable fallback.

---

# 28. The FlameWM decision filter

Every proposed user-facing feature should pass these questions.

| Question | Required answer |
|---|---|
| Does a normal desktop user understand what this is? | Yes |
| Does it solve an ordinary recurring task? | Usually yes |
| Can it use a familiar interaction? | Yes |
| Is the default behavior already good without configuring it? | Yes |
| Can it be represented without exposing IceWM internals? | Yes |
| Does it preserve low idle overhead? | Yes |
| Does it avoid adding another configuration authority? | Yes |
| Can the user recover from a bad choice? | Yes |
| Does it still feel like FlameWM after customization? | Yes |
| Is it necessary in the normal GUI rather than expert config? | Yes, or reject/hide it |

If a feature fails multiple rows, it does not belong in the default FlameWM UX.

---

# 29. Product acceptance scenario

A first-time user installs a Linux system with FlameWM.

They log in.

They see:

- a clean wallpaper;
- a small subtle FlameWM mark at the bottom-right;
- a familiar bottom taskbar;
- the Flame button at the left;
- virtual desktop/overview controls;
- application buttons;
- Wi-Fi, audio, battery, and time on the right.

They press Super.

Start opens instantly.

They type `firefox`, press Enter, and Firefox launches.

They drag Firefox to the left edge.

A snap preview appears and Firefox takes the left half.

They open another application and place it on the right.

They hover the maximize control and see a few clear layout choices instead of a complex tiling editor.

They right-click the desktop.

They see only the desktop actions they expect.

They choose Change background.

A simple settings page opens directly at wallpaper selection.

They select a wallpaper and see it immediately.

They change the accent.

The desktop updates immediately.

They drag the taskbar to the top.

It snaps into place without entering an edit mode.

They right-click the second workspace and remove it.

When only one desktop remains, Remove is disabled.

They never see:

- an IceWM preference key;
- an X11 term;
- a panel construction interface;
- an arbitrary pixel field;
- a requirement to edit a text file;
- an “unlock widgets” state;
- a warning that the desktop needs to restart for a color change.

That session is FlameWM.

---

# 30. Final-result definition

When FlameWM reaches its intended first complete form, it should be describable like this:

> **FlameWM is a lightweight desktop shell and IceWM fork from ArkFlame Studios that boots directly into a polished Windows/KDE-familiar workflow. It provides a fixed-but-personalizable taskbar, Flame-branded Start experience, modern window snapping and layouts, clear virtual desktops, coherent system controls, a minimal desktop, and a deliberately small settings application. Underneath, IceWM continues doing the efficient window-management work. Above it, FlameWM removes the configuration-tool feeling and replaces it with a finished product.**

The final result is not “IceWM with options.”

It is:

> **the desktop users expected IceWM-class performance to have.**

---

# 31. Non-negotiable first-release UX contract

The first release should not be considered FlameWM-complete unless all of the following are true:

- coherent FlameWM light theme;
- coherent FlameWM dark theme or an explicit documented milestone decision if dark is deferred;
- FlameWM logo and restrained brand system;
- branded default wallpaper;
- subtle desktop FlameWM watermark/mark;
- bottom taskbar default;
- taskbar Start button;
- taskbar running applications;
- taskbar workspace control;
- taskbar system tray;
- audio/network/time integrations appropriate to the host;
- taskbar movable to all four edges;
- taskbar movement without edit mode;
- conventional titlebar controls;
- modern snap behavior;
- snap layout chooser;
- Alt+Tab that looks intentional;
- functional virtual desktop add/remove with minimum one;
- Start menu with favorites, app search, all apps, and power/session actions;
- desktop context menu reduced to relevant desktop actions;
- FlameWM Settings with curated tabs;
- live wallpaper/accent/taskbar personalization;
- no mainstream UI exposing raw IceWM tuning;
- safe fallbacks when integrations are missing;
- low idle overhead verified against the project's performance budget.

---

# 32. Future-agent instructions

Future design and implementation work must treat this document as a product contract.

When requirements conflict:

1. explicit current user instruction wins;
2. source/API reality wins over assumptions;
3. this UX constitution defines default product behavior;
4. existing IceWM behavior is reused when it satisfies the constitution;
5. IceWM behavior is wrapped, hidden, replaced, or extended when it does not;
6. adding a user-facing preference is a last resort, not the default resolution to a design disagreement.

When implementing a feature, always answer:

- What user problem does it solve?
- What does the user see?
- What action do they take?
- What happens immediately?
- What is the safe fallback?
- Does this expose an engine detail?
- Does this require a permanent process?
- Can the same result be achieved more cheaply?
- Does it preserve FlameWM's finished default?
- Can a Windows/KDE-familiar user discover it without documentation?

---

# 33. Canonical slogans / internal product language

These are internal identity statements, not necessarily marketing copy.

> **IceWM speed. Finished-desktop UX.**

> **Lightweight without feeling unfinished.**

> **A desktop, not a construction kit.**

> **Familiar by default. Fast by construction.**

> **Keep the engine. Remove the friction.**

The strongest internal statement is:

> **FlameWM must make simplicity feel intentional, not limited.**

---

# 34. External grounding and sources

The source references below ground the parts of this document that depend on current IceWM capabilities and established desktop UX guidance.

**[S1] IceWM Preferences manual — official IceWM documentation**  
https://ice-wm.org/man/icewm-preferences.html  
Relevant: taskbar controls, workspaces, tray, start menu, window-task controls, snapping, QuickSwitch settings, theme colors.

**[S2] icewmbg manual — official IceWM documentation**  
https://ice-wm.org/man/icewmbg.html  
Relevant: wallpapers, scaling, centering, colors, per-workspace images, cycling, multi-monitor behavior.

**[S3] IceWM manual / icewm(1) — official IceWM documentation**  
https://ice-wm.org/man/icewm.html  
Relevant: IceWM role, taskbar, workspace model, tray, application switching, QuickSwitch previews, drag/drop, menu/config foundation.

**[S4] KDE HIG — Simple by default**  
https://develop.kde.org/hig/simple_by_default/  
Relevant: reduce visible complexity, show important elements, group related settings.

**[S5] GNOME HIG — Design Principles**  
https://developer.gnome.org/hig/principles.html  
Relevant: make interfaces simple, progressive disclosure, reduce user effort, minimize steps.

**[S6] KDE HIG — Powerful when needed**  
https://develop.kde.org/hig/powerful_when_needed/  
Relevant: do not use customizability to avoid design decisions; apply settings immediately.

**[S7] KDE Breeze repository**  
https://github.com/KDE/breeze  
Relevant: Breeze visual-system components and role as Plasma's default style.

**[S8] KDE Breeze Icons repository / KDE licensing guidance**  
https://github.com/KDE/breeze-icons  
https://community.kde.org/Policies/Licensing_Policy/Draft  
Relevant: Breeze icon availability and license considerations.

**[S9] KDE HIG — Layout and navigation**  
https://develop.kde.org/hig/layout_and_nav/  
Relevant: shallow, contextual navigation and conventional settings layout.

**[S10] KDE HIG — Accessibility and inclusiveness**  
https://develop.kde.org/hig/accessibility/  
Relevant: keyboard-only and pointer-only operability, visible focus, accessible controls.

**[S11] GNOME HIG — Writing Style**  
https://developer.gnome.org/hig/guidelines/writing-style.html  
Relevant: concise interface text and user-familiar terminology.

**[S12] IceWM older preference reference / official manual**  
https://ice-wm.org/manual/icewm-10.html  
Relevant: taskbar justification/width concepts and primary-screen behavior.

---

# 35. Canonical summary for future context

If a future conversation only has room for one paragraph, preserve this:

**FlameWM is an ArkFlame Studios fork of IceWM whose purpose is to turn IceWM's extremely lightweight window-management core into a finished, familiar desktop for normal users. Its default UX follows the desktop grammar users know from Windows and KDE/Breeze: a polished fixed-composition taskbar, Flame Start menu, overview/workspaces, system status controls, conventional window buttons, Alt+Tab, modern snap layouts, a quiet wallpaper desktop with subtle FlameWM branding, and a small settings application. Personalization is intentionally bounded to meaningful choices such as appearance, accent, wallpaper/fit, taskbar edge/size/transparency, icons, snapping, and workspace count. There is no panel edit mode, widget construction system, raw pixel tuning, or GUI exposure of IceWM internals. IceWM remains the engine; FlameWM owns the experience. The default must be good enough that most users never need to configure it, and every new feature must preserve low idle overhead, familiar behavior, recoverability, and the principle that simplicity should feel intentional rather than limited.**
