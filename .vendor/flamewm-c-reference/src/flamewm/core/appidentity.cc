#include "appidentity.h"
#include <cctype>

namespace flamewm {

std::string ApplicationIdentity::normalize(const std::string& raw) {
    std::string out;
    out.reserve(raw.size());
    for (size_t i = 0; i < raw.size(); ++i) {
        unsigned char c = (unsigned char)raw[i];
        if (c == ' ' || c == '\t' || c == '\n' || c == '\r') continue;
        out.push_back((char)tolower(c));
    }
    return out;
}

bool ApplicationIdentity::isDesktopId(const std::string& id) {
    if (id.empty()) return false;
    if (id.size() < 9) return false; // at least "a.desktop"
    if (id.compare(id.size()-8, 8, ".desktop") != 0) return false;
    return true;
}

std::string ApplicationIdentity::resolve(const AppIdentityInput& in) {
    if (!in.desktopId.empty()) {
        return normalize(in.desktopId);
    }
    if (!in.startupWMClass.empty()) {
        return normalize(in.startupWMClass);
    }
    if (!in.wmClass.empty()) {
        return normalize(in.wmClass);
    }
    if (!in.wmInstance.empty()) {
        return normalize(in.wmInstance);
    }
    return "unknown";
}

std::vector<std::string> ApplicationIdentity::mergePinnedRunning(
    const std::vector<std::string>& pinned,
    const std::vector<std::string>& running) {
    std::vector<std::string> out;
    out.reserve(pinned.size() + running.size());
    for (size_t i = 0; i < pinned.size(); ++i) {
        std::string n = normalize(pinned[i]);
        bool dup = false;
        for (size_t j = 0; j < out.size(); ++j) if (out[j]==n) { dup=true; break; }
        if (!dup) out.push_back(n);
    }
    for (size_t i = 0; i < running.size(); ++i) {
        std::string n = normalize(running[i]);
        bool dup = false;
        for (size_t j = 0; j < out.size(); ++j) if (out[j]==n) { dup=true; break; }
        if (!dup) out.push_back(n);
    }
    return out;
}

std::vector<ContextAction> ApplicationIdentity::contextActions(bool isPinned, bool hasWindow,
                                                               bool isVisible, bool isMaximized) {
    std::vector<ContextAction> v;
    if (isPinned) {
        if (!hasWindow) {
            v.push_back(ContextOpen);
            v.push_back(ContextUnpin);
            return v;
        }
        v.push_back(ContextUnpin);
    } else {
        if (!hasWindow) return v;
        v.push_back(ContextPin);
    }
    v.push_back(ContextSeparator);
    v.push_back(isVisible && isMaximized ? ContextMinimize : ContextMaximize);
    v.push_back(ContextClose);
    return v;
}

std::vector<ContextAction> ApplicationIdentity::contextActions(bool isPinned, bool isRunning) {
    // Without frame state, use V7's non-visible default rather than inventing Activate.
    if (!isRunning) return isPinned ? std::vector<ContextAction>{ContextOpen, ContextUnpin}
                                    : std::vector<ContextAction>();
    return contextActions(isPinned, true, false, false);
}

const char* ApplicationIdentity::contextActionName(ContextAction a) {
    switch (a) {
        case ContextOpen: return "Open";
        case ContextUnpin: return "Unpin";
        case ContextPin: return "Pin";
        case ContextMaximize: return "Maximize";
        case ContextMinimize: return "Minimize";
        case ContextClose: return "Close";
        case ContextSeparator: return "Separator";
        default: return "Unknown";
    }
}

} // namespace flamewm
