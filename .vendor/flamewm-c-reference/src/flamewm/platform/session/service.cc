#include "flamewm/platform/session/service.h"

namespace flamewm {
namespace platform {
namespace session {

SessionService::SessionService(api::SessionPort* port) : port_(port) {}

SessionService::~SessionService() {}

api::Status SessionService::lock() {
    if (!port_) return api::Status::make(api::Error::Unavailable, "no session backend");
    return port_->lock();
}

api::Status SessionService::logout() {
    if (!port_) return api::Status::make(api::Error::Unavailable, "no session backend");
    return port_->logout();
}

api::Status SessionService::suspend() {
    if (!port_) return api::Status::make(api::Error::Unavailable, "no session backend");
    return port_->suspend();
}

api::Status SessionService::reboot() {
    if (!port_) return api::Status::make(api::Error::Unavailable, "no session backend");
    return port_->reboot();
}

api::Status SessionService::shutdown() {
    if (!port_) return api::Status::make(api::Error::Unavailable, "no session backend");
    return port_->shutdown();
}

api::SessionCapabilities SessionService::capabilities() const {
    if (!port_) {
        api::SessionCapabilities c;
        c.canLock = false;
        c.canLogout = false;
        c.canSuspend = false;
        c.canReboot = false;
        c.canShutdown = false;
        return c;
    }
    return port_->capabilities();
}

} // namespace session
} // namespace platform
} // namespace flamewm
