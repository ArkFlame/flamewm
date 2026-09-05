#ifndef FLAMEWM_SETTINGS_KEYCAPTURE_H
#define FLAMEWM_SETTINGS_KEYCAPTURE_H
#include <string>
// C-CTRL-02/C-UI-01: KeyCaptureWidget is presentation for HotkeysPage;
// commit goes through ControlClient::applyShortcuts when
// FLAMEWM_USE_CONTROL. No engine includes here.
namespace flamewm { namespace settings {
struct KeyCaptureWidget {
    bool capturing;
    std::string captured;
    bool escaped;
    KeyCaptureWidget(): capturing(false), escaped(false) {}
    void begin(){ capturing=true; captured.clear(); escaped=false; }
    // Escape->clear contract: Escape clears to null, never stored.
    bool onKey(const std::string& keysym){
        if(!capturing) return false;
        if(keysym=="Escape"){ captured.clear(); escaped=true; capturing=false; return true; }
        captured=keysym; capturing=false; escaped=false; return true;
    }
    std::string display() const {
        if(escaped || captured.empty()) return "Not assigned";
        return captured;
    }
    bool isNull() const { return escaped || captured.empty(); }
};
}} // namespace
#endif
