#include "pages.h"
#include "flamewm/ui/iconroles.h"
namespace flamewm { namespace settings { namespace ui {
const std::vector<PageSpec>& pageSpecs() {
    static const PageSpec values[] = {
        {"Appearance", "Seven accent presets, icon theme and live preview", IconRoleAppearance},
        {"Desktop", "Wallpaper, watermark, reset and opacity", IconRoleDesktop},
        {"Taskbar", "Color, opacity, height and Start branding", IconRoleTaskbar},
        {"Displays", "Output topology, primary state, mode and scale", IconRoleDisplays},
        {"Fonts", "Family, bold and size offset from -3 to 6", IconRoleFonts},
        {"Hotkeys", "Five ordered actions with capture, clear and reset", IconRoleHotkeys},
        {"About", "FlameWM branding and three labeled links", IconRoleAbout}
    };
    static const std::vector<PageSpec> result(values, values + sizeof(values) / sizeof(values[0]));
    return result;
}

const std::vector<AppearancePresetSpec>& appearancePresets() {
    static const AppearancePresetSpec values[] = {
        {"Flame", "#ef4048", true},
        {"Ember", "#ff7043", false},
        {"Amber", "#f0b429", false},
        {"Meadow", "#55b86b", false},
        {"Sky", "#3daee9", false},
        {"Violet", "#7e66d7", false},
        {"Rose", "#d84ca3", false}
    };
    static const std::vector<AppearancePresetSpec> result(
        values, values + sizeof(values) / sizeof(values[0]));
    return result;
}

const DesktopContentSpec& desktopContent() {
    static const DesktopContentSpec value = {true, true, 20, 0, 60};
    return value;
}

const std::vector<std::string>& fontFamilies() {
    static const char* values[] = {
        "IBM Plex Sans", "Noto Sans", "Inter", "Segoe UI",
        "Ubuntu", "Arial", "sans-serif"
    };
    static const std::vector<std::string> result(
        values, values + sizeof(values) / sizeof(values[0]));
    return result;
}

int fontSizeOffsetMinimum() { return -3; }
int fontSizeOffsetMaximum() { return 6; }

const std::vector<HotkeySpec>& hotkeySpecs() {
    static const HotkeySpec values[] = {
        {"ToggleStartMenu", "Toggle Start menu"},
        {"WorkspaceLeft", "Desktop left"},
        {"WorkspaceRight", "Desktop right"},
        {"WorkspaceUp", "Desktop up"},
        {"WorkspaceDown", "Desktop down"}
    };
    static const std::vector<HotkeySpec> result(
        values, values + sizeof(values) / sizeof(values[0]));
    return result;
}

std::vector<DisplayTopologySpec> displayTopology(
        const flamewm::api::DisplaySnapshot& snapshot) {
    std::vector<DisplayTopologySpec> result;
    for (size_t i = 0; i < snapshot.outputs.size(); ++i) {
        const flamewm::api::OutputSnapshot& output = snapshot.outputs[i];
        DisplayTopologySpec item = {
            output.id.key, output.connector, output.primary, output.connected};
        result.push_back(item);
    }
    return result;
}

const flamewm::api::OutputSnapshot* primaryOutput(
        const flamewm::api::DisplaySnapshot& snapshot) {
    for (size_t i = 0; i < snapshot.outputs.size(); ++i) {
        if (snapshot.outputs[i].primary) return &snapshot.outputs[i];
    }
    return 0;
}

const BrandingAssetSpec& aboutBrandingAsset() {
    static const BrandingAssetSpec value = {
        flamewm::ui::AssetIdWordmark,
        "branding/flamewm-wordmark.png",
        true
    };
    return value;
}

const std::vector<AboutLinkSpec>& aboutLinks() {
    static const AboutLinkSpec values[] = {
        {"Donate", "https://paypal.me/LinsaFTW"},
        {"Source Code", "https://github.com/arkflame/flamewm"},
        {"Website", "https://wm.arkflame.com"}
    };
    static const std::vector<AboutLinkSpec> result(
        values, values + sizeof(values) / sizeof(values[0]));
    return result;
}
}}}
