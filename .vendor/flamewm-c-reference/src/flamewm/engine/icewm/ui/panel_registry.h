#ifndef FLAMEWM_ENGINE_ICEWM_UI_PANEL_REGISTRY_H
#define FLAMEWM_ENGINE_ICEWM_UI_PANEL_REGISTRY_H

#include "flamewm/api/ids.h"

#include <map>
#include <vector>

namespace flamewm {
namespace engine {
namespace icewm {
namespace ui {

// PanelRegistry — engine-only OutputId -> native panel surface.
// Native surface is an opaque YWindow* (stored as void*) so this header
// remains X11-free and C++11-compatible. One registry per process; engine
// owns lifetime, panels register/unregister on creation/destruction.
// No X calls are made here; reparenting is performed by TrayAdapter via
// the stored pointer under FLAMEWM_HAS_X11.
class PanelRegistry {
public:
    PanelRegistry();
    ~PanelRegistry();

    PanelRegistry(const PanelRegistry&) = delete;
    PanelRegistry& operator=(const PanelRegistry&) = delete;

    void registerPanel(const api::OutputId& output, void* nativePanel);
    void unregisterPanel(const api::OutputId& output);
    void unregisterPanel(const api::OutputId& output, void* nativePanel);
    void* panelFor(const api::OutputId& output) const;
    bool hasPanel(const api::OutputId& output) const;
    std::vector<api::OutputId> outputs() const;
    size_t size() const;
    void clear();

    // Process-wide singleton accessor. Engine may also instantiate its own
    // instance, but the singleton is the canonical registry for TrayAdapter
    // reparent lookups when no explicit instance is wired.
    static PanelRegistry& instance();

private:
    std::map<api::OutputId, void*> panels_;
};

} // namespace ui
} // namespace icewm
} // namespace engine
} // namespace flamewm

#endif // FLAMEWM_ENGINE_ICEWM_UI_PANEL_REGISTRY_H
