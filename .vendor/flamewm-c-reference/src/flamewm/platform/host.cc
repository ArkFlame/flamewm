#include "flamewm/platform/host.h"

#include <cstddef>
#include <string>

namespace flamewm {
namespace platform {

struct PlatformHost::Impl {
    settings::SettingsService* settings;
    reactor::ReactorService* reactor;
    windows::WindowService* windows;
    workspaces::WorkspaceService* workspaces;
    displays::DisplayService* displays;
    shortcuts::ShortcutService* shortcuts;
    panels::PanelService* panels;
    applications::ApplicationService* applications;
    scale::ScaleService* scale;
    session::SessionService* session;
    background::BackgroundService* background;
    system::SystemService* system;
    snap::SnapService* snap;
    chrome::ChromePolicy* chrome;

    Impl()
        : settings(nullptr)
        , reactor(nullptr)
        , windows(nullptr)
        , workspaces(nullptr)
        , displays(nullptr)
        , shortcuts(nullptr)
        , panels(nullptr)
        , applications(nullptr)
        , scale(nullptr)
        , session(nullptr)
        , background(nullptr)
        , system(nullptr)
        , snap(nullptr)
        , chrome(nullptr)
    {}
};

PlatformHost::PlatformHost(api::EnginePorts ports)
    : ports_(ports)
    , config_()
    , generation_(0)
    , started_(false)
    , impl_(nullptr)
{}

PlatformHost::PlatformHost(api::EnginePorts ports, const Config& config)
    : ports_(ports)
    , config_(config)
    , generation_(0)
    , started_(false)
    , impl_(nullptr)
{}

PlatformHost::~PlatformHost() {
    if (started_) {
        stop();
    } else if (impl_) {
        delete impl_;
        impl_ = nullptr;
    }
}

bool PlatformHost::validatePorts() const {
    return ports_.window      != nullptr
        && ports_.workspace   != nullptr
        && ports_.display     != nullptr
        && ports_.shortcut    != nullptr
        && ports_.workArea    != nullptr
        && ports_.mainLoop    != nullptr
        && ports_.input       != nullptr
        && ports_.application != nullptr
        && ports_.session     != nullptr
        && ports_.background  != nullptr
        && ports_.tray        != nullptr;
}

bool PlatformHost::start() {
    if (started_) {
        return false;
    }
    if (!validatePorts()) {
        return false;
    }

    Impl* impl = new Impl();

    impl->settings = new settings::SettingsService(config_.settingsPath);
    impl->reactor  = new reactor::ReactorService(ports_.mainLoop);
    impl->scale    = new scale::ScaleService();
    impl->chrome   = new chrome::ChromePolicy();
    impl->windows  = new windows::WindowService(ports_.window);
    impl->workspaces = new workspaces::WorkspaceService(ports_.workspace);
    impl->displays = new displays::DisplayService(ports_.display, ports_.mainLoop);
    impl->shortcuts = new shortcuts::ShortcutService(ports_.shortcut);
    impl->panels   = new panels::PanelService(ports_.workArea, ports_.tray);
    impl->applications = new applications::ApplicationService(ports_.application);
    impl->session  = new session::SessionService(ports_.session);
    if (!config_.settingsPath.empty()) {
        impl->background = new background::BackgroundService(
            ports_.background, config_.enginePreferencesPath, config_.settingsPath);
    } else {
        impl->background = new background::BackgroundService(
            ports_.background, config_.enginePreferencesPath);
    }
    impl->system   = new system::SystemService(impl->reactor);
    impl->snap     = new snap::SnapService(ports_.window, ports_.mainLoop);

    impl_ = impl;
    started_ = true;
    return true;
}

void PlatformHost::stop() {
    if (!started_) {
        if (impl_) {
            delete impl_;
            impl_ = nullptr;
        }
        return;
    }

    ++generation_;

    if (impl_) {
        if (impl_->reactor) impl_->reactor->stopAll();
        if (impl_->background) { /* no clearSubscriptions API */ }
        if (impl_->panels) impl_->panels->clearSubscriptions();
        if (impl_->scale) impl_->scale->clearSubscriptions();
        if (impl_->chrome) impl_->chrome->clearSubscriptions();
    }

    if (impl_) {
        delete impl_->snap;         impl_->snap = nullptr;
        delete impl_->system;       impl_->system = nullptr;
        delete impl_->background;   impl_->background = nullptr;
        delete impl_->session;      impl_->session = nullptr;
        delete impl_->applications; impl_->applications = nullptr;
        delete impl_->panels;       impl_->panels = nullptr;
        delete impl_->shortcuts;    impl_->shortcuts = nullptr;
        delete impl_->displays;     impl_->displays = nullptr;
        delete impl_->workspaces;   impl_->workspaces = nullptr;
        delete impl_->windows;      impl_->windows = nullptr;
        delete impl_->chrome;       impl_->chrome = nullptr;
        delete impl_->scale;        impl_->scale = nullptr;
        delete impl_->reactor;      impl_->reactor = nullptr;
        delete impl_->settings;     impl_->settings = nullptr;

        delete impl_;
        impl_ = nullptr;
    }

    started_ = false;
}

windows::WindowService* PlatformHost::windows() { return impl_ ? impl_->windows : nullptr; }
workspaces::WorkspaceService* PlatformHost::workspaces() { return impl_ ? impl_->workspaces : nullptr; }
settings::SettingsService* PlatformHost::settings() { return impl_ ? impl_->settings : nullptr; }
shortcuts::ShortcutService* PlatformHost::shortcuts() { return impl_ ? impl_->shortcuts : nullptr; }
displays::DisplayService* PlatformHost::displays() { return impl_ ? impl_->displays : nullptr; }
panels::PanelService* PlatformHost::panels() { return impl_ ? impl_->panels : nullptr; }
applications::ApplicationService* PlatformHost::applications() { return impl_ ? impl_->applications : nullptr; }
scale::ScaleService* PlatformHost::scale() { return impl_ ? impl_->scale : nullptr; }
session::SessionService* PlatformHost::session() { return impl_ ? impl_->session : nullptr; }
background::BackgroundService* PlatformHost::background() { return impl_ ? impl_->background : nullptr; }
reactor::ReactorService* PlatformHost::reactor() { return impl_ ? impl_->reactor : nullptr; }
system::SystemService* PlatformHost::system() { return impl_ ? impl_->system : nullptr; }
snap::SnapService* PlatformHost::snap() { return impl_ ? impl_->snap : nullptr; }
chrome::ChromePolicy* PlatformHost::chrome() { return impl_ ? impl_->chrome : nullptr; }

const windows::WindowService* PlatformHost::windows() const { return impl_ ? impl_->windows : nullptr; }
const workspaces::WorkspaceService* PlatformHost::workspaces() const { return impl_ ? impl_->workspaces : nullptr; }
const settings::SettingsService* PlatformHost::settings() const { return impl_ ? impl_->settings : nullptr; }
const shortcuts::ShortcutService* PlatformHost::shortcuts() const { return impl_ ? impl_->shortcuts : nullptr; }
const displays::DisplayService* PlatformHost::displays() const { return impl_ ? impl_->displays : nullptr; }
const panels::PanelService* PlatformHost::panels() const { return impl_ ? impl_->panels : nullptr; }
const applications::ApplicationService* PlatformHost::applications() const { return impl_ ? impl_->applications : nullptr; }
const scale::ScaleService* PlatformHost::scale() const { return impl_ ? impl_->scale : nullptr; }
const session::SessionService* PlatformHost::session() const { return impl_ ? impl_->session : nullptr; }
const background::BackgroundService* PlatformHost::background() const { return impl_ ? impl_->background : nullptr; }
const reactor::ReactorService* PlatformHost::reactor() const { return impl_ ? impl_->reactor : nullptr; }
const system::SystemService* PlatformHost::system() const { return impl_ ? impl_->system : nullptr; }
const snap::SnapService* PlatformHost::snap() const { return impl_ ? impl_->snap : nullptr; }
const chrome::ChromePolicy* PlatformHost::chrome() const { return impl_ ? impl_->chrome : nullptr; }

} // namespace platform
} // namespace flamewm
