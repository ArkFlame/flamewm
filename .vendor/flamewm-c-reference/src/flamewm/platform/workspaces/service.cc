#include "flamewm/platform/workspaces/service.h"
#include "flamewm/workspace/topology.h"

#include <map>
#include <vector>

namespace flamewm {
namespace platform {
namespace workspaces {

struct WorkspaceService::Impl {
    api::WorkspacePort* port;
    std::map<int, std::function<void(uint64_t)> > listeners;
    int nextId;

    explicit Impl(api::WorkspacePort* p) : port(p), nextId(1) {}

    void notify(uint64_t rev) {
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

    api::Result<api::WorkspaceSnapshot> portSnapshot() const {
        if (!port) return api::Result<api::WorkspaceSnapshot>::Err(api::Error::Unavailable, "no workspace port");
        return port->snapshot();
    }

    uint64_t currentRevision() const {
        api::Result<api::WorkspaceSnapshot> r = portSnapshot();
        if (!r.ok()) return 0;
        return r.value().revision;
    }
};

WorkspaceService::WorkspaceService(api::WorkspacePort* port) : impl_(new Impl(port)) {}
WorkspaceService::WorkspaceService(api::WorkspacePort* port, api::WorkAreaPort* /*ignored*/) : impl_(new Impl(port)) {}
WorkspaceService::~WorkspaceService() { delete impl_; }

api::WorkspaceSnapshot WorkspaceService::snapshot() const {
    if (!impl_ || !impl_->port) return api::WorkspaceSnapshot();
    api::Result<api::WorkspaceSnapshot> r = impl_->port->snapshot();
    if (!r.ok()) return api::WorkspaceSnapshot();
    return r.value();
}

uint64_t WorkspaceService::revision() const {
    if (!impl_) return 0;
    return impl_->currentRevision();
}

api::Status WorkspaceService::applyTransform(const api::WorkspaceTransform& t) {
    if (!impl_ || !impl_->port) {
        return api::Status::make(api::Error::Unavailable, "no workspace port");
    }
    // Validate revision/generation before delegation for early typed error.
    // Port is authoritative; stale check is also performed by port.
    // Keep local stale fast-path when snapshot is available.
    api::Result<api::WorkspaceSnapshot> cur = impl_->port->snapshot();
    if (cur.ok()) {
        if (t.expectedRevision != cur.value().revision) {
            return api::Status::make(api::Error::StaleRevision, "stale revision");
        }
    }
    api::Status s = impl_->port->applyTransform(t);
    if (s.ok()) {
        api::Result<api::WorkspaceSnapshot> fresh = impl_->port->snapshot();
        uint64_t rev = fresh.ok() ? fresh.value().revision : t.expectedRevision + 1;
        impl_->notify(rev);
    }
    return s;
}

api::Status WorkspaceService::activate(int index, uint64_t expectedRevision) {
    if (!impl_ || !impl_->port) {
        return api::Status::make(api::Error::Unavailable, "no workspace port");
    }
    api::Result<api::WorkspaceSnapshot> cur = impl_->port->snapshot();
    if (cur.ok()) {
        if (expectedRevision != cur.value().revision) {
            return api::Status::make(api::Error::StaleRevision, "stale revision");
        }
        if (index < 0 || index >= cur.value().count) {
            return api::Status::make(api::Error::InvalidArgument, "activate index out of range");
        }
    }
    api::Status s = impl_->port->activate(index, expectedRevision);
    if (s.ok()) {
        api::Result<api::WorkspaceSnapshot> fresh = impl_->port->snapshot();
        uint64_t rev = fresh.ok() ? fresh.value().revision : expectedRevision + 1;
        impl_->notify(rev);
    }
    return s;
}

api::Status WorkspaceService::insertAfter(int index, uint64_t expectedRevision) {
    // InsertAfter semantics: [-1, count-1]; delegated via applyTransform.
    return applyTransform(api::WorkspaceTransform::InsertAfter(index, expectedRevision));
}

api::Status WorkspaceService::remove(int index, uint64_t expectedRevision) {
    return applyTransform(api::WorkspaceTransform::Remove(index, expectedRevision));
}

api::Status WorkspaceService::moveWindow(api::WindowRef window, int targetWorkspace) {
    if (!impl_ || !impl_->port) {
        return api::Status::make(api::Error::Unavailable, "no workspace port");
    }
    if (!window.valid()) {
        return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
    }
    api::Result<api::WorkspaceSnapshot> cur = impl_->port->snapshot();
    if (cur.ok()) {
        if (targetWorkspace < 0 || targetWorkspace >= cur.value().count) {
            return api::Status::make(api::Error::InvalidArgument, "targetWorkspace out of range");
        }
    }
    api::Status s = impl_->port->moveWindow(window, targetWorkspace);
    if (s.ok()) {
        // Workspace move may not bump workspace revision but notify for subscribers.
        uint64_t rev = 0;
        api::Result<api::WorkspaceSnapshot> fresh = impl_->port->snapshot();
        if (fresh.ok()) rev = fresh.value().revision;
        impl_->notify(rev);
    }
    return s;
}

api::Status WorkspaceService::navigateLeft(uint64_t expectedRevision) {
    if (!impl_ || !impl_->port) return api::Status::make(api::Error::Unavailable, "no workspace port");
    api::Result<api::WorkspaceSnapshot> cur = impl_->port->snapshot();
    if (!cur.ok()) return api::Status::make(api::Error::Unavailable, "snapshot unavailable");
    if (expectedRevision != cur.value().revision) return api::Status::make(api::Error::StaleRevision, "stale revision");
    int nw = workspace::TwoRowTopology::moveLeft(cur.value().activeIndex, cur.value().count);
    if (nw == cur.value().activeIndex) return api::Status::Ok();
    return activate(nw, expectedRevision);
}

api::Status WorkspaceService::navigateRight(uint64_t expectedRevision) {
    if (!impl_ || !impl_->port) return api::Status::make(api::Error::Unavailable, "no workspace port");
    api::Result<api::WorkspaceSnapshot> cur = impl_->port->snapshot();
    if (!cur.ok()) return api::Status::make(api::Error::Unavailable, "snapshot unavailable");
    if (expectedRevision != cur.value().revision) return api::Status::make(api::Error::StaleRevision, "stale revision");
    int nw = workspace::TwoRowTopology::moveRight(cur.value().activeIndex, cur.value().count);
    if (nw == cur.value().activeIndex) return api::Status::Ok();
    return activate(nw, expectedRevision);
}

api::Status WorkspaceService::navigateUp(uint64_t expectedRevision) {
    if (!impl_ || !impl_->port) return api::Status::make(api::Error::Unavailable, "no workspace port");
    api::Result<api::WorkspaceSnapshot> cur = impl_->port->snapshot();
    if (!cur.ok()) return api::Status::make(api::Error::Unavailable, "snapshot unavailable");
    if (expectedRevision != cur.value().revision) return api::Status::make(api::Error::StaleRevision, "stale revision");
    int nw = workspace::TwoRowTopology::moveUp(cur.value().activeIndex, cur.value().count);
    if (nw == cur.value().activeIndex) return api::Status::Ok();
    return activate(nw, expectedRevision);
}

api::Status WorkspaceService::navigateDown(uint64_t expectedRevision) {
    if (!impl_ || !impl_->port) return api::Status::make(api::Error::Unavailable, "no workspace port");
    api::Result<api::WorkspaceSnapshot> cur = impl_->port->snapshot();
    if (!cur.ok()) return api::Status::make(api::Error::Unavailable, "snapshot unavailable");
    if (expectedRevision != cur.value().revision) return api::Status::make(api::Error::StaleRevision, "stale revision");
    int nw = workspace::TwoRowTopology::moveDown(cur.value().activeIndex, cur.value().count);
    if (nw == cur.value().activeIndex) return api::Status::Ok();
    return activate(nw, expectedRevision);
}

api::Status WorkspaceService::navigateLeft() {
    uint64_t rev = revision();
    return navigateLeft(rev);
}
api::Status WorkspaceService::navigateRight() {
    uint64_t rev = revision();
    return navigateRight(rev);
}
api::Status WorkspaceService::navigateUp() {
    uint64_t rev = revision();
    return navigateUp(rev);
}
api::Status WorkspaceService::navigateDown() {
    uint64_t rev = revision();
    return navigateDown(rev);
}

int WorkspaceService::addListener(std::function<void(uint64_t)> cb) {
    if (!impl_) return 0;
    if (!cb) return 0;
    int id = impl_->nextId++;
    impl_->listeners[id] = cb;
    return id;
}

void WorkspaceService::removeListener(int id) {
    if (!impl_) return;
    impl_->listeners.erase(id);
}

} // namespace workspaces
} // namespace platform
} // namespace flamewm
