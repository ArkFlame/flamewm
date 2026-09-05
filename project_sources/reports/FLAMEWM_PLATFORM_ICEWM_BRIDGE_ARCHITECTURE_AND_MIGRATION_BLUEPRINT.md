# FlameWM Platform + IceWM Bridge — Production Architecture and Upstream-Restoration Blueprint

**Project:** FlameWM  
**Studio:** ArkFlame Studios  
**Base engine audited:** IceWM 4.1.0 / X11  
**Current FlameWM source snapshot audited:** `130_3_9_26.tar(1).gz`  
**Architecture status:** proposed canonical target for the next architecture migration  
**Document role:** production architecture specification + current-state migration plan + upstream-maintainability contract  
**Audit date:** 2026-09-03

---

# EXECUTION

## Goal

Replace the current pattern of feature-specific FlameWM code being integrated directly into many IceWM source owners with one small, explicit, testable compatibility membrane:

```text
IceWM 4.x engine
    ↕ minimal hooks / narrow access
FlameWM IceWM Bridge
    ↕ engine-neutral ports
FlameWM Platform
    ↕ high-level services + immutable snapshots + typed commands
Flame Shell / Settings / Desktop / CLI
```

The target is not another generic framework. The target is a **product-specific platform layer** that makes ordinary FlameWM feature work simple while making upstream IceWM updates cheap.

The architecture must achieve all of these simultaneously:

1. IceWM remains authoritative for X11 window state, focus, stacking, geometry, EWMH/ICCCM, workspace engine state and existing protocol behavior.
2. FlameWM product policy never lives inline inside IceWM implementation files.
3. Existing IceWM lines changed only to support FlameWM should be restored to exact upstream behavior whenever the new adapter can replace them.
4. Permanent IceWM edits become a tiny audited hook set, preferably one-line calls or friend declarations.
5. High-level FlameWM code never needs to know `YWindowManager`, `YFrameWindow`, `TaskButton`, `WMKey`, `RROutput`, `RRCrtc`, X atoms, or IceWM globals.
6. Cross-process clients such as `flamewm-settings` use a versioned control API instead of trying to access the WM process singleton.
7. High-frequency paths such as move/resize/snap remain in-process and event-driven; they never use D-Bus.
8. Existing pure Flame models are reused rather than rewritten.
9. Future IceWM updates should normally require changes only inside `src/flamewm/engine/icewm/**` plus a small hook patch set.
10. Architecture checks must mechanically prevent future agents from widening IceWM coupling again.

---

# 1. Executive decision

## 1.1 Canonical name

Use:

> **FlameWM Platform**

and name the engine-specific compatibility layer:

> **IceWM Bridge**

Do **not** make `flamewm-icewm-framework` the primary architecture or package name.

Why:

- `framework` encourages feature code to depend on implementation machinery;
- `flamewm-icewm-framework` makes IceWM part of the upper product vocabulary;
- the desired dependency direction is the opposite: IceWM is one engine implementation below stable FlameWM contracts;
- `Platform` accurately describes reusable product-level capabilities;
- `IceWM Bridge` accurately describes the narrow compatibility membrane.

Canonical terminology:

```text
FlameWM Platform          product services and policy
IceWM Bridge              adapter from Platform ports to IceWM 4.x
Flame Control             versioned cross-process control API
Flame Shell               panel, Start, popovers and shell presentation
Flame UI                  small stable UI facade over the native IceWM toolkit
```

Canonical namespaces:

```cpp
flamewm::api
flamewm::platform
flamewm::engine::icewm
flamewm::control
flamewm::shell
flamewm::ui
```

Canonical internal build targets:

```text
flamewm_api
flamewm_platform
flamewm_engine_icewm
flamewm_control_common
flamewm_control_server
flamewm_control_client
flamewm_ui
flamewm_ui_icewm
flamewm_shell
```

These are internal static libraries for v1. Do not promise a public C++ ABI. The stable cross-process boundary is the versioned D-Bus protocol.

---

# 2. Current source audit — why migration is justified now

The current archive was compared byte-for-byte against the supplied pristine IceWM 4.1.0 tree.

## 2.1 Current upstream divergence

Exactly **30 original IceWM repository files differ** from pristine 4.1.0.

Across those files the current tree contains approximately:

```text
132 changed hunks
+1,789 added lines
-108 removed lines
```

Largest current IceWM-source deltas:

```text
src/wmmgr.cc        +352 / -3
src/wmframe.cc      +298 / -3
src/icesm.cc        +178 / -2
src/atasks.cc       +138 / -19
src/movesize.cc     +106 / -5
src/wmkey.cc         +87 / -0
src/decorate.cc      +52 / -9
src/wmtaskbar.cc     +41 / -43
src/wmtitle.cc       +39 / -6
```

Current Flame-owned implementation is already substantial:

```text
src/flamewm/**           83 C++ headers/sources, ~9,590 lines
src/flamewm-settings/**  14 C++ headers/sources, ~680 lines
src/flamewm-desktop/**   24 C++ headers/sources, ~1,892 lines
```

This means FlameWM already has enough product-owned code to justify a proper boundary. Continuing to add direct native bridges will increase integration cost faster than feature value.

## 2.2 Current architecture symptom

Current source contains good product models such as:

```text
ConfigStore / SettingsSnapshot
DisplayManager
ShortcutRegistry
ScaleManager
WorkspaceTransaction
TwoRowTopology
SnapController / SnapState
PanelManager / OutputPanel
StartController
FlameTaskStrip
NetworkManager / MPRIS / Pulse adapters
```

But these models still become real only after separate hand-written integrations into:

```text
wmapp
wmmgr
wmframe
movesize
wmtaskbar
atasks
wmkey
bindkey
wmconfig
yconfig
yprefs
icesm
```

That is why a new feature often requires another full IceWM source audit.

## 2.3 Current cross-process blocker proves the missing seam

`flamewm-settings` is a separate process, but the current effective Runtime is a WM-process singleton. The current Settings source owns local copies of `ConfigStore`, `DisplayManager` and `ShortcutRegistry`.

This cannot become a correct single-authority architecture by calling `Runtime::instance()` from Settings; process address spaces are separate.

A real control plane is required.

## 2.4 Current touchpoint ledger is no longer sufficient as architecture

The source already contains marker blocks for `FW-HOTKEY-01` and `FW-DISPLAY-01` while the current live ledger still describes several of these integrations as pending. The ledger is useful governance, but manual ledger maintenance alone does not prevent coupling drift.

The replacement must make illegal dependencies fail CI automatically.

---

# 3. Primary architecture law — Restore Upstream First

This is now a normative FlameWM rule.

For every IceWM-owned file that differs from the exact upstream base:

```text
DIFF HUNK
    -> RESTORE_UPSTREAM
    -> KEEP_GENERIC_FIX
    -> MINIMAL_ADAPTER_HOOK
```

No fourth category.

## 3.1 RESTORE_UPSTREAM

Flame product logic has moved behind Platform/Bridge and the original IceWM lines can return byte-for-byte.

Default classification.

## 3.2 KEEP_GENERIC_FIX

A source-proven generic IceWM bug fix that is useful without FlameWM.

Requirements:

- behavior independent of FlameWM;
- isolated commit;
- testable as IceWM behavior;
- submit upstream when practical;
- remove from Flame diff once upstream includes it.

Current example:

```text
G-01: wmTile center-origin correction
```

## 3.3 MINIMAL_ADAPTER_HOOK

A narrow call or friend seam is technically required because IceWM has no suitable public extension point.

Rules:

- no product algorithm in hook;
- no loops over Flame state;
- no persistence;
- no D-Bus/Pulse/NetworkManager code;
- no product configuration parsing;
- no product-owned member state added to IceWM classes when external state can work;
- normally <=5 changed executable lines per hook block;
- hook delegates immediately to `flamewm::engine::icewm`.

Example:

```cpp
// FLAMEWM-BRIDGE-HOOK-BEGIN(FB-MOVE-01)
flamewm::engine::icewm::Hooks::moveMotion(*this, motion);
// FLAMEWM-BRIDGE-HOOK-END(FB-MOVE-01)
```

All state and behavior live outside the IceWM file.

---

# 4. Target divergence budget

Current:

```text
30 modified original files
~1,789 additions / 108 deletions
large feature algorithms inside IceWM owners
```

Target after migration:

```text
<= 8 IceWM-owned src/*.cc|*.h files carrying product hooks
<= 5 build/install integration files
<= 80 net product-specific executable lines inside IceWM-owned source files
0 product data structures stored directly in IceWM classes unless unavoidable
0 feature algorithms inside IceWM-owned files
0 direct Flame config/DBus/XRandR transaction code inside IceWM-owned files
```

Generic upstreamable fixes are counted separately.

A future ordinary feature patch should meet:

```text
IceWM files touched:       0
Flame Platform files:      0..2 when capability already exists
Feature/UI files:          1..4
Focused tests:             1 group
Full IceWM source re-audit: not required
```

---

# 5. Final dependency architecture

