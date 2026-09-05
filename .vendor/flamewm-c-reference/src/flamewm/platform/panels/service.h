#ifndef FLAMEWM_PLATFORM_PANELS_SERVICE_H
#define FLAMEWM_PLATFORM_PANELS_SERVICE_H

#include "flamewm/api/panels.h"
#include "flamewm/api/errors.h"
#include "flamewm/api/ids.h"
#include "flamewm/api/display.h"
#include "flamewm/api/window.h"
#include "flamewm/api/settings.h"
#include "flamewm/api/ports.h"

#include <cstdint>
#include <functional>
#include <vector>

namespace flamewm {
namespace platform {
namespace panels {

class PanelService {
public:
    explicit PanelService(api::WorkAreaPort* workArea, api::TrayPort* tray);
    ~PanelService();

    PanelService(const PanelService&) = delete;
    PanelService& operator=(const PanelService&) = delete;

    // Snapshots
    api::PanelsSnapshot snapshot() const;
    uint64_t revision() const;

    // Explicit snapshot consumers (no direct IceWM)
    void onDisplaySnapshot(const api::DisplaySnapshot& snap);
    void onWindowSnapshots(const std::vector<api::WindowSnapshot>& windows);
    void onSettingsSnapshot(const api::SettingsSnapshot& snap);
    void clearSubscriptions();

    // Commands — validate then effect via ports where needed, publish only after success
    api::Status setEdge(const api::OutputId& output, api::PanelEdge edge, uint64_t expectedRevision);
    api::Status setSize(const api::OutputId& output, int logicalSize, uint64_t expectedRevision);
    api::Status pin(const api::DesktopAppId& app, uint64_t expectedRevision);
    api::Status unpin(const api::DesktopAppId& app, uint64_t expectedRevision);
    api::Status reorder(const api::TaskEntryId& entryId, int insertionIndex, uint64_t expectedRevision);
    void updateTasks(const std::vector<api::TaskEntry>& tasks);
    api::Status setStartOpen(bool open, const api::OutputId& output, uint64_t expectedRevision);

    int addListener(std::function<void(uint64_t)> cb);
    void removeListener(int listenerId);

private:
    struct Impl;
    Impl* impl_;
};

} // namespace panels
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_PANELS_SERVICE_H
