#ifndef FLAMEWM_ENGINE_ICEWM_BRIDGE_H
#define FLAMEWM_ENGINE_ICEWM_BRIDGE_H

#include <cstdint>

#include "flamewm/api/ports.h"
#include "flamewm/platform/host.h"

namespace flamewm {
namespace engine {
namespace icewm {

// Bridge: singleton that owns the PlatformHost attachment to the IceWM
// engine. Only active when FLAMEWM_PRODUCT_BUILD is defined; otherwise
// attach() is a no-op returning false. Detached state is the safe default —
// all Hooks become no-ops when not attached.
//
// Future upstream integration will insert call sites guarded by:
//   // FLAMEWM-BRIDGE-HOOK-BEGIN
//   // FLAMEWM-BRIDGE-HOOK-END
// Those markers do NOT yet exist in upstream src/*.cc files; they are
// documented here as design intent only (C-BRIDGE-01/02).

class Bridge {
public:
    static Bridge& instance();

    // Attach host+ports to the running IceWM process. Returns true on
    // first successful attach, false if already attached or if
    // FLAMEWM_PRODUCT_BUILD is not defined, or if host/ports are invalid.
    // Caller retains ownership of host (must outlive attachment; typically
    // owned by main() / YApplication).
    bool attach(flamewm::platform::PlatformHost* host,
                flamewm::api::EnginePorts ports);

    void detach();

    bool isAttached() const;
    flamewm::platform::PlatformHost* host() const;

private:
    Bridge();
    ~Bridge();
    Bridge(const Bridge&) = delete;
    Bridge& operator=(const Bridge&) = delete;

    struct Impl;
    Impl* impl_;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm

#endif // FLAMEWM_ENGINE_ICEWM_BRIDGE_H