```text
┌──────────────────────────────────────────────────────────────┐
│                         PRODUCT CODE                         │
│                                                              │
│  Flame Shell      flamewm-settings      flamewm-desktop      │
│  launcher/panel   on-demand process     filesystem desktop   │
│       │                  │                      │             │
└───────┼──────────────────┼──────────────────────┼─────────────┘
        │ direct C++       │ D-Bus                │ D-Bus
        │                  │                      │
┌───────▼──────────────────▼──────────────────────▼─────────────┐
│                     FLAMEWM PLATFORM                         │
│                                                              │
│  WindowService        SettingsService      ApplicationService │
│  WorkspaceService     ShortcutService      ScaleService       │
│  DisplayService       PanelService         SessionService     │
│  Audio/Media/Network services                                  │
│                                                              │
│  immutable snapshots / typed commands / typed subscriptions  │
└──────────────────────────────┬───────────────────────────────┘
                               │ engine ports
┌──────────────────────────────▼───────────────────────────────┐
│                         ICEWM BRIDGE                         │
│                                                              │
│  Bridge / Hooks / EngineAccess                               │
│  WindowAdapter      WorkspaceAdapter      DisplayAdapter      │
│  ShortcutAdapter    WorkAreaAdapter       MainLoopAdapter     │
│  NativeUiBackend    TrayHostAdapter                           │
│                                                              │
│  ONLY THIS AREA MAY KNOW ICEWM INTERNAL TYPES                │
└──────────────────────────────┬───────────────────────────────┘
                               │ minimal hooks / public APIs
┌──────────────────────────────▼───────────────────────────────┐
│                         ICEWM 4.x                            │
│ YWindowManager / YFrameWindow / X11 / EWMH / ICCCM          │
└──────────────────────────────────────────────────────────────┘
```

---

# 6. Dependency law

## 6.1 `src/flamewm/api/**`

May depend on:

```text
C++11 standard library only
```

Must not include:

```text
X11/*
wm*.h
yx*.h
ywindow.h
atasks.h
wmtaskbar.h
libdbus
libpulse
```

## 6.2 `src/flamewm/platform/**`

May depend on:

```text
flamewm/api
pure Flame models
```

Must not include IceWM or raw X11 types.

## 6.3 `src/flamewm/engine/icewm/**`

Only layer allowed to include engine internals such as:

```text
wmmgr.h
wmframe.h
wmtaskbar.h
wmkey.h
yxapp.h
yxtray.h
X11/Xlib.h
X11/extensions/Xrandr.h
```

## 6.4 `src/flamewm/control/**`

May depend on:

```text
flamewm/api
libdbus-1
```

Server side may depend on Platform services.

Client side must not depend on Platform implementation or IceWM internals.

## 6.5 `src/flamewm/shell/**` or current `src/flamewm/panel/**`, launcher, snap views

May depend on:

```text
api
Platform service interfaces
Flame UI
```

No IceWM engine headers.

## 6.6 `src/flamewm-settings/**`

May depend on:

```text
control client
Flame UI
page models
```

Must not own effective ConfigStore/DisplayManager/ShortcutRegistry instances.

## 6.7 `src/flamewm-desktop/**`

May own its own X11 desktop window implementation and filesystem state.

It must not include WM internals. Product/workspace/settings commands cross Flame Control.

---

# 7. Stable API vocabulary

High-level code should speak FlameWM concepts, never engine concepts.

## 7.1 IDs

```cpp
namespace flamewm { namespace api {

struct WindowRef {
    uint64_t id;          // session-scoped stable id
    uint64_t generation;  // rejects stale frame references
};

struct WorkspaceRef {
    int index;
    uint64_t revision;    // index valid only for this topology revision
};

struct OutputId {
    std::string key;      // durable EDID+connector identity
};

struct ModeId {
    uint64_t value;       // opaque outside engine adapter
};

struct TransactionId {
    uint64_t value;
};

}}
```

Do not expose `Window`, `RRMode`, `RROutput`, `RRCrtc` or raw pointers in Platform API headers.

## 7.2 Geometry

Canonical engine-neutral geometry:

```cpp
struct Point { int x; int y; };
struct Size  { int width; int height; };
struct Rect  { int x; int y; int width; int height; };
```

One utility owns intersection, clamp and work-area math.

## 7.3 Error model

Use typed status. No silent bool-only failure at service boundaries.

```cpp
enum class ErrorCode {
    None,
    InvalidArgument,
    NotFound,
    StaleRevision,
    Conflict,
    Busy,
    Unsupported,
    Unavailable,
    PermissionDenied,
    Timeout,
    IoFailure,
    EngineRejected,
    InternalFailure
};

struct Status {
    ErrorCode code;
    std::string message;
    bool ok() const { return code == ErrorCode::None; }
};

template<class T>
struct Result {
    Status status;
    T value;
    bool ok() const { return status.ok(); }
};
```

Adapter converts X/IceWM errors into this vocabulary once.

Upper UI never interprets raw X error codes.

---

# 8. Snapshot law

Reads use immutable snapshots.

Commands mutate through services.

Never hand mutable IceWM objects to upper code.

Every long-lived snapshot includes a monotonic version:

```text
revision    logical state version

generation  lifetime/topology epoch
```

Use revision for optimistic concurrency and stale Settings writes.

Use generation when external topology/object identity may disappear, such as:

```text
frame destruction
RandR hotplug
Runtime restart
panel output removal
service owner replacement
```

---

# 9. Engine port contracts

Ports are the only contracts Platform services use to reach the WM engine.

## 9.1 `WindowPort`

```cpp
class WindowPort {
public:
    virtual ~WindowPort() {}

    virtual api::WindowSnapshot snapshot() const = 0;
    virtual api::Result<api::WindowInfo> get(api::WindowRef window) const = 0;

    virtual api::Status activate(api::WindowRef window) = 0;
    virtual api::Status minimize(api::WindowRef window) = 0;
    virtual api::Status maximize(api::WindowRef window) = 0;
    virtual api::Status restore(api::WindowRef window) = 0;
    virtual api::Status close(api::WindowRef window) = 0;
    virtual api::Status setOuterGeometry(api::WindowRef window,
                                         const api::Rect& geometry) = 0;

    virtual api::Result<api::Rect> workArea(api::WindowRef window) const = 0;
    virtual api::Result<api::OutputId> output(api::WindowRef window) const = 0;
};
```

IceWM remains frame authority.

## 9.2 `WorkspacePort`

```cpp
class WorkspacePort {
public:
    virtual ~WorkspacePort() {}

    virtual api::WorkspaceSnapshot snapshot() const = 0;
    virtual api::Status activate(int index) = 0;
    virtual api::Status moveWindow(api::WindowRef window, int index) = 0;

    // Applies one already-validated old-index -> new-index transaction.
    virtual api::Status applyTransform(const api::WorkspaceTransform& plan) = 0;
};
```

`WorkspaceTransaction` remains pure Flame logic. Adapter applies it to IceWM state atomically.

## 9.3 `DisplayPort`

```cpp
class DisplayPort {
public:
    virtual ~DisplayPort() {}

    virtual api::Result<api::DisplaySnapshot> queryFresh() const = 0;
    virtual api::Result<api::CapturedDisplayState>
        capture(const api::OutputId& output) const = 0;
    virtual api::Status applyMode(const api::OutputId& output,
                                  api::ModeId mode) = 0;
    virtual api::Status restore(const api::CapturedDisplayState& state) = 0;
};
```

Only IceWM Bridge knows XRandR native IDs.

## 9.4 `ShortcutPort`

```cpp
class ShortcutPort {
public:
    virtual ~ShortcutPort() {}

    virtual api::Result<api::PreparedShortcutSet>
        prepare(const api::ShortcutMap& desired) = 0;
    virtual api::Status commit(api::PreparedShortcutSet& prepared) = 0;
    virtual api::Status rollback(api::PreparedShortcutSet& prepared) = 0;
};
```

Platform owns desired/effective binding policy.

Bridge owns actual X grabs.

No new `gKeyFlame*` globals are required.

## 9.5 `WorkAreaPort`

```cpp
class WorkAreaPort {
public:
    virtual ~WorkAreaPort() {}
    virtual api::DisplayWorkAreas baseWorkAreas() const = 0;
    virtual void requestRecompute() = 0;
};
```

Platform PanelService supplies additional Flame panel reservations through a minimal work-area hook.

## 9.6 `MainLoopPort`

```cpp
class MainLoopPort {
public:
    virtual ~MainLoopPort() {}
    virtual void addPoll(/* typed wrapper */) = 0;
    virtual void removePoll(/* typed wrapper */) = 0;
    virtual void addTimer(/* typed wrapper */) = 0;
    virtual void removeTimer(/* typed wrapper */) = 0;
};
```

This preserves one IceWM X/event-loop owner and lets D-Bus/Pulse remain event-driven.

---

# 10. Platform services

## 10.1 `PlatformHost`

Replace the vague global Runtime role with one explicit composition root:

```cpp
class PlatformHost final {
public:
    PlatformHost(api::EnginePorts& ports,
                 control::ServerFactory& controlFactory,
                 api::Clock& clock);

    Status start();
    void stop();

    WindowService& windows();
    WorkspaceService& workspaces();
    SettingsService& settings();
    ShortcutService& shortcuts();
    DisplayService& displays();
    PanelService& panels();
    ApplicationService& applications();
    ScaleService& scale();
    SystemServices& system();
};
```

`PlatformHost` is passed by reference to Flame components.

Do not let feature modules call a general `Runtime::instance()` service locator.

The only process-global pointer allowed is confined inside IceWM Bridge hooks so upstream one-line hooks can find the attached bridge.

## 10.2 `WindowService`

Responsibilities:

- convert engine snapshots into stable Flame window snapshots;
- coalesce window invalidation events;
- expose activate/minimize/maximize/restore/close commands;
- expose focused-window/output query;
- never own authoritative window state.

## 10.3 `WorkspaceService`

Responsibilities:

- consume authoritative engine snapshot;
- use existing `WorkspaceTransaction` and `TwoRowTopology` pure logic;
- build insert/remove plans;
- reject stale topology revisions;
- ask `WorkspacePort` to apply exactly once;
- publish one resulting snapshot event.

