#ifndef FLAMEWM_PLATFORM_WINDOWS_SERVICE_H
#define FLAMEWM_PLATFORM_WINDOWS_SERVICE_H

#include "flamewm/api/ports.h"
#include "flamewm/api/errors.h"
#include "flamewm/api/ids.h"
#include "flamewm/api/geometry.h"
#include "flamewm/api/window.h"

#include <cstdint>
#include <functional>
#include <vector>

namespace flamewm {
namespace platform {
namespace windows {

class WindowService {
public:
    explicit WindowService(api::WindowPort* port);
    ~WindowService();

    WindowService(const WindowService&) = delete;
    WindowService& operator=(const WindowService&) = delete;

    std::vector<api::WindowSnapshot> snapshot() const;
    api::Result<api::WindowSnapshot> get(api::WindowRef ref) const;
    api::Result<api::Rect> workArea(api::WindowRef ref) const;
    api::Result<api::OutputId> output(api::WindowRef ref) const;

    api::Status activate(api::WindowRef ref);
    api::Status minimize(api::WindowRef ref);
    api::Status maximize(api::WindowRef ref);
    api::Status restore(api::WindowRef ref);
    api::Status close(api::WindowRef ref);
    api::Status setOuterGeometry(api::WindowRef ref, api::Rect rect);

    // Compatibility shim: refresh cache from authoritative port. Publish only
    // if fresh carries valid generation. Prefer engine push via port.
    void invalidate(const std::vector<api::WindowSnapshot>& fresh);

    uint64_t revision() const;
    uint64_t generation() const;
    int addListener(std::function<void(uint64_t)> cb);
    void removeListener(int id);

private:
    struct Impl;
    Impl* impl_;
};

} // namespace windows
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_WINDOWS_SERVICE_H
