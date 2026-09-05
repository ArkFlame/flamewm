# IceWM Private Field Dependencies — FlameWM Bridge (C-BRIDGE-01/02)

> Every entry here is a private IceWM field or method that FlameWM's `src/flamewm/engine/icewm/` needs to read or trigger.
> Format: `field | reason | alternative public API if any`.
> No upstream `src/*.cc` file has been edited yet; markers `FLAMEWM-BRIDGE-HOOK-BEGIN` / `FLAMEWM-BRIDGE-HOOK-END` do not exist in upstream files — they are design intent only.

## YWindowManager (`src/wmmgr.h`)

| field | reason | alternative public API if any |
|-------|--------|-------------------------------|
| `YWindowManager::fLayers[WinLayerCount]` (`YLayeredList`) | Enumerate all managed frames per layer; `EngineAccess::forEachWindow` | None — `topLayer()`/`bottomLayer()`/`top()`/`bottom()` expose only extremes |
| `YWindowManager::fCreationOrder` (`YCreatedList`) | Creation-order iteration for window model snapshot | None |
| `YWindowManager::fFocusedOrder` (`YFocusedList`) | Focus-order / stacking-order snapshot | `focusedIterator()` / `focusedReverseIterator()` / `focusedCount()` are public — prefer those |
| `YWindowManager::fFocusWin` (`YFrameWindow*`) | `EngineAccess::focusedWindowId()` | `getFocus()` is public — use that |
| `YWindowManager::fActiveWorkspace` (`int`) | `EngineAccess::activeWorkspace()` / `Hooks::workspaceChanged` | `getActiveWorkspace()` if present (check — public accessor exists in some IceWM revisions); currently workspace access is via `fActiveWorkspace` with no public getter in this tree |
| `YWindowManager::fLastWorkspace` (`int`) | Detect workspace switch direction / animation | None |
| `YWindowManager::fWorkArea` (`WorkAreaRect**`) | `Hooks::reserveWorkAreas` — read and reserve Flame panel struts | `getWorkArea(int*,int*,int*,int*,int)` is public but per-workspace/per-screen query; no bulk getter |
| `YWindowManager::fWorkAreaWorkspaceCount` / `fWorkAreaScreenCount` (`int`) | Dimension `fWorkArea` array | None |
| `YWindowManager::fWorkAreaLock` / `fWorkAreaUpdate` (`int`) | Coalesce work-area recomputes (`YWorkAreaLock` idiom) | `lockWorkArea()` / `unlockWorkArea()` / `requestWorkAreaUpdate()` are public — use those |
| `YWindowManager::fActiveWindow` (`Window`) | EWMH `_NET_ACTIVE_WINDOW` tracking | `netActiveWindow()` is public — use that |
| `YWindowManager::fRestackLock` / `fRestackUpdate` (`int`) | Batch restacks during bulk operations | `lockRestack()` / `unlockRestack()` are public — use those |
| `YWindowManager::fColormapWindow` (`YFrameWindow*`) | Colormap correctness when Flame changes focus | `colormapWindow()` is public — use that |
| `YWindowManager::fLayout` (`DesktopLayout`) | Pager layout (rows/cols/orient) for Flame topology | `layout()` is public — use that |
| `YWindowManager::fDockApp` (`DockApp*`) | Dock integration / tray ownership | None public |
| `YWindowManager::updateWorkArea()` (`void`) | Trigger recompute after Flame reserves work areas | Public indirectly via `lockWorkArea`/`unlockWorkArea` cycle; no direct public `updateWorkArea()` |
| `YWindowManager::restackWindows()` (`void`) | Restack after Flame reorder | `restackWindows()` is public — use that |
| `YWindowManager::setFocus(YFrameWindow*,bool,bool)` | Flame-initiated focus change | Public — use directly |
| `::manager` (`extern YWindowManager*`) | `EngineAccess::manager()` | None — global is the API; null-check required |
| `YWindowManager::fWmState` (`WMState`) | Guard hooks during startup/shutdown | `wmState()` / `isRunning()` / `notRunning()` / `shuttingDown()` are public — use those |

## YDesktop / YWindow (`src/ywindow.h`)

