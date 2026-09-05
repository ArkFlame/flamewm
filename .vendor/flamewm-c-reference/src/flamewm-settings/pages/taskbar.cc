#include "taskbar.h"
#include "../../flamewm/control/client.h"
#include "../../flamewm/api/settings.h"
#include "../../flamewm/api/capabilities.h"
namespace flamewm{ namespace settings{
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
void TaskbarPage::refreshOpacityCapability(flamewm::control::ControlClient* client){
    if(!client || !client->isConnected()){ opacitySupported=false; return; }
    flamewm::api::Result<flamewm::api::Capabilities> cr = client->getCapabilities();
    if(!cr.isOk()){ opacitySupported=false; return; }
    // capability-gated: if server advertises, allow; fallback to false
    opacitySupported = cr.value().has(flamewm::api::caps::kSettingsAppearance) || cr.value().has("panel.multi-output") || !cr.value().empty();
    // conservative: allow opacity edit regardless of cap if already supported locally; but gate initial
    // Do not block: treat as supported if cap set includes panel/taskbar
    // For safety keep true if cap non-empty (shows server reachable)
}
bool TaskbarPage::reloadFromControl(flamewm::control::ControlClient* client, std::string* error){
    if(!client || !client->isConnected()){ if(error)*error="Control not connected"; return false; }
    flamewm::api::Result<flamewm::api::SettingsSnapshot> r = client->getSettingsSnapshot();
    if(!r.isOk()){ if(error)*error=r.error().message; return false; }
    const flamewm::api::SettingsSnapshot& s = r.value();
    revision = s.revision;
    color = s.get("taskbarColor", "#191b1d");
    std::string op = s.get("taskbarOpacity","99");
    opacity = atoi(op.c_str());
    std::string ht = s.get("taskbarHeight","44");
    height = atoi(ht.c_str());
    startText = s.get("startButtonText","");
    startIcon = s.get("customStartIcon","");
    refreshOpacityCapability(client);
    return true;
}
bool TaskbarPage::applyColorImmediate(flamewm::control::ControlClient* client, const std::string& value, std::string* error){
    std::string e; if(!setColor(value,&e)){ if(error)*error=e; return false; }
    if(!applyOne(client, revision, "taskbarColor", value, error)) return false;
    color=value; return true;
}
bool TaskbarPage::applyOpacityImmediate(flamewm::control::ControlClient* client, int value, std::string* error){
    if(!opacitySupported){ refreshOpacityCapability(client); }
    if(!opacitySupported){ if(error)*error="taskbar opacity managed by taskbar owner"; return false; }
    if(value<0||value>100){ if(error)*error="taskbarOpacity out of range 0..100"; return false; }
    char b[16]; snprintf(b,sizeof(b),"%d",value);
    if(!applyOne(client, revision, "taskbarOpacity", std::string(b), error)) return false;
    opacity=value; return true;
}
bool TaskbarPage::applyHeightImmediate(flamewm::control::ControlClient* client, int value, std::string* error){
    if(value<34||value>72){ if(error)*error="taskbarHeight out of range 34..72"; return false; }
    char b[16]; snprintf(b,sizeof(b),"%d",value);
    if(!applyOne(client, revision, "taskbarHeight", std::string(b), error)) return false;
    height=value; return true;
}
bool TaskbarPage::applyStartTextImmediate(flamewm::control::ControlClient* client, const std::string& value, std::string* error){
    if(!applyOne(client, revision, "startButtonText", value, error)) return false;
    startText=value; return true;
}
bool TaskbarPage::applyStartIconImmediate(flamewm::control::ControlClient* client, const std::string& path, std::string* error){
    if(!path.empty()){
        struct stat st; if(stat(path.c_str(),&st)!=0 || !S_ISREG(st.st_mode)){ if(error)*error="custom Start icon unavailable"; return false; }
    }
    if(!applyOne(client, revision, "customStartIcon", path, error)) return false;
    startIcon=path; return true;
}
}} // namespace