## 10.4 `SettingsService`

Single effective Flame settings authority.

Owns:

```text
SettingsSnapshot
revision
ConfigStore/persistence
settings validation
reversible apply coordinator
```

Correct transaction:

```text
candidate
-> parse
-> validate entire snapshot
-> reject stale expected revision
-> ask affected services to PREPARE reversible changes
-> APPLY prepared native effects without publishing
-> atomic persistence
-> COMMIT/publish one new effective snapshot
-> emit one SettingsChanged revision
```

On native apply failure:

```text
rollback every already-applied plan
no persistence
no publish
```

On persistence failure after native apply:

```text
rollback native effects
old effective state remains authoritative
```

Display resolution changes are not part of the normal settings transaction; they use the explicit Display transaction with Keep/Revert timeout.

## 10.5 `ShortcutService`

Owns:

```text
ActionId -> KeyBinding desired/effective map
normalization
Escape == unassigned
modifier-only metadata
collision validation
transaction orchestration
```

IceWM Bridge does only native grabs/ungrabs and key-event interception.

This permits restoring current changes in `bindkey.*` and `wmkey.*`.

## 10.6 `DisplayService`

Owns:

```text
DisplaySnapshot
output topology generation
shell scale metadata
pending mode transaction
15-second deadline
Keep/Revert/crash/hotplug policy
```

Bridge owns:

```text
XRRGetScreenResources
XRRGetOutputInfo
XRRGetCrtcInfo
XRRSetCrtcConfig
native XRandR IDs
```

No `RRMode`, `RROutput` or `RRCrtc` in `flamewm/api` or Settings.

## 10.7 `PanelService`

Owns product panel state only:

```text
one panel per active output
edge
logical size
visibility/fullscreen policy
pinned application membership
persistent task ordering policy
one Start open globally
single tray host assignment
work-area reservations
```

It consumes WindowService/WorkspaceService/ApplicationService snapshots.

It does not become window/workspace authority.

## 10.8 `ApplicationService`

Centralize application identity once:

```text
desktop ID
-> StartupWMClass
-> WM_CLASS
-> normalized fallback
```

Taskbar, launcher, pinning and recent-app behavior use this service.

No feature reimplements application identity.

## 10.9 System services

`AudioService`, `MediaService`, `NetworkService` remain standard-service adapters:

```text
Audio     libpulse async / PipeWire-Pulse compatible
Media     MPRIS on session D-Bus
Network   NetworkManager on system D-Bus
```

One shared D-Bus reactor/event-loop integration.

No recurring command polling.

---

# 11. IceWM Bridge

## 11.1 Purpose

The Bridge is not a new engine.

It is the one place that understands both sides:

```text
IceWM internal objects
and
FlameWM engine-neutral ports
```

Directory:

```text
src/flamewm/engine/icewm/
├── bridge.h
├── bridge.cc
├── hooks.h
├── hooks.cc
├── access.h
├── access.cc
├── window_adapter.h/.cc
├── workspace_adapter.h/.cc
├── display_adapter.h/.cc
├── shortcut_adapter.h/.cc
├── workarea_adapter.h/.cc
├── mainloop_adapter.h/.cc
├── native_overlay.h/.cc
└── tray_adapter.h/.cc
```

## 11.2 `Bridge`

Composition:

```cpp
class Bridge final {
public:
    Bridge(YWMApp& app, YWindowManager& manager);
    ~Bridge();

    api::EnginePorts& ports();
    platform::PlatformHost& platform();

    void onWindowInvalidated(YFrameWindow& frame);
    void onWindowMembershipChanged(YFrameWindow& frame);
    void onWindowRemoved(YFrameWindow& frame);
    bool onRootKey(const XKeyEvent& key, bool repeating);
    void onWorkspaceChanged();
    void onTopologyChanged();
    void reserveFlameWorkAreas(YWindowManager& manager);
};
```

## 11.3 `Hooks`

Static shim used only by minimal IceWM source patches.

```cpp
class Hooks final {
public:
    static void attach(YWMApp& app, YWindowManager& manager);
    static void detach();

    static void windowInvalidated(YFrameWindow& frame);
    static void windowMembershipChanged(YFrameWindow& frame);
    static void windowRemoved(YFrameWindow& frame);

    static bool rootKey(const XKeyEvent& key, bool repeating);

    static void moveBegin(YFrameWindow& frame, const XButtonEvent& event);
    static void moveMotion(YFrameWindow& frame, const XMotionEvent& event);
    static void moveEnd(YFrameWindow& frame, const XButtonEvent& event);
    static void moveCancel(YFrameWindow& frame);

    static void workspaceChanged();
    static void topologyChanged();
    static void reserveWorkAreas(YWindowManager& manager);
};
```

When unattached, all hooks are deterministic no-ops.

That makes the IceWM code path safe and trivial.

## 11.4 `EngineAccess`

Use public IceWM API first.

Only when a required atomic operation needs private state may `EngineAccess` receive friend access.

Example:

```cpp
namespace flamewm { namespace engine { namespace icewm {
class EngineAccess;
}}}

class YWindowManager {
    friend class flamewm::engine::icewm::EngineAccess;
    ...
};
```

This deliberately trades **compile-time adapter breakage** for **merge conflict reduction**.

If upstream renames a private field, the adapter fails to compile in one place instead of forcing a textual conflict through hundreds of product lines embedded in `wmmgr.cc`.

Rules:

1. one friend type only;
2. no direct friend access from feature modules;
3. every private field access documented in `engine/icewm/COMPATIBILITY.md`;
4. prefer existing public methods;
5. private access must be covered by adapter tests and upstream-update compile gates.

---

# 12. Fast path versus control path

This distinction is mandatory.

## 12.1 Fast path — direct C++

Use for:

```text
window move/resize events
snap target computation
snap overlay
frame invalidation
focus/task presentation updates
panel painting
pointer drag
```

Requirements:

- same X owner thread;
- no D-Bus serialization;
- no disk I/O;
- no subprocess;
- no heap churn per motion event where avoidable;
- no string lookup inside hot pointer-motion loops.

## 12.2 Control path — D-Bus

Use for separate-process actions:

```text
Settings apply
hotkey editing
display mode changes
panel preferences
workspace management from desktop/settings
shell scale
appearance changes
flamewmctl
```

This solves process ownership without making frame motion depend on IPC.

---

# 13. Flame Control — versioned D-Bus API

## 13.1 Naming

Use reverse-domain naming based on ArkFlame's domain.

```text
Bus name:
  com.arkflame.FlameWM1

Root object:
  /com/arkflame/FlameWM1

Interfaces:
  com.arkflame.FlameWM1
  com.arkflame.FlameWM1.Settings
  com.arkflame.FlameWM1.Workspaces
  com.arkflame.FlameWM1.Displays
  com.arkflame.FlameWM1.Shortcuts
  com.arkflame.FlameWM1.Panels
  com.arkflame.FlameWM1.Session
```

Major version belongs in bus/interface/object namespace. Additive v1 evolution is permitted. Breaking change creates `FlameWM2`; do not silently change v1 meaning.

## 13.2 Root API

```text
GetVersion()
GetCapabilities()
Ping()
```

Capabilities are semantic strings, for example:

```text
panel.multi-output
panel.transparency
settings.live
shortcuts.transactional
display.mode-change
display.shell-scale
network.networkmanager
audio.pulse
media.mpris
desktop.sticky-notes
```

## 13.3 Settings interface

```text
GetSnapshot()
Apply(expected_revision, changes)
ResetSection(expected_revision, section)
```

Signal:

```text
SettingsChanged(new_revision, changed_keys)
```

## 13.4 Workspaces interface

```text
GetSnapshot()
Activate(index, expected_revision)
InsertAfter(index, expected_revision)
Remove(index, expected_revision)
```

Signal:

```text
WorkspacesChanged(new_revision)
```

## 13.5 Displays interface

```text
GetSnapshot()
BeginModeChange(output_key, mode_id, topology_generation)
Keep(transaction_id)
Revert(transaction_id)
SetShellScale(output_key, percent, expected_revision)
```

Signals:

```text
TopologyChanged(new_generation)
DisplayTransactionChanged(transaction_id, state, deadline)
```

The display transaction is WM-owned. If Settings dies, timeout/revert still happens.

## 13.6 Shortcuts interface

```text
GetSnapshot()
SetBinding(action_id, binding, expected_revision)
ClearBinding(action_id, expected_revision)
ResetBinding(action_id, expected_revision)
```

Signal:

```text
ShortcutsChanged(new_revision)
```

## 13.7 Panels interface

```text
GetSnapshot()
SetEdge(output_key, edge, expected_revision)
SetSize(output_key, logical_size, expected_revision)
Pin(desktop_app_id, expected_revision)
Unpin(desktop_app_id, expected_revision)
Reorder(entry_id, insertion_index, expected_revision)
```

Signal:

```text
PanelsChanged(new_revision)
```

## 13.8 D-Bus error names

```text
com.arkflame.FlameWM1.Error.InvalidArgument
com.arkflame.FlameWM1.Error.NotFound
com.arkflame.FlameWM1.Error.StaleRevision
com.arkflame.FlameWM1.Error.Conflict
com.arkflame.FlameWM1.Error.Busy
com.arkflame.FlameWM1.Error.Unsupported
com.arkflame.FlameWM1.Error.Unavailable
com.arkflame.FlameWM1.Error.Timeout
com.arkflame.FlameWM1.Error.EngineRejected
```

## 13.9 Canonical protocol source

Store introspection XML as source of truth:

