#ifndef FLAMEWM_PLATFORM_HOST_H
#define FLAMEWM_PLATFORM_HOST_H

#ifdef None
#pragma push_macro("None")
#undef None
#define FLAMEWM_HOST_RN
#endif
#ifdef Status
#pragma push_macro("Status")
#undef Status
#define FLAMEWM_HOST_RS
#endif

#include <cstdint>
#include <string>

#include "flamewm/api/ports.h"
#include "flamewm/platform/windows/service.h"
#include "flamewm/platform/workspaces/service.h"
#include "flamewm/platform/settings/service.h"
#include "flamewm/platform/shortcuts/service.h"
#include "flamewm/platform/displays/service.h"
#include "flamewm/platform/panels/service.h"
#include "flamewm/platform/applications/service.h"
#include "flamewm/platform/scale/service.h"
#include "flamewm/platform/session/service.h"
#include "flamewm/platform/background/service.h"
#include "flamewm/platform/reactor/service.h"
#include "flamewm/platform/system/service.h"
#include "flamewm/platform/snap/service.h"
#include "flamewm/platform/chrome/policy.h"

namespace flamewm {
namespace platform {

class PlatformHost {
public:
    struct Config {
        std::string settingsPath;
        std::string enginePreferencesPath;
    };

    explicit PlatformHost(api::EnginePorts ports);
    PlatformHost(api::EnginePorts ports, const Config& config);
    ~PlatformHost();

    PlatformHost(const PlatformHost&) = delete;
    PlatformHost& operator=(const PlatformHost&) = delete;

    bool start();
    void stop();

    bool isStarted() const { return started_; }
    uint64_t generation() const { return generation_; }

    windows::WindowService* windows();
    workspaces::WorkspaceService* workspaces();
    settings::SettingsService* settings();
    shortcuts::ShortcutService* shortcuts();
    displays::DisplayService* displays();
    panels::PanelService* panels();
    applications::ApplicationService* applications();
    scale::ScaleService* scale();
    session::SessionService* session();
    background::BackgroundService* background();
    reactor::ReactorService* reactor();
    system::SystemService* system();
    snap::SnapService* snap();
    chrome::ChromePolicy* chrome();

    const windows::WindowService* windows() const;
    const workspaces::WorkspaceService* workspaces() const;
    const settings::SettingsService* settings() const;
    const shortcuts::ShortcutService* shortcuts() const;
    const displays::DisplayService* displays() const;
    const panels::PanelService* panels() const;
    const applications::ApplicationService* applications() const;
    const scale::ScaleService* scale() const;
    const session::SessionService* session() const;
    const background::BackgroundService* background() const;
    const reactor::ReactorService* reactor() const;
    const system::SystemService* system() const;
    const snap::SnapService* snap() const;
    const chrome::ChromePolicy* chrome() const;

private:
    bool validatePorts() const;

    api::EnginePorts ports_;
    Config config_;
    uint64_t generation_;
    bool started_;

    struct Impl;
    Impl* impl_;
};

} // namespace platform
} // namespace flamewm

#ifdef FLAMEWM_HOST_RS
#pragma pop_macro("Status")
#undef FLAMEWM_HOST_RS
#endif
#ifdef FLAMEWM_HOST_RN
#pragma pop_macro("None")
#undef FLAMEWM_HOST_RN
#endif

#endif // FLAMEWM_PLATFORM_HOST_H
