#ifndef FLAMEWM_SETTINGS_HOTKEYS_H
#define FLAMEWM_SETTINGS_HOTKEYS_H
#include "../../flamewm/api/shortcuts.h"
#include <map>
#include <string>
#include <cstdint>
namespace flamewm { namespace control { class ControlClient; } }
namespace flamewm { namespace settings {
struct HotkeysPage {
    bool capturing;
    std::string captureAction;
    uint64_t revision;
    flamewm::api::ShortcutSnapshot cachedSnapshot;
    flamewm::control::ControlClient* boundControlClient;
    HotkeysPage(): capturing(false), captureAction(flamewm::api::ActionToggleStartMenu), revision(0), cachedSnapshot(), boundControlClient(0) {}
    bool reloadFromControl(flamewm::control::ControlClient* client, std::string* error);
    void beginCapture(const std::string& action){ capturing=true; captureAction=action; }
    bool handleCaptureKey(flamewm::control::ControlClient* client, const std::string& keysym, std::string* err);
    bool commitCapture(flamewm::control::ControlClient* client, const std::string& keysym, std::string* err);
    // Escape clears capture and maps to unassigned (Not assigned), never stored.
    bool isCapturing() const { return capturing; }
    bool setBinding(flamewm::control::ControlClient* client, const std::string& action, const std::string& keysym, std::string* err);
    bool clearBinding(flamewm::control::ControlClient* client, const std::string& action, std::string* err);
    bool resetBinding(flamewm::control::ControlClient* client, const std::string& action, std::string* err);
    static std::string displayFor(const flamewm::api::KeyBinding& b){ return b.assigned()?b.key:"Not assigned"; }
    static bool isEscape(const std::string& s){ return s=="Escape"; }
};
}} // namespace
#endif
