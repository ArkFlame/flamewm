#ifndef FLAMEWM_ENGINE_ICEWM_HOOKS_H
#define FLAMEWM_ENGINE_ICEWM_HOOKS_H

#include <cstdint>

namespace flamewm {
namespace engine {
namespace icewm {

// Hooks: stateless trampoline called from IceWM event sites. Each method
// checks Bridge::isAttached() and is a no-op when detached. No IceWM/X11
// headers are included here; call sites will pass primitive values only.
//
// Design: upstream src/*.cc files will eventually contain guarded call sites:
//   // FLAMEWM-BRIDGE-HOOK-BEGIN
//   Hooks::windowInvalidated();
//   // FLAMEWM-BRIDGE-HOOK-END
// Those markers are NOT yet present in upstream files (C-BRIDGE-02). They
// are documented here and in COMPATIBILITY.md as the integration plan.

struct Hooks {
    static void windowInvalidated();
    static void windowMembershipChanged();
    static void windowRemoved(uint64_t id);
    static bool rootKey(int keyCode, unsigned state);
    static void moveBegin(uint64_t winId, int x, int y);
    static void moveMotion(int x, int y);
    static void moveEnd();
    static void moveCancel();
    static void workspaceChanged();
    static void topologyChanged();
    static void reserveWorkAreas();
};

} // namespace icewm
} // namespace engine
} // namespace flamewm

#endif // FLAMEWM_ENGINE_ICEWM_HOOKS_H
