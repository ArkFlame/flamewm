#pragma once

#ifdef Status
#pragma push_macro("Status")
#undef Status
#define FLAMEWM_APPLICATION_ADAPTER_RS
#endif

#include "flamewm/api/ports.h"

#include <string>
#include <vector>

namespace flamewm {
namespace engine {
namespace icewm {

class ApplicationAdapter : public api::ApplicationPort {
public:
    ApplicationAdapter();
    ~ApplicationAdapter();

    api::Status launch(const api::DesktopAppId& id,
                       const std::vector<std::string>& args) override;
    api::Status launchUri(const std::string& uri) override;

private:
    static bool isSafeUri(const std::string& uri);
    static api::Status launchExecVector(const std::vector<std::string>& argv);
};

} // namespace icewm
} // namespace engine
} // namespace flamewm

#ifdef FLAMEWM_APPLICATION_ADAPTER_RS
#pragma pop_macro("Status")
#undef FLAMEWM_APPLICATION_ADAPTER_RS
#endif
