#include "service.h"
#include "effect.h"

#include <algorithm>
#include <cstdio>
#include <fstream>
#include <map>
#include <sstream>
#include <string>
#include <vector>

#include <cerrno>
#include <sys/stat.h>

namespace flamewm {
namespace platform {
namespace settings {

namespace {

inline std::string trim(const std::string& s) {
    size_t a = 0;
    while (a < s.size() && (s[a] == ' ' || s[a] == '\t' || s[a] == '\r'))
        ++a;
    size_t b = s.size();
    while (b > a && (s[b - 1] == ' ' || s[b - 1] == '\t' || s[b - 1] == '\r'))
        --b;
    return s.substr(a, b - a);
}

bool isValidKey(const std::string& k) {
    if (k.empty()) return false;
    if (k == "revision") return false;
    for (size_t i = 0; i < k.size(); ++i) {
        unsigned char c = static_cast<unsigned char>(k[i]);
        if (c == '=' || c == '\n' || c == '\r') return false;
        if (c < 0x20 || c == 0x7f) return false;
    }
    return true;
}

bool isValidSection(const std::string& s) {
    if (s.empty()) return false;
    if (s == "revision") return false;
    for (size_t i = 0; i < s.size(); ++i) {
        unsigned char c = static_cast<unsigned char>(s[i]);
        if (c == '=' || c == '\n' || c == '\r' || c == '.') {
            if (c == '.') return false;
        }
        if (c < 0x20 || c == 0x7f) return false;
        if (c == ' ' || c == '\t') return false;
    }
    return true;
}

bool ensureParentDir(const std::string& file) {
    size_t slash = file.rfind('/');
    if (slash == std::string::npos)
        return true;
    std::string dir = file.substr(0, slash);
    if (dir.empty())
        return true;

    struct stat st;
    if (stat(dir.c_str(), &st) == 0)
        return S_ISDIR(st.st_mode);

    std::string current;
    if (dir[0] == '/')
        current = "/";
    size_t start = dir[0] == '/' ? 1 : 0;
    while (start <= dir.size()) {
        size_t next = dir.find('/', start);
        std::string segment = next == std::string::npos
            ? dir.substr(start) : dir.substr(start, next - start);
        if (!segment.empty()) {
            if (!current.empty() && current[current.size() - 1] != '/')
                current += "/";
            current += segment;
            if (stat(current.c_str(), &st) != 0 &&
                mkdir(current.c_str(), 0755) != 0 && errno != EEXIST)
                return false;
        }
        if (next == std::string::npos)
            break;
        start = next + 1;
    }
    return stat(dir.c_str(), &st) == 0 && S_ISDIR(st.st_mode);
}

} // namespace

struct SettingsService::Impl {
    std::string path;
    uint64_t revision;
    std::map<std::string, std::string> values;
    std::map<int, Listener> listeners;
    int nextId;
    std::vector<SettingsEffect*> effects;

    explicit Impl(const std::string& p) : path(p), revision(1), values(), listeners(), nextId(1), effects() {
        load();
    }