```text
src/flamewm/control/dbus/com.arkflame.FlameWM1.xml
```

Install a copy for introspection/debugging:

```text
/usr/share/dbus-1/interfaces/com.arkflame.FlameWM1.xml
```

Do not D-Bus-activate the WM service. The running FlameWM session owns the name.

---

# 14. `FlameControlClient`

Every external Flame process uses one client abstraction.

```cpp
class Client final {
public:
    Result<Capabilities> capabilities();
    SettingsClient& settings();
    WorkspaceClient& workspaces();
    DisplayClient& displays();
    ShortcutClient& shortcuts();
    PanelClient& panels();
};
```

`flamewm-settings` should eventually contain no code equivalent to:

```text
XRRGetScreenResources
XGrabKey
Runtime::instance()
local effective DisplayManager
local effective ShortcutRegistry
local effective ConfigStore
```

It asks Flame Control.

---

# 15. `flamewmctl`

Add one first-party command client.

Examples:

```bash
flamewmctl status
flamewmctl workspaces list
flamewmctl workspaces add-after 1
flamewmctl workspaces remove 2
flamewmctl panels list
flamewmctl panels set-edge HDMI-1 left
flamewmctl shortcuts get
flamewmctl shortcuts set ToggleStartMenu Super
flamewmctl displays list
flamewmctl settings get
```

This is not feature bloat. It is an operational interface.

Benefits:

- agents can prove behavior without clicking GUI;
- bug reports become reproducible;
- nested-X tests have stable commands;
- user scripting comes almost free;
- D-Bus protocol is exercised continuously by first-party software.

---

# 16. Typed events — no generic EventBus

Do not introduce:

```text
EventBus.emit("windowChanged", map<string, any>)
```

Use domain listeners with RAII subscriptions.

Concept:

```cpp
class WindowListener {
public:
    virtual ~WindowListener() {}
    virtual void onWindowSnapshotChanged(uint64_t revision) = 0;
};

class Subscription {
public:
    ~Subscription(); // automatically unregisters
};
```

Use coalesced invalidation where many engine events happen in one X loop iteration.

Example:

```text
five frame properties change
-> mark WindowService dirty
-> one deferred refresh
-> one snapshot revision
-> one panel repaint request
```

No polling.

---

# 17. Window/task architecture

Current `TaskPane` modifications are a major upstream-maintenance hotspot.

The long-term architecture should stop using `TaskPane` as the Flame taskbar product model.

## 17.1 Authority

```text
IceWM/YFrameWindow   running/focused/minimized/maximized truth
ApplicationService  app identity
PanelService        pin membership/order policy
TaskModel            derived presentation snapshot
```

## 17.2 Derived task model

```cpp
struct TaskEntry {
    TaskEntryId id;
    DesktopAppId app;
    bool pinned;
    bool running;
    bool focused;
    bool minimized;
    api::WindowRef window; // meaningful only for live entry through explicit flag
};
```

TaskModel merges current window snapshot with persisted pin state.

V7/V8 semantics remain product behavior, but implementation moves fully to Flame-owned code.

This allows restoration of `src/atasks.cc` and `src/atasks.h` to upstream.

## 17.3 Reorder

Flame task strip owns its own visible entry order.

No X11 window identity changes.

No modification to IceWM task grouping implementation required.

---

# 18. Panel architecture — stop extending upstream TaskBar

This is one of the most important decisions in this report.

## 18.1 Flame session default

Ship Flame defaults with upstream IceWM taskbar disabled:

```text
ShowTaskBar=0
```

Flame Shell creates its own lightweight in-process panel surfaces using the same native X/IceWM toolkit primitives.

This means FlameWM is no longer constrained by IceWM's singleton horizontal `TaskBar` implementation.

## 18.2 Panel surfaces

```text
FlamePanelSurface
    one per active output
    native YWindow/Graphics implementation
    product layout from PanelService
    no duplicate window/workspace authority
```

Primary panel owns the single XEmbed system tray through existing public `YXTray`.

Secondary panels never claim the tray selection.

`YXTray` itself can return to pristine upstream code.

## 18.3 Work areas

Panel surfaces are Flame-owned shell windows. IceWM Bridge supplies their reserved rectangles to the manager through one work-area hook.

Concept:

```cpp
// inside existing IceWM updateWorkArea flow
flamewm::engine::icewm::Hooks::reserveWorkAreas(*this);
```

Bridge calls the manager's existing work-area mechanisms through `EngineAccess`.

This is vastly smaller than teaching upstream `TaskBar` about per-output four-edge Flame product layout.

## 18.4 Fullscreen behavior

WindowService publishes fullscreen/focus/output changes.

PanelService decides whether a panel surface is visible/raised on that output.

No periodic check.

## 18.5 Result

Target final state:

```text
src/wmtaskbar.cc   pristine upstream
src/wmtaskbar.h    pristine upstream
src/atasks.cc      pristine upstream
src/atasks.h       pristine upstream
src/yxtray.cc      pristine upstream
```

If a final implementation proves one tiny TaskBar compatibility hook is still required, it must be justified against this cleaner target before landing.

---

# 19. Snap architecture

Current implementation stores substantial Flame snap state in `YFrameWindow` and hundreds of lines around `wmframe.cc`.

Move all Flame snap state out.

## 19.1 `SnapService`

Owns:

```text
WindowRef -> SnapSession
saved floating geometry
current target
preview target
400 ms workspace dwell state
generation
```

Engine hook flow:

```text
startMoveSize
-> Hooks::moveBegin

motion
-> Hooks::moveMotion
-> SnapService computes target
-> WindowPort supplies work area
-> native overlay updates

release
-> Hooks::moveEnd
-> SnapService commits via WindowPort

cancel/destruction
-> clear state
```

## 19.2 One geometry authority

The same pure function returns target geometry for preview and commit.

No duplicate preview math.

## 19.3 Restore target

After migration:

```text
src/wmframe.h     pristine upstream
src/wmframe.cc    pristine except generic G-01 + tiny window invalidation hook if needed
src/movesize.cc   original logic + small move lifecycle hooks
```

---

# 20. Workspace architecture

Current indexed workspace implementation adds large product transaction logic directly to `wmmgr.cc`.

Move it behind `WorkspaceService` + `WorkspaceAdapter`.

## 20.1 Pure plan

Reuse existing:

```text
WorkspaceTransaction
TwoRowTopology
```

`WorkspaceService` produces an immutable `WorkspaceTransform`.

## 20.2 Native apply

`IcewmWorkspaceAdapter` applies the transform using existing public manager/frame functions wherever possible.

For the small amount of private active/last-workspace state needed to preserve exact transaction semantics, use the single `EngineAccess` friend.

## 20.3 No second authority

Workspace IDs remain IceWM EWMH indices.

Flame uses `WorkspaceRef(index, revision)` to protect against stale UI operations rather than inventing a second permanent workspace registry.

## 20.4 Restore target

Large `insertWorkspaceAt/removeWorkspaceAt` product algorithms disappear from `wmmgr.cc`.

The manager only needs small invalidation/hooks plus one access friend.

---

# 21. Shortcut architecture

Current shortcut integration added Flame globals to `bindkey.*` and native transaction implementation to `wmkey.cc`/`wmmgr.cc`.

Replace it.

## 21.1 Platform owner

`ShortcutService` owns product action mapping and desired/effective bindings.

## 21.2 Bridge owner

`IcewmShortcutAdapter` owns:

```text
keysym/modifier parsing
XGrabKey/XUngrabKey
scoped X error capture
staged rollback
root-event match
```

## 21.3 One manager hook

At the top of the appropriate existing key dispatch:

```cpp
if (flamewm::engine::icewm::Hooks::rootKey(key, repeating))
    return true;
```

If not handled, original IceWM flow proceeds unchanged.

## 21.4 Restore target

Return these to exact upstream bytes:

```text
src/bindkey.cc
src/bindkey.h
src/wmkey.cc
src/wmkey.h
```

No new `gKeyFlame*` globals.

---

# 22. Configuration architecture

Flame settings should not be injected into IceWM's general preference parser.

IceWM preferences remain IceWM preferences.

Flame product configuration remains Flame configuration.

## 22.1 Startup defaults

Use packaged IceWM preference/theme files for fixed product defaults such as disabling legacy taskbar components.

## 22.2 Dynamic Flame settings

`SettingsService` owns dynamic settings and applies resulting semantic changes through Platform services.

Examples:

```text
TaskbarHeight
-> PanelService.setSize

Accent
-> Appearance/Palette service

Workspace shortcut
-> ShortcutService

Display scale
-> ScaleService
```

Do not mutate IceWM parser globals just because a GUI setting exists.

## 22.3 Restore target

Return to exact upstream:

```text
src/wmconfig.cc
src/wmconfig.h
src/yconfig.cc
src/yconfig.h
src/yprefs.cc
```

This removes a major source of difficult update collisions.

---

# 23. Display architecture

Current `core/display.h` leaks XRandR native types when `CONFIG_XRANDR` is enabled.

That violates the desired boundary.

Split it into:

```text
api/display.h                 engine-neutral types only
platform/display_service.*    transaction/state policy
engine/icewm/display_adapter.* native XRandR implementation
```

Delete the redundant forwarding alias `integrations/displaymanager.h` once callers migrate.

Display transaction lifecycle:

```text
Settings D-Bus request
-> DisplayService validates snapshot generation/output/mode
-> Bridge captures fresh native CRTC state
-> apply
-> WM-owned 15s timer starts
-> RandR event refreshes topology
-> Keep commits persistence
OR Revert/timeout/settings-owner-loss/hotplug restores captured state
```

