#include "desktop.h"
#include "../../flamewm/control/client.h"
#include "../../flamewm/api/settings.h"
#include <cstdlib>
namespace flamewm{ namespace settings{
static std::string toStr(int v){ char b[32]; snprintf(b,sizeof(b),"%d",v); return std::string(b); }
bool DesktopPage::reloadFromControl(flamewm::control::ControlClient* client, std::string* error){
    if(!client || !client->isConnected()){ if(error)*error="Control not connected"; return false; }
    flamewm::api::Result<flamewm::api::SettingsSnapshot> r = client->getSettingsSnapshot();
    if(!r.isOk()){ if(error)*error=r.error().message; return false; }
    const flamewm::api::SettingsSnapshot& s = r.value();
    revision = s.revision;
    wallpaper = s.get("wallpaper", "");
    std::string d = s.get("DesktopSelectionFillOpacity","20");
    std::string w = s.get("WindowSnapPreviewFillOpacity","20");
    desktopSelectionFillOpacity = atoi(d.c_str());
    windowSnapPreviewFillOpacity = atoi(w.c_str());
    std::string sticky = s.get("stickyNotesEnabled","1");
    stickyNotesEnabled = !(sticky=="0"||sticky=="false"||sticky=="no");
    return true;
}
static bool applyOne(flamewm::control::ControlClient* c, uint64_t& rev, const std::string& k, const std::string& v, std::string* err){
    if(!c || !c->isConnected()){ if(err)*err="Control not connected"; return false; }
    if(rev==0){
        flamewm::api::Result<flamewm::api::SettingsSnapshot> r=c->getSettingsSnapshot();
        if(!r.isOk()){ if(err)*err=r.error().message; return false; }
        rev=r.value().revision;
    }
    std::vector<flamewm::api::SettingsChange> ch; ch.push_back(flamewm::api::SettingsChange(k,v));
    flamewm::api::Status st=c->applySettings(rev,ch);
    if(!st.ok() && st.code==flamewm::api::Error::StaleRevision){
        flamewm::api::Result<flamewm::api::SettingsSnapshot> r=c->getSettingsSnapshot();
        if(r.isOk()){ rev=r.value().revision; st=c->applySettings(rev,ch); }
    }
    if(!st.ok()){ if(err)*err=st.message; return false; }
    flamewm::api::Result<flamewm::api::SettingsSnapshot> r=c->getSettingsSnapshot();
    if(r.isOk()) rev=r.value().revision;
    return true;
}
bool DesktopPage::applyWallpaperImmediate(flamewm::control::ControlClient* client, const std::string& path, std::string* error){
    if(!path.empty()){
        struct stat st; if(stat(path.c_str(),&st)!=0 || !S_ISREG(st.st_mode)){ if(error)*error="wallpaper unavailable"; return false; }
    }
    if(!applyOne(client, revision, "wallpaper", path, error)) return false;
    wallpaper = path;
    return true;
}
bool DesktopPage::applyDesktopOpacityImmediate(flamewm::control::ControlClient* client, int v, std::string* error){
    if(v<0||v>60){ if(error)*error="DesktopSelectionFillOpacity 0..60"; return false; }
    if(!applyOne(client, revision, "DesktopSelectionFillOpacity", toStr(v), error)) return false;
    desktopSelectionFillOpacity=v;
    return true;
}
bool DesktopPage::applySnapOpacityImmediate(flamewm::control::ControlClient* client, int v, std::string* error){
    if(v<0||v>60){ if(error)*error="WindowSnapPreviewFillOpacity 0..60"; return false; }
    if(!applyOne(client, revision, "WindowSnapPreviewFillOpacity", toStr(v), error)) return false;
    windowSnapPreviewFillOpacity=v;
    return true;
}
bool DesktopPage::applyStickyImmediate(flamewm::control::ControlClient* client, bool enabled, bool confirmed, std::string* error){
    if(!enabled && stickyNotesEnabled && !confirmed){
        stickyDisableConfirmationPending=true;
        if(error)*error="confirm disabling sticky notes; all notes will be removed";
        return false;
    }
    if(!applyOne(client, revision, "stickyNotesEnabled", enabled?"1":"0", error)) return false;
    stickyNotesEnabled=enabled;
    stickyDisableConfirmationPending=false;
    return true;
}
}} // namespace
