# Taskbar, Tasks and Start — V4

## Taskbar composition

Keep IceWM taskbar/window as host; modernize composition/layout. Current order:

```text
Start
Pinned + running applications
Flexible unused docking surface
Two-row virtual desktops
Media
Audio
Wi-Fi/network
System tray when needed
Centered clock/date
```

No Activities/Overview button. Disable legacy CPU/MEM/net-throughput/mailbox/address-bar clutter by Flame defaults, not by deleting mature upstream features.

## Four-edge taskbar

Current IceWM is fundamentally top/bottom through boolean `TaskBarAtTop`; V4 requires:

```cpp
enum class PanelEdge { Bottom, Top, Left, Right };
```

Generalize geometry/struts/work-area reservation, layout axis, task/workspace/status flow, clock treatment, popover anchoring and output constraints. Do not encode left/right as magic values in the old boolean.

Direct docking starts **only on unused taskbar surface**; child controls consume normal input. Press/drag blank region -> nearest legal edge preview -> release -> persist edge -> update strut/workarea -> deterministic relayout. No edit mode.

Use semantic spacing (`statusIconGap`, `statusHitTarget`, `clockInset`, `trayGap`) rather than per-applet magic numbers.

The taskbar lives on the primary output by default. If the XRandR primary output changes, move the product taskbar to the new primary unless a later explicit placement setting owns a different target. Persist edge separately from transient output index.

## Pinned + running apps

IceWM task/frame state is authority. Stable application identity:

1. desktop application id;
2. `.desktop` `StartupWMClass`;
3. X11 `WM_CLASS`;
4. normalized fallback.

Never group by current window title.

States:

```text
pinned not running -> icon, no activity marker
running background -> subtle running marker
focused -> strong Flame accent marker
minimized -> still running, de-emphasized icon
```

V4 context policy is exact:

```text
Pinned item -> Unpin only
Running non-pinned item -> Activate / Close
```

Do not expose Close on a pinned task merely because it has a live window.

Menus/popovers use one edge-aware anchor helper. Bottom edge: menu left aligns clicked icon left and menu stays above taskbar.

## Start launcher

Reuse IceWM/Freedesktop desktop-entry discovery, icons, launch semantics, keyboard/menu primitives and existing session actions. Do not hardcode production applications.

V4 is a compact cascading app/category tree with a visible search field/icon and deliberate Power/Session subtree; remove legacy IceWM management clutter from default product surface.

Search requirements:

- local installed apps only;
- reusable/cached XDG app model;
- in-memory filtering per keystroke;
- arrows + Enter + Escape;
- search text never becomes shell command text;
- preserve `.desktop` Exec/argument-vector semantics.

Prototype-only visual test apps are not a runtime catalog.

Power/Session reuses IceWM lock/logout/reboot/shutdown/suspend action backends and configured commands. V4 work is presentation/icon reliability, not power-management reimplementation.

## Clock/tray/battery

- Keep XEmbed tray.
- Keep battery foundation; V4 does not expand its system scope beyond modern visual treatment.
- Keep IceWM `strftime` time engine; add Flame layout only if needed for exact centered visible time+date.
- Calendar popup contract is in `INTEGRATIONS.md`.

## Verification

- Bottom/Top/Left/Right reserve correct work area.
- Docking drag starts only from blank taskbar space.
- Popover/context placement correct on every edge.
- Pinned menu and non-pinned running menu exactly match V4 policy.
- Running/focused/minimized visuals reflect real IceWM state.
- Installed XDG apps discovered; category/search keyboard path works; search icon and session icons visible.
- No hard-coded test-app dependency or legacy IceWM configuration clutter in default Start.
- Clock/date centered; no critical missing icon.