| field | reason | alternative public API if any |
|-------|--------|-------------------------------|
| `YDesktop::xiInfo` (`YArray<DesktopScreenInfo>`) | `Hooks::topologyChanged` — RandR/Xinerama output list | `getScreenCount()` / `getScreenInfo(int)` / `getScreenGeometry(int)` / `getScreenForRect(...)` are public — use those |
| `YWindow::fHandle` (`Window`) | X window handle for port calls (`setOuterGeometry`, etc.) | `handle()` is public (creates if needed) — use that; no private access needed |
| `YWindow::fX,fY,fWidth,fHeight` | Geometry snapshot for `WindowSnapshot` | `x()` / `y()` / `width()` / `height()` / `geometry()` are public — use those |
| `YWindow::fParent` (`YWindow*`) | Walk parent chain for frame lookup | `parent()` is public — use that |

## YFrameWindow (`src/wmframe.h`)

| field | reason | alternative public API if any |
|-------|--------|-------------------------------|
| `YFrameWindow::fClient` / `YFrameWindow::clients()` (`ClientData`/`YFrameClient*`) | Map `YFrameWindow` → `Window` id and title/icon for `WindowSnapshot` | `getClient()` / `client()` / `clients()` accessors are public (verify per revision) — prefer public |
| `YFrameWindow::fWorkspace` / `getWorkspace()` (`int`) | Window membership (`Hooks::windowMembershipChanged`) | `getWorkspace()` is typically public — use that |
| `YFrameWindow::fState` / `getState()` (`long`/`FrameState`) | Minimized/maximized/fullscreen for snapshot | `getState()` / `isMinimized()` / `isMaximized*()` are public — use those |
| `YFrameWindow::fLayer` / `getLayer()` | Layer for work-area exclusion | `getLayer()` is typically public |
| `YFrameWindow::fXFrame` geometry / `geometry()` | Outer geometry for `setOuterGeometry` / snap overlay | `geometry()` / `x()` / `y()` are public via `YWindow` |

## YFrameClient (`src/wmclient.h`)

| field | reason | alternative public API if any |
|-------|--------|-------------------------------|
| `YFrameClient::fFrame` (`YFrameWindow*`) | Reverse lookup `Window` → `YFrameWindow` for `moveBegin` | `getFrame()` / `obtainFrame()` are public — use those |
| `YFrameClient::fWindow` (`Window`) | X client window id for `windowRemoved(uint64_t)` | Public via `handle()` (`YWindow::handle()`) — use that |
| `ClassHint` (`res_name`/`res_class`) | `WindowSnapshot.appId` / task identity | `classHint()` is public via `ClientData` — use that |

## X11 / Atom / EWMH globals (`src/wmmgr.h`, `src/ywindow.h`)

| field | reason | alternative public API if any |
|-------|--------|-------------------------------|
| `Atom _XA_NET_*` / `_XA_WIN_*` | Property handling for `reserveWorkAreas` / client list | Globals are `extern` — no access issue |
| `YExtension xrandr / damage / composite` | RandR topology & damage tracking | `extern` globals — no access issue |

## Notes

- **Friend vs accessor**: Prefer public accessors listed in the third column. Where third column says "None", FlameWM will need either a `friend class flamewm::engine::icewm::EngineAccess` declaration or a new public accessor added to IceWM. The Bridge itself must not `friend` IceWM internals; `EngineAccess` is the only class that may be friended.
- **Headers**: `access.cc` (not `access.h`) is the only file that may include `wmmgr.h` / `wmclient.h` / `wmframe.h` / `ywindow.h` / `<X11/Xlib.h>` to keep `src/flamewm/**` consumers X11-free.
- **Markers**: `FLAMEWM-BRIDGE-HOOK-BEGIN` / `FLAMEWM-BRIDGE-HOOK-END` are not present in any `src/*.cc` or `src/*.h` upstream file. Verify with `grep -rn FLAMEWM-BRIDGE-HOOK src/` — expected no hits.
- **Build**: `Bridge::attach` is only effective under `-DFLAMEWM_PRODUCT_BUILD`; otherwise it returns `false` and `isAttached()` stays `false`.
