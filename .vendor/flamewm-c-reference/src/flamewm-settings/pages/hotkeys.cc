#include "hotkeys.h"
#include "../../flamewm/control/client.h"
#include "../../flamewm/api/shortcuts.h"
#include "../../flamewm/api/errors.h"
namespace flamewm{ namespace settings{
bool HotkeysPage::reloadFromControl(flamewm::control::ControlClient* client, std::string* error){
    if(!client || !client->isConnected()){ if(error)*error="Control not connected"; return false; }
    flamewm::api::Result<flamewm::api::ShortcutSnapshot> r = client->getShortcutSnapshot();
    if(!r.isOk()){ if(error)*error=r.error().message; return false; }
    cachedSnapshot = r.value();
    revision = r.value().revision;
    return true;
}
bool HotkeysPage::setBinding(flamewm::control::ControlClient* client, const std::string& action, const std::string& keysym, std::string* err){
    if(!client || !client->isConnected()){ if(err)*err="Control not connected"; return false; }
    if(action.empty()){ if(err)*err="invalid action"; return false; }
    if(revision==0){
        std::string e; if(!reloadFromControl(client,&e)){ if(err)*err=e; return false; }
    }
    // Escape never stored: treat as clear
    if(isEscape(keysym)) return clearBinding(client, action, err);
    flamewm::api::Status st = client->setBinding(action, keysym, revision);
    if(!st.ok() && st.code==flamewm::api::Error::StaleRevision){
        std::string e;
        if(reloadFromControl(client,&e)) st = client->setBinding(action, keysym, revision);
    }
    if(!st.ok()){ if(err)*err=st.message; return false; }
    std::string e; reloadFromControl(client,&e);
    return true;
}
bool HotkeysPage::clearBinding(flamewm::control::ControlClient* client, const std::string& action, std::string* err){
    if(!client || !client->isConnected()){ if(err)*err="Control not connected"; return false; }
    if(revision==0){
        std::string e; if(!reloadFromControl(client,&e)){ if(err)*err=e; return false; }
    }
    flamewm::api::Status st = client->clearBinding(action, revision);
    if(!st.ok() && st.code==flamewm::api::Error::StaleRevision){
        std::string e;
        if(reloadFromControl(client,&e)) st = client->clearBinding(action, revision);
    }
    if(!st.ok()){ if(err)*err=st.message; return false; }
    std::string e; reloadFromControl(client,&e);
    return true;
}
bool HotkeysPage::resetBinding(flamewm::control::ControlClient* client, const std::string& action, std::string* err){
    if(!client || !client->isConnected()){ if(err)*err="Control not connected"; return false; }
    if(revision==0){
        std::string e; if(!reloadFromControl(client,&e)){ if(err)*err=e; return false; }
    }
    flamewm::api::Status st = client->resetBinding(action, revision);
    if(!st.ok() && st.code==flamewm::api::Error::StaleRevision){
        std::string e;
        if(reloadFromControl(client,&e)) st = client->resetBinding(action, revision);
    }
    if(!st.ok()){ if(err)*err=st.message; return false; }
    std::string e; reloadFromControl(client,&e);
    return true;
}
bool HotkeysPage::commitCapture(flamewm::control::ControlClient* client, const std::string& keysym, std::string* err){
    if(!capturing) return false;
    std::string act = captureAction;
    capturing=false;
    return setBinding(client, act, keysym, err);
}
bool HotkeysPage::handleCaptureKey(flamewm::control::ControlClient* client, const std::string& keysym, std::string* err){
    if(!capturing) return false;
    if(isEscape(keysym)){
        std::string act = captureAction;
        capturing=false;
        return clearBinding(client, act, err);
    }
    return false;
}
}} // namespace
