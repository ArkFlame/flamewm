#pragma once

#include "flamewm/api/ports.h"

#include <map>
#include <string>

namespace flamewm {
namespace engine {
namespace icewm {

class ShortcutAdapter : public api::ShortcutPort {
public:
    ShortcutAdapter();
    ~ShortcutAdapter() override;

    api::Status prepare(const std::map<std::string, api::KeyBinding>& desired) override;
    api::Status commit() override;
    void rollback() override;

    // Root-key hook: called from YWindowManager::handleKey / Hooks::rootKey.
    // Returns true if event matched a committed Flame binding and was dispatched.
    // Primitive args keep header X11-free; dispatch does not use D-Bus.
    bool handleKey(unsigned keycode, unsigned state) const;
    bool handleButton(unsigned button, unsigned state) const;

private:
    struct Impl;
    Impl* impl_;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm
