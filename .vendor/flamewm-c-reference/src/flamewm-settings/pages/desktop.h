#ifndef FLAMEWM_SETTINGS_DESKTOP_H
#define FLAMEWM_SETTINGS_DESKTOP_H
#include <string>
#include <cstdint>
#include <sys/stat.h>
namespace flamewm { namespace control { class ControlClient; } }
namespace flamewm { namespace settings {
struct DesktopPage {
    std::string wallpaper;
    int desktopSelectionFillOpacity;
    int windowSnapPreviewFillOpacity;
    bool stickyNotesEnabled;
    bool stickyDisableConfirmationPending;
    uint64_t revision;
    flamewm::control::ControlClient* boundControlClient;
    DesktopPage(): wallpaper(), desktopSelectionFillOpacity(20), windowSnapPreviewFillOpacity(20), stickyNotesEnabled(true), stickyDisableConfirmationPending(false), revision(0), boundControlClient(0) {}
    bool reloadFromControl(flamewm::control::ControlClient* client, std::string* error);
    bool applyWallpaperImmediate(flamewm::control::ControlClient* client, const std::string& path, std::string* error);
    bool applyDesktopOpacityImmediate(flamewm::control::ControlClient* client, int value, std::string* error);
    bool applySnapOpacityImmediate(flamewm::control::ControlClient* client, int value, std::string* error);
    bool applyStickyImmediate(flamewm::control::ControlClient* client, bool enabled, bool confirmed, std::string* error);
    bool setWallpaper(const std::string& path, std::string* err) {
        if (path.empty()) { wallpaper.clear(); return true; }
        struct stat info;
        if (stat(path.c_str(), &info) != 0 || !S_ISREG(info.st_mode)) {
            if (err) *err = "wallpaper unavailable";
            return false;
        }
        wallpaper = path;
        return true;
    }
    bool setStickyNotesEnabled(bool enabled, bool confirmed, std::string* err) {
        if (!enabled && stickyNotesEnabled && !confirmed) {
            stickyDisableConfirmationPending = true;
            if (err) *err = "confirm disabling sticky notes; all notes will be removed";
            return false;
        }
        stickyNotesEnabled = enabled;
        stickyDisableConfirmationPending = false;
        return true;
    }
    bool setDesktopOpacity(int v, std::string* err){
        if(v<0||v>60){ if(err)*err="DesktopSelectionFillOpacity 0..60"; return false; }
        desktopSelectionFillOpacity=v; return true;
    }
    bool setSnapOpacity(int v, std::string* err){
        if(v<0||v>60){ if(err)*err="WindowSnapPreviewFillOpacity 0..60"; return false; }
        windowSnapPreviewFillOpacity=v; return true;
    }
    bool borderStrongAtZero() const { return true; }
};
}} // namespace
#endif
