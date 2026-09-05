#ifndef FLAMEWM_UI_ICONRESOLVER_H
#define FLAMEWM_UI_ICONRESOLVER_H

#include "iconroles.h"
#include <string>
#include <vector>

namespace flamewm {

// Icon resolution chain:
//   FlameOverride -> Breeze/installed theme -> system theme -> packaged fallback -> built-in glyph
enum IconSource {
    IconSourceFlameOverride = 0,
    IconSourceBreeze,
    IconSourceSystemTheme,
    IconSourcePackagedFallback,
    IconSourceBuiltinGlyph
};

inline const char* iconSourceName(IconSource s) {
    switch (s) {
        case IconSourceFlameOverride:    return "FlameOverride";
        case IconSourceBreeze:           return "Breeze";
        case IconSourceSystemTheme:      return "SystemTheme";
        case IconSourcePackagedFallback: return "PackagedFallback";
        case IconSourceBuiltinGlyph:     return "BuiltinGlyph";
        default: return "Unknown";
    }
}

struct IconLookupResult {
    std::string pathOrGlyph;
    IconSource source;
    RenderMode renderMode;
    int physicalSize;
    bool found;

    IconLookupResult()
        : source(IconSourceBuiltinGlyph), renderMode(RenderSymbolic), physicalSize(0), found(false) {}
};

struct IconFallbackEntry {
    IconRole role;
    const char* freedesktopName;
    const char* fallbackFilename;
    RenderMode renderMode;
    const char* builtinGlyph;
};

// Registry of critical fallback icons — license: LGPL-3.0, Breeze 6.29.0 provenance
// Only audited subset; not a full vendor.
const IconFallbackEntry* iconFallbackRegistry(size_t* outCount);
const IconFallbackEntry* iconFallbackForRole(IconRole role);

class IconResolver {
public:
    virtual ~IconResolver() {}
    virtual IconLookupResult resolve(IconRole role, int scalePercent, int themeGeneration) = 0;
    virtual void setFlameOverride(IconRole role, const std::string& path) = 0;
    virtual void clearFlameOverride(IconRole role) = 0;
    virtual void notifyThemeChanged(int newGeneration) = 0;
};

// Pure-logic (no X) resolver used for tests / headless validation.
class SimpleIconResolver : public IconResolver {
public:
    SimpleIconResolver();
    virtual ~SimpleIconResolver() {}

    virtual IconLookupResult resolve(IconRole role, int scalePercent, int themeGeneration);
    virtual void setFlameOverride(IconRole role, const std::string& path);
    virtual void clearFlameOverride(IconRole role);
    virtual void notifyThemeChanged(int newGeneration);

    // Test hooks
    void setBreezeAvailable(bool v) { breezeAvailable_ = v; }
    void setSystemThemeAvailable(bool v) { systemThemeAvailable_ = v; }
    void setPackagedFallbackAvailable(bool v) { packagedFallbackAvailable_ = v; }
    int themeGeneration() const { return themeGeneration_; }
    static int logicalSizeForRole(IconRole role);
    static int logicalSizeForRoleStatic(IconRole r) { return logicalSizeForRole(r); }

private:
    int themeGeneration_;
    bool breezeAvailable_;
    bool systemThemeAvailable_;
    bool packagedFallbackAvailable_;
    std::string overrides_[16];
};

// Helper: logical icon size by role (at 100%)
int logicalIconSizeForRole(IconRole role);

} // namespace flamewm
#endif
