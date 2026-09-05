#ifndef FLAMEWM_PLATFORM_BACKGROUND_SERVICE_H
#define FLAMEWM_PLATFORM_BACKGROUND_SERVICE_H

#include "flamewm/api/background.h"
#include "flamewm/api/errors.h"
#include "flamewm/api/ports.h"

#include <string>

namespace flamewm {
namespace platform {
namespace background {

class BackgroundService {
public:
    // C3: product setting + engine projection + port reload.
    // enginePrefsPath: XDG_CONFIG_HOME/flamewm/engine-preferences (icewm prefs)
    // productSettingsPath: XDG_CONFIG_HOME/flamewm/flame.conf (Flame product setting) — optional
    // port: BackgroundPort (icewmbg backend). No second painter — Flame never paints root.
    explicit BackgroundService(api::BackgroundPort* port,
                               const std::string& enginePrefsPath);
    BackgroundService(api::BackgroundPort* port,
                      const std::string& enginePrefsPath,
                      const std::string& productSettingsPath);
    ~BackgroundService();

    BackgroundService(const BackgroundService&) = delete;
    BackgroundService& operator=(const BackgroundService&) = delete;

    // Validates, persists product setting, projects engine prefs via port,
    // calls reload. Port failure before publish — no state publish on error.
    // Typed errors (InvalidArgument, IoFailure, EngineRejected, Unavailable).
    // C++11, no X11.
    api::Status setWallpaper(const std::string& path);
    api::BackgroundState current() const;

    // Product key used in flame.conf for wallpaper (unknownKeys-compatible)
    static const char* kProductKey;

private:
    struct Impl;
    Impl* impl_;
};

} // namespace background
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_BACKGROUND_SERVICE_H
