#ifndef FLAMEWM_PLATFORM_SCALE_SERVICE_H
#define FLAMEWM_PLATFORM_SCALE_SERVICE_H

#include "flamewm/api/errors.h"
#include "flamewm/api/ids.h"

#include <cstdint>
#include <functional>

namespace flamewm {
namespace platform {
namespace scale {

class ScaleService {
public:
    ScaleService();
    ~ScaleService();

    ScaleService(const ScaleService&) = delete;
    ScaleService& operator=(const ScaleService&) = delete;

    // Query — always returns a valid percent (100 if unknown/invalid output).
    // No X11, no ports. Pure in-memory scale table.
    int scaleFor(const api::OutputId& output) const;
    double scaleFactorFor(const api::OutputId& output) const;
    int logicalToPhysical(int logical, const api::OutputId& output) const;
    int physicalToLogical(int physical, const api::OutputId& output) const;

    // Mutators — validated, typed errors, revision-checked.
    // Allowed buckets: 100/125/150/175/200. Invalid -> InvalidArgument.
    // Stale expectedRevision -> StaleRevision (generation validation via
    // DisplayService-equivalent revision). No X11.
    api::Status setScale(const api::OutputId& output, int percent);
    api::Status setScale(const api::OutputId& output, int percent, uint64_t expectedRevision);

    // Shell-scale revision (monotonic). Bumps only on actual change.
    // Intended to mirror DisplayService generation for validation.
    uint64_t revision() const;
    uint64_t generation() const { return revision(); }

    // Called when external topology/DisplayService generation advanced.
    // Keeps revision coherent; no X11.
    void onDisplayGeneration(uint64_t generation);

    // Lifetime / subscriptions
    void clearSubscriptions();
    int addListener(std::function<void(uint64_t)> cb);
    void removeListener(int id);

private:
    struct Impl;
    Impl* impl_;
};

} // namespace scale
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_SCALE_SERVICE_H
