#include "fonts.h"
#include "../../flamewm/control/client.h"
#include "../../flamewm/api/settings.h"
#include <cstdlib>
namespace flamewm{ namespace settings{
bool FontsPage::reloadFromControl(flamewm::control::ControlClient* client, std::string* error){
    if(!client || !client->isConnected()){ if(error)*error="Control not connected"; return false; }
    flamewm::api::Result<flamewm::api::SettingsSnapshot> r = client->getSettingsSnapshot();
    if(!r.isOk()){ if(error)*error=r.error().message; return false; }
    const flamewm::api::SettingsSnapshot& s = r.value();
    revision = s.revision;
    family = s.get("fontFamily","IBM Plex Sans");
    if(family.empty()) family="IBM Plex Sans";
    std::string bold = s.get("fontBold","0");
    globalBold = (bold=="1"||bold=="true"||bold=="yes");
    std::string off = s.get("fontSizeOffset","0");
    sizeOffset = atoi(off.c_str());
    return true;
}
bool FontsPage::applyImmediate(flamewm::control::ControlClient* client, const std::string& fam, bool bold, int offset, std::string* error){
    if(fam.empty()){ if(error)*error="font family cannot be empty"; return false; }
    if(!isSupportedFamily(fam)){ if(error)*error="font family is not available"; return false; }
    if(offset < -3 || offset > 6){ if(error)*error="font size offset out of range -3..6"; return false; }
    FontCache staged;
    if(!buildCache(fam,bold,offset,&staged,error)){ if(error && error->empty()) *error="font load failed, rollback"; return false; }
    if(!client || !client->isConnected()){ if(error)*error="Control not connected"; return false; }
    if(revision==0){
        std::string e;
        if(!reloadFromControl(client,&e)){ if(error)*error=e; return false; }
    }
    std::vector<flamewm::api::SettingsChange> ch;
    ch.push_back(flamewm::api::SettingsChange("fontFamily", fam));
    ch.push_back(flamewm::api::SettingsChange("fontBold", bold?"1":"0"));
    char b[16]; snprintf(b,sizeof(b),"%d",offset);
    ch.push_back(flamewm::api::SettingsChange("fontSizeOffset", std::string(b)));
    flamewm::api::Status st = client->applySettings(revision, ch);
    if(!st.ok() && st.code==flamewm::api::Error::StaleRevision){
        std::string e;
        if(reloadFromControl(client,&e)) st = client->applySettings(revision, ch);
    }
    if(!st.ok()){ if(error)*error=st.message; return false; }
    family=fam; globalBold=bold; sizeOffset=offset;
    std::string e; reloadFromControl(client,&e);
    return true;
}
}} // namespace
