#ifndef FLAMEWM_SETTINGS_COLORPICKER_H
#define FLAMEWM_SETTINGS_COLORPICKER_H
#include <string>
// C-CTRL-02/C-UI-01: ColorPickerWidget is presentation-only; future
// accent lives in ControlClient::getSettingsSnapshot()/applySettings.
// No Control/engine/X11 includes in this widget — data flows from page.
namespace flamewm { namespace settings {
struct ColorPickerWidget {
    std::string color; // hex
    std::string staged;
    void setColor(const std::string& hex){ color=hex; staged=hex; }
    void stage(const std::string& hex){ staged=hex; }
    void commit(){ color=staged; }
    void cancel(){ staged=color; }
};
}} // namespace
#endif
