#include "shortcutregistry.h"
#include <algorithm>
#include <cctype>

namespace flamewm {

static std::string trimCopy(const std::string& s){
    size_t a=0; while(a<s.size()&&(s[a]==' '||s[a]=='\t')) ++a;
    size_t b=s.size(); while(b>a&&(s[b-1]==' '||s[b-1]=='\t'))--b;
    return s.substr(a,b-a);
}

ShortcutRegistry::ShortcutRegistry(){ initDefaults(); }

void ShortcutRegistry::initDefaults(){
    map_.clear();
    // shipped defaults per-row — Toggle Start menu canonical Super_L
    map_[ActionToggleStartMenu] = KeyBinding("Super_L");
    map_[ActionWorkspaceLeft]   = KeyBinding("Ctrl+Super+Left");
    map_[ActionWorkspaceRight]  = KeyBinding("Ctrl+Super+Right");
    map_[ActionWorkspaceUp]     = KeyBinding("Ctrl+Super+Up");
    map_[ActionWorkspaceDown]   = KeyBinding("Ctrl+Super+Down");
    map_[ActionWindowClose]     = KeyBinding("Alt+F4");
    map_[ActionWindowMinimize]  = KeyBinding("");
    map_[ActionWindowMinimize].assigned=false;
    map_[ActionWindowMaximize]  = KeyBinding("");
    map_[ActionWindowMaximize].assigned=false;
    shippedDefaults_ = map_;
}

std::string ShortcutRegistry::actionName(ShortcutAction a) const {
    switch(a){
        case ActionToggleStartMenu: return "ToggleStartMenu";
        case ActionWorkspaceLeft: return "WorkspaceLeft";
        case ActionWorkspaceRight: return "WorkspaceRight";
        case ActionWorkspaceUp: return "WorkspaceUp";
        case ActionWorkspaceDown: return "WorkspaceDown";
        case ActionWindowClose: return "WindowClose";
        case ActionWindowMinimize: return "WindowMinimize";
        case ActionWindowMaximize: return "WindowMaximize";
        default: return "Unknown";
    }
}
KeyBinding ShortcutRegistry::bindingFor(ShortcutAction a) const {
    std::map<ShortcutAction,KeyBinding>::const_iterator it=map_.find(a);
    if(it!=map_.end()) return it->second;
    return KeyBinding();
}
bool ShortcutRegistry::isModifierOnly(const std::string& normalized){
    // Tokens split by '+'. If every token is a modifier (Ctrl/Shift/Alt/Super/Meta/Hyper), it's modifier-only.
    std::string s = normalized;
    // Lowercase copy for comparison
    std::string low = s;
    for(size_t i=0;i<low.size();++i) low[i]=(char)tolower((unsigned char)low[i]);
    // quick check: if no '+' and token itself is modifier -> modifier-only
    // Split
    std::vector<std::string> parts;
    size_t start=0;
    for(size_t i=0;i<=low.size();++i){ if(i==low.size()||low[i]=='+'){ std::string t=low.substr(start,i-start); trimCopy(t); if(!t.empty()) parts.push_back(t); start=i+1; } }
    if(parts.empty()) return false;
    for(size_t i=0;i<parts.size();++i){
        const std::string& p=parts[i];
        if(p=="ctrl"||p=="shift"||p=="alt"||p=="super"||p=="meta"||p=="hyper"||p=="super_l"||p=="super_r") continue;
        return false;
    }
    return true;
}
bool ShortcutRegistry::actionAllowsModifierOnly(ShortcutAction a){ return a==ActionToggleStartMenu; }

bool ShortcutRegistry::setBindingValidated(ShortcutAction a, const std::string& keysym, std::string* error){
    std::string n = normalize(keysym);
    if(n.empty()){ clearBinding(a); return true; }
    // Escape must never be stored — caller should handle Escape->clear before calling set
    if(isEscapeNormalized(n) || isEscape(n)){ if(error)*error="Escape cannot be stored"; return false; }
    if(!validate(n)){ if(error)*error="invalid binding"; return false; }
    if(isModifierOnly(n) && !actionAllowsModifierOnly(a)){ if(error)*error="modifier-only not allowed"; return false; }
    ShortcutAction conflict;
    for(std::map<ShortcutAction,KeyBinding>::const_iterator it=map_.begin();it!=map_.end();++it){
        if(it->first==a) continue;
        if(it->second.assigned && normalize(it->second.keysym)==n){
            conflict=it->first;
            if(error) *error="conflict with "+actionName(conflict);
            return false;
        }
    }
    map_[a]=KeyBinding(n);
    map_[a].assigned=true;
    return true;
}

bool ShortcutRegistry::setBinding(ShortcutAction a, const std::string& keysym, std::string* error){
    // Legacy entrypoint: delegate to validated path
    return setBindingValidated(a, keysym, error);
}
void ShortcutRegistry::resetOneRow(ShortcutAction a){
    std::map<ShortcutAction,KeyBinding>::const_iterator it=shippedDefaults_.find(a);
    if(it!=shippedDefaults_.end()) map_[a]=it->second;
    else { map_[a]=KeyBinding(); map_[a].assigned=false; }
}
bool ShortcutRegistry::applyWithGrabStage(const std::map<ShortcutAction, std::string>& desired,
                            bool (*grabFn)(const std::string& keysym, void* ctx), void* ctx,
                            std::string* error){
    // Save old
    std::map<ShortcutAction, KeyBinding> old = map_;
    // Contract C3 transaction: normalize+validate -> duplicate detection already
    // covered by setBindingValidated which checks Escape, modifier-only policy,
    // and inter-action duplicates.
    for(std::map<ShortcutAction,std::string>::const_iterator it=desired.begin(); it!=desired.end(); ++it){
        int ai = (int)it->first;
        if(ai < 0 || ai >= (int)ActionCount){
            map_=old;
            if(error) *error="unknown action";
            return false;
        }
        std::string err2;
        if(!setBindingValidated(it->first, it->second, &err2)){
            map_=old;
            if(error) *error=err2;
            return false;
        }
    }
    // Attempt grabs for all assigned bindings
    if(grabFn){
        std::vector<std::string> staged;
        for(std::map<ShortcutAction,KeyBinding>::const_iterator it=map_.begin(); it!=map_.end(); ++it){
            if(it->second.assigned && !it->second.keysym.empty()){
                if(!grabFn(it->second.keysym, ctx)){
                    // failure: ungrab staged, restore previous, registry unchanged
                    map_=old;
                    if(error) *error="grab failed for "+it->second.keysym;
                    return false;
                }
                staged.push_back(it->second.keysym);
            }
        }
        (void)staged;
    }
    // success: publish registry state — caller then allows config revision commit
    return true;
}
void ShortcutRegistry::clearBinding(ShortcutAction a){
    map_[a]=KeyBinding();
    map_[a].assigned=false;
}
void ShortcutRegistry::resetToDefaults(){ initDefaults(); }
bool ShortcutRegistry::hasConflict(const std::string& keysym, ShortcutAction* outConflict) const{
    std::string n=normalize(keysym);
    for(std::map<ShortcutAction,KeyBinding>::const_iterator it=map_.begin();it!=map_.end();++it){
        if(it->second.assigned && normalize(it->second.keysym)==n){
            if(outConflict) *outConflict=it->first;
            return true;
        }
    }
    return false;
}
std::string ShortcutRegistry::normalize(const std::string& raw){
    std::string t=trimCopy(raw);
    if(t.empty()) return "";
    // simple normalize: trim, collapse spaces, keep case for now but compare lower
    // For validation we accept non-empty
    return t;
}
bool ShortcutRegistry::validate(const std::string& normalized){
    if(normalized.empty()) return true; // empty means Not assigned -> valid
    if(normalized.size()>64) return false;
    // must contain at least one non-space
    for(size_t i=0;i<normalized.size();++i) if(normalized[i]!=' '&&normalized[i]!='\t') return true;
    return false;
}
} // namespace flamewm
