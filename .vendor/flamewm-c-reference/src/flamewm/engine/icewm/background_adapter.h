#pragma once

#ifdef Status
#undef Status
#endif
#include "flamewm/api/ports.h"
#include "flamewm/api/background.h"

#include <string>

namespace flamewm {
namespace engine {
namespace icewm {

class BackgroundAdapter : public api::BackgroundPort {
public:
    explicit BackgroundAdapter(const std::string& enginePrefsPath = std::string());
    ~BackgroundAdapter();

    api::Status project(const api::BackgroundState& state) override;
    api::Status reload() override;

    void setEnginePrefsPath(const std::string& path);
    std::string enginePrefsPath() const;

private:
    static api::Status reloadIcewmbg();
    std::string effectivePreferencesPath() const;
    static std::string defaultEnginePrefsPath();
    static api::Status writePreferencesEntry(const std::string& file,
                                             const std::string& key,
                                             const std::string& value);

    std::string enginePrefsPath_;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm
