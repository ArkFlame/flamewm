#ifndef FLAMEWM_PLATFORM_SETTINGS_SERVICE_H
#define FLAMEWM_PLATFORM_SETTINGS_SERVICE_H

#include "flamewm/api/settings.h"
#include "flamewm/api/errors.h"

#include <cstdint>
#include <functional>
#include <string>
#include <vector>

namespace flamewm {
namespace platform {
namespace settings {

class SettingsEffect;

class SettingsService {
public:
    explicit SettingsService(const std::string& configPath);
    ~SettingsService();

    SettingsService(const SettingsService&) = delete;
    SettingsService& operator=(const SettingsService&) = delete;

    api::SettingsSnapshot snapshot() const;
    uint64_t revision() const;
    api::Status apply(uint64_t expectedRevision,
                      const std::vector<api::SettingsChange>& changes);
    api::Status resetSection(uint64_t expectedRevision,
                             const std::string& section);

    // Reversible participant registration. Only services that actually consume
    // settings are registered (no generic plugin discovery). Non-owning.
    void addEffect(SettingsEffect* effect);
    void removeEffect(SettingsEffect* effect);

    using Listener = std::function<void(uint64_t newRev,
                                        const std::vector<std::string>& changedKeys)>;
    int addListener(Listener cb);
    void removeListener(int id);

private:
    struct Impl;
    Impl* impl_;
};

} // namespace settings
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_SETTINGS_SERVICE_H
