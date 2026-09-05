#ifndef FLAMEWM_CONTROL_CLIENT_H
#define FLAMEWM_CONTROL_CLIENT_H

#include <cstdint>
#include <map>
#include <string>
#include <vector>

#include "flamewm/api/capabilities.h"
#include "flamewm/api/display.h"
#include "flamewm/api/errors.h"
#include "flamewm/api/panels.h"
#include "flamewm/api/session.h"
#include "flamewm/api/settings.h"
#include "flamewm/api/shortcuts.h"
#include "flamewm/api/workspace.h"

namespace flamewm {
namespace control {

class ControlClient {
public:
    ControlClient();
    ~ControlClient();
    ControlClient(const ControlClient&) = delete;
    ControlClient& operator=(const ControlClient&) = delete;

    bool connect();
    void disconnect();
    bool isConnected() const;

    // Core / introspection (com.arkflame.FlameWM1)
    api::Result<std::string> ping();
    api::Result<std::string> getVersion();
    api::Result<api::Capabilities> getCapabilities();

    // Settings (com.arkflame.FlameWM1.Settings)
    api::Result<api::SettingsSnapshot> getSettingsSnapshot();
    api::Status applySettings(uint64_t expectedRevision,
                              const std::vector<api::SettingsChange>& changes);
    api::Status resetSettingsSection(uint64_t expectedRevision,
                                     const std::string& section);

    // Workspaces (com.arkflame.FlameWM1.Workspaces)
    api::Result<api::WorkspaceSnapshot> getWorkspaceSnapshot();
    api::Status applyWorkspaceTransform(const api::WorkspaceTransform& transform);
    api::Status activateWorkspace(uint32_t index, uint64_t expectedRevision);
    api::Status insertWorkspaceAfter(uint32_t index, uint64_t expectedRevision);
    api::Status removeWorkspace(uint32_t index, uint64_t expectedRevision);

    // Displays (com.arkflame.FlameWM1.Displays)
    api::Result<api::DisplaySnapshot> getDisplaySnapshot();
    api::Status beginDisplayModeChange(const api::OutputId& output,
                                       const api::ModeId& mode,
                                       uint64_t generation,
                                       api::TransactionId* outTx);
    api::Status keepDisplayMode(const api::TransactionId& tx);
    api::Status revertDisplayMode(const api::TransactionId& tx);
    api::Status setShellScale(const api::OutputId& output,
                              uint32_t percent,
                              uint64_t expectedRevision);
    // Legacy aliases kept for compat - delegate to new exact methods
    api::Status applyDisplayMode(const api::OutputId& output,
                                 const api::ModeId& mode,
                                 api::TransactionId* outTx);
    api::Status restoreDisplayMode(const api::TransactionId& tx);

    // Shortcuts (com.arkflame.FlameWM1.Shortcuts)
    api::Result<api::ShortcutSnapshot> getShortcutSnapshot();
    api::Status setBinding(const std::string& action,
                           const std::string& binding,
                           uint64_t expectedRevision);
    api::Status clearBinding(const std::string& action,
                             uint64_t expectedRevision);
    api::Status resetBinding(const std::string& action,
                             uint64_t expectedRevision);
    api::Status applyShortcuts(uint64_t expectedRevision,
                               const std::map<std::string, api::KeyBinding>& bindings);

    // Panels (com.arkflame.FlameWM1.Panels)
    api::Result<api::PanelsSnapshot> getPanelsSnapshot();
    api::Status setPanelEdge(const api::OutputId& output, api::PanelEdge edge, uint64_t expectedRevision);
    api::Status setPanelEdge(const api::OutputId& output, api::PanelEdge edge);
    api::Status setPanelSize(const api::OutputId& output, uint32_t size, uint64_t expectedRevision);
    api::Status pin(const api::DesktopAppId& appId, uint64_t expectedRevision);
    api::Status unpin(const api::DesktopAppId& appId, uint64_t expectedRevision);
    api::Status reorder(const api::TaskEntryId& entryId, uint32_t index, uint64_t expectedRevision);

    // Session (com.arkflame.FlameWM1.Session)
    api::Result<api::SessionCapabilities> getSessionCapabilities();
    api::Status requestSessionAction(api::SessionAction action);

    struct Impl;
private:
    Impl* impl_;
};

} // namespace control
} // namespace flamewm

#endif // FLAMEWM_CONTROL_CLIENT_H