    void load() {
        std::ifstream in(path.c_str());
        if (!in.good()) {
            revision = 1;
            values.clear();
            return;
        }
        std::string content((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());
        std::istringstream iss(content);
        std::string line;
        uint64_t fileRev = 0;
        bool hasRev = false;
        std::map<std::string, std::string> tmp;
        while (std::getline(iss, line)) {
            std::string t = trim(line);
            if (t.empty() || t[0] == '#') continue;
            size_t eq = t.find('=');
            if (eq == std::string::npos) continue;
            std::string key = trim(t.substr(0, eq));
            std::string val = trim(t.substr(eq + 1));
            if (key.empty()) continue;
            if (key == "revision") {
                if (val.empty()) continue;
                bool digits = true;
                for (size_t i = 0; i < val.size(); ++i) {
                    if (val[i] < '0' || val[i] > '9') { digits = false; break; }
                }
                if (!digits) continue;
                char* e = 0;
                unsigned long long v = strtoull(val.c_str(), &e, 10);
                if (e && *e == '\0') {
                    fileRev = static_cast<uint64_t>(v);
                    hasRev = true;
                }
                continue;
            }
            tmp[key] = val;
        }
        values = tmp;
        if (hasRev && fileRev != 0) {
            revision = fileRev;
        } else {
            revision = 1;
        }
    }

    std::string serialize(uint64_t rev, const std::map<std::string, std::string>& vals) const {
        std::ostringstream oss;
        oss << "revision=" << rev << "\n";
        for (std::map<std::string, std::string>::const_iterator it = vals.begin(); it != vals.end(); ++it) {
            oss << it->first << "=" << it->second << "\n";
        }
        return oss.str();
    }

    api::Status persist(uint64_t rev, const std::map<std::string, std::string>& vals) {
        if (path.empty()) {
            return api::Status::make(api::Error::InvalidArgument, "empty persistence path");
        }
        if (!ensureParentDir(path)) {
            return api::Status::make(api::Error::IoFailure, "cannot create settings directory");
        }
        std::string tmp = path + ".tmp";
        std::string data = serialize(rev, vals);
        FILE* f = fopen(tmp.c_str(), "wb");
        if (!f) {
            return api::Status::make(api::Error::IoFailure, "open tmp failed");
        }
        size_t w = fwrite(data.data(), 1, data.size(), f);
        if (w != data.size()) {
            fclose(f);
            std::remove(tmp.c_str());
            return api::Status::make(api::Error::IoFailure, "write failed");
        }
        if (fflush(f) != 0) {
            fclose(f);
            std::remove(tmp.c_str());
            return api::Status::make(api::Error::IoFailure, "flush failed");
        }
        if (fclose(f) != 0) {
            std::remove(tmp.c_str());
            return api::Status::make(api::Error::IoFailure, "close failed");
        }
        if (rename(tmp.c_str(), path.c_str()) != 0) {
            std::remove(tmp.c_str());
            return api::Status::make(api::Error::IoFailure, "rename failed");
        }
        return api::Status::Ok();
    }

    void rollbackPrepared(size_t preparedCount) {
        for (size_t i = 0; i < preparedCount; ++i) {
            if (effects[i]) effects[i]->rollbackPrepared();
        }
    }

    void commitPrepared() {
        for (size_t i = 0; i < effects.size(); ++i) {
            if (effects[i]) effects[i]->commitPrepared();
        }
    }

    // Core transactional commit: stale check already done by caller,
    // candidate already validated, changedKeys computed and non-empty.
    // Order: prepare(all) -> applyPrepared(all) -> persist atomic
    //        -> publish -> commit(all) -> notify.
    // Any failure before publish: rollback, no persist, no revision bump.
    api::Status transact(const std::map<std::string, std::string>& candidate,
                         const std::vector<std::string>& changedKeys) {
        if (changedKeys.empty()) {
            return api::Status::Ok();
        }
        uint64_t newRev = revision + 1;
        api::SettingsSnapshot candidateSnap;
        candidateSnap.revision = newRev;
        candidateSnap.values = candidate;

        // Phase 1: prepare participants
        size_t prepared = 0;
        for (size_t i = 0; i < effects.size(); ++i) {
            SettingsEffect* e = effects[i];
            if (!e) { prepared++; continue; }
            api::Status st = e->prepare(candidateSnap, changedKeys);
            if (!st.ok()) {
                rollbackPrepared(prepared);
                return st;
            }
            prepared++;
        }

        // Phase 2: apply prepared
        for (size_t i = 0; i < effects.size(); ++i) {
            SettingsEffect* e = effects[i];
            if (!e) continue;
            api::Status st = e->applyPrepared();
            if (!st.ok()) {
                rollbackPrepared(effects.size());
                return st;
            }
        }

        // Phase 3: atomic persistence
        api::Status pst = persist(newRev, candidate);
        if (!pst.ok()) {
            rollbackPrepared(effects.size());
            return pst;
        }

        // Phase 4: publish new effective snapshot/revision
        revision = newRev;
        values = candidate;

        // Phase 5: participant commit
        commitPrepared();

        // Phase 6: notify listeners (copy for re-entrancy)
        std::map<int, Listener> copy = listeners;
        for (std::map<int, Listener>::const_iterator it = copy.begin(); it != copy.end(); ++it) {
            if (it->second) {
                it->second(newRev, changedKeys);
            }
        }

        return api::Status::Ok();
    }
};

SettingsService::SettingsService(const std::string& configPath) : impl_(new Impl(configPath)) {}

SettingsService::~SettingsService() {
    delete impl_;
}

api::SettingsSnapshot SettingsService::snapshot() const {
    api::SettingsSnapshot s;
    s.revision = impl_->revision;
    s.values = impl_->values;
    return s;
}

uint64_t SettingsService::revision() const {
    return impl_->revision;
}

void SettingsService::addEffect(SettingsEffect* effect) {
    if (!effect) return;
    for (size_t i = 0; i < impl_->effects.size(); ++i) {
        if (impl_->effects[i] == effect) return;
    }
    impl_->effects.push_back(effect);
}

void SettingsService::removeEffect(SettingsEffect* effect) {
    if (!effect) return;
    for (size_t i = 0; i < impl_->effects.size(); ++i) {
        if (impl_->effects[i] == effect) {
            impl_->effects.erase(impl_->effects.begin() + i);
            return;
        }
    }
}

api::Status SettingsService::apply(uint64_t expectedRevision,
                                   const std::vector<api::SettingsChange>& changes) {
    if (expectedRevision != impl_->revision) {
        return api::Status::make(api::Error::StaleRevision, "stale revision");
    }
    for (size_t i = 0; i < changes.size(); ++i) {
        const api::SettingsChange& c = changes[i];
        std::string key = trim(c.key);
        if (key.empty() || key != c.key) {
            if (trim(c.key).empty()) {
                return api::Status::make(api::Error::InvalidArgument, "empty key");
            }
            if (key != c.key) {
                return api::Status::make(api::Error::InvalidArgument, "key has whitespace: " + c.key);
            }
        }
        if (!isValidKey(key)) {
            return api::Status::make(api::Error::InvalidArgument, "invalid key: " + key);
        }
        for (size_t j = 0; j < c.value.size(); ++j) {
            if (c.value[j] == '\n' || c.value[j] == '\r') {
                return api::Status::make(api::Error::InvalidArgument, "invalid value for key: " + key);
            }
        }
    }

    std::map<std::string, std::string> candidate = impl_->values;
    for (size_t i = 0; i < changes.size(); ++i) {
        const api::SettingsChange& c = changes[i];
        std::string key = trim(c.key);
        if (c.value.empty()) {
            candidate.erase(key);
        } else {
            candidate[key] = c.value;
        }
    }

    std::vector<std::string> changedKeys;
    for (std::map<std::string, std::string>::const_iterator it = candidate.begin(); it != candidate.end(); ++it) {
        std::map<std::string, std::string>::const_iterator oit = impl_->values.find(it->first);
        if (oit == impl_->values.end() || oit->second != it->second) {
            changedKeys.push_back(it->first);
        }
    }
    for (std::map<std::string, std::string>::const_iterator it = impl_->values.begin(); it != impl_->values.end(); ++it) {
        if (candidate.find(it->first) == candidate.end()) {
            changedKeys.push_back(it->first);
        }
    }
    if (changedKeys.empty()) {
        return api::Status::Ok();
    }
    std::sort(changedKeys.begin(), changedKeys.end());
    changedKeys.erase(std::unique(changedKeys.begin(), changedKeys.end()), changedKeys.end());

    return impl_->transact(candidate, changedKeys);
}

api::Status SettingsService::resetSection(uint64_t expectedRevision, const std::string& section) {
    if (expectedRevision != impl_->revision) {
        return api::Status::make(api::Error::StaleRevision, "stale revision");
    }
    std::string sec = trim(section);
    if (sec.empty() || sec != section) {
        return api::Status::make(api::Error::InvalidArgument, "invalid section");
    }
    if (!isValidSection(sec)) {
        return api::Status::make(api::Error::InvalidArgument, "invalid section: " + sec);
    }
    std::string prefix = sec + ".";
    std::vector<std::string> toRemove;
    for (std::map<std::string, std::string>::const_iterator it = impl_->values.begin(); it != impl_->values.end(); ++it) {
        if (it->first.compare(0, prefix.size(), prefix) == 0) {
            toRemove.push_back(it->first);
        }
    }
    if (toRemove.empty()) {
        return api::Status::Ok();
    }

    std::map<std::string, std::string> candidate = impl_->values;
    for (size_t i = 0; i < toRemove.size(); ++i) {
        candidate.erase(toRemove[i]);
    }

    std::sort(toRemove.begin(), toRemove.end());

    return impl_->transact(candidate, toRemove);
}

int SettingsService::addListener(Listener cb) {
    if (!cb) return 0;
    int id = impl_->nextId++;
    impl_->listeners[id] = cb;
    return id;
}

void SettingsService::removeListener(int id) {
    impl_->listeners.erase(id);
}

} // namespace settings
} // namespace platform
} // namespace flamewm
