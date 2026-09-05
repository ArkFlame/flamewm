#include "flamewm/core/config.h"
#include "../ui/metrics.h"
#include <sstream>
#include <cstdio>
#include <cstdlib>
#include <cerrno>
#include <cctype>
#include <limits>

namespace flamewm {

SettingsSnapshot::SettingsSnapshot()
    : revision(0)
    , desktopSelectionFillOpacity(20)
    , windowSnapPreviewFillOpacity(20)
    , fontBold(false)
    , fontSizeOffset(0)
    , taskbarOpacity(99)
    , taskbarHeight(44)
    , stickyNotesEnabled(true)
{}

SettingsSnapshot SettingsSnapshot::defaults() {
    SettingsSnapshot s;
    s.revision = 1;
    s.accent = "#EF4048";
    s.iconTheme = "*:-HighContrast";
    s.desktopSelectionFillOpacity = 20;
    s.windowSnapPreviewFillOpacity = 20;
    s.fontFamily = "IBM Plex Sans";
    s.fontBold = false;
    s.fontSizeOffset = 0;
    s.taskbarColor = "#191b1d";
    s.taskbarOpacity = 99;
    s.taskbarHeight = 44;
    s.startButtonText = "";
    s.customStartIcon = "";
    s.stickyNotesEnabled = true;
    return s;
}

static bool isCanonicalColor(const std::string& value) {
    if (value.size() != 7 || value[0] != '#') return false;
    for (size_t i = 1; i < value.size(); ++i) {
        if (!std::isxdigit(static_cast<unsigned char>(value[i]))) return false;
    }
    return true;
}

static bool isDurableOutputId(const std::string& value) {
    if (value.empty()) return false;
    for (size_t i = 0; i < value.size(); ++i) {
        unsigned char c = static_cast<unsigned char>(value[i]);
        if (std::isspace(c) || std::iscntrl(c) || value[i] == '=') return false;
    }
    return true;
}

static bool isHotkeyActionId(const std::string& value) {
    return value == "ToggleStartMenu" || value == "WorkspaceLeft" ||
           value == "WorkspaceRight" || value == "WorkspaceUp" ||
           value == "WorkspaceDown" || value == "WindowClose" ||
           value == "WindowMinimize" || value == "WindowMaximize";
}

static bool isStableKey(const std::string& key) {
    return key == "revision" || key == "accent" || key == "iconTheme" ||
           key == "DesktopSelectionFillOpacity" ||
           key == "WindowSnapPreviewFillOpacity" || key == "fontFamily" ||
           key == "fontBold" || key == "fontSizeOffset" ||
           key == "taskbarColor" || key == "taskbarOpacity" ||
           key == "taskbarHeight" || key == "startButtonText" ||
           key == "customStartIcon" || key == "stickyNotesEnabled" ||
           key.compare(0, 6, "scale.") == 0 ||
           key.compare(0, 7, "hotkey.") == 0;
}

bool SettingsSnapshot::isValid(std::string* error) const {
    if (desktopSelectionFillOpacity < 0 || desktopSelectionFillOpacity > 60) {
        if (error) *error = "DesktopSelectionFillOpacity out of range 0..60";
        return false;
    }
    if (windowSnapPreviewFillOpacity < 0 || windowSnapPreviewFillOpacity > 60) {
        if (error) *error = "WindowSnapPreviewFillOpacity out of range 0..60";
        return false;
    }
    if (!isCanonicalColor(taskbarColor)) {
        if (error) *error = "taskbarColor must be #RRGGBB";
        return false;
    }
    if (!accent.empty() && !isCanonicalColor(accent)) {
        if (error) *error = "accent must be #RRGGBB or empty";
        return false;
    }
    if (fontSizeOffset < -2 || fontSizeOffset > 4) {
        if (error) *error = "fontSizeOffset out of range -2..4";
        return false;
    }
    if (taskbarOpacity < 0 || taskbarOpacity > 100) {
        if (error) *error = "taskbarOpacity out of range 0..100";
        return false;
    }
    if (taskbarHeight < 34 || taskbarHeight > 72) {
        if (error) *error = "taskbarHeight out of range 34..72";
        return false;
    }
    for (std::map<std::string,int>::const_iterator it = perOutputScale.begin(); it != perOutputScale.end(); ++it) {
        if (!isDurableOutputId(it->first)) {
            if (error) *error = "perOutputScale output ID invalid";
            return false;
        }
        if (!isSupportedScale(it->second)) {
            if (error) *error = "perOutputScale value " + it->first + " invalid scale";
            return false;
        }
    }
    for (std::map<std::string,std::string>::const_iterator it = hotkeys.begin(); it != hotkeys.end(); ++it) {
        if (!isHotkeyActionId(it->first)) {
            if (error) *error = "hotkey action ID invalid";
            return false;
        }
    }
    for (std::map<std::string,std::string>::const_iterator it = unknownKeys.begin(); it != unknownKeys.end(); ++it) {
        if (isStableKey(it->first)) {
            if (error) *error = "unknown key duplicates stable key";
            return false;
        }
    }
    return true;
}

ConfigStore::ConfigStore() : current_(SettingsSnapshot::defaults()) {}
ConfigStore::~ConfigStore() {}

static inline std::string trim(const std::string& s) {
    size_t a = 0; while (a < s.size() && (s[a]==' '||s[a]=='\t'||s[a]=='\r')) ++a;
    size_t b = s.size(); while (b > a && (s[b-1]==' '||s[b-1]=='\t'||s[b-1]=='\r')) --b;
    return s.substr(a, b-a);
}

static bool parseInt(const std::string& value, int& result) {
    char* end = 0;
    errno = 0;
    long parsed = std::strtol(value.c_str(), &end, 10);
    if (value.empty() || end == value.c_str() || *end != '\0' || errno == ERANGE ||
        parsed < -2147483647L - 1L || parsed > 2147483647L) return false;
    result = static_cast<int>(parsed);
    return true;
}

static bool parseBool(const std::string& value, bool& result) {
    if (value == "1" || value == "true" || value == "yes") {
        result = true;
        return true;
    }
    if (value == "0" || value == "false" || value == "no") {
        result = false;
        return true;
    }
    return false;
}

bool ConfigStore::parseSnapshot(const std::string& text, SettingsSnapshot& out, std::string* error) const {
    SettingsSnapshot snap;
    // preserve current revision as base unless overridden
    snap.revision = current_.revision;
    snap.accent = current_.accent;
    snap.iconTheme = current_.iconTheme;
    snap.desktopSelectionFillOpacity = current_.desktopSelectionFillOpacity;
    snap.windowSnapPreviewFillOpacity = current_.windowSnapPreviewFillOpacity;
    snap.fontFamily = current_.fontFamily;
    snap.fontBold = current_.fontBold;
    snap.fontSizeOffset = current_.fontSizeOffset;
    snap.taskbarColor = current_.taskbarColor;
    snap.taskbarOpacity = current_.taskbarOpacity;
    snap.taskbarHeight = current_.taskbarHeight;
    snap.startButtonText = current_.startButtonText;
    snap.customStartIcon = current_.customStartIcon;
    snap.stickyNotesEnabled = current_.stickyNotesEnabled;
    snap.perOutputScale = current_.perOutputScale;
    snap.hotkeys = current_.hotkeys;
    snap.unknownKeys = current_.unknownKeys;

    std::istringstream iss(text);
    std::string line;
    while (std::getline(iss, line)) {
        std::string t = trim(line);
        if (t.empty() || t[0]=='#' ) continue;
        size_t eq = t.find('=');
        if (eq == std::string::npos) {
            if (error) *error = "missing = in line: " + t;
            return false;
        }
        std::string key = trim(t.substr(0, eq));
        std::string val = trim(t.substr(eq+1));
        if (key == "revision") {
            if (val.empty()) { if (error) *error = "bad revision"; return false; }
            for (size_t i = 0; i < val.size(); ++i) {
                if (val[i] < '0' || val[i] > '9') { if (error) *error = "bad revision"; return false; }
            }
            char* e = 0;
            errno = 0;
            unsigned long long v = strtoull(val.c_str(), &e, 10);
            if (!e || *e!='\0' || errno == ERANGE ||
                v > static_cast<unsigned long long>(std::numeric_limits<uint64_t>::max())) {
                if (error) *error = "bad revision";
                return false;
            }
            snap.revision = (uint64_t)v;
        } else if (key == "accent") {
            snap.accent = val;
        } else if (key == "iconTheme") {
            snap.iconTheme = val;
        } else if (key == "DesktopSelectionFillOpacity") {
            if (!parseInt(val, snap.desktopSelectionFillOpacity)) { if (error) *error = "bad DesktopSelectionFillOpacity"; return false; }
        } else if (key == "WindowSnapPreviewFillOpacity") {
            if (!parseInt(val, snap.windowSnapPreviewFillOpacity)) { if (error) *error = "bad WindowSnapPreviewFillOpacity"; return false; }
        } else if (key == "fontFamily") {
            snap.fontFamily = val;
        } else if (key == "fontBold") {
            if (!parseBool(val, snap.fontBold)) { if (error) *error = "bad fontBold"; return false; }
        } else if (key == "fontSizeOffset") {
            if (!parseInt(val, snap.fontSizeOffset)) { if (error) *error = "bad fontSizeOffset"; return false; }
        } else if (key == "taskbarColor") {
            snap.taskbarColor = val;
        } else if (key == "taskbarOpacity") {
            if (!parseInt(val, snap.taskbarOpacity)) { if (error) *error = "bad taskbarOpacity"; return false; }
        } else if (key == "taskbarHeight") {
            if (!parseInt(val, snap.taskbarHeight)) { if (error) *error = "bad taskbarHeight"; return false; }
        } else if (key == "startButtonText") {
            snap.startButtonText = val;
        } else if (key == "customStartIcon") {
            snap.customStartIcon = val;
        } else if (key == "stickyNotesEnabled") {
            if (!parseBool(val, snap.stickyNotesEnabled)) { if (error) *error = "bad stickyNotesEnabled"; return false; }
        } else if (key.compare(0,6,"scale.")==0) {
            std::string outId = key.substr(6);
            if (!isDurableOutputId(outId)) { if (error) *error = "bad scale output ID"; return false; }
            int pct = 0;
            if (!parseInt(val, pct)) { if (error) *error = "bad scale"; return false; }
            snap.perOutputScale[outId] = pct;
        } else if (key.compare(0,7,"hotkey.")==0) {
            std::string act = key.substr(7);
            if (!isHotkeyActionId(act)) { if (error) *error = "bad hotkey action ID"; return false; }
            snap.hotkeys[act] = val;
        } else {
            snap.unknownKeys[key] = val;
        }
        if (isStableKey(key)) snap.unknownKeys.erase(key);
    }
    if (!snap.isValid(error)) return false;
    out = snap;
    return true;
}

bool ConfigStore::applySnapshot(const SettingsSnapshot& snap, std::string* error) {
    if (isStale(snap)) {
        if (error) *error = "stale revision";
        return false;
    }
    if (!snap.isValid(error)) return false;
    current_ = snap;
    return true;
}

bool ConfigStore::isStale(const SettingsSnapshot& snap) const {
    // 0 means unset? treat 0 as stale if current>0 ; but if snap.revision==0 keep as stale unless current is 0
    if (snap.revision == 0) return true;
    return snap.revision <= current_.revision;
}

std::string ConfigStore::serialize(const SettingsSnapshot& snap) const {
    std::ostringstream oss;
    oss << "revision=" << snap.revision << "\n";
    oss << "accent=" << snap.accent << "\n";
    oss << "iconTheme=" << snap.iconTheme << "\n";
    oss << "DesktopSelectionFillOpacity=" << snap.desktopSelectionFillOpacity << "\n";
    oss << "WindowSnapPreviewFillOpacity=" << snap.windowSnapPreviewFillOpacity << "\n";
    oss << "fontFamily=" << snap.fontFamily << "\n";
    oss << "fontBold=" << (snap.fontBold?1:0) << "\n";
    oss << "fontSizeOffset=" << snap.fontSizeOffset << "\n";
    oss << "taskbarColor=" << snap.taskbarColor << "\n";
    oss << "taskbarOpacity=" << snap.taskbarOpacity << "\n";
    oss << "taskbarHeight=" << snap.taskbarHeight << "\n";
    oss << "startButtonText=" << snap.startButtonText << "\n";
    oss << "customStartIcon=" << snap.customStartIcon << "\n";
    oss << "stickyNotesEnabled=" << (snap.stickyNotesEnabled?1:0) << "\n";
    for (std::map<std::string,int>::const_iterator it=snap.perOutputScale.begin(); it!=snap.perOutputScale.end();++it)
        oss << "scale." << it->first << "=" << it->second << "\n";
    for (std::map<std::string,std::string>::const_iterator it=snap.hotkeys.begin(); it!=snap.hotkeys.end();++it)
        oss << "hotkey." << it->first << "=" << it->second << "\n";
    for (std::map<std::string,std::string>::const_iterator it=snap.unknownKeys.begin(); it!=snap.unknownKeys.end();++it)
        oss << it->first << "=" << it->second << "\n";
    return oss.str();
}

bool ConfigStore::persistToFile(const std::string& path, const SettingsSnapshot& snap, std::string* error) const {
    std::string validationError;
    if (!snap.isValid(&validationError)) {
        if (error) *error = "invalid snapshot: " + validationError;
        return false;
    }
    std::string tmp = path + ".tmp";
    std::string data = serialize(snap);
    FILE* f = fopen(tmp.c_str(), "wb");
    if (!f) { if(error)*error="open tmp failed"; return false; }
    size_t w = fwrite(data.data(), 1, data.size(), f);
    if (w != data.size()) { fclose(f); if(error)*error="write failed"; return false; }
    if (fflush(f)!=0) { fclose(f); if(error)*error="flush failed"; return false; }
    fclose(f);
    if (rename(tmp.c_str(), path.c_str())!=0) { if(error)*error="rename failed"; return false; }
    return true;
}

} // namespace flamewm
