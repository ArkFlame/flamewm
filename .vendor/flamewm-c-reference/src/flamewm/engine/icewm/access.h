#ifndef FLAMEWM_ENGINE_ICEWM_ACCESS_H
#define FLAMEWM_ENGINE_ICEWM_ACCESS_H

#include <cstdint>
#include <functional>

class YWindowManager;
class YWMApp;

namespace flamewm {
namespace engine {
namespace icewm {

// EngineAccess: minimal, documented facade for IceWM private state that
// FlameWM needs to read. Attach context is stored directly (manager/app +
// generation) and does NOT depend on Bridge::isAttached() — avoids circular
// gate where Bootstrap would query manager() that only becomes valid after
// Bridge attach. Generation monotonically increases on attach/detach and
// invalidates stale callbacks.
//
// Design note: real implementations live in access.cc and may include
// IceWM/X11 headers there (not in this header) to avoid leaking Xlib into
// flamewm/** consumers. Keeping this header X11-free preserves C++11
// build cleanliness for -fsyntax-only checks and for Autotools C++11 floor.

class EngineAccess {
public:
    // Attach/detach context — called only by Bootstrap::attachIfProduct /
    // Bootstrap::detach on the Flame-only build path. Detach invalidates
    // generation so stale poll/timer/D-Bus callbacks can be ignored.
    static void attachContext(YWindowManager* manager, YWMApp* app);
    static void detachContext();

    // Monotonic generation: incremented on every attachContext/detachContext.
    // Callbacks must capture generation and check before mutating.
    static uint64_t generation();

    // Typed accessors — return stored pointers directly, never via Bridge.
    static YWindowManager* managerTyped();
    static YWMApp* appTyped();

    // Global YWindowManager singleton (extern YWindowManager* manager).
    // Returns stored manager as void* (X11-free header). Returns nullptr
    // when no context has been attached. Does NOT check Bridge::isAttached().
    static void* manager();

    // Global YWMApp singleton (extern YWMApp* wmapp) as void*.
    static void* app();

    // Iterate each managed YFrameWindow* as opaque void*. Callback receives
    // the raw pointer; caller must not store it beyond the call. No-op when
    // no manager context. Iteration order is IceWM creation order (fCreationOrder).
    static void forEachWindow(std::function<void(void*)> fn);

    // Focused window handle, or 0 when no context / none.
    static uint64_t focusedWindowId();

    // Active workspace index, or -1 when no context.
    static int activeWorkspace();

    // Count of workspaces (fWorkAreaWorkspaceCount or equivalent).
    // Returns 1 when no context (single-workspace neutral default).
    static int workspaceCount();

private:
    EngineAccess() = delete;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm

#endif // FLAMEWM_ENGINE_ICEWM_ACCESS_H
