#ifndef FLAMEWM_SETTINGS_UI_PAGES_H
#define FLAMEWM_SETTINGS_UI_PAGES_H
#include "../../flamewm/api/display.h"
#include "../../flamewm/ui/assets/asset.h"
#include <string>
#include <vector>
namespace flamewm { namespace settings { namespace ui {
struct PageSpec { const char* name; const char* description; int iconRole; };
const std::vector<PageSpec>& pageSpecs();

struct AppearancePresetSpec {
    const char* name;
    const char* color;
    bool builtIn;
};
const std::vector<AppearancePresetSpec>& appearancePresets();

struct DesktopContentSpec {
    bool watermarkVisible;
    bool resetAvailable;
    int opacity;
    int opacityMinimum;
    int opacityMaximum;
};
const DesktopContentSpec& desktopContent();

const std::vector<std::string>& fontFamilies();
int fontSizeOffsetMinimum();
int fontSizeOffsetMaximum();

struct HotkeySpec {
    const char* action;
    const char* label;
};
const std::vector<HotkeySpec>& hotkeySpecs();

struct DisplayTopologySpec {
    std::string id;
    std::string connector;
    bool primary;
    bool connected;
};
std::vector<DisplayTopologySpec> displayTopology(const flamewm::api::DisplaySnapshot& snapshot);
const flamewm::api::OutputSnapshot* primaryOutput(const flamewm::api::DisplaySnapshot& snapshot);

struct BrandingAssetSpec {
    flamewm::ui::AssetId asset;
    const char* path;
    bool preserveAspect;
};
const BrandingAssetSpec& aboutBrandingAsset();

struct AboutLinkSpec {
    const char* label;
    const char* url;
};
const std::vector<AboutLinkSpec>& aboutLinks();
}}}
#endif
