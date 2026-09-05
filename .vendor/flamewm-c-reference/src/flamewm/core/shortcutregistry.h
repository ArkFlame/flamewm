#ifndef FLAMEWM_CORE_SHORTCUTREGISTRY_H
#define FLAMEWM_CORE_SHORTCUTREGISTRY_H

#include <string>
#include <map>
#include <vector>

namespace flamewm {

enum ShortcutAction {
    ActionToggleStartMenu = 0,
    ActionWorkspaceLeft,
    ActionWorkspaceRight,
    ActionWorkspaceUp,
    ActionWorkspaceDown,
    ActionWindowClose,
    ActionWindowMinimize,
    ActionWindowMaximize,
    ActionCount
};

struct KeyBinding {
    bool assigned; // false => Not assigned
    std::string keysym; // e.g. "Ctrl+Super+Left"
    KeyBinding() : assigned(false) {}
    explicit KeyBinding(const std::string& k) : assigned(!k.empty()), keysym(k) {}
    std::string display() const { return assigned ? keysym : "Not assigned"; }
    bool empty() const { return !assigned || keysym.empty(); }
};

class ShortcutRegistry {
public:
    ShortcutRegistry();
    std::string actionName(ShortcutAction a) const;
    KeyBinding bindingFor(ShortcutAction a) const;
    bool setBinding(ShortcutAction a, const std::string& keysym, std::string* error);
    void clearBinding(ShortcutAction a); // -> Not assigned
    void resetToDefaults();
    bool hasConflict(const std::string& keysym, ShortcutAction* outConflict) const;
    static std::string normalize(const std::string& raw);
    static bool validate(const std::string& normalized);
    // Modifier-only check: true if binding is only modifiers (e.g. "Ctrl", "Shift+Ctrl")
    static bool isModifierOnly(const std::string& normalized);
    // Action allows modifier-only chord (ToggleStartMenu allows Super)
    static bool actionAllowsModifierOnly(ShortcutAction a);
    // Per-row reset: reset exactly one action to shipped default; others untouched.
    void resetOneRow(ShortcutAction a);
    // Effective grab stage: attempt to grab all bindings; on any failure release staged and keep old set.
    // grabFn returns true if keysym can be grabbed. On failure, error set, old map retained, persist not called.
    bool applyWithGrabStage(const std::map<ShortcutAction, std::string>& desired,
                            bool (*grabFn)(const std::string& keysym, void* ctx), void* ctx,
                            std::string* error);
    // Escape clears capture: helper — Escape must never be stored as binding.
    static bool isEscape(const std::string& raw) { return raw=="Escape"; }
    static bool isEscapeNormalized(const std::string& normalized) { return normalized=="Escape"; }
    // Display helper: nullable -> "Not assigned" via KeyBinding::display()
    // Validate with modifier-only policy for given action.
    bool setBindingValidated(ShortcutAction a, const std::string& keysym, std::string* error);
private:
    std::map<ShortcutAction, KeyBinding> map_;
    std::map<ShortcutAction, KeyBinding> shippedDefaults_;
    void initDefaults();
};

} // namespace flamewm
#endif
