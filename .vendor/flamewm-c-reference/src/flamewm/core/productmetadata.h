#ifndef FLAMEWM_CORE_PRODUCTMETADATA_H
#define FLAMEWM_CORE_PRODUCTMETADATA_H

#include <string>
#include <vector>

namespace flamewm {

struct ProductMetadata {
    static const char* donateUrl()  { return "https://paypal.me/LinsaFTW"; }
    static const char* sourceUrl()  { return "https://github.com/arkflame/flamewm"; }
    static const char* websiteUrl() { return "https://wm.arkflame.com"; }
};

// Safe URI launcher: argv vector, no shell interpolation.
// Implementations must exec via vector, never /bin/sh -c.
class SafeUriLauncher {
public:
    virtual ~SafeUriLauncher() {}
    // Returns true if launch dispatched
    virtual bool launchUri(const std::string& uri, std::string* error) = 0;
    // Build argv for fallback xdg-open vector
    static std::vector<std::string> buildXdgOpenArgv(const std::string& uri);
    static bool isSafeUri(const std::string& uri);
};

// Default implementation using Gio::AppInfo / g_app_info_launch_default_for_uri / xdg-open vector
class DefaultSafeUriLauncher : public SafeUriLauncher {
public:
    bool launchUri(const std::string& uri, std::string* error) override;
};

} // namespace flamewm
#endif
