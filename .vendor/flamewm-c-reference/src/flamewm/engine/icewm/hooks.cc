#include "flamewm/engine/icewm/hooks.h"
#include "flamewm/engine/icewm/bridge.h"
#include "flamewm/engine/icewm/bootstrap.h"

namespace flamewm {
namespace engine {
namespace icewm {

void Hooks::windowInvalidated() {
    if (!Bridge::instance().isAttached())
        return;
}

void Hooks::windowMembershipChanged() {
    if (!Bridge::instance().isAttached())
        return;
}

void Hooks::windowRemoved(uint64_t id) {
    if (!Bridge::instance().isAttached())
        return;
    (void)id;
}

bool Hooks::rootKey(int keyCode, unsigned state) {
    if (!Bridge::instance().isAttached())
        return false;
    return Bootstrap::toggleStart(keyCode, state);
}

void Hooks::moveBegin(uint64_t winId, int x, int y) {
    if (!Bridge::instance().isAttached())
        return;
    (void)winId;
    (void)x;
    (void)y;
}

void Hooks::moveMotion(int x, int y) {
    if (!Bridge::instance().isAttached())
        return;
    (void)x;
    (void)y;
}

void Hooks::moveEnd() {
    if (!Bridge::instance().isAttached())
        return;
}

void Hooks::moveCancel() {
    if (!Bridge::instance().isAttached())
        return;
}

void Hooks::workspaceChanged() {
    if (!Bridge::instance().isAttached())
        return;
}

void Hooks::topologyChanged() {
    if (!Bridge::instance().isAttached())
        return;
}

void Hooks::reserveWorkAreas() {
    if (!Bridge::instance().isAttached())
        return;
}

} // namespace icewm
} // namespace engine
} // namespace flamewm
