#include "about.h"
#include "../../flamewm/control/client.h"
namespace flamewm{ namespace settings{
bool AboutPage::verifyControlVersion(flamewm::control::ControlClient* client, std::string* versionOut, std::string* error){
    if(!client || !client->isConnected()){ if(error)*error="Control not connected"; return false; }
    flamewm::api::Result<std::string> r = client->getVersion();
    if(!r.isOk()){ if(error)*error=r.error().message; return false; }
    if(versionOut) *versionOut = r.value();
    return true;
}
}} // namespace
