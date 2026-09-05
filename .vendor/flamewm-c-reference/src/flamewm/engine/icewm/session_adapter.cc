#if defined(HAVE_CONFIG_H)
#include "config.h"
#elif __has_include("config.h")
#include "config.h"
#endif

#include "flamewm/engine/icewm/session_adapter.h"
#include "flamewm/engine/icewm/access.h"

#include <string>
#include <vector>

#if __has_include(<unistd.h>)
#include <unistd.h>
#endif

#if __has_include(<X11/Xlib.h>)
#include <X11/Xlib.h>
#endif

// Reuse IceWM YWMApp authority when available. X11/IceWM headers are
// optional for syntax-only / non-product builds. Guard on X11 availability
// so syntax-only without Xrender headers falls back to Unavailable path.
// config.h must precede any IceWM header that transitively includes
// prefs.h -> default.h -> themable.h -> fontmacro.h (which #errors if
// CFGDIR not defined via config.h).
#if __has_include(<X11/extensions/Xrender.h>)
#if __has_include("wmapp.h")
#include "wmapp.h"
#endif
#if __has_include("wmaction.h")
#include "wmaction.h"
#endif
#if __has_include("yaction.h")
#include "yaction.h"
#endif
#if __has_include("prefs.h")
#include "prefs.h"
#endif
#endif
#ifdef Status
#undef Status
#endif

namespace flamewm {
namespace engine {
namespace icewm {

SessionAdapter::SessionAdapter() {}

SessionAdapter::~SessionAdapter() {}

api::Status SessionAdapter::dispatchLock() {
#if __has_include(<X11/extensions/Xrender.h>) && __has_include("wmapp.h") && __has_include("wmaction.h")
    YWMApp* app = EngineAccess::appTyped();
    if (app == nullptr) {
        return api::Status::make(api::Error::Unavailable, "no session context");
    }
    if (!canLock()) {
        return api::Status::make(api::Error::Unavailable, "lock not configured");
    }
    app->actionPerformed(YAction(actionLock));
    return api::Status::Ok();
#else
    (void)EngineAccess::appTyped;
    return api::Status::make(api::Error::Unavailable, "no session authority in this build");
#endif
}

api::Status SessionAdapter::dispatchLogout(int rebootShutdown) {
#if __has_include(<X11/extensions/Xrender.h>) && __has_include("wmapp.h") && __has_include("wmaction.h")
    YWMApp* app = EngineAccess::appTyped();
    if (app == nullptr) {
        return api::Status::make(api::Error::Unavailable, "no session context");
    }
    app->doLogout(static_cast<RebootShutdown>(rebootShutdown));
    return api::Status::Ok();
#else
    (void)rebootShutdown;
    return api::Status::make(api::Error::Unavailable, "no session authority in this build");
#endif
}

api::Status SessionAdapter::dispatchSuspend() {
#if __has_include(<X11/extensions/Xrender.h>) && __has_include("wmapp.h") && __has_include("wmaction.h")
    YWMApp* app = EngineAccess::appTyped();
    if (app == nullptr) {
        return api::Status::make(api::Error::Unavailable, "no session context");
    }
    if (!canSuspend()) {
        return api::Status::make(api::Error::Unavailable, "suspend not configured");
    }
    app->actionPerformed(YAction(actionSuspend));
    return api::Status::Ok();
#else
    return api::Status::make(api::Error::Unavailable, "no session authority in this build");
#endif
}

api::Status SessionAdapter::systemAction(const char* action) {
    if (action == nullptr || action[0] == '\0') {
        return api::Status::make(api::Error::InvalidArgument, "empty action");
    }
    std::string act(action);
    // Allowlist: no arbitrary exec. Each maps to existing IceWM authority.
    if (act == "lock") return dispatchLock();
    if (act == "logout") return dispatchLogout(0); // Logout
    if (act == "suspend") return dispatchSuspend();
    if (act == "reboot") return dispatchLogout(1); // Reboot
    if (act == "shutdown") return dispatchLogout(2); // Shutdown
    return api::Status::make(api::Error::InvalidArgument, "unknown session action");
}

api::Status SessionAdapter::lock() {
    return systemAction("lock");
}

api::Status SessionAdapter::logout() {
    return systemAction("logout");
}

api::Status SessionAdapter::suspend() {
    return systemAction("suspend");
}

api::Status SessionAdapter::reboot() {
    return systemAction("reboot");
}

api::Status SessionAdapter::shutdown() {
    return systemAction("shutdown");
}

api::SessionCapabilities SessionAdapter::capabilities() {
    api::SessionCapabilities caps;
    caps.canLock = false;
    caps.canLogout = false;
    caps.canSuspend = false;
    caps.canReboot = false;
    caps.canShutdown = false;
#if __has_include(<X11/extensions/Xrender.h>) && __has_include("wmaction.h")
    YWMApp* app = EngineAccess::appTyped();
    if (app == nullptr) {
        return caps;
    }
    caps.canLock = canLock();
    caps.canLogout = true;
    caps.canSuspend = canSuspend();
    caps.canReboot = canShutdown(Reboot);
    caps.canShutdown = canShutdown(Shutdown);
#else
    (void)EngineAccess::appTyped;
#endif
    return caps;
}

} // namespace icewm
} // namespace engine
} // namespace flamewm
