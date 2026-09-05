# Settings, Displays, Fonts and Hotkeys — V4

## Process and authority

`flamewm-settings` is a separate on-demand executable: zero resident cost when closed and failure-isolated from WM. It writes only a Flame-owned typed config; it must not rewrite the user's general IceWM preferences file.

Suggested precedence:

```text
IceWM defaults -> shipped Flame defaults/theme -> Flame user settings -> expert prefoverride
```

Persist atomically (temporary file -> close/flush -> rename). Unknown keys survive compatible reads where practical.

Live-safe change path:

```text
validate -> atomically persist -> narrow Flame reload/control command
-> WM reads new snapshot -> computes delta -> applies only owned values
```

No arbitrary shell execution in the control protocol.

## Current pages only

```text
Appearance
Desktop
Displays
Fonts
Hotkeys
About
```

Do not resurrect old Taskbar/Windows/Workspaces/Overview pages merely because archived reports contain them; their relevant V4 controls are integrated according to the approved prototype.

## Appearance

One semantic palette authority; never scatter literal accent values:

```text
accent / accentHover / accentPressed / accentMuted
surface / surfaceRaised
textPrimary / textSecondary
border / danger
```

Default accent: Flame red. Presets/custom color update the same authority. Invalid custom values never partially apply.

## Desktop

Wallpaper/fit and watermark controls route to existing wallpaper/desktop authorities. Default remains pitch-black until user selects an image. Do not destructively bake watermark into arbitrary user wallpaper.

## Displays

Build on XRandR output/mode observation; do not shell out to `xrandr` as the persistent product architecture.

Requirements:

- enumerate durable outputs and valid modes;
- per-output resolution/mode setting;
- per-output Flame shell scale;
- safe apply/revert for resolution changes that could make UI unusable; the WM/display controller owns the pending transaction/revert timer so a Settings crash cannot strand the mode;
- persist scale by durable identity (prefer unique EDID hash with connector tie-breaker, connector fallback), not transient monitor index;
- output hotplug recomputes scale/metrics/workarea/desktop grid coherently.

V4 shell scaling applies Flame-owned chrome/panel/menus/desktop/Settings surfaces. Do not claim the WM can universally rescale arbitrary third-party X11 application interiors.

## Fonts

IceWM already has many role-specific fonts. V4 exposes a simple abstraction:

```text
family (default IBM Plex Sans)
global bold
size offset
```

Define semantic Flame roles with relative base sizes, map them to IceWM/Flame text owners, then apply user family/weight/offset and current output scale. Global bold affects Flame-owned shell/chrome text, not arbitrary third-party apps.

Do not bundle font binaries unless separately required/licensed; a missing preferred family must degrade to a sane sans fallback without missing text.

## Hotkeys

Expose only curated Flame actions. Current V4 core set:

```text
Open Start
Workspace Left
Workspace Right
Workspace Up
Workspace Down
```

plus any currently approved window shortcuts represented by the ingested V4 contract. Capture chord -> validate -> detect conflict -> stage X grabs -> commit only if the full new grab set succeeds; otherwise restore old grabs/state. Persist only the effective accepted binding. Provide Restore Defaults. Modifier-only chords are rejected unless the action intentionally uses one (for example Super for Start).

Do not expose the entire IceWM key-binding laboratory as mainstream UX.

## About

Visible V4 content is intentionally minimal:

```text
FlameWM branding/logo
A lightweight desktop by ArkFlame Studios
```

Keep IceWM lineage, exact versions, licenses and diagnostics in package/docs/diagnostic output unless a newer product decision expands visible About.

## Verification

- Accent preset/custom valid/invalid and live apply.
- Font family/bold/offset live apply without destroying role hierarchy.
- Resolution change has recoverable revert path.
- Per-output scale survives by durable identity and hotplug.
- Hotkey conflicts handled deterministically and reset works.
- Settings crash cannot kill WM; closed Settings consumes no process memory.
- Visible About contains only approved V4 content.
