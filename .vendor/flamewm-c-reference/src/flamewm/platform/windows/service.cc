#include "flamewm/platform/windows/service.h"

#include <map>
#include <vector>
#include <functional>

namespace flamewm {
namespace platform {
namespace windows {

struct WindowService::Impl {
    api::WindowPort* port;
    std::vector<api::WindowSnapshot> cached;
    uint64_t rev;
    uint64_t gen;
    std::map<int, std::function<void(uint64_t)> > listeners;
    int nextId;

    explicit Impl(api::WindowPort* p) : port(p), rev(0), gen(1), nextId(1) {}

    void notify() {
        std::vector<std::function<void(uint64_t)> > cbs;
        cbs.reserve(listeners.size());
        for (std::map<int, std::function<void(uint64_t)> >::const_iterator it = listeners.begin();
             it != listeners.end(); ++it) {
            cbs.push_back(it->second);
        }
        for (std::size_t i = 0; i < cbs.size(); ++i) {
            if (cbs[i]) cbs[i](rev);
        }
    }

    api::Status validateRefForMutation(api::WindowRef ref) {
        if (!ref.valid()) {
            return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
        }
        if (!port) {
            return api::Status::make(api::Error::Unavailable, "WindowPort unavailable");
        }
        api::Result<api::WindowSnapshot> cur = port->get(ref);
        if (!cur.ok()) {
            // Propagate port error directly (NotFound, Unavailable, etc.)
            return cur.status();
        }
        if (cur.value().ref.generation != ref.generation) {
            return api::Status::make(api::Error::InvalidArgument, "stale WindowRef generation");
        }
        return api::Status::Ok();
    }
};

WindowService::WindowService(api::WindowPort* port) : impl_(new Impl(port)) {}
WindowService::~WindowService() { delete impl_; }

std::vector<api::WindowSnapshot> WindowService::snapshot() const {
    if (!impl_->port) {
        return std::vector<api::WindowSnapshot>();
    }
    std::vector<api::WindowSnapshot> out = impl_->port->snapshot();
    // Cache immutable snapshot for revision tracking only; do not duplicate
    // authoritative IceWM state beyond this read-only copy.
    impl_->cached = out;
    return out;
}

api::Result<api::WindowSnapshot> WindowService::get(api::WindowRef ref) const {
    if (!ref.valid()) {
        return api::Result<api::WindowSnapshot>::Err(api::Error::InvalidArgument, "invalid WindowRef");
    }
    if (!impl_->port) {
        return api::Result<api::WindowSnapshot>::Err(api::Error::Unavailable, "WindowPort unavailable");
    }
    api::Result<api::WindowSnapshot> cur = impl_->port->get(ref);
    if (!cur.ok()) {
        return cur;
    }
    if (cur.value().ref.generation != ref.generation) {
        return api::Result<api::WindowSnapshot>::Err(api::Error::InvalidArgument, "stale WindowRef generation");
    }
    return cur;
}

api::Result<api::Rect> WindowService::workArea(api::WindowRef ref) const {
    if (!ref.valid()) {
        return api::Result<api::Rect>::Err(api::Error::InvalidArgument, "invalid WindowRef");
    }
    if (!impl_->port) {
        return api::Result<api::Rect>::Err(api::Error::Unavailable, "WindowPort unavailable");
    }
    // Enforce generation lifetime before delegating.
    api::Result<api::WindowSnapshot> cur = impl_->port->get(ref);
    if (!cur.ok()) {
        return api::Result<api::Rect>::Err(cur.status());
    }
    if (cur.value().ref.generation != ref.generation) {
        return api::Result<api::Rect>::Err(api::Error::InvalidArgument, "stale WindowRef generation");
    }
    return impl_->port->workArea(ref);
}

api::Result<api::OutputId> WindowService::output(api::WindowRef ref) const {
    if (!ref.valid()) {
        return api::Result<api::OutputId>::Err(api::Error::InvalidArgument, "invalid WindowRef");
    }
    if (!impl_->port) {
        return api::Result<api::OutputId>::Err(api::Error::Unavailable, "WindowPort unavailable");
    }
    api::Result<api::WindowSnapshot> cur = impl_->port->get(ref);
    if (!cur.ok()) {
        return api::Result<api::OutputId>::Err(cur.status());
    }
    if (cur.value().ref.generation != ref.generation) {
        return api::Result<api::OutputId>::Err(api::Error::InvalidArgument, "stale WindowRef generation");
    }
    return impl_->port->output(ref);
}

void WindowService::invalidate(const std::vector<api::WindowSnapshot>& fresh) {
    // Publish only valid generation: filter to snapshots with valid WindowRef.
    std::vector<api::WindowSnapshot> filtered;
    filtered.reserve(fresh.size());
    for (std::size_t i = 0; i < fresh.size(); ++i) {
        const api::WindowSnapshot& s = fresh[i];
        if (s.ref.valid()) {
            filtered.push_back(s);
        }
    }
    impl_->cached = filtered;
    impl_->rev += 1;
    impl_->gen += 1;
    impl_->notify();
}

api::Status WindowService::activate(api::WindowRef ref) {
    api::Status v = impl_->validateRefForMutation(ref);
    if (!v.ok()) return v;
    api::Status s = impl_->port->activate(ref);
    // Never publish new effective state after native effect fails.
    return s;
}

api::Status WindowService::minimize(api::WindowRef ref) {
    api::Status v = impl_->validateRefForMutation(ref);
    if (!v.ok()) return v;
    return impl_->port->minimize(ref);
}

api::Status WindowService::maximize(api::WindowRef ref) {
    api::Status v = impl_->validateRefForMutation(ref);
    if (!v.ok()) return v;
    return impl_->port->maximize(ref);
}

api::Status WindowService::restore(api::WindowRef ref) {
    api::Status v = impl_->validateRefForMutation(ref);
    if (!v.ok()) return v;
    return impl_->port->restore(ref);
}

api::Status WindowService::close(api::WindowRef ref) {
    api::Status v = impl_->validateRefForMutation(ref);
    if (!v.ok()) return v;
    return impl_->port->close(ref);
}

api::Status WindowService::setOuterGeometry(api::WindowRef ref, api::Rect rect) {
    if (!rect.valid()) {
        return api::Status::make(api::Error::InvalidArgument, "invalid geometry");
    }
    api::Status v = impl_->validateRefForMutation(ref);
    if (!v.ok()) return v;
    return impl_->port->setOuterGeometry(ref, rect);
}

uint64_t WindowService::revision() const {
    return impl_->rev;
}

uint64_t WindowService::generation() const {
    return impl_->gen;
}

int WindowService::addListener(std::function<void(uint64_t)> cb) {
    int id = impl_->nextId++;
    impl_->listeners[id] = cb;
    return id;
}

void WindowService::removeListener(int id) {
    impl_->listeners.erase(id);
}

} // namespace windows
} // namespace platform
} // namespace flamewm