No stale X IDs survive a topology generation change.

---

# 24. Chrome/scale architecture

Some IceWM frame-rendering hooks are genuinely unavoidable because IceWM draws the decorations.

Keep them tiny.

## 24.1 `ChromePolicy`

Flame-owned service calculates:

```text
border visual metric
resize hit metric
titlebar metric
button hit metric
title text position
palette
scale bucket
```

## 24.2 IceWM hooks

Target shape:

```cpp
int YFrameWindow::borderXN() const {
    if (flamewm::engine::icewm::Hooks::hasChromePolicy())
        return flamewm::engine::icewm::Hooks::borderX(*this, /* upstream fallback */ ...);
    return ORIGINAL_ICEWM_EXPRESSION;
}
```

Keep original fallback visibly intact.

`wmtitle.cc` asks one high-level title-layout helper for final text bounds; it does not know ScaleManager/Runtime.

## 24.3 No global per-output mutation

Do not change global `wsTitleBar`/border metrics as windows cross outputs.

Per-frame query remains output-aware.

---

# 25. Native UI facade

Upper UI code currently still reaches IceWM toolkit classes directly in places such as Settings and panel widgets.

This is not as dangerous as WM-state coupling, but it still forces future agents to understand toolkit internals.

Create a deliberately small facade.

## 25.1 Scope

Only primitives FlameWM actually uses:

```text
Window
Label
Button
IconButton
TextField
Slider
Toggle
Select/Menu
List
ScrollView
Dialog
Popover
Icon
Form/Stack layout helper
```

No plugin/widget framework.

## 25.2 Backend

```text
src/flamewm/ui/**                 stable Flame UI contracts/tokens
src/flamewm/engine/icewm/ui/**    YWindow/YButton/Graphics/YIcon backend
```

Upper page code becomes:

```cpp
class TaskbarPage {
public:
    TaskbarPage(control::Client& control, ui::Form& form);
};
```

not a subclass that directly knows taskbar globals or X11.

---

# 26. Session architecture — restore `icesm.cc`

Current source adds roughly 178 lines to upstream `icesm.cc` for Flame session selection and desktop-helper supervision.

This should be fully moved out.

## 26.1 Use upstream capability already present

Upstream `icewm-session` supports selecting the WM executable through its `--icewm=FILE` option.

Create a Flame-owned `flamewm-session` wrapper instead of modifying `icesm.cc`.

## 26.2 Wrapper model

```text
flamewm-session
  -> starts Flame desktop supervisor child
  -> execs upstream icewm-session --icewm=/usr/bin/flamewm
```

Desktop supervisor:

```text
parent-bound lifetime
bounded restart/backoff for flamewm-desktop
clean SIGTERM
no ownership of WM state
```

On Linux, parent-death handling can use a native parent-death mechanism and still retain explicit shutdown handling.

## 26.3 Result

Restore:

```text
src/icesm.cc
```

to pristine upstream.

Flame packaging owns the session desktop file and wrapper executable.

---

# 27. External integration reactor

Keep one event-driven reactor per WM process.

Current `DBusDispatcher` work is reusable, but responsibility should be clearer.

Final structure:

```text
platform/reactor/
    mainloop.*

platform/system/
    networkmanager.*
    mpris.*
    pulse.*

control/dbus/
    server.*
    client.*
```

NetworkManager uses system bus.

MPRIS and Flame Control use session bus.

Pulse uses async API.

All callbacks re-enter legal Flame/WM event-loop ownership before UI/X mutation.

---

# 28. Proposed source tree

```text
src/flamewm/
├── api/
│   ├── errors.h
│   ├── ids.h
│   ├── geometry.h
│   ├── capabilities.h
│   ├── window.h
│   ├── workspace.h
│   ├── display.h
│   ├── shortcuts.h
│   ├── panels.h
│   ├── settings.h
│   ├── applications.h
│   └── ports.h
│
├── platform/
│   ├── host.h/.cc
│   ├── windows/
│   ├── workspaces/
│   ├── settings/
│   ├── shortcuts/
│   ├── displays/
│   ├── panels/
│   ├── applications/
│   ├── scale/
│   ├── reactor/
│   └── system/
│
├── engine/
│   └── icewm/
│       ├── bridge.h/.cc
│       ├── hooks.h/.cc
│       ├── access.h/.cc
│       ├── window_adapter.h/.cc
│       ├── workspace_adapter.h/.cc
│       ├── display_adapter.h/.cc
│       ├── shortcut_adapter.h/.cc
│       ├── workarea_adapter.h/.cc
│       ├── mainloop_adapter.h/.cc
│       ├── tray_adapter.h/.cc
│       └── ui/
│
├── control/
│   ├── dbus/
│   │   ├── com.arkflame.FlameWM1.xml
│   │   ├── server.h/.cc
│   │   └── client.h/.cc
│   └── types_codec.h/.cc
│
├── shell/
│   ├── panel/
│   ├── launcher/
│   ├── taskmodel/
│   ├── snap/
│   └── popovers/
│
└── ui/
    ├── metrics.*
    ├── palette.*
    ├── iconroles.*
    ├── controls.*
    └── layout.*

src/flamewm-settings/
src/flamewm-desktop/
src/flamewm-session/
src/flamewmctl/
```

This is the final target tree. Do not perform a mass source move as the first migration operation. Move files only after their dependency boundary is proven.

---

# 29. Current source -> target module mapping

| Current source | Target |
|---|---|
| `core/runtime.*` | replace with `platform/host.*`; temporary compatibility wrapper allowed during migration |
| `core/types.h` | split into `api/ids.h`, `geometry.h`, panel enums |
| `core/config.*` | `api/settings.h` + `platform/settings/*` |
| `core/display.*` | `api/display.h` + `platform/displays/*` + IceWM DisplayAdapter |
| `core/shortcutregistry.*` | `api/shortcuts.h` + `platform/shortcuts/*` |
| `core/scalemanager.*` | `platform/scale/*` |
| `core/appidentity.*` | `platform/applications/*` |
| `workspace/transaction.*` | keep pure, move under `platform/workspaces/*` |
| `workspace/topology.*` | keep pure, move under `platform/workspaces/*` |
| `snap/controller.*`, `state.*` | keep pure, owned by shell/platform snap service |
| `snap/overlay.*` | native overlay implementation moves behind native UI/engine adapter |
| `panel/panelmanager.*` | `platform/panels/*` |
| `panel/outputpanel.*` | pure panel geometry/model retained |
| `panel/taskstrip.*` | replace with Flame TaskModel + Flame task view; no TaskPane inheritance |
| `panel/pinnedlauncher.*` | replace native TaskButton subclass with Flame-owned task view |
| `integrations/dbusdispatcher.*` | reactor/control D-Bus infrastructure |
| `integrations/networkmanager.*` | `platform/system/networkmanager.*` |
| `integrations/mpris.*` | `platform/system/mpris.*` |
| `integrations/pulse.*` | `platform/system/pulse.*` |
| `integrations/displaymanager.h` | delete after callers use canonical Display API |
| Settings local stores/managers | remove; replace with `control::Client` |

---

# 30. Exact current IceWM-file restoration map

This table is based on the audited current archive versus pristine IceWM 4.1.0.

| Current modified original file | Target action | Final reason to differ from upstream |
|---|---|---|
| `.gitignore` | KEEP product repo hygiene | generated Flame build/runtime paths only |
| `configure.ac` | REDUCE | one Flame dependency macro include/call only |
| `lib/CMakeLists.txt` | REDUCE | one Flame runtime-assets include/subdirectory |
| `lib/Makefile.am` | REDUCE | one Flame install fragment include |
| `src/CMakeLists.txt` | REDUCE | one Flame build subdirectory/include + link |
| `src/Makefile.am` | REDUCE | one Flame build fragment include |
| `src/wmapp.cc` | RESTORE_MOST + HOOK | Bridge attach/detach only |
| `src/wmapp.h` | RESTORE_UPSTREAM | Bridge state lives outside YWMApp |
| `src/wmmgr.cc` | RESTORE_MOST + HOOK | key intercept, topology/workspace invalidation, work-area reservation |
| `src/wmmgr.h` | RESTORE_MOST + FRIEND | single `EngineAccess` friend only if public API insufficient |
| `src/wmframe.cc` | RESTORE_MOST | generic G-01 until upstreamed + window invalidation hook(s) |
| `src/wmframe.h` | RESTORE_UPSTREAM | snap/product state externalized |
| `src/movesize.cc` | RESTORE_MOST + HOOK | begin/motion/end/cancel snap hooks only |
| `src/decorate.cc` | RESTORE_MOST + HOOK | per-frame Flame metric query |
| `src/wmtitle.cc` | RESTORE_MOST + HOOK | Flame title-layout query |
| `src/wmtaskbar.cc` | RESTORE_UPSTREAM | Flame uses independent in-process panel surfaces |
| `src/wmtaskbar.h` | RESTORE_UPSTREAM | no Flame member/TaskStrip |
| `src/atasks.cc` | RESTORE_UPSTREAM | Flame task strip becomes independent view/model |
| `src/atasks.h` | RESTORE_UPSTREAM | no V8 product drag state in IceWM TaskPane |
| `src/yxtray.cc` | RESTORE_UPSTREAM | reuse public YXTray unchanged |
| `src/bindkey.cc` | RESTORE_UPSTREAM | FlameShortcutService owns product bindings |
| `src/bindkey.h` | RESTORE_UPSTREAM | no `gKeyFlame*` globals |
| `src/wmkey.cc` | RESTORE_UPSTREAM | native grab adapter moves to Bridge |
| `src/wmkey.h` | RESTORE_UPSTREAM | no Flame helper methods needed |
| `src/wmconfig.cc` | RESTORE_UPSTREAM | Flame config separate from IceWM parser |
| `src/wmconfig.h` | RESTORE_UPSTREAM | same |
| `src/yconfig.cc` | RESTORE_UPSTREAM | same |
| `src/yconfig.h` | RESTORE_UPSTREAM | same |
| `src/yprefs.cc` | RESTORE_UPSTREAM | same |
| `src/icesm.cc` | RESTORE_UPSTREAM | Flame session wrapper/supervisor owns helper lifecycle |

