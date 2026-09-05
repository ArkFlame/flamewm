#include "flamewm/engine/icewm/ui/panel_registry.h"

namespace flamewm {
namespace engine {
namespace icewm {
namespace ui {

PanelRegistry::PanelRegistry() {}
PanelRegistry::~PanelRegistry() {}

void PanelRegistry::registerPanel(const api::OutputId& output, void* nativePanel) {
    if (!output.valid() || nativePanel == nullptr)
        return;
    panels_[output] = nativePanel;
}

void PanelRegistry::unregisterPanel(const api::OutputId& output) {
    if (!output.valid())
        return;
    std::map<api::OutputId, void*>::iterator it = panels_.find(output);
    if (it != panels_.end())
        panels_.erase(it);
}

void PanelRegistry::unregisterPanel(const api::OutputId& output, void* nativePanel) {
    if (!output.valid())
        return;
    std::map<api::OutputId, void*>::iterator it = panels_.find(output);
    if (it != panels_.end() && it->second == nativePanel)
        panels_.erase(it);
}

void* PanelRegistry::panelFor(const api::OutputId& output) const {
    if (!output.valid())
        return nullptr;
    std::map<api::OutputId, void*>::const_iterator it = panels_.find(output);
    if (it == panels_.end())
        return nullptr;
    return it->second;
}

bool PanelRegistry::hasPanel(const api::OutputId& output) const {
    if (!output.valid())
        return false;
    return panels_.find(output) != panels_.end();
}

std::vector<api::OutputId> PanelRegistry::outputs() const {
    std::vector<api::OutputId> out;
    out.reserve(panels_.size());
    for (std::map<api::OutputId, void*>::const_iterator it = panels_.begin();
         it != panels_.end(); ++it)
        out.push_back(it->first);
    return out;
}

size_t PanelRegistry::size() const {
    return panels_.size();
}

void PanelRegistry::clear() {
    panels_.clear();
}

PanelRegistry& PanelRegistry::instance() {
    static PanelRegistry s;
    return s;
}

} // namespace ui
} // namespace icewm
} // namespace engine
} // namespace flamewm
