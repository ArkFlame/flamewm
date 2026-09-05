#include "iconresolver.h"
#include "../ui/metrics.h"

namespace flamewm {

static const IconFallbackEntry kRegistry[] = {
    // Settings categories (semantic icons)
    { IconRoleAppearance,   "preferences-desktop-theme-global", "appearance.svg",   RenderSymbolic,  "[A]" },
    { IconRoleDesktop,      "preferences-desktop",       "desktop.svg",      RenderSymbolic,  "[D]" },
    { IconRoleDisplays,     "preferences-desktop-display","displays.svg",    RenderSymbolic,  "[M]" },
    { IconRoleFonts,        "preferences-desktop-font",  "fonts.svg",        RenderSymbolic,  "[F]" },
    { IconRoleHotkeys,      "preferences-desktop-keyboard","hotkeys.svg",    RenderSymbolic,  "[K]" },
    { IconRoleAbout,        "help-about",                "about.svg",        RenderSymbolic,  "[?]" },
    // Shell
    { IconRoleTitleButton,  "window-close",              "title-close.svg", RenderSymbolic,  "[X]" },
    { IconRoleTaskbar,      "start-here-kde",             "flamewm-start.svg", RenderSymbolic,  "[T]" },
    { IconRoleTaskbarStatus,"audio-volume-high",         "status.svg",       RenderSymbolic,  "[S]" },
    { IconRoleMenu,         "application-menu",          "menu.svg",         RenderSymbolic,  "[M]" },
    { IconRoleLauncher,     "application-x-executable",  "launcher.svg",     RenderFullColor, "[L]" },
    { IconRoleSearch,       "edit-find",                 "fallback-search.svg", RenderSymbolic,  "[Q]" },
    { IconRoleSession,      "system-shutdown",           "fallback-session.svg", RenderSymbolic,  "[P]" },
    { IconRoleNetwork,      "network-connect",            "fallback-network.svg", RenderSymbolic,  "[N]" },
    { IconRoleFileAction,   "folder-new",                "fallback-fileaction.svg", RenderSymbolic,  "[F]" },
};

const IconFallbackEntry* iconFallbackRegistry(size_t* outCount) {
    if (outCount) *outCount = sizeof(kRegistry)/sizeof(kRegistry[0]);
    return kRegistry;
}

const IconFallbackEntry* iconFallbackForRole(IconRole role) {
    for (size_t i = 0; i < sizeof(kRegistry)/sizeof(kRegistry[0]); ++i) {
        if (kRegistry[i].role == role) return &kRegistry[i];
    }
    return 0;
}

int logicalIconSizeForRole(IconRole role) {
    return SimpleIconResolver::logicalSizeForRole(role);
}

int SimpleIconResolver::logicalSizeForRole(IconRole role) {
    switch (role) {
        case IconRoleTitleButton:    return 16;
        case IconRoleTaskbarStatus:  return 22;
        case IconRoleMenu:           return 22;
        case IconRoleSearch:         return 16;
        case IconRoleSession:        return 22;
        case IconRoleNetwork:        return 22;
        case IconRoleFileAction:     return 16;
        case IconRoleTaskbar:        return 32;
        case IconRoleLauncher:       return 32;
        case IconRoleAppearance:
        case IconRoleDesktop:
        case IconRoleDisplays:
        case IconRoleFonts:
        case IconRoleHotkeys:
        case IconRoleAbout:
        case IconRoleSettingsCategory: return 32;
        default: return 22;
    }
}

SimpleIconResolver::SimpleIconResolver()
    : themeGeneration_(0), breezeAvailable_(true), systemThemeAvailable_(true), packagedFallbackAvailable_(true)
{
    for (int i = 0; i < 16; ++i) overrides_[i] = "";
}

IconLookupResult SimpleIconResolver::resolve(IconRole role, int scalePercent, int /*themeGeneration*/) {
    IconLookupResult r;
    int logical = logicalSizeForRole(role);
    if (!isSupportedScale(scalePercent)) scalePercent = nearestSupportedScale(scalePercent);
    r.physicalSize = FlameMetrics::logicalToPhysical(logical, scalePercent);
    const IconFallbackEntry* e = iconFallbackForRole(role);
    RenderMode mode = e ? e->renderMode : RenderSymbolic;

    // 1) Flame override
    if (role >= 0 && role < 16 && !overrides_[role].empty()) {
        r.pathOrGlyph = overrides_[role];
        r.source = IconSourceFlameOverride;
        r.renderMode = mode;
        r.found = true;
        return r;
    }
    // 2) Breeze
    if (breezeAvailable_ && e) {
        r.pathOrGlyph = std::string("breeze:") + e->freedesktopName;
        r.source = IconSourceBreeze;
        r.renderMode = mode;
        r.found = true;
        return r;
    }
    // 3) System theme
    if (systemThemeAvailable_ && e) {
        r.pathOrGlyph = std::string("system:") + e->freedesktopName;
        r.source = IconSourceSystemTheme;
        r.renderMode = mode;
        r.found = true;
        return r;
    }
    // 4) Packaged fallback
    if (packagedFallbackAvailable_ && e) {
        r.pathOrGlyph = std::string("fallback:") + e->fallbackFilename;
        r.source = IconSourcePackagedFallback;
        r.renderMode = mode;
        r.found = true;
        return r;
    }
    // 5) Built-in glyph
    r.pathOrGlyph = e ? e->builtinGlyph : "[?]";
    r.source = IconSourceBuiltinGlyph;
    r.renderMode = RenderSymbolic;
    r.found = true;
    return r;
}

void SimpleIconResolver::setFlameOverride(IconRole role, const std::string& path) {
    if (role >= 0 && role < 16) overrides_[role] = path;
}
void SimpleIconResolver::clearFlameOverride(IconRole role) {
    if (role >= 0 && role < 16) overrides_[role].clear();
}
void SimpleIconResolver::notifyThemeChanged(int newGeneration) {
    themeGeneration_ = newGeneration;
}

} // namespace flamewm
