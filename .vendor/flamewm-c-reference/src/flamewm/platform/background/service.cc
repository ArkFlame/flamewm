#include "flamewm/platform/background/service.h"

#include <cstdio>
#include <fstream>
#include <sstream>
#include <map>
#include <vector>
#include <string>

namespace flamewm {
namespace platform {
namespace background {

const char* BackgroundService::kProductKey = "wallpaper";

namespace {

inline std::string trimCopy(const std::string& s) {
    size_t a = 0;
    while (a < s.size() && (s[a] == ' ' || s[a] == '\t' || s[a] == '\r'))
        ++a;
    size_t b = s.size();
    while (b > a && (s[b - 1] == ' ' || s[b - 1] == '\t' || s[b - 1] == '\r'))
        --b;
    return s.substr(a, b - a);
}

api::Status validateWallpaperPath(const std::string& path) {
    for (size_t i = 0; i < path.size(); ++i) {
        if (path[i] == '\n' || path[i] == '\r' || path[i] == '\0')
            return api::Status::make(api::Error::InvalidArgument, "invalid wallpaper path");
    }
    if (path.find('\0') != std::string::npos)
        return api::Status::make(api::Error::InvalidArgument, "invalid wallpaper path");
    return api::Status::Ok();
}

std::string readEngineWallpaper(const std::string& path) {
    std::ifstream in(path.c_str());
    if (!in.good())
        return std::string();
    std::string line;
    std::string result;
    while (std::getline(in, line)) {
        std::string t = trimCopy(line);
        if (t.empty() || t[0] == '#')
            continue;
        size_t eq = t.find('=');
        if (eq == std::string::npos)
            continue;
        std::string k = trimCopy(t.substr(0, eq));
        std::string v = trimCopy(t.substr(eq + 1));
        if (k == "DesktopBackgroundImage")
            result = v;
    }
    return result;
}

std::string readProductWallpaper(const std::string& path, bool* found) {
    if (found) *found = false;
    if (path.empty())
        return std::string();
    std::ifstream in(path.c_str());
    if (!in.good())
        return std::string();
    std::string line;
    std::string result;
    while (std::getline(in, line)) {
        std::string t = trimCopy(line);
        if (t.empty() || t[0] == '#')
            continue;
        size_t eq = t.find('=');
        if (eq == std::string::npos)
            continue;
        std::string k = trimCopy(t.substr(0, eq));
        std::string v = trimCopy(t.substr(eq + 1));
        if (k == BackgroundService::kProductKey) {
            result = v;
            if (found) *found = true;
        }
    }
    return result;
}

api::Status writeEnginePrefsAtomically(const std::string& path,
                                       const std::string& wallpaper) {
    if (path.empty())
        return api::Status::make(api::Error::InvalidArgument, "empty engine prefs path");
    std::map<std::string, std::string> kv;
    std::vector<std::string> order;
    {
        std::ifstream in(path.c_str());
        if (in.good()) {
            std::string line;
            while (std::getline(in, line)) {
                std::string t = trimCopy(line);
                if (t.empty() || t[0] == '#')
                    continue;
                size_t eq = t.find('=');
                if (eq == std::string::npos)
                    continue;
                std::string k = trimCopy(t.substr(0, eq));
                std::string v = trimCopy(t.substr(eq + 1));
                if (k.empty())
                    continue;
                if (kv.find(k) == kv.end())
                    order.push_back(k);
                kv[k] = v;
            }
        }
    }
    kv["DesktopBackgroundImage"] = wallpaper;
    bool found = false;
    for (size_t i = 0; i < order.size(); ++i) {
        if (order[i] == "DesktopBackgroundImage") { found = true; break; }
    }
    if (!found)
        order.push_back("DesktopBackgroundImage");

    std::ostringstream oss;
    for (size_t i = 0; i < order.size(); ++i) {
        const std::string& k = order[i];
        std::map<std::string, std::string>::const_iterator it = kv.find(k);
        if (it != kv.end())
            oss << it->first << "=" << it->second << "\n";
    }
    for (std::map<std::string, std::string>::const_iterator it = kv.begin(); it != kv.end(); ++it) {
        bool inOrder = false;
        for (size_t i = 0; i < order.size(); ++i) {
            if (order[i] == it->first) { inOrder = true; break; }
        }
        if (!inOrder)
            oss << it->first << "=" << it->second << "\n";
    }

    std::string tmp = path + ".tmp";
    std::string data = oss.str();
    FILE* f = fopen(tmp.c_str(), "wb");
    if (!f) {
        return api::Status::make(api::Error::IoFailure, "open tmp failed");
    }
    size_t w = fwrite(data.data(), 1, data.size(), f);
    if (w != data.size()) {
        fclose(f);
        return api::Status::make(api::Error::IoFailure, "write failed");
    }
    if (fflush(f) != 0) {
        fclose(f);
        return api::Status::make(api::Error::IoFailure, "flush failed");
    }
    fclose(f);
    if (rename(tmp.c_str(), path.c_str()) != 0) {
        return api::Status::make(api::Error::IoFailure, "rename failed");
    }
    return api::Status::Ok();
}

api::Status writeProductWallpaperAtomically(const std::string& path,
                                            const std::string& wallpaper) {
    if (path.empty())
        return api::Status::Ok();
    std::map<std::string, std::string> kv;
    std::vector<std::string> order;
    {
        std::ifstream in(path.c_str());
        if (in.good()) {
            std::string line;
            while (std::getline(in, line)) {
                std::string t = trimCopy(line);
                if (t.empty() || t[0] == '#')
                    continue;
                size_t eq = t.find('=');
                if (eq == std::string::npos)
                    continue;
                std::string k = trimCopy(t.substr(0, eq));
                std::string v = trimCopy(t.substr(eq + 1));
                if (k.empty())
                    continue;
                if (kv.find(k) == kv.end())
                    order.push_back(k);
                kv[k] = v;
            }
        }
    }
    bool had = kv.find(BackgroundService::kProductKey) != kv.end();
    kv[BackgroundService::kProductKey] = wallpaper;
    if (!had) {
        bool inOrder = false;
        for (size_t i = 0; i < order.size(); ++i) {
            if (order[i] == BackgroundService::kProductKey) { inOrder = true; break; }
        }
        if (!inOrder)
            order.push_back(BackgroundService::kProductKey);
    }
    std::ostringstream oss;
    for (size_t i = 0; i < order.size(); ++i) {
        const std::string& k = order[i];
        std::map<std::string, std::string>::const_iterator it = kv.find(k);
        if (it != kv.end())
            oss << it->first << "=" << it->second << "\n";
    }
    for (std::map<std::string, std::string>::const_iterator it = kv.begin(); it != kv.end(); ++it) {
        bool inOrder = false;
        for (size_t i = 0; i < order.size(); ++i) {
            if (order[i] == it->first) { inOrder = true; break; }
        }
        if (!inOrder)
            oss << it->first << "=" << it->second << "\n";
    }
    std::string tmp = path + ".tmp";
    std::string data = oss.str();
    FILE* f = fopen(tmp.c_str(), "wb");
    if (!f) {
        return api::Status::make(api::Error::IoFailure, "open tmp failed");
    }
    size_t w = fwrite(data.data(), 1, data.size(), f);
    if (w != data.size()) {
        fclose(f);
        return api::Status::make(api::Error::IoFailure, "write failed");
    }
    if (fflush(f) != 0) {
        fclose(f);
        return api::Status::make(api::Error::IoFailure, "flush failed");
    }
    fclose(f);
    if (rename(tmp.c_str(), path.c_str()) != 0) {
        return api::Status::make(api::Error::IoFailure, "rename failed");
    }
    return api::Status::Ok();
}

} // namespace

struct BackgroundService::Impl {
    api::BackgroundPort* port;
    std::string enginePrefsPath;
    std::string productPath;
    api::BackgroundState state;

