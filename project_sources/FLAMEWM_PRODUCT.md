# Product Contract — V4

## Definition

**FlameWM is IceWM transformed into a complete, familiar, opinionated desktop while preserving IceWM-class efficiency.** IceWM is the engine; FlameWM is the user experience.

Character: **familiar, calm, fast, finished, focused, lightweight**.

## Authority

Newest explicit product instruction > verified current source/API > approved V4 prototype/contract > canonical packets > archive > historical defaults.

Do not reconcile contradictions by adding an option. Adopt the higher authority and update the active packet.

## Product laws

- Finished by default; no assembly/config-file knowledge required.
- Familiar conventions beat novelty.
- Expose semantic choices, never raw IceWM implementation values in mainstream UX.
- No panel/widget edit mode or arbitrary applet construction system.
- Safe cosmetic changes apply live.
- Expert IceWM configuration may remain as an escape hatch without becoming product UI.
- One state authority per concern; Flame wraps IceWM state rather than duplicating it.
- Context menus belong to the thing clicked.
- Subtle branding, not advertising.
- No core compositor or heavyweight shell dependency.
- Idle means idle: event subscriptions, lazy surfaces, no periodic command polling.

## V4 required product

- scale-aware modern window chrome with whole-titlebar-centered application title;
- drag edge/corner half/quarter/top-maximize snap + preview + restore;
- virtual desktops with compact two-row topology and directional shortcuts;
- one four-edge directly dockable taskbar;
- compact searchable Start application tree with session/power actions;
- unified pinned/running application area;
- XEmbed tray, battery foundation, network/audio/media, centered clock/date + calendar;
- isolated filesystem desktop with grid, selection, group drag, Trash and watermark;
- on-demand Settings: Appearance, Desktop, Displays, Fonts, Hotkeys, About;
- wallpaper selection while default desktop remains pitch-black;
- intentional QuickSwitch/Alt+Tab with previews;
- measured low-memory release gate.

## Explicit non-features for V4

```text
NO Overview / Activities system
NO maximize-hover snap chooser
NO Plasma widgets / applet marketplace
NO panel edit mode
NO compositor or blur requirement
NO Wayland compositor
NO KRunner clone / online Start search
NO custom NetworkManager/audio/media daemon
NO permanent Settings daemon
NO hard-coded production app catalog
NO WM attempt to scale arbitrary client application interiors
```

## Defaults

```text
Shell/background: pitch-black until user chooses wallpaper
Accent: Flame red
Font: IBM Plex Sans
Taskbar edge: Bottom
Taskbar edit mode: nonexistent
Start/tasks/workspaces/tray/clock+date: On
QuickSwitch: On; previews On in Flame defaults
Snap: On
Passive pointer edge workspace switching: Off
Dragged-window edge traversal: On
Flame watermark: On, subtle
CPU/MEM/legacy throughput/mailbox/address bar: Off
```

Taskbar order:

```text
Start -> pinned/running apps -> flexible unused docking surface -> two-row desktops
      -> media -> audio -> network -> tray when needed -> centered clock/date
```

## Settings surface

Only current V4 pages:

- **Appearance** — Flame red presets/custom accent and product appearance controls.
- **Desktop** — wallpaper/fit, watermark, desktop behavior.
- **Displays** — output modes/resolution and per-output Flame shell scale with safe revert.
- **Fonts** — family, global bold, size offset; IBM Plex Sans default.
- **Hotkeys** — curated Flame actions, capture/conflict handling/reset.
- **About** — visible content stays intentionally minimal: FlameWM branding + `A lightweight desktop by ArkFlame Studios`.

Technical lineage, licenses and diagnostics may exist in package/docs/diagnostic output; do not bloat the visible V4 About page.

## Current desktop context menu

```text
Open Terminal
Create New Folder
Add Virtual Desktop
Desktop and Wallpaper
```

Do not replace this with older archived minimal-menu proposals unless a newer product decision explicitly changes it.
