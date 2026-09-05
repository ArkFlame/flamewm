#ifndef FLAMEWM_API_CAPABILITIES_H
#define FLAMEWM_API_CAPABILITIES_H

#include <string>
#include <vector>
#include <algorithm>

namespace flamewm {
namespace api {

namespace caps {
// Panel / taskbar
static const char kPanelMultiOutput[]      = "panel.multi-output";
static const char kPanelPerOutputScale[]   = "panel.per-output-scale";
static const char kTaskbarOrdering[]       = "taskbar.ordering";
static const char kTaskbarDragReorder[]    = "taskbar.drag-reorder";
static const char kTaskbarPinnedRunning[]  = "taskbar.pinned-running";

// Workspace
static const char kWorkspacePager[]        = "workspace.pager";
static const char kWorkspaceTransactions[] = "workspace.transactions";

// Window management
static const char kSnapHalfQuarter[]       = "snap.half-quarter";
static const char kSnapPreview[]           = "snap.preview";
static const char kWorkAreaStruts[]        = "workarea.struts";

// Desktop
static const char kDesktopSelection[]      = "desktop.selection";
static const char kDesktopStickyNotes[]    = "desktop.sticky-notes";
static const char kDesktopMenu[]           = "desktop.menu";

// Settings / appearance
static const char kSettingsAppearance[]    = "settings.appearance";
static const char kSettingsDisplays[]      = "settings.displays";
static const char kSettingsHotkeys[]       = "settings.hotkeys";
static const char kSettingsFonts[]         = "settings.fonts";
static const char kAccentLiveApply[]       = "settings.accent-live-apply";
static const char kIconThemeFreedesktop[]  = "settings.icon-theme";

// System integrations (optional backends)
static const char kIntegrationMpris[]      = "integration.mpris";
static const char kIntegrationNetworkMgr[] = "integration.network-manager";
static const char kIntegrationPulse[]      = "integration.pulse";
static const char kIntegrationDbus[]       = "integration.dbus";
}

struct Capabilities {
    std::vector<std::string> items;

    Capabilities() {}
    explicit Capabilities(const std::vector<std::string>& v) : items(v) {}
    explicit Capabilities(std::vector<std::string>&& v) : items(std::move(v)) {}

    bool has(const std::string& cap) const {
        return std::find(items.begin(), items.end(), cap) != items.end();
    }

    bool empty() const { return items.empty(); }
    std::size_t size() const { return items.size(); }

    static Capabilities current() {
        Capabilities c;
        c.items.reserve(22);
        c.items.push_back(caps::kPanelMultiOutput);
        c.items.push_back(caps::kPanelPerOutputScale);
        c.items.push_back(caps::kTaskbarOrdering);
        c.items.push_back(caps::kTaskbarDragReorder);
        c.items.push_back(caps::kTaskbarPinnedRunning);
        c.items.push_back(caps::kWorkspacePager);
        c.items.push_back(caps::kWorkspaceTransactions);
        c.items.push_back(caps::kSnapHalfQuarter);
        c.items.push_back(caps::kSnapPreview);
        c.items.push_back(caps::kWorkAreaStruts);
        c.items.push_back(caps::kDesktopSelection);
        c.items.push_back(caps::kDesktopStickyNotes);
        c.items.push_back(caps::kDesktopMenu);
        c.items.push_back(caps::kSettingsAppearance);
        c.items.push_back(caps::kSettingsDisplays);
        c.items.push_back(caps::kSettingsHotkeys);
        c.items.push_back(caps::kSettingsFonts);
        c.items.push_back(caps::kAccentLiveApply);
        c.items.push_back(caps::kIconThemeFreedesktop);
        c.items.push_back(caps::kIntegrationMpris);
        c.items.push_back(caps::kIntegrationNetworkMgr);
        c.items.push_back(caps::kIntegrationPulse);
        c.items.push_back(caps::kIntegrationDbus);
        return c;
    }
};

} // namespace api
} // namespace flamewm

#endif // FLAMEWM_API_CAPABILITIES_H
