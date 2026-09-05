#include "flamewm/platform/applications/service.h"

#include <algorithm>
#include <cctype>
#include <cstdlib>
#include <cstring>
#include <dirent.h>
#include <fstream>
#include <map>
#include <string>
#include <sys/stat.h>
#include <vector>
#include <unordered_map>

namespace flamewm {
namespace platform {
namespace applications {

namespace {

inline std::string trim(const std::string& s) {
    size_t a = 0;
    while (a < s.size() && std::isspace(static_cast<unsigned char>(s[a]))) ++a;
    size_t b = s.size();
    while (b > a && std::isspace(static_cast<unsigned char>(s[b - 1]))) --b;
    return s.substr(a, b - a);
}

inline std::string toLower(const std::string& s) {
    std::string o;
    o.reserve(s.size());
    for (size_t i = 0; i < s.size(); ++i) {
        o.push_back(static_cast<char>(std::tolower(static_cast<unsigned char>(s[i]))));
    }
    return o;
}

inline bool isTrue(const std::string& v) {
    std::string l = toLower(trim(v));
    return l == "true" || l == "1" || l == "yes";
}

inline std::vector<std::string> splitCategories(const std::string& s) {
    std::vector<std::string> out;
    std::string cur;
    for (size_t i = 0; i < s.size(); ++i) {
        char c = s[i];
        if (c == ';') {
            std::string t = trim(cur);
            if (!t.empty()) out.push_back(t);
            cur.clear();
        } else {
            cur.push_back(c);
        }
    }
    std::string t = trim(cur);
    if (!t.empty()) out.push_back(t);
    return out;
}

bool pathExistsDir(const std::string& p) {
    struct stat st;
    if (stat(p.c_str(), &st) != 0) return false;
    return S_ISDIR(st.st_mode);
}

bool pathExistsFile(const std::string& p) {
    struct stat st;
    if (stat(p.c_str(), &st) != 0) return false;
    return S_ISREG(st.st_mode);
}

std::vector<std::string> collectAppDirs() {
    std::vector<std::string> dirs;
    const char* home = std::getenv("HOME");
    std::string homeStr = home ? std::string(home) : std::string();

    const char* xdgDataHome = std::getenv("XDG_DATA_HOME");
    std::string dataHome;
    if (xdgDataHome && xdgDataHome[0] != '\0') {
        dataHome = std::string(xdgDataHome);
    } else if (!homeStr.empty()) {
        dataHome = homeStr + "/.local/share";
    }
    if (!dataHome.empty()) {
        std::string p = dataHome + "/applications";
        if (pathExistsDir(p)) dirs.push_back(p);
    }

    const char* xdgDataDirsEnv = std::getenv("XDG_DATA_DIRS");
    std::string xdgDataDirs = xdgDataDirsEnv ? std::string(xdgDataDirsEnv) : std::string();
    if (xdgDataDirs.empty()) {
        xdgDataDirs = "/usr/local/share:/usr/share";
    }
    std::string cur;
    for (size_t i = 0; i <= xdgDataDirs.size(); ++i) {
        char c = (i < xdgDataDirs.size()) ? xdgDataDirs[i] : ':';
        if (c == ':') {
            std::string d = trim(cur);
            cur.clear();
            if (d.empty()) continue;
            std::string p = d + "/applications";
            bool dup = false;
            for (size_t k = 0; k < dirs.size(); ++k) if (dirs[k] == p) { dup = true; break; }
            if (!dup && pathExistsDir(p)) dirs.push_back(p);
        } else {
            cur.push_back(c);
        }
    }
    return dirs;
}

struct ParsedDesktop {
    bool valid;
    bool noDisplay;
    bool hidden;
    std::string type;
    std::string name;
    std::string execCmd;
    std::string icon;
    std::string startupWMClass;
    std::string categoriesRaw;
};

ParsedDesktop parseDesktopFile(const std::string& path) {
    ParsedDesktop p;
    p.valid = false;
    p.noDisplay = false;
    p.hidden = false;
    std::ifstream in(path.c_str());
    if (!in) return p;
    bool inEntry = false;
    bool seenEntry = false;
    std::string line;
    while (std::getline(in, line)) {
        if (!line.empty() && line[line.size() - 1] == '\r') line.erase(line.size() - 1);
        std::string t = trim(line);
        if (t.empty() || t[0] == '#') continue;
        if (t.size() >= 2 && t[0] == '[' && t[t.size() - 1] == ']') {
            std::string sec = trim(t.substr(1, t.size() - 2));
            inEntry = (sec == "Desktop Entry");
            if (inEntry) seenEntry = true;
            continue;
        }
        if (!inEntry) continue;
        size_t eq = t.find('=');
        if (eq == std::string::npos) continue;
        std::string key = trim(t.substr(0, eq));
        std::string val = trim(t.substr(eq + 1));
        std::string keyBase = key;
        size_t br = key.find('[');
        if (br != std::string::npos) keyBase = key.substr(0, br);
        if (keyBase == "Name") {
            if (key == "Name" || p.name.empty()) {
                if (key == "Name") p.name = val;
                else if (p.name.empty()) p.name = val;
            }
        } else if (key == "Exec") {
            p.execCmd = val;
        } else if (key == "Icon") {
            p.icon = val;
        } else if (key == "StartupWMClass") {
            p.startupWMClass = val;
        } else if (key == "Categories") {
            p.categoriesRaw = val;
        } else if (key == "NoDisplay") {
            p.noDisplay = isTrue(val);
        } else if (key == "Hidden") {
            p.hidden = isTrue(val);
        } else if (key == "Type") {
            p.type = val;
        }
    }
    if (!seenEntry) return p;
    if (!p.type.empty() && p.type != "Application") return p;
    if (p.noDisplay || p.hidden) return p;
    p.valid = true;
    return p;
}

} // namespace

struct ApplicationService::Impl {
    api::ApplicationPort* port;
    std::map<std::string, api::DesktopApplication> apps;
    std::unordered_map<std::string, std::string> wmIndex;
    std::unordered_map<std::string, std::string> normKeys;

