#ifndef FLAMEWM_SETTINGS_PREVIEWCARD_H
#define FLAMEWM_SETTINGS_PREVIEWCARD_H
#include <string>
// C-CTRL-02/C-UI-01: PreviewCard is presentation-only; backed by
// Appearance/Desktop settings snapshot (future: ControlClient).
namespace flamewm { namespace settings {
struct PreviewCard {
    std::string title;
    std::string accent;
    bool valid;
    PreviewCard(): valid(false) {}
    void update(const std::string& t, const std::string& a){ title=t; accent=a; valid=true; }
};
}} // namespace
#endif