This restoration map is the central migration objective.

---

# 31. Proposed permanent IceWM hook set

Target hook budget after migration:

## `FB-BOOT-01` — `src/wmapp.cc`

Purpose:

```text
attach Bridge after manager/X/icon authorities exist
detach before manager/X teardown
```

Target diff: one include + two calls.

## `FB-WINDOW-01` — `src/wmframe.cc`

Purpose:

```text
window invalidation / task-membership / removal notification
```

Target diff: a few one-line event calls in existing centralized update paths.

## `FB-MOVE-01` — `src/movesize.cc`

Purpose:

```text
move begin / motion / release / cancel notifications for SnapService
```

Target diff: four small calls.

## `FB-MANAGER-01` — `src/wmmgr.cc`

Purpose:

```text
Flame shortcut interception
workspace invalidation after authoritative change
RandR topology invalidation
Flame work-area reservations
```

Target: no product algorithms.

## `FB-ACCESS-01` — `src/wmmgr.h`

Purpose:

```text
single friend seam for atomic workspace/work-area native operations not exposed publicly
```

Target: forward declaration + one friend line.

## `FB-CHROME-01` — `src/decorate.cc`, `src/wmtitle.cc`

Purpose:

```text
per-frame metrics + title-layout policy
```

Target: small provider queries with exact upstream fallback.

## `G-01` — `src/wmframe.cc`

Generic center-tile correctness fix until upstream carries it.

No other product hook is accepted without updating this architecture and proving that an existing port cannot represent the required operation.

---

# 32. Build-system isolation

Current `src/CMakeLists.txt` and `src/Makefile.am` contain large Flame source registrations.

Move these lists to Flame-owned files.

## 32.1 CMake

Target:

```text
src/CMakeLists.txt
    add_subdirectory(flamewm)
```

`src/flamewm/CMakeLists.txt` owns every Flame library/source/test target.

If source-tree layout prevents direct `add_subdirectory`, use one Flame-owned included CMake fragment. Do not keep 100+ Flame filenames in upstream `src/CMakeLists.txt`.

## 32.2 Autotools

Target:

```make
include $(srcdir)/flamewm/Makefile.inc
```

Flame-owned fragment contains source lists/targets.

## 32.3 Configure dependencies

Move Flame dependency checks to:

```text
m4/flamewm.m4
```

`configure.ac` gets one bounded macro include/call.

## 32.4 Runtime assets

Flame-owned install manifest handles:

```text
themes
icons
wallpapers
session files
D-Bus interface XML
```

Upstream lib build files contain one include/subdirectory only.

---

# 33. Architecture enforcement tools

Create:

```text
tools/flamewm-architecture/check-layers.py
tools/flamewm-upstream/audit-diff.py
tools/flamewm-upstream/merge-rehearsal.sh
```

## 33.1 Layer checker

Fail CI if:

```text
src/flamewm/api/** includes X11 or IceWM
src/flamewm/platform/** includes IceWM
src/flamewm-settings/** includes WM engine headers
src/flamewm-desktop/** includes WM engine headers
any file outside engine/icewm includes wmmgr.h/wmframe.h/wmtaskbar.h/atasks.h/wmkey.h
```

Native UI backend is the only explicit exception for toolkit headers.

## 33.2 Upstream diff checker

Store exact base:

```text
flamewm/upstream/BASE
```

Contents:

```text
IceWM version
upstream commit SHA
import date
```

Store hook manifest:

```text
flamewm/upstream/hooks.yml
```

For every changed upstream file it lists only permitted hook/generic patch IDs.

CI compares current tree to exact upstream Git object and fails if:

- a new upstream-owned file changes without manifest entry;
- code outside approved marker blocks differs;
- product hook exceeds budget;
- generic fix lacks its own patch ID.

## 33.3 Upstream patch budget

CI summary should print:

```text
UPSTREAM_BASE=<sha>
MODIFIED_UPSTREAM_SOURCE_FILES=7
PRODUCT_HOOK_LINES=...
GENERIC_PATCH_LINES=...
UNREGISTERED_DIFF=0
```

This makes maintainability measurable rather than aspirational.

---

# 34. Upstream update workflow

## 34.1 Branch model

Recommended remotes:

```text
origin    ArkFlame FlameWM fork
upstream  ice-wm/icewm
```

Keep exact upstream commits reachable.

Do not copy future upstream releases into the tree manually.

## 34.2 Update procedure

For each IceWM release:

```text
1. fetch upstream
2. record new candidate base
3. run architecture layer checker on current tree
4. run merge/rebase rehearsal in isolated worktree
5. resolve only minimal hook/build conflicts
6. compile IceWM Bridge against changed private/public API
7. adapter compile errors identify semantic drift
8. run platform service tests
9. run full native build/CTest
10. run nested-X shell smoke
11. update BASE + compatibility notes only after green
```

## 34.3 `git rerere`

Enable recorded conflict resolution for the tiny recurring hook locations. It is useful after the hook surface is small and stable; it is not a substitute for shrinking the patch surface.

## 34.4 Generic fixes upstream first

For `G-*` changes:

- isolate generic patch;
- submit upstream when practical;
- once included in a later IceWM release, delete local patch entry;
- never keep product dependence on an unsubmitted generic change when an adapter can avoid it.

---

# 35. Migration strategy from current 0.0.5 state

Do not throw away the current work.

Do not keep expanding it either.

Use **Branch by Abstraction**: introduce stable contracts alongside current implementations, redirect one domain at a time, then delete the old direct integration.

Current documented checkpoint remains useful as a behavioral reference point. The current post-checkpoint source should be treated as the migration input, not the final architecture.

---

# 36. Migration phases

## Phase 0 — Freeze evidence and development environment

Before architecture mutation:

- record exact current HEAD/checkpoint and dirty delta;
- record pristine IceWM base commit/version;
- regenerate current 30-file divergence table;
- make full build environment reproducible;
- no new feature-specific IceWM hooks after this freeze.

Create a canonical development environment:

```text
containers/dev/Containerfile
or equivalent reproducible environment

tools/dev/doctor --strict
tools/dev/build
tools/dev/test
tools/dev/run-nested
```

Current repeated missing XRender/fontconfig/XRandR/D-Bus/Pulse/Autotools dependencies must stop being rediscovered by each agent.

Gate:

```text
full configure/build is either green
or explicitly environment-blocked before source work begins
```

## Phase 1 — Add `api` + port contracts

CREATE only.

No behavior change.

Move/copy the stable concepts from current pure modules into engine-neutral API types.

Add fake engine ports for tests.

Gate:

```text
api headers compile with no X11/IceWM includes
platform tests use fake ports
```

## Phase 2 — Introduce `PlatformHost` and IceWM Bridge bootstrap

Create Bridge and adapters around existing engine APIs.

Reduce current Runtime singleton role to a compatibility wrapper temporarily.

Modify `wmapp.cc` toward final attach/detach hook immediately.

Restore `wmapp.h` to upstream by moving owner state into Bridge static attachment storage.

Gate:

```text
normal IceWM behavior unchanged when Bridge inactive
Flame session PlatformHost starts/stops once
no callback survives detach generation
```

## Phase 3 — Flame Control first

This is highest-ROI migration because Settings is currently blocked on process ownership.

Implement:

```text
control protocol XML
D-Bus server in WM
D-Bus client
flamewmctl status/settings/display/shortcut minimum
```

Migrate Settings to Control client.

Remove local effective `ConfigStore`, `DisplayManager`, `ShortcutRegistry` from Settings.

Gate:

```text
kill/restart Settings does not change WM authority
settings readback comes from WM
D-Bus round-trip tests pass
```

## Phase 4 — Config/shortcuts migration + upstream restoration

Move current Runtime config behavior into SettingsService.

Move hotkey X-grab implementation to `engine/icewm/shortcut_adapter.*`.

Add one root-key hook to manager.

Then restore exact upstream bytes for:

```text
src/wmconfig.cc
src/wmconfig.h
src/yconfig.cc
src/yconfig.h
src/yprefs.cc
src/bindkey.cc
src/bindkey.h
src/wmkey.cc
src/wmkey.h
```

Gate must compare these files byte-for-byte to the selected upstream base.

## Phase 5 — Display migration

Split current display model from native XRandR types.

Move XRandR transaction implementation to DisplayAdapter.

Manager retains only topology invalidation hook.

Restore any product display logic from `wmmgr.cc`.

Gate:

```text
fresh query
unsupported mode rejection
Keep
Revert
timeout
Settings death
hotplug generation invalidation
```

## Phase 6 — Session migration

Create Flame session wrapper/supervisor.

Use upstream `icewm-session --icewm=...` support.

Prove bounded desktop restart behavior in new component.

Restore `src/icesm.cc` exactly.

## Phase 7 — Workspace migration

Keep pure `WorkspaceTransaction`/`TwoRowTopology`.

Implement native apply in WorkspaceAdapter using public manager APIs + narrow `EngineAccess` where required.

Move directional shortcut actions through ShortcutService -> WorkspaceService.

