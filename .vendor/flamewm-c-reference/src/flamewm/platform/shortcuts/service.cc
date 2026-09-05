#include "service.h"

#include <algorithm>
#include <cctype>
#include <vector>

// api action string definitions (single TU owning the symbols)
namespace flamewm { namespace api {
const std::string ActionToggleStartMenu("ToggleStartMenu");
const std::string ActionWorkspaceLeft("WorkspaceLeft");
const std::string ActionWorkspaceRight("WorkspaceRight");
const std::string ActionWorkspaceUp("WorkspaceUp");
const std::string ActionWorkspaceDown("WorkspaceDown");
const std::string ActionWindowClose("WindowClose");
const std::string ActionWindowMinimize("WindowMinimize");
const std::string ActionWindowMaximize("WindowMaximize");
} }

namespace flamewm {
namespace platform {
namespace shortcuts {

static std::string trimCopy(const std::string& s) {
    size_t a = 0;
    while (a < s.size() && (s[a] == ' ' || s[a] == '\t' || s[a] == '\n' || s[a] == '\r')) ++a;
    size_t b = s.size();
    while (b > a && (s[b - 1] == ' ' || s[b - 1] == '\t' || s[b - 1] == '\n' || s[b - 1] == '\r')) --b;
    return s.substr(a, b - a);
}

static std::string toLowerCopy(const std::string& s) {
    std::string out = s;
    for (size_t i = 0; i < out.size(); ++i) out[i] = static_cast<char>(::tolower(static_cast<unsigned char>(out[i])));
    return out;
}

static std::string normalizeKey(const std::string& raw) {
    return trimCopy(raw);
}

static bool isEscape(const std::string& normalized) {
    return normalized == "Escape";
}

static bool isModifierOnly(const std::string& normalized) {
    std::string low = toLowerCopy(normalized);
    std::vector<std::string> parts;
    size_t start = 0;
    for (size_t i = 0; i <= low.size(); ++i) {
        if (i == low.size() || low[i] == '+') {
            std::string t = low.substr(start, i - start);
            t = trimCopy(t);
            if (!t.empty()) parts.push_back(t);
            start = i + 1;
        }
    }
    if (parts.empty()) return false;
    for (size_t i = 0; i < parts.size(); ++i) {
        const std::string& p = parts[i];
        if (p == "ctrl" || p == "shift" || p == "alt" || p == "super" || p == "meta" || p == "hyper" || p == "super_l" || p == "super_r") continue;
        return false;
    }
    return true;
}

static bool actionAllowsModifierOnly(const std::string& action) {
    return action == api::ActionToggleStartMenu;
}

static bool isKnownAction(const std::string& action) {
    return action == api::ActionToggleStartMenu
        || action == api::ActionWorkspaceLeft
        || action == api::ActionWorkspaceRight
        || action == api::ActionWorkspaceUp
        || action == api::ActionWorkspaceDown
        || action == api::ActionWindowClose
        || action == api::ActionWindowMinimize
        || action == api::ActionWindowMaximize;
}

static bool validateNormalized(const std::string& n, std::string* err) {
    if (n.empty()) return true;
    if (n.size() > 64) {
        if (err) *err = "binding too long";
        return false;
    }
    bool hasNonSpace = false;
    for (size_t i = 0; i < n.size(); ++i) {
        if (n[i] != ' ' && n[i] != '\t') { hasNonSpace = true; break; }
    }
    if (!hasNonSpace) {
        if (err) *err = "invalid binding";
        return false;
    }
    return true;
}

struct ShortcutService::Impl {
    api::ShortcutPort* port;
    uint64_t revision;
    std::map<std::string, api::KeyBinding> bindings;
    std::map<std::string, api::KeyBinding> defaults;
    bool hasStaged;
    std::map<std::string, api::KeyBinding> staged;
    std::map<int, std::function<void(uint64_t)> > listeners;
    int nextListenerId;

    explicit Impl(api::ShortcutPort* p) : port(p), revision(1), hasStaged(false), nextListenerId(1) {
        defaults[api::ActionToggleStartMenu] = api::KeyBinding(std::string("Super_L"));
        defaults[api::ActionWorkspaceLeft]   = api::KeyBinding(std::string("Ctrl+Super+Left"));
        defaults[api::ActionWorkspaceRight]  = api::KeyBinding(std::string("Ctrl+Super+Right"));
        defaults[api::ActionWorkspaceUp]     = api::KeyBinding(std::string("Ctrl+Super+Up"));
        defaults[api::ActionWorkspaceDown]   = api::KeyBinding(std::string("Ctrl+Super+Down"));
        defaults[api::ActionWindowClose]     = api::KeyBinding(std::string("Alt+F4"));
        defaults[api::ActionWindowMinimize]  = api::KeyBinding::Unassigned();
        defaults[api::ActionWindowMaximize]  = api::KeyBinding::Unassigned();
        bindings = defaults;
    }

