#ifndef FLAMEWM_API_SHORTCUTS_H
#define FLAMEWM_API_SHORTCUTS_H

#ifdef None
#pragma push_macro("None")
#undef None
#define FLAMEWM_SHORT_RN
#endif
#ifdef Status
#pragma push_macro("Status")
#undef Status
#define FLAMEWM_SHORT_RS
#endif

#include "errors.h"
#include "ids.h"

#include <cstdint>
#include <map>
#include <string>

namespace flamewm {
namespace api {

enum class ModMask {
    None    = 0,
    Shift   = 1,
    Control = 2,
    Alt     = 4,
    Super   = 8
};

inline ModMask operator|(ModMask a, ModMask b) {
    return static_cast<ModMask>(static_cast<int>(a) | static_cast<int>(b));
}
inline ModMask operator&(ModMask a, ModMask b) {
    return static_cast<ModMask>(static_cast<int>(a) & static_cast<int>(b));
}
inline ModMask& operator|=(ModMask& a, ModMask b) {
    a = a | b;
    return a;
}
inline ModMask& operator&=(ModMask& a, ModMask b) {
    a = a & b;
    return a;
}
inline bool modHas(ModMask m, ModMask flag) {
    return (static_cast<int>(m) & static_cast<int>(flag)) != 0;
}

struct KeyBinding {
    std::string key; // normalized e.g. "Super+Space", empty means unassigned

    KeyBinding() : key() {}
    explicit KeyBinding(const std::string& k) : key(k) {}
    explicit KeyBinding(std::string&& k) : key(std::move(k)) {}

    bool assigned() const { return !key.empty(); }

    static KeyBinding Unassigned() { return KeyBinding(std::string()); }
};

struct ShortcutSnapshot {
    uint64_t revision;
    std::map<std::string, KeyBinding> bindings; // ActionId -> binding

    ShortcutSnapshot() : revision(0), bindings() {}
    explicit ShortcutSnapshot(uint64_t rev) : revision(rev), bindings() {}
    ShortcutSnapshot(uint64_t rev, const std::map<std::string, KeyBinding>& b)
        : revision(rev), bindings(b) {}
    ShortcutSnapshot(uint64_t rev, std::map<std::string, KeyBinding>&& b)
        : revision(rev), bindings(std::move(b)) {}

    KeyBinding get(const std::string& action) const {
        std::map<std::string, KeyBinding>::const_iterator it = bindings.find(action);
        if (it == bindings.end()) return KeyBinding::Unassigned();
        return it->second;
    }

    bool has(const std::string& action) const {
        return bindings.find(action) != bindings.end();
    }
};

// Known action IDs (match core/ShortcutAction string names)
extern const std::string ActionToggleStartMenu;
extern const std::string ActionWorkspaceLeft;
extern const std::string ActionWorkspaceRight;
extern const std::string ActionWorkspaceUp;
extern const std::string ActionWorkspaceDown;
extern const std::string ActionWindowClose;
extern const std::string ActionWindowMinimize;
extern const std::string ActionWindowMaximize;

} // namespace api
} // namespace flamewm

#ifdef FLAMEWM_SHORT_RS
#pragma pop_macro("Status")
#undef FLAMEWM_SHORT_RS
#endif
#ifdef FLAMEWM_SHORT_RN
#pragma pop_macro("None")
#undef FLAMEWM_SHORT_RN
#endif

#endif // FLAMEWM_API_SHORTCUTS_H
