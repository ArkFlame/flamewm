#ifndef FLAMEWM_PLATFORM_DISPLAYS_SERVICE_H
#define FLAMEWM_PLATFORM_DISPLAYS_SERVICE_H

#include "flamewm/api/display.h"
#include "flamewm/api/errors.h"
#include "flamewm/api/ids.h"
#include "flamewm/api/ports.h"

#include <cstdint>
#include <functional>

namespace flamewm {
namespace platform {
namespace displays {

enum class DisplayTxState {
    Idle = 0,
    Pending = 1,
    Keeping = 2,
    Reverting = 3
};

class DisplayService {
public:
    DisplayService(api::DisplayPort* display, api::MainLoopPort* loop);
    ~DisplayService();

    DisplayService(const DisplayService&) = delete;
    DisplayService& operator=(const DisplayService&) = delete;

    api::DisplaySnapshot snapshot() const;
    uint64_t generation() const;

    // Mode transaction — 15s deadline via MainLoopPort. Fresh snapshot via DisplayPort.
    api::Result<api::TransactionId> beginModeChange(const api::OutputId& output,
                                                     api::ModeId mode,
                                                     uint64_t topologyGeneration);
    api::Status keep(api::TransactionId tx);
    api::Status revert(api::TransactionId tx);

    // Called when topology generation advanced externally (hotplug).
    // If pending tx base generation stale, auto-reverts via DisplayPort::restore.
    void onTopologyChanged(const api::DisplaySnapshot& fresh);

    // Flame chrome scale only. Allowed: 100/125/150/175/200. Bumps generation.
    api::Status setShellScale(const api::OutputId& output,
                               int percent,
                               uint64_t expectedRevision);

    int addListener(std::function<void(uint64_t)> cb);
    void removeListener(int listenerId);

private:
    struct Impl;
    Impl* impl_;
};

} // namespace displays
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_DISPLAYS_SERVICE_H
