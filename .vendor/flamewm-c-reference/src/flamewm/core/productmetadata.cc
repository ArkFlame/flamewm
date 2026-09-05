#include "productmetadata.h"
#include <cstring>

namespace flamewm {

std::vector<std::string> SafeUriLauncher::buildXdgOpenArgv(const std::string& uri) {
    std::vector<std::string> v;
    v.push_back("xdg-open");
    v.push_back(uri);
    return v;
}

bool SafeUriLauncher::isSafeUri(const std::string& uri) {
    if (uri.empty()) return false;
    if (uri.size() > 2048) return false;
    bool isHttp = uri.compare(0, 8, "https://") == 0 || uri.compare(0, 7, "http://") == 0;
    if (!isHttp) return false;
    // reject shell metachars / spaces — defense in depth (exec vector is primary)
    const char* bad = " \t\n\r\"'\\`$&();<>|*?~#{}[]!;"; // includes ';' and space
    for (size_t i = 0; i < uri.size(); ++i) {
        if (strchr(bad, uri[i])) return false;
    }
    return true;
}

bool DefaultSafeUriLauncher::launchUri(const std::string& uri, std::string* error) {
    if (!isSafeUri(uri)) {
        if (error) *error = "unsafe uri";
        return false;
    }
    // Skeleton: no actual launch without GLib linked in this phase.
    // Caller should use Gio::AppInfo::launch_default_for_uri or exec vector.
    // Return true to indicate uri was validated (real exec wired in later phase).
    (void)error;
    return true;
}

} // namespace flamewm