    explicit Impl(api::ApplicationPort* p) : port(p) {}

    void clear() {
        apps.clear();
        wmIndex.clear();
        normKeys.clear();
    }
};

ApplicationService::ApplicationService(api::ApplicationPort* port) : impl_(new Impl(port)) {
    rescan();
}

ApplicationService::~ApplicationService() {
    delete impl_;
}

void ApplicationService::rescan() {
    impl_->clear();
    std::vector<std::string> dirs = collectAppDirs();
    for (size_t di = 0; di < dirs.size(); ++di) {
        const std::string& dir = dirs[di];
        DIR* d = opendir(dir.c_str());
        if (!d) continue;
        struct dirent* ent;
        while ((ent = readdir(d)) != NULL) {
            std::string fname(ent->d_name);
            if (fname == "." || fname == "..") continue;
            if (fname.size() < 8 || fname.substr(fname.size() - 8) != ".desktop") continue;
            std::string full = dir + "/" + fname;
            if (!pathExistsFile(full)) continue;
            if (impl_->apps.find(fname) != impl_->apps.end()) continue;
            ParsedDesktop pd = parseDesktopFile(full);
            if (!pd.valid) continue;
            api::DesktopApplication app;
            app.id = api::DesktopAppId(fname);
            app.displayName = pd.name.empty() ? fname : pd.name;
            app.execCmd = pd.execCmd;
            app.iconName = pd.icon;
            app.categories = splitCategories(pd.categoriesRaw);
            app.startupWMClass = pd.startupWMClass;
            std::string base = fname.substr(0, fname.size() - 8);
            app.wmClassFallback = base;
            app.desktopFilePath = full;

            impl_->apps[fname] = app;

            if (!pd.startupWMClass.empty()) {
                std::string k = toLower(pd.startupWMClass);
                if (impl_->wmIndex.find(k) == impl_->wmIndex.end()) {
                    impl_->wmIndex[k] = fname;
                }
            }
            std::string fbLower = toLower(base);
            if (impl_->wmIndex.find(fbLower) == impl_->wmIndex.end()) {
                impl_->wmIndex[fbLower] = fname;
            }

            std::string norm = toLower(app.displayName + " " + fname + " " + pd.categoriesRaw + " " + pd.startupWMClass);
            impl_->normKeys[fname] = norm;
        }
        closedir(d);
    }
}

api::Result<api::DesktopApplication> ApplicationService::findById(const api::DesktopAppId& id) const {
    if (id.empty()) {
        return api::Result<api::DesktopApplication>::Err(api::Error::InvalidArgument, "empty app id");
    }
    std::map<std::string, api::DesktopApplication>::const_iterator it = impl_->apps.find(id.value);
    if (it == impl_->apps.end()) {
        return api::Result<api::DesktopApplication>::Err(api::Error::NotFound, "application not found");
    }
    return api::Result<api::DesktopApplication>::Ok(it->second);
}

api::Result<api::DesktopApplication> ApplicationService::findByWindowClass(const std::string& wmClass) const {
    std::string q = trim(wmClass);
    if (q.empty()) {
        return api::Result<api::DesktopApplication>::Err(api::Error::InvalidArgument, "empty wmClass");
    }
    std::string k = toLower(q);
    std::unordered_map<std::string, std::string>::const_iterator it = impl_->wmIndex.find(k);
    if (it == impl_->wmIndex.end()) {
        return api::Result<api::DesktopApplication>::Err(api::Error::NotFound, "wmClass not found");
    }
    std::map<std::string, api::DesktopApplication>::const_iterator ait = impl_->apps.find(it->second);
    if (ait == impl_->apps.end()) {
        return api::Result<api::DesktopApplication>::Err(api::Error::NotFound, "wmClass target missing");
    }
    return api::Result<api::DesktopApplication>::Ok(ait->second);
}

std::vector<api::DesktopApplication> ApplicationService::search(const std::string& query) const {
    std::vector<api::DesktopApplication> out;
    std::string q = toLower(trim(query));
    if (q.empty()) {
        out.reserve(impl_->apps.size());
        for (std::map<std::string, api::DesktopApplication>::const_iterator it = impl_->apps.begin(); it != impl_->apps.end(); ++it) {
            out.push_back(it->second);
        }
        return out;
    }
    for (std::map<std::string, api::DesktopApplication>::const_iterator it = impl_->apps.begin(); it != impl_->apps.end(); ++it) {
        std::unordered_map<std::string, std::string>::const_iterator nit = impl_->normKeys.find(it->first);
        if (nit == impl_->normKeys.end()) continue;
        if (nit->second.find(q) != std::string::npos) {
            out.push_back(it->second);
        }
    }
    return out;
}

std::vector<api::DesktopApplication> ApplicationService::all() const {
    std::vector<api::DesktopApplication> out;
    out.reserve(impl_->apps.size());
    for (std::map<std::string, api::DesktopApplication>::const_iterator it = impl_->apps.begin(); it != impl_->apps.end(); ++it) {
        out.push_back(it->second);
    }
    return out;
}

void ApplicationService::invalidate() {
    impl_->clear();
}

api::Status ApplicationService::launch(const api::DesktopAppId& id, const std::vector<std::string>& args) {
    if (!impl_->port) {
        return api::Status::make(api::Error::Unavailable, "no application backend");
    }
    if (id.empty()) {
        return api::Status::make(api::Error::InvalidArgument, "empty app id");
    }
    std::map<std::string, api::DesktopApplication>::const_iterator it = impl_->apps.find(id.value);
    if (it == impl_->apps.end()) {
        return api::Status::make(api::Error::NotFound, "application not found");
    }
    for (size_t i = 0; i < args.size(); ++i) {
        if (args[i].find('\0') != std::string::npos) {
            return api::Status::make(api::Error::InvalidArgument, "arg contains NUL");
        }
    }
    return impl_->port->launch(id, args);
}

api::Status ApplicationService::launch(const api::DesktopAppId& id) {
    std::vector<std::string> empty;
    return launch(id, empty);
}

api::Status ApplicationService::launchUri(const std::string& uri) {
    if (!impl_->port) {
        return api::Status::make(api::Error::Unavailable, "no application backend");
    }
    if (trim(uri).empty()) {
        return api::Status::make(api::Error::InvalidArgument, "empty uri");
    }
    if (uri.find('\0') != std::string::npos) {
        return api::Status::make(api::Error::InvalidArgument, "uri contains NUL");
    }
    return impl_->port->launchUri(uri);
}

} // namespace applications
} // namespace platform
} // namespace flamewm