Restore large workspace/hotkey blocks from `wmmgr.cc` and leave only final hooks/friend.

## Phase 8 — Snap migration

Move all persistent SnapState/overlay/dwell bookkeeping out of frame class into SnapService.

Replace current large `wmframe.*` and `movesize.cc` modifications with tiny lifecycle callbacks.

Restore `wmframe.h` exactly.

Keep only G-01 and event hook(s) in `wmframe.cc`.

## Phase 9 — New Flame panel view

This phase removes the most troublesome taskbar coupling.

Implement Flame-owned in-process panel surfaces from current pure PanelManager/OutputPanel models.

Use:

```text
WindowService snapshot
WorkspaceService snapshot
ApplicationService identity
PanelService pin/order state
existing YXTray as one primary tray host
```

Set Flame default `ShowTaskBar=0`.

Migrate V7/V8 task behavior to Flame task view.

After native proof, restore exactly:

```text
src/wmtaskbar.cc
src/wmtaskbar.h
src/atasks.cc
src/atasks.h
src/yxtray.cc
```

## Phase 10 — Chrome reduction

Replace direct Runtime/ScaleManager access in `decorate.cc` and `wmtitle.cc` with one Bridge provider API.

Restore every surrounding original line.

Only minimal chrome hooks remain.

## Phase 11 — Build-file reduction

Move Flame source/install lists to Flame-owned build fragments.

Reduce upstream build files to one-line/very small inclusions.

## Phase 12 — Architecture enforcement

Turn on layer checker and upstream diff budget as required CI gates.

From this point, new feature PRs that directly modify an unapproved IceWM file fail automatically.

## Phase 13 — Upstream-update rehearsal

Before calling Platform v1 complete:

- rehearse update against latest upstream IceWM candidate branch;
- measure textual conflicts;
- require conflicts to be limited to registered hook/build locations;
- compile Bridge against new source;
- document only actual adapter drift.

---

# 37. Migration order rationale

Order is deliberate:

```text
Control/API first
    fixes current Settings process blocker

Config/hotkeys/display next
    removes many unnecessary direct IceWM files early

Session next
    restores one of the largest single upstream diffs

Workspace/snap
    moves state-heavy algorithms out of core engine files

Panel/taskbar
    removes the largest long-term shared-file collision surface

Chrome last
    unavoidable native rendering hooks become tiny after shared services exist
```

Do not begin with mass file moves or naming cleanup.

---

# 38. Per-file migration algorithm

Every upstream file migration follows exactly:

```text
1. Diff current file against exact upstream base.
2. Classify every hunk RESTORE_UPSTREAM / KEEP_GENERIC_FIX / MINIMAL_ADAPTER_HOOK.
3. Identify Platform/Bridge owner for each RESTORE behavior.
4. Implement/verify replacement outside upstream file.
5. Run focused semantic tests against replacement.
6. Restore exact upstream lines for replaced hunk.
7. Reapply only approved hook lines around exact current upstream anchor.
8. Run byte/diff verifier.
9. Run focused native gate.
10. Update hook manifest.
```

Never restore a whole file blindly if it contains another independently verified generic fix or hook.

Never keep a feature hunk merely because it already works.

The goal is intentionally to delete working inline integration after its adapter replacement is proven.

---

# 39. Build and packaging model

Final binaries:

```text
flamewm                 IceWM engine + IceWM Bridge + Platform + Shell
flamewm-settings        Flame Control client + Flame UI
flamewm-desktop         desktop helper + Flame Control client
flamewm-session         wrapper/supervisor around upstream icewm-session
flamewmctl              CLI Flame Control client
```

Internal static linkage avoids public library ABI overhead and version skew.

All installed first-party binaries must report the same FlameWM product major/minor compatibility version.

D-Bus API major version remains independently explicit as `FlameWM1`.

---

# 40. Lifecycle model

## WM startup

```text
IceWM X application initializes
-> preferences/theme initialize
-> YWindowManager constructed
-> icon/pixmap authorities ready
-> FB-BOOT-01 attach
-> IceWM Bridge creates adapters
-> PlatformHost creates services
-> Flame Control claims session bus name
-> system integrations subscribe
-> Flame Shell creates per-output panels
-> normal WM loop
```

## WM shutdown

```text
FB-BOOT-01 detach
-> reject new control requests
-> mark Platform stopping / bump generation
-> revert pending display transaction
-> close Start/popovers/overlays
-> destroy panel surfaces + detach tray
-> unsubscribe service watches/timers
-> release D-Bus name
-> destroy services
-> destroy adapters
-> original IceWM manager/X teardown continues
```

No callback may mutate X after generation mismatch/stopping begins.

---

# 41. Concurrency/event ownership

WM process:

```text
single IceWM X/main-loop mutation authority
```

D-Bus/Pulse/NetworkManager watchers integrate into that loop.

No blocking DBus calls in X handlers.

No blocking subprocess calls in X handlers.

No background thread may mutate IceWM objects directly.

If future heavy work uses a worker thread, completion posts immutable result back to owner loop before any X/engine mutation.

---

# 42. Performance constraints

Platform abstraction must not compromise FlameWM's low-overhead goal.

Rules:

- internal ports are direct C++ calls;
- no IPC in pointer-motion/render paths;
- no reflection/service locator on hot path;
- no string-based generic events;
- snapshots allocate only at bounded state refresh points, not every motion event;
- window invalidations coalesce;
- icon/image caches remain bounded;
- Settings process exits when closed;
- D-Bus/Pulse are event-driven;
- no recurring filesystem or service polling;
- no second permanent WM state registry.

Architecture acceptance should measure PSS/idle CPU before and after migration. The architecture is intended to reduce code coupling, not add resident daemons.

---

# 43. Testing architecture

## Layer 1 — API compile tests

Prove headers are engine-neutral.

## Layer 2 — Platform service tests

Use fake ports.

Tests cover policy without X:

```text
workspace transforms
settings rollback
shortcut collision/rollback
display transaction state machine
panel pin/order model
Start one-open invariant
stale revision/generation rejection
```

## Layer 3 — IceWM Bridge tests

Source/compile tests plus nested X where needed:

```text
WindowRef mapping
workspace native apply
hotkey X grabs
RandR native apply/revert
work-area reservations
engine event hook correctness
```

## Layer 4 — Flame Control tests

Temporary session D-Bus:

```text
server/client round trip
stale revision error
owner loss
signals
malformed requests
capabilities
```

## Layer 5 — Shell/UI tests

Use Platform fake snapshots where possible.

## Layer 6 — Xephyr/native smoke

One installed/staged session:

```text
window manage
snap
workspace add/remove
panel/task pin/reorder
Start
Settings live apply
display transaction when capability exists
desktop
shutdown
```

## Layer 7 — Upstream cleanliness gate

Compare against exact upstream base.

No unregistered diff.

---

# 44. Architecture test cases that prevent regression to current coupling

Required CI checks:

1. `src/flamewm-settings/**` contains zero `#include` of `wmmgr.h`, `wmframe.h`, `wmkey.h`, `wmtaskbar.h`, `atasks.h`.
2. `src/flamewm/platform/**` contains zero X11 headers.
3. Raw XRandR types appear only under `engine/icewm/**` and upstream IceWM.
4. `Runtime::instance()` is absent from upper feature/UI code after migration.
5. No `gKeyFlame` symbols exist.
6. `ShowTaskBar=0` in Flame default session config once Flame panel is native.
7. Upstream `atasks.*`, `wmtaskbar.*`, `yxtray.cc`, config/key/session files match exact base.
8. Product hook budget remains under configured threshold.
9. Every D-Bus method has client/server round-trip proof.
10. Settings process death cannot cancel WM authority or strand display/hotkey transactions.

---

# 45. Agent-development workflow after Platform v1

Normal future feature request:

```text
1. Read feature contract.
2. Find existing Platform service.
3. If service already exposes required semantic operation:
     implement feature entirely above Platform.
     do not read IceWM source.
4. If Platform lacks operation:
     extend engine-neutral service contract.
5. Only if IceWM Bridge cannot implement the new port using existing public/access seam:
     perform targeted IceWM source audit.
6. New IceWM hook requires architecture-review entry.
```

This is the desired productivity change.

Example after migration:

```text
Request: add Taskbar auto-hide delay setting.

Expected touch:
  api/settings field
  PanelService policy
  Flame panel view
  Settings Taskbar page
  tests

IceWM touch:
  none
```

Another:

```text
Request: add per-output panel size.

Settings -> Flame Control -> SettingsService -> PanelService

IceWM knowledge required by feature agent:
  none
```

---

# 46. Explicit anti-patterns

Do not permit these after migration:

```text
feature code includes wmmgr.h
Settings owns a second effective DisplayManager
Settings writes config then waits for WM to rediscover it
product hotkeys represented by new gKey globals
raw RRMode exposed to page code
Flame fields added directly to YFrameWindow for transient product state
large feature functions inserted into wmmgr.cc
Flame taskbar behavior patched into TaskPane
product session supervision added to icesm.cc
new feature creates another polling timer instead of subscribing
stringly-typed EventBus
service locator used everywhere
custom generic plugin system
```

---

# 47. What should remain intentionally coupled to IceWM

Not every dependency should be abstracted away.

The Bridge intentionally depends on:

- exact IceWM frame lifecycle;
- manager workspace internals needed for atomic indexed transform;
- IceWM work-area lifecycle;
- native X event loop;
- decoration rendering callback points;
- X11 client/window mechanics;
- `YXTray` implementation;
- native lightweight UI primitives.

Trying to hide these behind a fully generic multi-WM abstraction would add complexity with no current product value.

