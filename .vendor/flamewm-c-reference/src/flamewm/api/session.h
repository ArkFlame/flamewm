#ifndef FLAMEWM_API_SESSION_H
#define FLAMEWM_API_SESSION_H

#include "errors.h"

namespace flamewm {
namespace api {

enum class SessionAction {
    Lock,
    Logout,
    Suspend,
    Reboot,
    Shutdown
};

struct SessionCapabilities {
    bool canLock;
    bool canLogout;
    bool canSuspend;
    bool canReboot;
    bool canShutdown;
};

} // namespace api
} // namespace flamewm

#endif // FLAMEWM_API_SESSION_H
