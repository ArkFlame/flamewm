#include "flamewm/engine/icewm/access.h"

namespace flamewm {
namespace engine {
namespace icewm {

namespace {
struct AccessState {
    YWindowManager* manager;
    YWMApp* app;
    uint64_t generation;
    AccessState() : manager(nullptr), app(nullptr), generation(0) {}
};
static AccessState g_access;
}

void EngineAccess::attachContext(YWindowManager* manager, YWMApp* app) {
    g_access.manager = manager;
    g_access.app = app;
    ++g_access.generation;
}

void EngineAccess::detachContext() {
    g_access.manager = nullptr;
    g_access.app = nullptr;
    ++g_access.generation;
}

uint64_t EngineAccess::generation() {
    return g_access.generation;
}

YWindowManager* EngineAccess::managerTyped() {
    return g_access.manager;
}

YWMApp* EngineAccess::appTyped() {
    return g_access.app;
}

void* EngineAccess::manager() {
    return static_cast<void*>(g_access.manager);
}

void* EngineAccess::app() {
    return static_cast<void*>(g_access.app);
}

void EngineAccess::forEachWindow(std::function<void(void*)> fn) {
    if (g_access.manager == nullptr)
        return;
    if (!fn)
        return;
    // Real impl: iterate manager->focusedIterator / fCreationOrder via
    // friend or accessor. Stub is empty until private access is granted.
}

uint64_t EngineAccess::focusedWindowId() {
    if (g_access.manager == nullptr)
        return 0;
    return 0;
}

int EngineAccess::activeWorkspace() {
    if (g_access.manager == nullptr)
        return -1;
    return -1;
}

int EngineAccess::workspaceCount() {
    if (g_access.manager == nullptr)
        return 1;
    return 1;
}

} // namespace icewm
} // namespace engine
} // namespace flamewm
