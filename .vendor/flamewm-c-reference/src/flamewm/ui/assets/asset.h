#ifndef FLAMEWM_UI_ASSETS_ASSET_H
#define FLAMEWM_UI_ASSETS_ASSET_H

#include "../iconroles.h"

#include <string>

namespace flamewm {
namespace ui {

// Asset identifiers are semantic. They are not filesystem paths.
enum AssetId {
    AssetIdInvalid = 0,
    AssetIdStart,
    AssetIdWordmark,
    AssetIdWallpaper,
    AssetIdSearch,
    AssetIdSession,
    AssetIdNetwork,
    AssetIdFileAction,
    AssetIdSettingsCategory,
    AssetIdAppearance,
    AssetIdDesktop,
    AssetIdDisplays,
    AssetIdFonts,
    AssetIdHotkeys,
    AssetIdAbout,
    AssetIdTitleButton,
    AssetIdTaskbar,
    AssetIdTaskbarStatus,
    AssetIdMenu,
    AssetIdLauncher
};

const char* assetIdName(AssetId id);

class AssetRef {
public:
    AssetRef();
    explicit AssetRef(AssetId id);
    AssetRef(AssetId id, flamewm::IconRole role);

    // Validates a path-shaped input but never retains or opens it. Engineering
    // and prototype paths are therefore not runtime asset references.
    static AssetRef fromPath(const std::string& path);

    bool valid() const;
    AssetId id() const;
    bool hasIconRole() const;
    flamewm::IconRole iconRole() const;

private:
    AssetId id_;
    bool hasIconRole_;
    flamewm::IconRole iconRole_;
};

bool isRuntimeAssetPath(const std::string& path);

} // namespace ui
} // namespace flamewm

#endif