The architecture is engine-neutral **above the Bridge**, not a promise that FlameWM can swap IceWM tomorrow without adapter work.

---

# 48. Decision on current pure modules

Do not rewrite working pure modules merely to fit new naming.

Reuse first.

Migration policy:

```text
existing pure implementation correct
-> wrap behind new service contract
-> move/rename only in later cleanup
```

Examples:

```text
WorkspaceTransaction: keep
TwoRowTopology: keep
SnapController math: keep
PanelManager geometry: keep
AppIdentity logic: keep
ConfigStore parsing/atomic persistence: keep
DBus/Pulse/NM/MPRIS backend work: keep where correct
```

Architecture work should delete redundant bridges, not recreate solved logic.

---

# 49. Current-state specific repairs before further feature expansion

Based on current source, these are architecture corrections rather than normal bugs:

1. Stop extending Settings local effective state; build Flame Control first.
2. Stop adding direct Runtime dependencies to `wmconfig/yconfig/yprefs`; plan their restoration.
3. Stop adding `gKeyFlame*`; move the product keymap to ShortcutService/ShortcutAdapter.
4. Move XRandR native types out of the public/pure display model.
5. Move snap state out of `YFrameWindow` before adding more snap behaviors.
6. Do not invest further in `PinnedLauncherButton : TaskButton`; move toward independent Flame task view.
7. Do not invest further in multi-output extensions to upstream `TaskBar`; create Flame panel surfaces instead.
8. Move `flamewm-desktop` supervision out of `icesm.cc`.
9. Convert current direct upper includes of IceWM internals into Bridge/UI backend dependencies.
10. Make the upstream-diff checker a required gate before the next broad feature cycle.

---

# 50. Production acceptance criteria for Platform v1

Platform v1 is complete only when:

```text
ARCHITECTURE
[ ] api layer has zero IceWM/X11 dependency
[ ] platform layer has zero IceWM/X11 dependency
[ ] Settings uses Control client only for effective WM state
[ ] Desktop uses Control client for WM/workspace/settings actions
[ ] only engine/icewm knows WM internals

UPSTREAM RESTORATION
[ ] atasks.cc/h exact upstream
[ ] wmtaskbar.cc/h exact upstream or one separately approved unavoidable hook
[ ] yxtray.cc exact upstream
[ ] bindkey.cc/h exact upstream
[ ] wmkey.cc/h exact upstream
[ ] wmconfig.cc/h exact upstream
[ ] yconfig.cc/h exact upstream
[ ] yprefs.cc exact upstream
[ ] icesm.cc exact upstream
[ ] wmframe.h exact upstream
[ ] current large wmmgr/wmframe/movesize feature blocks removed

HOOK BUDGET
[ ] <=8 IceWM source files with Flame hook regions
[ ] <=80 product-specific executable hook lines
[ ] every hook registered
[ ] no unregistered upstream diff

CONTROL
[ ] com.arkflame.FlameWM1 owned by running WM
[ ] version/capabilities work
[ ] Settings round trips work
[ ] shortcut rollback works
[ ] display Keep/Revert/timeout works
[ ] workspace operations work
[ ] panel settings work
[ ] flamewmctl works

SHELL
[ ] Flame panel native with ShowTaskBar=0
[ ] one panel per output
[ ] one XEmbed tray owner
[ ] V7/V8 task semantics without TaskPane modification
[ ] Start one-open invariant

BUILD/TEST
[ ] CMake green
[ ] CTest green
[ ] required Autotools path green while retained
[ ] architecture checker green
[ ] upstream diff checker green
[ ] nested-X smoke green
[ ] no new idle polling
[ ] resource delta measured
```

---

# 51. Planner execution program

The coding migration should be split into independently verifiable handoffs, not one giant implementation request.

Recommended program:

```text
H1  Platform API + fake ports + architecture checker
H2  IceWM Bridge bootstrap + PlatformHost
H3  Flame Control D-Bus server/client + flamewmctl minimum
H4  Settings authority migration
H5  Shortcut adapter migration + restore key files
H6  Display adapter migration + restore display integrations
H7  Session wrapper + restore icesm.cc
H8  Workspace adapter + reduce wmmgr.cc
H9  SnapService migration + restore wmframe.h/reduce movesize/wmframe
H10 Flame panel/task view + restore wmtaskbar/atasks/yxtray
H11 Chrome provider reduction
H12 Build-file isolation + full upstream restoration audit
H13 Upstream IceWM merge rehearsal
```

Each handoff must reduce or hold constant the number of modified upstream lines. No handoff after H2 may increase permanent upstream-hook count without an explicit architecture amendment.

---

# JOBS

The following are the first implementation-ready planner jobs for the migration program.

## JOB PLATFORM-AUDIT-01 — freeze exact current divergence

```text
MODE=AUDIT
READ=current repository + exact upstream base
TOUCH=none

PRODUCES:
  exact 30-file current divergence ledger
  per-hunk RESTORE/GENERIC/HOOK classification
  current direct-dependency graph

PASS:
  every upstream diff hunk classified
  no source assumptions from old reports used without current readback
```

## JOB PLATFORM-API-01 — create engine-neutral contracts

```text
MODE=IMPLEMENT
TOUCH=src/flamewm/api/** + tests only

PRODUCES:
  ids/errors/geometry/snapshots/ports

PASS:
  standard C++11 only
  zero X11/IceWM includes
  fake-port tests compile
```

## JOB PLATFORM-HOST-01 — create Host and service wiring

```text
MODE=IMPLEMENT
REQUIRES=PLATFORM-API-01
TOUCH=src/flamewm/platform/** + tests

PASS:
  constructor-injected ports
  no global Runtime dependency
  lifecycle/generation tests
```

## JOB BRIDGE-BOOT-01 — attach Bridge with minimal wmapp diff

```text
MODE=IMPLEMENT
REQUIRES=PLATFORM-HOST-01
TOUCH=src/flamewm/engine/icewm/**, src/wmapp.cc, src/wmapp.h

TASK:
  create bridge/access skeleton
  migrate current Runtime ownership
  restore wmapp.h to upstream if final static Bridge attachment works

PASS:
  wmapp.cc diff contains only approved attach/detach hook
```

## JOB CONTROL-01 — D-Bus protocol/server/client

```text
MODE=IMPLEMENT
REQUIRES=PLATFORM-HOST-01
TOUCH=src/flamewm/control/**, tests, build fragment

PASS:
  temporary session-bus round trip
  version/capabilities
  typed errors
```

## JOB SETTINGS-MIGRATE-01

```text
MODE=IMPLEMENT
REQUIRES=CONTROL-01
TOUCH=src/flamewm-settings/**

TASK:
  remove local effective config/display/shortcut authorities
  use control client

PASS:
  Settings process restart does not alter authority
```

Subsequent jobs follow the H5-H13 program above.

---

# VERIFICATION + STOP

## Required stop conditions

Stop affected migration lane if:

1. replacement requires a second authoritative window/workspace/display registry;
2. adapter operation cannot be made rollback-safe where current behavior is transactional;
3. restoring an IceWM hunk would remove unrelated user work not represented in exact current diff classification;
4. an upper layer needs a raw IceWM/X11 type to satisfy its contract;
5. new permanent hook would carry product algorithm/state rather than delegate;
6. Settings would need synchronous/blocking calls on WM X owner thread;
7. panel implementation would require a second XEmbed tray selection owner;
8. full build failure is source-caused and cannot be traced to one bounded migration change;
9. upstream-diff budget increases without explicit architecture amendment;
10. a migration rewrites a correct pure model instead of adapting it without source-proven necessity.

## Required verification after each restoration

```text
semantic replacement test PASS
-> exact upstream diff readback
-> architecture layer check
-> focused native build/test
-> no unregistered upstream changes
```

## Final migration proof

Final report must include:

```text
UPSTREAM BASE SHA
original modified files before migration = 30
modified files after migration
product hook executable line count
remaining generic patches
exact restored files
exact permanent hooks
architecture checker result
CMake/CTest/Autotools result
D-Bus control round-trip result
nested-X result
resource delta
upstream merge rehearsal result
```

---

# 52. Research basis

Architecture choices are consistent with established modernization/interface practices:

- **Branch by Abstraction**: introduce an abstraction around a deeply coupled legacy component, redirect clients through it, then replace/move implementation incrementally rather than performing a disruptive rewrite.
- **D-Bus API design guidance**: use reverse-domain versioned names, semantic methods, properties/signals, asynchronous behavior and minimized round trips for desktop IPC.
- **D-Bus specification**: versioned reversed-DNS bus/interface/object namespaces allow incompatible API generations to coexist.
- **Git rerere**: useful for replaying recurring conflict resolutions after the conflict surface has been reduced to a small stable hook set.

The important application to FlameWM is not copying another project's framework. It is establishing one explicit product/engine seam, then enforcing it.

---

# 53. Final architecture decision

The final direction is:

> **FlameWM Platform is the stable product API. IceWM Bridge is the only compatibility layer that knows IceWM internals. Flame Shell, Settings and Desktop depend on FlameWM concepts, not IceWM classes. Existing direct Flame modifications inside IceWM are migrated outward, then their original IceWM lines are restored.**

The best long-term outcome is not “better organized Flame code inside IceWM.”

It is:

```text
IceWM mostly looks like IceWM again.

A small Bridge makes it useful to FlameWM.

FlameWM becomes simple because all difficult engine semantics are solved once below it.
```

That is the architecture that minimizes future agent work, merge conflicts, upstream drift and repeated source archaeology while preserving IceWM's mature behavior and FlameWM's product freedom.
