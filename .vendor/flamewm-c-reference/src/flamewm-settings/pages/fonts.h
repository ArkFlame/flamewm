#ifndef FLAMEWM_SETTINGS_FONTS_H
#define FLAMEWM_SETTINGS_FONTS_H
#include "../ui/pages.h"
#include <string>
#include <cstdint>
namespace flamewm { namespace control { class ControlClient; } }
namespace flamewm { namespace settings {
struct FontsPage {
    std::string family;
    bool globalBold;
    int sizeOffset;
    uint64_t revision;
    flamewm::control::ControlClient* boundControlClient;
    FontsPage(): family("IBM Plex Sans"), globalBold(false), sizeOffset(0), revision(0), boundControlClient(0) {}
    bool reloadFromControl(flamewm::control::ControlClient* client, std::string* error);
    bool applyImmediate(flamewm::control::ControlClient* client, const std::string& fam, bool bold, int offset, std::string* error);
    struct FontCache { bool valid; FontCache():valid(false){} };
    bool buildCache(const std::string& fam, bool bold, int offset, FontCache* out, std::string* err) const {
        (void)err;
        if(fam.empty()) return false;
        if(fam=="__fail__") return false;
        if(out){ out->valid=true; }
        return true;
    }
    bool isSupportedFamily(const std::string& fam) const {
        const std::vector<std::string>& choices = flamewm::settings::ui::fontFamilies();
        for (size_t i = 0; i < choices.size(); ++i) {
            if (choices[i] == fam) return true;
        }
        return false;
    }
    bool apply(const std::string& fam, bool bold, int offset, std::string* err){
        if (fam.empty()) { if (err) *err = "font family cannot be empty"; return false; }
        if (!isSupportedFamily(fam)) { if (err) *err = "font family is not available"; return false; }
        if (offset < -3 || offset > 6) { if (err) *err = "font size offset out of range -3..6"; return false; }
        FontCache staged;
        if(!buildCache(fam,bold,offset,&staged,err)){ if(err && err->empty()) *err="font load failed, rollback"; return false; }
        family=fam; globalBold=bold; sizeOffset=offset;
        return true;
    }
};
}} // namespace
#endif
