#include "flamewm/engine/icewm/background_adapter.h"
#include "flamewm/engine/icewm/access.h"

#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <string>
#include <vector>

#if __has_include(<unistd.h>)
#include <unistd.h>
#endif
#if __has_include(<sys/wait.h>)
#include <sys/wait.h>
#endif
#if __has_include(<sys/stat.h>)
#include <sys/stat.h>
#endif
#include <cerrno>
#include <cstring>

#if __has_include(<X11/extensions/Xrender.h>)
#if __has_include("wmapp.h")
#include "wmapp.h"
#endif
#if __has_include("wmaction.h")
#include "wmaction.h"
#endif
#if __has_include("yaction.h")
#include "yaction.h"
#endif
#endif
#ifdef Status
#undef Status
#endif

namespace flamewm {
namespace engine {
namespace icewm {

BackgroundAdapter::BackgroundAdapter(const std::string& enginePrefsPath)
    : enginePrefsPath_(enginePrefsPath) {}

BackgroundAdapter::~BackgroundAdapter() {}

void BackgroundAdapter::setEnginePrefsPath(const std::string& path) {
    enginePrefsPath_ = path;
}

std::string BackgroundAdapter::enginePrefsPath() const {
    return enginePrefsPath_;
}

std::string BackgroundAdapter::defaultEnginePrefsPath() {
    const char* xdg = getenv("XDG_CONFIG_HOME");
    if (xdg != nullptr && xdg[0] != '\0') {
        return std::string(xdg) + "/flamewm/engine-preferences";
    }
    const char* home = getenv("HOME");
    if (home != nullptr && home[0] != '\0') {
        return std::string(home) + "/.config/flamewm/engine-preferences";
    }
    return std::string("/tmp/flamewm/engine-preferences");
}

std::string BackgroundAdapter::effectivePreferencesPath() const {
    if (!enginePrefsPath_.empty()) {
        return enginePrefsPath_;
    }
    return defaultEnginePrefsPath();
}

static bool ensureParentDir(const std::string& file) {
    size_t slash = file.rfind('/');
    if (slash == std::string::npos) return true;
    std::string dir = file.substr(0, slash);
    if (dir.empty()) return true;
    struct stat st;
    if (stat(dir.c_str(), &st) == 0) {
        return true;
    }
    // mkdir -p
    std::string cur;
    if (!dir.empty() && dir[0] == '/') cur = "/";
    size_t start = (dir[0] == '/' ? 1 : 0);
    while (start <= dir.size()) {
        size_t s = dir.find('/', start);
        std::string seg;
        if (s == std::string::npos) seg = dir.substr(start);
        else seg = dir.substr(start, s - start);
        if (!seg.empty()) {
            if (!cur.empty() && cur[cur.size() - 1] != '/') cur += "/";
            cur += seg;
            if (stat(cur.c_str(), &st) != 0) {
                if (mkdir(cur.c_str(), 0755) != 0 && errno != EEXIST) return false;
            }
        }
        if (s == std::string::npos) break;
        start = s + 1;
    }
    return stat(dir.c_str(), &st) == 0;
}

api::Status BackgroundAdapter::writePreferencesEntry(const std::string& file,
                                                     const std::string& key,
                                                     const std::string& value) {
    if (file.empty() || key.empty()) {
        return api::Status::make(api::Error::InvalidArgument, "empty file/key");
    }
    // Defensive: never overwrite generic icewm preferences. Caller must have
    // resolved effectivePreferencesPath() to derivative flamewm/engine-preferences.
    // If path ends with icewm/preferences, reject to avoid upstream clobber.
    if (file.find("/icewm/preferences") != std::string::npos) {
        return api::Status::make(api::Error::InvalidArgument, "refusing to overwrite icewm/preferences");
    }
    if (value.find('\n') != std::string::npos || value.find('\r') != std::string::npos ||
        value.find('\0') != std::string::npos) {
        return api::Status::make(api::Error::InvalidArgument, "invalid value");
    }
    if (!ensureParentDir(file)) {
        return api::Status::make(api::Error::IoFailure, "cannot create prefs dir");
    }

    std::ifstream in(file.c_str());
    std::string line;
    bool found = false;
    std::string entry = key + "=\"" + value + "\"";

    std::string out;
    if (in.good()) {
        while (std::getline(in, line)) {
            if (!found && line.compare(0, key.size() + 1, key + "=") == 0) {
                out += entry + "\n";
                found = true;
            } else {
                out += line + "\n";
            }
        }
        in.close();
    }
    if (!found) {
        out += entry + "\n";
    }

    std::string tmp = file + ".tmp";
    FILE* f = fopen(tmp.c_str(), "wb");
    if (!f) {
        return api::Status::make(api::Error::IoFailure, "cannot write preferences tmp");
    }
    size_t w = fwrite(out.data(), 1, out.size(), f);
    if (w != out.size()) {
        fclose(f);
        unlink(tmp.c_str());
        return api::Status::make(api::Error::IoFailure, "write failed");
    }
    if (fflush(f) != 0) {
        fclose(f);
        unlink(tmp.c_str());
        return api::Status::make(api::Error::IoFailure, "flush failed");
    }
    fclose(f);
    if (rename(tmp.c_str(), file.c_str()) != 0) {
        unlink(tmp.c_str());
        return api::Status::make(api::Error::IoFailure, std::string("rename failed: ") + strerror(errno));
    }
    return api::Status::Ok();
}

api::Status BackgroundAdapter::reloadIcewmbg() {
#if __has_include(<X11/extensions/Xrender.h>) && __has_include("wmapp.h") && __has_include("wmaction.h") && __has_include("yaction.h")
    YWMApp* app = EngineAccess::appTyped();
    if (app != nullptr) {
        app->actionPerformed(YAction(actionIcewmbg));
        return api::Status::Ok();
    }
#endif
    // Fallback: direct icewmbg --restart via double-fork so no zombie remains.
    // Parent reaps intermediate child; grandchild is reparented to init.
    pid_t pid = fork();
    if (pid < 0) {
        return api::Status::make(api::Error::IoFailure, "fork failed");
    }
    if (pid == 0) {
        pid_t gpid = fork();
        if (gpid < 0) _exit(127);
        if (gpid == 0) {
            execlp("icewmbg", "icewmbg", "--restart", static_cast<char*>(nullptr));
            _exit(127);
        }
        _exit(0);
    }
#if __has_include(<sys/wait.h>)
    int st = 0;
    while (waitpid(pid, &st, 0) < 0) {
        if (errno != EINTR) break;
    }
#else
    (void)pid;
#endif
    return api::Status::Ok();
}

api::Status BackgroundAdapter::project(const api::BackgroundState& state) {
    std::string prefFile = effectivePreferencesPath();
    if (prefFile.empty()) {
        return api::Status::make(api::Error::Unavailable, "no prefs path");
    }

    if (state.wallpaperPath.find('\0') != std::string::npos ||
        state.wallpaperPath.find('\n') != std::string::npos ||
        state.wallpaperPath.find('\r') != std::string::npos) {
        return api::Status::make(api::Error::InvalidArgument, "invalid wallpaperPath");
    }
    if (state.color.find('\0') != std::string::npos ||
        state.color.find('\n') != std::string::npos ||
        state.color.find('\r') != std::string::npos) {
        return api::Status::make(api::Error::InvalidArgument, "invalid color");
    }

    if (!state.wallpaperPath.empty()) {
        api::Status s = writePreferencesEntry(prefFile, "DesktopBackgroundImage",
                                             state.wallpaperPath);
        if (!s.ok()) return s;
    }
    if (!state.color.empty()) {
        api::Status s = writePreferencesEntry(prefFile, "DesktopBackgroundColor",
                                             state.color);
        if (!s.ok()) return s;
    }
    if (!state.fillMode.empty()) {
        if (state.fillMode.find('\0') != std::string::npos ||
            state.fillMode.find('\n') != std::string::npos ||
            state.fillMode.find('\r') != std::string::npos) {
            return api::Status::make(api::Error::InvalidArgument, "invalid fillMode");
        }
        api::Status s = writePreferencesEntry(prefFile, "DesktopBackgroundScaled",
                                             state.fillMode);
        if (!s.ok()) return s;
    }

    return reloadIcewmbg();
}

api::Status BackgroundAdapter::reload() {
    return reloadIcewmbg();
}

} // namespace icewm
} // namespace engine
} // namespace flamewm
