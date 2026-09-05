#include "displays.h"
#include "../../flamewm/control/client.h"
#include "../../flamewm/api/errors.h"
namespace flamewm{ namespace settings{
bool DisplaysPage::reloadFromControl(flamewm::control::ControlClient* client, std::string* error){
    if(!client || !client->isConnected()){ if(error)*error="Control not connected"; return false; }
    flamewm::api::Result<flamewm::api::DisplaySnapshot> r = client->getDisplaySnapshot();
    if(!r.isOk()){ if(error)*error=r.error().message; return false; }
    cachedSnapshot_ = r.value();
    hasSnapshot_ = true;
    // keep previous selection if still valid; otherwise clear so caller must re-select
    if(!selectedOutputId_.empty()){
        bool found=false;
        for(size_t i=0;i<cachedSnapshot_.outputs.size();++i) if(cachedSnapshot_.outputs[i].id.key==selectedOutputId_) found=true;
        if(!found) selectedOutputId_.clear();
    }
    return true;
}
bool DisplaysPage::setResolutionForSelected(flamewm::control::ControlClient* client, const flamewm::api::ModeId& mode, std::string* err){
    if(!client || !client->isConnected()){ if(err)*err="Control not connected"; return false; }
    if(selectedOutputId_.empty()){ if(err)*err="no selected output"; return false; }
    if(!mode.valid()){ if(err)*err="invalid mode"; return false; }
    if(!hasSnapshot_){
        std::string e;
        if(!reloadFromControl(client,&e)){ if(err)*err=e; return false; }
    }
    flamewm::api::OutputId oid(selectedOutputId_);
    flamewm::api::TransactionId tx;
    flamewm::api::Status st = client->beginDisplayModeChange(oid, mode, cachedSnapshot_.generation, &tx);
    if(!st.ok()){ if(err)*err=st.message; return false; }
    std::string e;
    reloadFromControl(client,&e);
    return true;
}
bool DisplaysPage::setScaleForSelected(flamewm::control::ControlClient* client, int pct, std::string* err){
    if(!client || !client->isConnected()){ if(err)*err="Control not connected"; return false; }
    if(selectedOutputId_.empty()){ if(err)*err="no selected output"; return false; }
    if(pct!=100&&pct!=125&&pct!=150&&pct!=175&&pct!=200){ if(err)*err="scale must be 100/125/150/175/200"; return false; }
    if(!hasSnapshot_){
        std::string e;
        if(!reloadFromControl(client,&e)){ if(err)*err=e; return false; }
    }
    flamewm::api::OutputId oid(selectedOutputId_);
    // DisplaysPage uses generation-as-revision for scale: DisplayService expects expectedRevision==generation.
    // ControlClient::setShellScale requires expectedRevision; pass cached generation.
    flamewm::api::Status st = client->setShellScale(oid, (uint32_t)pct, cachedSnapshot_.generation);
    if(!st.ok()){
        if(st.code==flamewm::api::Error::StaleRevision){
            std::string e;
            if(reloadFromControl(client,&e)) st = client->setShellScale(oid, (uint32_t)pct, cachedSnapshot_.generation);
        }
        if(!st.ok()){ if(err)*err=st.message; return false; }
    }
    std::string e; reloadFromControl(client,&e);
    return true;
}
bool DisplaysPage::keep(flamewm::control::ControlClient* client, std::string* err){
    if(!client || !client->isConnected()){ if(err)*err="Control not connected"; return false; }
    if(!hasSnapshot_){
        std::string e; if(!reloadFromControl(client,&e)){ if(err)*err=e; return false; }
    }
    if(!cachedSnapshot_.hasPending()){ if(err)*err="no pending display tx"; return false; }
    flamewm::api::Status st = client->keepDisplayMode(cachedSnapshot_.pending.tx);
    if(!st.ok()){ if(err)*err=st.message; return false; }
    std::string e; reloadFromControl(client,&e);
    return true;
}
bool DisplaysPage::revert(flamewm::control::ControlClient* client, std::string* err){
    if(!client || !client->isConnected()){ if(err)*err="Control not connected"; return false; }
    if(!hasSnapshot_){
        std::string e; if(!reloadFromControl(client,&e)){ if(err)*err=e; return false; }
    }
    if(!cachedSnapshot_.hasPending()){ if(err)*err="no pending display tx"; return false; }
    flamewm::api::Status st = client->revertDisplayMode(cachedSnapshot_.pending.tx);
    if(!st.ok()){ if(err)*err=st.message; return false; }
    std::string e; reloadFromControl(client,&e);
    return true;
}
}} // namespace
