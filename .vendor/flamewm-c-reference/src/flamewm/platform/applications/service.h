#ifndef FLAMEWM_PLATFORM_APPLICATIONS_SERVICE_H
#define FLAMEWM_PLATFORM_APPLICATIONS_SERVICE_H

#include "flamewm/api/applications.h"
#include "flamewm/api/errors.h"
#include "flamewm/api/ids.h"
#include "flamewm/api/ports.h"

#include <string>
#include <vector>

namespace flamewm {
namespace platform {
namespace applications {

class ApplicationService {
public:
    explicit ApplicationService(api::ApplicationPort* port);
    ~ApplicationService();

    ApplicationService(const ApplicationService&) = delete;
    ApplicationService& operator=(const ApplicationService&) = delete;

    void rescan();
    api::Result<api::DesktopApplication> findById(const api::DesktopAppId& id) const;
    api::Result<api::DesktopApplication> findByWindowClass(const std::string& wmClass) const;
    std::vector<api::DesktopApplication> search(const std::string& query) const;
    std::vector<api::DesktopApplication> all() const;
    void invalidate();

    api::Status launch(const api::DesktopAppId& id, const std::vector<std::string>& args);
    api::Status launch(const api::DesktopAppId& id);
    api::Status launchUri(const std::string& uri);

private:
    struct Impl;
    Impl* impl_;
};

} // namespace applications
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_APPLICATIONS_SERVICE_H