    void notify() {
        for (std::map<int, std::function<void(uint64_t)> >::const_iterator it = listeners.begin(); it != listeners.end(); ++it) {
            if (it->second) it->second(revision);
        }
    }

    api::KeyBinding normalizeBinding(const api::KeyBinding& in) const {
        std::string n = normalizeKey(in.key);
        if (isEscape(n)) return api::KeyBinding::Unassigned();
        if (n.empty()) return api::KeyBinding::Unassigned();
        return api::KeyBinding(n);
    }

    api::Status checkConflicts(const std::map<std::string, api::KeyBinding>& candidate) const {
        std::map<std::string, std::string> normLower;
        for (std::map<std::string, api::KeyBinding>::const_iterator it = candidate.begin(); it != candidate.end(); ++it) {
            if (!it->second.assigned()) continue;
            std::string n = normalizeKey(it->second.key);
            if (n.empty()) continue;
            normLower[it->first] = toLowerCopy(n);
        }
        for (std::map<std::string, std::string>::const_iterator a = normLower.begin(); a != normLower.end(); ++a) {
            std::map<std::string, std::string>::const_iterator b = a;
            ++b;
            for (; b != normLower.end(); ++b) {
                if (a->second == b->second) {
                    return api::Status::make(api::Error::Conflict, std::string("conflict: ") + a->first + " vs " + b->first + " (" + a->second + ")");
                }
            }
        }
        return api::Status::Ok();
    }
};

ShortcutService::ShortcutService(api::ShortcutPort* port) : impl_(new Impl(port)) {}
ShortcutService::~ShortcutService() { delete impl_; }

api::ShortcutSnapshot ShortcutService::snapshot() const {
    return api::ShortcutSnapshot(impl_->revision, impl_->bindings);
}

uint64_t ShortcutService::revision() const {
    return impl_->revision;
}

api::Status ShortcutService::setBinding(const std::string& action, const api::KeyBinding& binding, uint64_t expectedRevision) {
    if (expectedRevision != impl_->revision) {
        return api::Status::make(api::Error::StaleRevision, "stale revision");
    }
    if (!isKnownAction(action)) {
        return api::Status::make(api::Error::NotFound, std::string("unknown action: ") + action);
    }
    api::KeyBinding norm = impl_->normalizeBinding(binding);
    std::string n = norm.assigned() ? normalizeKey(norm.key) : std::string();
    if (norm.assigned()) {
        std::string verr;
        if (!validateNormalized(n, &verr)) {
            return api::Status::make(api::Error::InvalidArgument, verr);
        }
        if (isModifierOnly(n) && !actionAllowsModifierOnly(action)) {
            return api::Status::make(api::Error::InvalidArgument, "modifier-only not allowed for " + action);
        }
    }
    std::map<std::string, api::KeyBinding> candidate = impl_->bindings;
    candidate[action] = norm;
    api::Status cs = impl_->checkConflicts(candidate);
    if (!cs.ok()) return cs;

    // Transaction: port.prepare -> port.commit, publish revision; failure -> rollback + old revision.
    if (!impl_->port) {
        return api::Status::make(api::Error::Unavailable, "no shortcut port");
    }
    api::Status ps = impl_->port->prepare(candidate);
    if (!ps.ok()) {
        impl_->port->rollback();
        return ps;
    }
    api::Status cm = impl_->port->commit();
    if (!cm.ok()) {
        impl_->port->rollback();
        return cm;
    }
    impl_->bindings = candidate;
    impl_->hasStaged = false;
    impl_->staged.clear();
    impl_->revision += 1;
    impl_->notify();
    return api::Status::Ok();
}

api::Status ShortcutService::clearBinding(const std::string& action, uint64_t expectedRevision) {
    if (expectedRevision != impl_->revision) {
        return api::Status::make(api::Error::StaleRevision, "stale revision");
    }
    if (!isKnownAction(action)) {
        return api::Status::make(api::Error::NotFound, std::string("unknown action: ") + action);
    }
    std::map<std::string, api::KeyBinding> candidate = impl_->bindings;
    candidate[action] = api::KeyBinding::Unassigned();
    api::Status cs = impl_->checkConflicts(candidate);
    if (!cs.ok()) return cs;

    if (!impl_->port) {
        return api::Status::make(api::Error::Unavailable, "no shortcut port");
    }
    api::Status ps = impl_->port->prepare(candidate);
    if (!ps.ok()) {
        impl_->port->rollback();
        return ps;
    }
    api::Status cm = impl_->port->commit();
    if (!cm.ok()) {
        impl_->port->rollback();
        return cm;
    }
    impl_->bindings[action] = api::KeyBinding::Unassigned();
    impl_->hasStaged = false;
    impl_->staged.clear();
    impl_->revision += 1;
    impl_->notify();
    return api::Status::Ok();
}

api::Status ShortcutService::resetBinding(const std::string& action, uint64_t expectedRevision) {
    if (expectedRevision != impl_->revision) {
        return api::Status::make(api::Error::StaleRevision, "stale revision");
    }
    if (!isKnownAction(action)) {
        return api::Status::make(api::Error::NotFound, std::string("unknown action: ") + action);
    }
    std::map<std::string, api::KeyBinding>::const_iterator it = impl_->defaults.find(action);
    api::KeyBinding def = (it != impl_->defaults.end()) ? it->second : api::KeyBinding::Unassigned();
    std::map<std::string, api::KeyBinding> candidate = impl_->bindings;
    candidate[action] = def;
    api::Status cs = impl_->checkConflicts(candidate);
    if (!cs.ok()) return cs;

    if (!impl_->port) {
        return api::Status::make(api::Error::Unavailable, "no shortcut port");
    }
    api::Status ps = impl_->port->prepare(candidate);
    if (!ps.ok()) {
        impl_->port->rollback();
        return ps;
    }
    api::Status cm = impl_->port->commit();
    if (!cm.ok()) {
        impl_->port->rollback();
        return cm;
    }
    impl_->bindings[action] = def;
    impl_->hasStaged = false;
    impl_->staged.clear();
    impl_->revision += 1;
    impl_->notify();
    return api::Status::Ok();
}

api::Status ShortcutService::prepare(const std::map<std::string, api::KeyBinding>& desired) {
    for (std::map<std::string, api::KeyBinding>::const_iterator it = desired.begin(); it != desired.end(); ++it) {
        if (!isKnownAction(it->first)) {
            return api::Status::make(api::Error::InvalidArgument, std::string("unknown action: ") + it->first);
        }
    }
    std::map<std::string, api::KeyBinding> candidate = impl_->bindings;
    for (std::map<std::string, api::KeyBinding>::const_iterator it = desired.begin(); it != desired.end(); ++it) {
        api::KeyBinding norm = impl_->normalizeBinding(it->second);
        if (norm.assigned()) {
            std::string n = normalizeKey(norm.key);
            std::string verr;
            if (!validateNormalized(n, &verr)) {
                return api::Status::make(api::Error::InvalidArgument, verr + " for " + it->first);
            }
            if (isModifierOnly(n) && !actionAllowsModifierOnly(it->first)) {
                return api::Status::make(api::Error::InvalidArgument, std::string("modifier-only not allowed for ") + it->first);
            }
        }
        candidate[it->first] = norm;
    }
    api::Status cs = impl_->checkConflicts(candidate);
    if (!cs.ok()) return cs;

    if (!impl_->port) {
        return api::Status::make(api::Error::Unavailable, "no shortcut port");
    }
    api::Status ps = impl_->port->prepare(candidate);
    if (!ps.ok()) {
        impl_->port->rollback();
        return ps;
    }
    impl_->staged = candidate;
    impl_->hasStaged = true;
    return api::Status::Ok();
}

api::Status ShortcutService::commit() {
    if (!impl_->hasStaged) {
        return api::Status::Ok();
    }
    api::Status cs = impl_->checkConflicts(impl_->staged);
    if (!cs.ok()) {
        if (impl_->port) impl_->port->rollback();
        impl_->staged.clear();
        impl_->hasStaged = false;
        return cs;
    }
    if (!impl_->port) {
        impl_->staged.clear();
        impl_->hasStaged = false;
        return api::Status::make(api::Error::Unavailable, "no shortcut port");
    }
    api::Status cm = impl_->port->commit();
    if (!cm.ok()) {
        impl_->port->rollback();
        impl_->staged.clear();
        impl_->hasStaged = false;
        return cm;
    }
    impl_->bindings = impl_->staged;
    impl_->staged.clear();
    impl_->hasStaged = false;
    impl_->revision += 1;
    impl_->notify();
    return api::Status::Ok();
}

void ShortcutService::rollback() {
    impl_->staged.clear();
    impl_->hasStaged = false;
    if (impl_->port) impl_->port->rollback();
}

int ShortcutService::addListener(std::function<void(uint64_t)> cb) {
    int id = impl_->nextListenerId++;
    impl_->listeners[id] = cb;
    return id;
}

void ShortcutService::removeListener(int id) {
    std::map<int, std::function<void(uint64_t)> >::iterator it = impl_->listeners.find(id);
    if (it != impl_->listeners.end()) impl_->listeners.erase(it);
}

} // namespace shortcuts
} // namespace platform
} // namespace flamewm
