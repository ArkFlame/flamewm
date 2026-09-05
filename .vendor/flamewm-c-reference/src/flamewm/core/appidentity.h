#ifndef FLAMEWM_CORE_APPIDENTITY_H
#define FLAMEWM_CORE_APPIDENTITY_H

#include <string>
#include <vector>

namespace flamewm {

// Central identity resolver:
// desktopId -> StartupWMClass -> WM_CLASS -> normalized fallback
// Window title is NEVER identity.
struct AppIdentityInput {
    std::string desktopId;      // e.g. "firefox.desktop"
    std::string startupWMClass; // from StartupWMClass or _NET_WM_PID mapping
    std::string wmClass;        // WM_CLASS second string (or first fallback)
    std::string wmInstance;     // WM_CLASS first string
};

enum ContextAction {
    ContextOpen     = 0,
    ContextUnpin    = 1,
    ContextPin      = 2,
    ContextMaximize = 3,
    ContextMinimize = 4,
    ContextClose    = 5,
    ContextSeparator = 6
};

class ApplicationIdentity {
public:
    static std::string resolve(const AppIdentityInput& in);
    static std::string normalize(const std::string& raw);
    static bool isDesktopId(const std::string& id);

    // Pinned/running merge: stable identity dedup, no title grouping.
    // Pinned order preserved, then running items not already present.
    static std::vector<std::string> mergePinnedRunning(
        const std::vector<std::string>& pinned,
        const std::vector<std::string>& running);

    // V7 menu policy. hasWindow means represented open window, not focus.
    // A visible maximized window gets Minimize; every other window gets Maximize.
    static std::vector<ContextAction> contextActions(bool isPinned, bool hasWindow,
                                                     bool isVisible, bool isMaximized);
    // Compatibility-free two-state convenience for callers without window state.
    static std::vector<ContextAction> contextActions(bool isPinned, bool isRunning);
    static const char* contextActionName(ContextAction a);
};

} // namespace flamewm
#endif
