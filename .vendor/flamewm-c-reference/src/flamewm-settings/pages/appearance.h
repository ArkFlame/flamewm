#ifndef FLAMEWM_SETTINGS_APPEARANCE_H
#define FLAMEWM_SETTINGS_APPEARANCE_H
#include <string>
#include <vector>
#include <cstdint>
namespace flamewm { namespace control { class ControlClient; } }
namespace flamewm { namespace settings {
struct AppearancePage {
    std::string selectedPreset;
    std::string customColor;
    std::string iconTheme;
    bool liveApply;
    uint64_t revision;
    AppearancePage(): selectedPreset("#EF4048"), iconTheme("*:-HighContrast"), liveApply(true), revision(0), boundControlClient(0) {}
    static std::vector<std::string> presets() {
        std::vector<std::string> v;
        v.push_back("#EF4048");
        v.push_back("#FF3B30");
        v.push_back("#FF9500");
        v.push_back("#007AFF");
        return v;
    }
    bool setPreset(const std::string& hex, std::string* err);
    bool setCustom(const std::string& hex, std::string* err);
    std::string effectiveAccent() const { return customColor.empty()?selectedPreset:customColor; }
    static std::vector<std::string> iconThemes();
    static void invalidateIconCache();
    bool setIconThemeChecked(const std::string& theme, std::string* err) {
        if (theme.empty()) { if (err) *err = "icon theme cannot be empty"; return false; }
        setIconTheme(theme);
        return true;
    }
    void setIconTheme(const std::string& theme) { iconTheme = theme; invalidateIconCache(); }
    // Control-only: effective state lives in WM via ControlClient.
    bool reloadFromControl(flamewm::control::ControlClient* client, std::string* error);
    // Immediate apply — writes through ControlClient::applySettings with expected revision.
    bool applyAccentImmediate(flamewm::control::ControlClient* client, const std::string& hex, std::string* error);
    bool applyIconThemeImmediate(flamewm::control::ControlClient* client, const std::string& theme, std::string* error);
    flamewm::control::ControlClient* boundControlClient;
};
}} // namespace
#endif
