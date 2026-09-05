#ifndef FLAMEWM_API_APPLICATIONS_H
#define FLAMEWM_API_APPLICATIONS_H

#include "flamewm/api/ids.h"

#include <string>
#include <vector>

namespace flamewm {
namespace api {

struct DesktopApplication {
    DesktopAppId id;
    std::string displayName;
    std::string execCmd;
    std::string iconName;
    std::vector<std::string> categories;
    std::string startupWMClass;
    std::string wmClassFallback;
    std::string desktopFilePath;

    DesktopApplication() {}

    bool valid() const { return !id.empty(); }
};

struct ApplicationLaunchOptions {
    DesktopAppId appId;
    std::vector<std::string> argvExtra;

    ApplicationLaunchOptions() {}
    explicit ApplicationLaunchOptions(const DesktopAppId& id) : appId(id) {}
    ApplicationLaunchOptions(const DesktopAppId& id, const std::vector<std::string>& extra)
        : appId(id), argvExtra(extra) {}
};

} // namespace api
} // namespace flamewm

#endif // FLAMEWM_API_APPLICATIONS_H