    Impl(api::BackgroundPort* p, const std::string& ep, const std::string& pp)
        : port(p), enginePrefsPath(ep), productPath(pp), state() {
        // Prefer product setting as source of truth; fallback to engine prefs.
        bool found = false;
        std::string prod = readProductWallpaper(productPath, &found);
        if (!productPath.empty() && found) {
            state.wallpaperPath = prod;
        } else if (!enginePrefsPath.empty()) {
            state.wallpaperPath = readEngineWallpaper(enginePrefsPath);
        } else {
            state.wallpaperPath = std::string();
        }
        state.fillMode = std::string();
        state.color = std::string();
    }
};

BackgroundService::BackgroundService(api::BackgroundPort* port,
                                     const std::string& enginePrefsPath)
    : impl_(new Impl(port, enginePrefsPath, std::string())) {}

BackgroundService::BackgroundService(api::BackgroundPort* port,
                                     const std::string& enginePrefsPath,
                                     const std::string& productSettingsPath)
    : impl_(new Impl(port, enginePrefsPath, productSettingsPath)) {}

BackgroundService::~BackgroundService() {
    delete impl_;
}

api::Status BackgroundService::setWallpaper(const std::string& path) {
    // C3: validate -> persist product -> project engine via port -> reload -> publish.
    // No second painter — Flame never paints root. Port failure before publish.
    // Typed errors: InvalidArgument, IoFailure, EngineRejected/Unavailable.
    api::Status vs = validateWallpaperPath(path);
    if (!vs.ok())
        return vs;

    if (!impl_->port && impl_->enginePrefsPath.empty() && impl_->productPath.empty()) {
        return api::Status::make(api::Error::Unavailable, "no port and no prefs path");
    }

    api::BackgroundState candidate = impl_->state;
    candidate.wallpaperPath = path;

    // Persist product setting first — authoritative source on restart.
    if (!impl_->productPath.empty()) {
        api::Status pws = writeProductWallpaperAtomically(impl_->productPath, path);
        if (!pws.ok())
            return pws;
    }

    // Project engine preferences via port; handle failure before publish.
    if (impl_->port) {
        api::Status ps = impl_->port->project(candidate);
        if (!ps.ok())
            return ps;
        api::Status rs = impl_->port->reload();
        if (!rs.ok())
            return rs;
    } else if (!impl_->enginePrefsPath.empty()) {
        // Fallback when no port (tests/headless) — write engine prefs directly.
        api::Status es = writeEnginePrefsAtomically(impl_->enginePrefsPath, path);
        if (!es.ok())
            return es;
    }

    // Publish only after all persistence/projection succeeded.
    impl_->state.wallpaperPath = path;
    return api::Status::Ok();
}

api::BackgroundState BackgroundService::current() const {
    return impl_->state;
}

} // namespace background
} // namespace platform
} // namespace flamewm
