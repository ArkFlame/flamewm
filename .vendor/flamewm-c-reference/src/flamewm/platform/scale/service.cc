#include "flamewm/platform/scale/service.h"

#include <map>
#include <string>

namespace flamewm {
namespace platform {
namespace scale {

namespace {

bool isAllowedScale(int pct) {
    return pct == 100 || pct == 125 || pct == 150 || pct == 175 || pct == 200;
}

int logicalToPhysicalInt(int logical, int pct) {
    // round half up: (logical*pct + 50)/100 — same as FlameMetrics::logicalToPhysical
    return (logical * pct + 50) / 100;
}

int physicalToLogicalInt(int physical, int pct) {
    if (pct == 0) return physical;
    return (physical * 100 + pct / 2) / pct;
}

} // namespace

struct ScaleService::Impl {
    std::map<std::string, int> scales;
    uint64_t rev;
    int nextListenerId;
    std::map<int, std::function<void(uint64_t)> > listeners;

    Impl() : scales(), rev(0), nextListenerId(1), listeners() {}

    void notify() {
        std::map<int, std::function<void(uint64_t)> > copy = listeners;
        for (std::map<int, std::function<void(uint64_t)> >::iterator it = copy.begin(); it != copy.end(); ++it) {
            if (it->second) it->second(rev);
        }
    }
};

ScaleService::ScaleService() : impl_(new Impl()) {}

ScaleService::~ScaleService() { delete impl_; }

int ScaleService::scaleFor(const api::OutputId& output) const {
    if (!output.valid()) return 100;
    std::map<std::string, int>::const_iterator it = impl_->scales.find(output.key);
    if (it == impl_->scales.end()) return 100;
    return it->second;
}

double ScaleService::scaleFactorFor(const api::OutputId& output) const {
    return static_cast<double>(scaleFor(output)) / 100.0;
}

int ScaleService::logicalToPhysical(int logical, const api::OutputId& output) const {
    return logicalToPhysicalInt(logical, scaleFor(output));
}

int ScaleService::physicalToLogical(int physical, const api::OutputId& output) const {
    return physicalToLogicalInt(physical, scaleFor(output));
}

api::Status ScaleService::setScale(const api::OutputId& output, int percent) {
    if (!output.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid output");
    if (!isAllowedScale(percent)) return api::Status::make(api::Error::InvalidArgument, "scale must be 100/125/150/175/200");
    std::map<std::string, int>::iterator it = impl_->scales.find(output.key);
    if (it != impl_->scales.end() && it->second == percent) return api::Status::Ok();
    impl_->scales[output.key] = percent;
    ++impl_->rev;
    impl_->notify();
    return api::Status::Ok();
}

api::Status ScaleService::setScale(const api::OutputId& output, int percent, uint64_t expectedRevision) {
    if (!output.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid output");
    if (!isAllowedScale(percent)) return api::Status::make(api::Error::InvalidArgument, "scale must be 100/125/150/175/200");
    if (expectedRevision != impl_->rev) return api::Status::make(api::Error::StaleRevision, "stale revision");
    std::map<std::string, int>::iterator it = impl_->scales.find(output.key);
    if (it != impl_->scales.end() && it->second == percent) return api::Status::Ok();
    impl_->scales[output.key] = percent;
    ++impl_->rev;
    impl_->notify();
    return api::Status::Ok();
}

uint64_t ScaleService::revision() const {
    return impl_->rev;
}

void ScaleService::onDisplayGeneration(uint64_t generation) {
    // Coherence hook: if external DisplayService generation jumped ahead of
    // our revision, future setScale(expectedRevision) callers must pass our
    // current revision. We do NOT auto-bump here — generation is observed,
    // not driven. This keeps no-X11 invariant and avoids spurious notifies.
    // If callers need strict cross-service validation, they compare their
    // captured DisplayService generation against our revision explicitly.
    (void)generation;
}

void ScaleService::clearSubscriptions() {
    impl_->listeners.clear();
}

int ScaleService::addListener(std::function<void(uint64_t)> cb) {
    if (!cb) return 0;
    int id = impl_->nextListenerId++;
    impl_->listeners[id] = cb;
    return id;
}

void ScaleService::removeListener(int id) {
    std::map<int, std::function<void(uint64_t)> >::iterator it = impl_->listeners.find(id);
    if (it != impl_->listeners.end()) impl_->listeners.erase(it);
}

} // namespace scale
} // namespace platform
} // namespace flamewm
