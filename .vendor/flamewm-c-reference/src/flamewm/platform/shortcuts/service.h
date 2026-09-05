#ifndef FLAMEWM_PLATFORM_SHORTCUTS_SERVICE_H
#define FLAMEWM_PLATFORM_SHORTCUTS_SERVICE_H

#include "flamewm/api/shortcuts.h"
#include "flamewm/api/errors.h"
#include "flamewm/api/ports.h"

#include <cstdint>
#include <functional>
#include <map>
#include <string>

namespace flamewm {
namespace platform {
namespace shortcuts {

class ShortcutService {
public:
    explicit ShortcutService(api::ShortcutPort* port);
    ~ShortcutService();

    api::ShortcutSnapshot snapshot() const;
    uint64_t revision() const;

    api::Status setBinding(const std::string& action, const api::KeyBinding& binding, uint64_t expectedRevision);
    api::Status clearBinding(const std::string& action, uint64_t expectedRevision);
    api::Status resetBinding(const std::string& action, uint64_t expectedRevision);

    // Transaction helpers: validate entire desired map, port.prepare, port.commit,
    // publish new revision; failure -> port.rollback + old revision.
    api::Status prepare(const std::map<std::string, api::KeyBinding>& desired);
    api::Status commit();
    void rollback();

    int addListener(std::function<void(uint64_t)> cb);
    void removeListener(int id);

private:
    struct Impl;
    Impl* impl_;

    // non-copyable
    ShortcutService(const ShortcutService&);
    ShortcutService& operator=(const ShortcutService&);
};

} // namespace shortcuts
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_SHORTCUTS_SERVICE_H
