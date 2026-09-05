#include "../../flamewm/api/settings.h"
#include "../../flamewm/api/errors.h"
#include "../../flamewm/control/client.h"
#include "../../ref.h"
#include "../../base.h"
#include "../../yimage.h"
#include "../../yicon.h"
// X11/Xlib.h defines Status as `int` (via X.h/Xlib.h pulled transitively
// through yimage.h -> ypaint.h -> ypixmap.h -> Xrender.h -> Xlib.h).
// flamewm::api::Status would be macro-expanded to flamewm::api::int after
// that point. api/errors.h push/pop only protects its own definition site,
// so undefine the X macro after the X chain for the remainder of this TU.
#ifdef Status
#undef Status
#endif
#ifdef None
#undef None
#endif
#include "appearance.h"
#include <dirent.h>
#include <cstdlib>
#include <set>
#include <fstream>
#include <algorithm>
#include <cctype>

namespace flamewm{ namespace settings{
static bool isHexColor(const std::string& s){
    if(s.size()!=7||s[0]!='#') return false;
    for(size_t i=1;i<7;++i){ char c=s[i]; if(!((c>='0'&&c<='9')||(c>='A'&&c<='F')||(c>='a'&&c<='f'))) return false; }
    return true;
}
bool AppearancePage::setPreset(const std::string& hex, std::string* err){
    if(!isHexColor(hex)){ if(err)*err="invalid preset hex"; return false; }
    selectedPreset=hex; customColor.clear(); return true;
}
bool AppearancePage::setCustom(const std::string& hex, std::string* err){
    if(!isHexColor(hex)){ if(err)*err="invalid custom hex"; return false; }
    customColor=hex; return true;
}
bool AppearancePage::reloadFromControl(flamewm::control::ControlClient* client, std::string* error){
    if(!client || !client->isConnected()){ if(error)*error="Control not connected"; return false; }
    flamewm::api::Result<flamewm::api::SettingsSnapshot> r = client->getSettingsSnapshot();
    if(!r.isOk()){ if(error)*error=r.error().message; return false; }
    const flamewm::api::SettingsSnapshot& snap = r.value();
    revision = snap.revision;
    iconTheme = snap.get("iconTheme", "*:-HighContrast");
    std::string acc = snap.get("accent", "#EF4048");
    if(acc.empty()) acc = "#EF4048";
    // keep preset/custom invariant: treat effective accent as selectedPreset
    selectedPreset = acc;
    customColor.clear();
    return true;
}
bool AppearancePage::applyAccentImmediate(flamewm::control::ControlClient* client, const std::string& hex, std::string* error){
    if(!isHexColor(hex)){ if(error)*error="invalid accent hex"; return false; }
    if(!client || !client->isConnected()){ if(error)*error="Control not connected"; return false; }
    // ensure we have current revision
    if(revision==0){
        std::string e;
        if(!reloadFromControl(client,&e)){
            if(error)*error=e;
            return false;
        }
    }
    std::vector<flamewm::api::SettingsChange> ch;
    ch.push_back(flamewm::api::SettingsChange("accent", hex));
    flamewm::api::Status st = client->applySettings(revision, ch);
    if(!st.ok()){
        if(st.code==flamewm::api::Error::StaleRevision){
            // refresh and retry once
            std::string e;
            if(reloadFromControl(client,&e)){
                st = client->applySettings(revision, ch);
            }
        }
        if(!st.ok()){ if(error)*error=st.message; return false; }
    }
    // sync local
    selectedPreset=hex; customColor.clear();
    std::string e;
    reloadFromControl(client,&e);
    return true;
}
bool AppearancePage::applyIconThemeImmediate(flamewm::control::ControlClient* client, const std::string& theme, std::string* error){
    if(theme.empty()){ if(error)*error="icon theme cannot be empty"; return false; }
    if(!client || !client->isConnected()){ if(error)*error="Control not connected"; return false; }
    if(revision==0){
        std::string e;
        if(!reloadFromControl(client,&e)){
            if(error)*error=e;
            return false;
        }
    }
    std::vector<flamewm::api::SettingsChange> ch;
    ch.push_back(flamewm::api::SettingsChange("iconTheme", theme));
    flamewm::api::Status st = client->applySettings(revision, ch);
    if(!st.ok()){
        if(st.code==flamewm::api::Error::StaleRevision){
            std::string e;
            if(reloadFromControl(client,&e)) st = client->applySettings(revision, ch);
        }
        if(!st.ok()){ if(error)*error=st.message; return false; }
    }
    iconTheme=theme;
    invalidateIconCache();
    std::string e;
    reloadFromControl(client,&e);
    return true;
}
std::vector<std::string> AppearancePage::iconThemes(){
    std::set<std::string> found;
    const char *home = getenv("HOME");
    std::vector<std::string> roots;
    if (home) roots.push_back(std::string(home) + "/.icons");
    if (home) roots.push_back(std::string(home) + "/.local/share/icons");
    roots.push_back("/usr/share/icons");
    roots.push_back("/usr/local/share/icons");
    for (size_t r = 0; r < roots.size(); ++r) {
        DIR *dir = opendir(roots[r].c_str());
        if (!dir) continue;
        struct dirent *entry;
        while ((entry = readdir(dir)) != 0) {
            if (entry->d_name[0] == '.' || entry->d_type != DT_DIR) continue;
            std::ifstream index((roots[r] + "/" + entry->d_name + "/index.theme").c_str());
            if (!index) continue;
            bool inIconTheme = false;
            bool hidden = false;
            std::string line;
            while (std::getline(index, line)) {
                if (!line.empty() && line[line.size() - 1] == '\r') line.erase(line.size() - 1);
                if (!line.empty() && line[0] == '[') {
                    inIconTheme = line == "[Icon Theme]";
                    continue;
                }
                if (!inIconTheme) continue;
                size_t equal = line.find('=');
                if (equal == std::string::npos || line.substr(0, equal) != "Hidden") continue;
                std::string value = line.substr(equal + 1);
                std::transform(value.begin(), value.end(), value.begin(), ::tolower);
                hidden = value == "true" || value == "1" || value == "yes";
            }
            if (!hidden) found.insert(entry->d_name);
        }
        closedir(dir);
    }
    return std::vector<std::string>(found.begin(), found.end());
}
void AppearancePage::invalidateIconCache(){ YIcon::freeIcons(); }
}} // namespace
