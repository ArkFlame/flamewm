#include "asset.h"

namespace flamewm {
namespace ui {

namespace {

bool isKnownAssetId(AssetId id) {
    return id >= AssetIdStart && id <= AssetIdLauncher;
}

bool hasUnsafePathComponent(const std::string& path) {
    if (path.empty() || path.find('\0') != std::string::npos ||
        path.find('\\') != std::string::npos) return true;

    std::string component;
    for (std::string::const_iterator it = path.begin(); it != path.end(); ++it) {
        if (*it == '/') {
            if (component == ".." || component == ".") return true;
            component.clear();
        } else {
            component += *it;
        }
    }
    return component == ".." || component == ".";
}

bool hasRuntimeRoot(const std::string& path) {
    // Accept the source-tree-relative install spelling and normal absolute
    // install prefixes, including IceWM's default share directory.
    const std::string relativeRoot("lib/flamewm/");
    if (path.compare(0, relativeRoot.size(), relativeRoot) == 0)
        return path.size() > relativeRoot.size();

    const std::string libRoot("/lib/flamewm/");
    std::string::size_type position = path.find(libRoot);
    if (position != std::string::npos)
        return path.size() > position + libRoot.size();

    const std::string icewmRoot("/share/icewm/flamewm/");
    position = path.find(icewmRoot);
    return position != std::string::npos && path.size() > position + icewmRoot.size();
}

bool endsWith(const std::string& path, const char* suffix) {
    const std::string value(suffix);
    return path.size() >= value.size() &&
           path.compare(path.size() - value.size(), value.size(), value) == 0;
}

} // namespace

const char* assetIdName(AssetId id) {
    switch (id) {
        case AssetIdStart: return "Start";
        case AssetIdWordmark: return "Wordmark";
        case AssetIdWallpaper: return "Wallpaper";
        case AssetIdSearch: return "Search";
        case AssetIdSession: return "Session";
        case AssetIdNetwork: return "Network";
        case AssetIdFileAction: return "FileAction";
        case AssetIdSettingsCategory: return "SettingsCategory";
        case AssetIdAppearance: return "Appearance";
        case AssetIdDesktop: return "Desktop";
        case AssetIdDisplays: return "Displays";
        case AssetIdFonts: return "Fonts";
        case AssetIdHotkeys: return "Hotkeys";
        case AssetIdAbout: return "About";
        case AssetIdTitleButton: return "TitleButton";
        case AssetIdTaskbar: return "Taskbar";
        case AssetIdTaskbarStatus: return "TaskbarStatus";
        case AssetIdMenu: return "Menu";
        case AssetIdLauncher: return "Launcher";
        default: return "Invalid";
    }
}

AssetRef::AssetRef()
    : id_(AssetIdInvalid), hasIconRole_(false), iconRole_(flamewm::IconRoleMenu) {}

AssetRef::AssetRef(AssetId id)
    : id_(isKnownAssetId(id) ? id : AssetIdInvalid), hasIconRole_(false), iconRole_(flamewm::IconRoleMenu) {}

AssetRef::AssetRef(AssetId id, flamewm::IconRole role)
    : id_(isKnownAssetId(id) ? id : AssetIdInvalid), hasIconRole_(true), iconRole_(role) {}

AssetRef AssetRef::fromPath(const std::string& path) {
    if (!isRuntimeAssetPath(path)) return AssetRef();

    if (endsWith(path, "/icons/flamewm-start.svg")) return AssetRef(AssetIdStart);
    if (endsWith(path, "/branding/flamewm-wordmark.png")) return AssetRef(AssetIdWordmark);
    if (endsWith(path, "/themes/flame-dark/flamewm-wallpaper.webp")) return AssetRef(AssetIdWallpaper);
    if (endsWith(path, "/icons/fallback-search.svg")) return AssetRef(AssetIdSearch);
    if (endsWith(path, "/icons/fallback-session.svg")) return AssetRef(AssetIdSession);
    if (endsWith(path, "/icons/fallback-network.svg")) return AssetRef(AssetIdNetwork);
    if (endsWith(path, "/icons/fallback-fileaction.svg")) return AssetRef(AssetIdFileAction);
    if (endsWith(path, "/icons/appearance.svg")) return AssetRef(AssetIdAppearance);
    if (endsWith(path, "/icons/desktop.svg")) return AssetRef(AssetIdDesktop);
    if (endsWith(path, "/icons/displays.svg")) return AssetRef(AssetIdDisplays);
    if (endsWith(path, "/icons/fonts.svg")) return AssetRef(AssetIdFonts);
    if (endsWith(path, "/icons/hotkeys.svg")) return AssetRef(AssetIdHotkeys);
    if (endsWith(path, "/icons/about.svg")) return AssetRef(AssetIdAbout);
    return AssetRef();
}

bool AssetRef::valid() const {
    return isKnownAssetId(id_);
}

AssetId AssetRef::id() const {
    return id_;
}

bool AssetRef::hasIconRole() const {
    return hasIconRole_;
}

flamewm::IconRole AssetRef::iconRole() const {
    return iconRole_;
}

bool isRuntimeAssetPath(const std::string& path) {
    // This is deliberately lexical: validating a candidate must not perform
    // filesystem access or turn an engineering path into a runtime reference.
    return !hasUnsafePathComponent(path) && hasRuntimeRoot(path);
}

} // namespace ui
} // namespace flamewm
