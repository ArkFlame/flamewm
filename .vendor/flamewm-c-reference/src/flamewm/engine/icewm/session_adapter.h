#pragma once

#include "flamewm/api/ports.h"

namespace flamewm {
namespace engine {
namespace icewm {

class SessionAdapter : public api::SessionPort {
public:
    SessionAdapter();
    ~SessionAdapter();

    api::Status lock() override;
    api::Status logout() override;
    api::Status suspend() override;
    api::Status reboot() override;
    api::Status shutdown() override;
    api::SessionCapabilities capabilities() override;

private:
    static api::Status systemAction(const char* action);
    static api::Status dispatchLock();
    static api::Status dispatchLogout(int rebootShutdown);
    static api::Status dispatchSuspend();
};

} // namespace icewm
} // namespace engine
} // namespace flamewm
