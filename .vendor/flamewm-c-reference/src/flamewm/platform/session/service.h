#ifndef FLAMEWM_PLATFORM_SESSION_SERVICE_H
#define FLAMEWM_PLATFORM_SESSION_SERVICE_H

#include "flamewm/api/errors.h"
#include "flamewm/api/ports.h"
#include "flamewm/api/session.h"

namespace flamewm {
namespace platform {
namespace session {

class SessionService {
public:
    explicit SessionService(api::SessionPort* port);
    ~SessionService();

    SessionService(const SessionService&) = delete;
    SessionService& operator=(const SessionService&) = delete;

    api::Status lock();
    api::Status logout();
    api::Status suspend();
    api::Status reboot();
    api::Status shutdown();
    api::SessionCapabilities capabilities() const;

private:
    api::SessionPort* port_;
};

} // namespace session
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_SESSION_SERVICE_H
