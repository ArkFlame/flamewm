#include "flamewm/shell/shell.h"

#include "flamewm/platform/host.h"
#include "flamewm/platform/panels/service.h"
#include "flamewm/platform/displays/service.h"
#include "flamewm/platform/workspaces/service.h"
#include "flamewm/platform/windows/service.h"
#include "flamewm/platform/settings/service.h"
#include "flamewm/platform/system/service.h"
#include "flamewm/platform/applications/service.h"
#include "flamewm/platform/scale/service.h"
#include "flamewm/platform/session/service.h"
#include "flamewm/platform/background/service.h"
#include "flamewm/shell/panel_host.h"
#include "flamewm/shell/task_model.h"
#include "flamewm/shell/task_view.h"
#include "flamewm/shell/start_view.h"
#include "flamewm/shell/workspace_view.h"
#include "flamewm/shell/status_area.h"
#include "flamewm/shell/clock_view.h"
#include "flamewm/shell/tray_host.h"
#include "flamewm/shell/popovers.h"

#include "flamewm/api/ports.h"
#include "flamewm/api/panels.h"
#include "flamewm/api/display.h"
#include "flamewm/api/workspace.h"
#include "flamewm/api/window.h"
#include "flamewm/api/settings.h"

#include <functional>
#include <vector>

namespace flamewm {
namespace shell {

struct Shell::Impl {
    platform::PlatformHost* host;
    control::ControlServer* bus;

    PanelHost* panelHost;
    TaskModel* taskModel;
    TaskView* taskView;
    StartView* startView;
    WorkspaceView* workspaceView;
    StatusArea* statusArea;
    ClockView* clockView;
    TrayHost* trayHost;
    Popovers* popovers;

    bool started;
    uint64_t generation;

    // Subscription ids — 0 means not subscribed.
    int panelsListener;
    int displaysListener;
    int workspacesListener;
    int windowsListener;
    int settingsListener;
    int systemListener;
    int scaleListener;

    explicit Impl(platform::PlatformHost* h, control::ControlServer* b)
        : host(h)
        , bus(b)
        , panelHost(nullptr)
        , taskModel(nullptr)
        , taskView(nullptr)
        , startView(nullptr)
        , workspaceView(nullptr)
        , statusArea(nullptr)
        , clockView(nullptr)
        , trayHost(nullptr)
        , popovers(nullptr)
        , started(false)
        , generation(0)
        , panelsListener(0)
        , displaysListener(0)
        , workspacesListener(0)
        , windowsListener(0)
        , settingsListener(0)
        , systemListener(0)
        , scaleListener(0) {}
};

Shell::Shell(platform::PlatformHost* host) : impl_(new Impl(host, nullptr)) {
    // Build owned graph eagerly so accessors are valid immediately.
    // PanelHost needs WorkAreaPort/TrayPort from host's EnginePorts if
    // available, but we defer port binding to start() — here we create with
    // nulls so header stays X11-free and construction never touches X.
    impl_->popovers = new Popovers();
    impl_->taskModel = new TaskModel();
    impl_->taskView = new TaskView(impl_->taskModel);
    impl_->startView = new StartView();
    // PanelHost: start with no ports; real ports wired in syncInitial via
    // PlatformHost snapshot when started.
    impl_->panelHost = new PanelHost(nullptr, nullptr);
    impl_->workspaceView = new WorkspaceView(nullptr);
    impl_->statusArea = new StatusArea();
    impl_->clockView = new ClockView();
    impl_->trayHost = new TrayHost(nullptr);
}

Shell::Shell(platform::PlatformHost* host, control::ControlServer* bus)
    : impl_(new Impl(host, bus)) {
    impl_->popovers = new Popovers();
    impl_->taskModel = new TaskModel();
    impl_->taskView = new TaskView(impl_->taskModel);
    impl_->startView = new StartView();
    impl_->panelHost = new PanelHost(nullptr, nullptr);
    impl_->workspaceView = new WorkspaceView(nullptr);
    impl_->statusArea = new StatusArea();
    impl_->clockView = new ClockView();
    impl_->trayHost = new TrayHost(nullptr);
}

Shell::~Shell() {
    if (impl_->started)
        stop();
    delete impl_->popovers;   impl_->popovers = nullptr;
    delete impl_->trayHost;   impl_->trayHost = nullptr;
    delete impl_->clockView;  impl_->clockView = nullptr;
    delete impl_->statusArea; impl_->statusArea = nullptr;
    delete impl_->workspaceView; impl_->workspaceView = nullptr;
    delete impl_->startView;  impl_->startView = nullptr;
    delete impl_->taskView;   impl_->taskView = nullptr;
    delete impl_->taskModel;  impl_->taskModel = nullptr;
    delete impl_->panelHost;  impl_->panelHost = nullptr;
    delete impl_;
}

platform::PlatformHost* Shell::host() const { return impl_->host; }
control::ControlServer* Shell::controlServer() const { return impl_->bus; }
void Shell::setControlServer(control::ControlServer* bus) { impl_->bus = bus; }

PanelHost* Shell::panelHost() const { return impl_->panelHost; }
TaskModel* Shell::taskModel() const { return impl_->taskModel; }
TaskView* Shell::taskView() const { return impl_->taskView; }
StartView* Shell::startView() const { return impl_->startView; }
WorkspaceView* Shell::workspaceView() const { return impl_->workspaceView; }
StatusArea* Shell::statusArea() const { return impl_->statusArea; }
ClockView* Shell::clockView() const { return impl_->clockView; }
TrayHost* Shell::trayHost() const { return impl_->trayHost; }
Popovers* Shell::popovers() const { return impl_->popovers; }

bool Shell::isStarted() const { return impl_->started; }
uint64_t Shell::generation() const { return impl_->generation; }

bool Shell::toggleStart(int keyCode, unsigned state) {
    (void)keyCode;
    (void)state;
    if (!impl_->started || !impl_->panelHost || !impl_->host)
        return false;

    if (impl_->panelHost->isStartOpen())
        return impl_->panelHost->setStartOpen(false, impl_->panelHost->startOutput());

    api::OutputId target;
    if (impl_->host->windows()) {
        std::vector<api::WindowSnapshot> windows = impl_->host->windows()->snapshot();
        for (size_t i = 0; i < windows.size(); ++i) {
            if (windows[i].focused && windows[i].output.valid()) {
                target = windows[i].output;
                break;
            }
        }
    }
    if (!target.valid() && impl_->host->displays()) {
        api::DisplaySnapshot displays = impl_->host->displays()->snapshot();
        for (size_t i = 0; i < displays.outputs.size(); ++i) {
            if (displays.outputs[i].primary && displays.outputs[i].connected) {
                target = displays.outputs[i].id;
                break;
            }
        }
        if (!target.valid()) {
            for (size_t i = 0; i < displays.outputs.size(); ++i) {
                if (displays.outputs[i].connected) {
                    target = displays.outputs[i].id;
                    break;
                }
            }
        }
    }
    return impl_->panelHost->toggleStart(target);
}

bool Shell::start() {
    if (impl_->started)
        return false;
    if (!impl_->host || !impl_->host->isStarted())
        return false;

    // Wire ports where possible from PlatformHost services that wrap the
    // authoritative EnginePorts. Views receive service-backed ports so
    // mutations route through service validation.
    syncInitial();
    subscribe();

    impl_->started = true;
    bumpGeneration();
    return true;
}

void Shell::stop() {
    if (!impl_->started) {
        // Still need to clear any stray subscriptions if start() never completed.
        unsubscribe();
        return;
    }

    unsubscribe();

    // Hide all panels/popovers without destroying owned graph (keeps
    // accessors valid until dtor).
    if (impl_->popovers)
        impl_->popovers->hideAll();
    if (impl_->panelHost) {
        impl_->panelHost->closeStart();
        std::vector<PanelSurface*> panels = impl_->panelHost->panels();
        for (size_t i = 0; i < panels.size(); ++i)
            if (panels[i]) panels[i]->hide();
        impl_->panelHost->stopViewTimers();
        // Keep surfaces for reuse; just hide via clear.
        // We intentionally do NOT delete panelHost here.
    }
    if (impl_->startView)
        impl_->startView->close();
    if (impl_->statusArea)
        impl_->statusArea->closeAll();

    impl_->started = false;
    bumpGeneration();
}

void Shell::bumpGeneration() {
    ++impl_->generation;
}

void Shell::syncInitial() {
    if (!impl_->host)
        return;

    // Displays -> PanelHost + PanelService display snapshot
    if (impl_->host->displays()) {
        try {
            api::DisplaySnapshot d = impl_->host->displays()->snapshot();
            if (impl_->host->panels())
                impl_->host->panels()->onDisplaySnapshot(d);
            if (impl_->panelHost)
                impl_->panelHost->sync(d);
        } catch (...) {}
    }

    // Panels snapshot -> TaskModel
    if (impl_->host->panels() && impl_->taskModel) {
        api::PanelsSnapshot ps = impl_->host->panels()->snapshot();
        if (impl_->panelHost)
            impl_->panelHost->applyPanelSnapshot(ps);
        impl_->taskModel->setPanelService(impl_->host->panels());
        impl_->taskModel->setSnapshot(ps);
        if (impl_->trayHost)
            impl_->trayHost->adopt(ps.trayOwner);
        if (impl_->taskView)
            impl_->taskView->invalidate();
    }

    // Workspaces snapshot -> WorkspaceView
    bool hasWs = false;
    if (impl_->host->workspaces() && impl_->workspaceView) {
        try {
            api::WorkspaceSnapshot ws = impl_->host->workspaces()->snapshot();
            impl_->workspaceView->setWorkspaceService(impl_->host->workspaces());
            impl_->workspaceView->setSnapshot(ws);
            hasWs = true;
        } catch (...) {}
    }
    if (!hasWs && impl_->workspaceView) {
        // Ensure view knows single-workspace pager should hide.
        api::WorkspaceSnapshot empty;
        impl_->workspaceView->setSnapshot(empty);
    }

    // Windows -> PanelService + TaskModel reconciliation
    if (impl_->host->windows() && impl_->host->panels()) {
        try {
            std::vector<api::WindowSnapshot> wins = impl_->host->windows()->snapshot();
            impl_->host->panels()->onWindowSnapshots(wins);
        } catch (...) {}
    }

    // Settings -> PanelService
    if (impl_->host->settings() && impl_->host->panels()) {
        try {
            api::SettingsSnapshot ss = impl_->host->settings()->snapshot();
            impl_->host->panels()->onSettingsSnapshot(ss);
        } catch (...) {}
    }

    // System -> StatusArea
    if (impl_->host->system() && impl_->statusArea) {
        impl_->statusArea->setSystemService(impl_->host->system());
        try {
            impl_->statusArea->refreshFromService();
        } catch (...) {}
    }

    // Applications -> StartView
    if (impl_->host->applications() && impl_->startView) {
        impl_->startView->setApplicationService(impl_->host->applications());
        impl_->startView->refresh();
    }

    if (impl_->taskView) {
        impl_->taskView->setWindowService(impl_->host->windows());
        impl_->taskView->setApplicationService(impl_->host->applications());
    }
    if (impl_->panelHost)
        impl_->panelHost->bindViews(impl_->startView, impl_->taskView,
                                    impl_->workspaceView, impl_->statusArea,
                                    impl_->trayHost, impl_->clockView,
                                    impl_->host->reactor() ? impl_->host->reactor()->port() : 0,
                                    impl_->host->applications(), impl_->host->windows(),
                                    impl_->host->workspaces(), impl_->host->system());

    // Scale: no direct view wiring needed beyond service revision.

    // Rebuild views
    if (impl_->taskView)
        impl_->taskView->invalidate();
    if (impl_->workspaceView)
        impl_->workspaceView->invalidate();
    if (impl_->statusArea)
        impl_->statusArea->invalidate();
    if (impl_->clockView)
        impl_->clockView->invalidate();
    if (impl_->trayHost)
        impl_->trayHost->invalidate();
}

void Shell::subscribe() {
    if (!impl_->host)
        return;

    // Panels revision
    if (impl_->host->panels()) {
        impl_->panelsListener = impl_->host->panels()->addListener(
            [this](uint64_t rev) { this->onPanelsRevision(rev); });
    }
    // Displays generation
    if (impl_->host->displays()) {
        impl_->displaysListener = impl_->host->displays()->addListener(
            [this](uint64_t gen) { this->onDisplaysGeneration(gen); });
    }
    // Workspaces revision
    if (impl_->host->workspaces()) {
        impl_->workspacesListener = impl_->host->workspaces()->addListener(
            [this](uint64_t rev) { this->onWorkspacesRevision(rev); });
    }
    // Windows revision
    if (impl_->host->windows()) {
        impl_->windowsListener = impl_->host->windows()->addListener(
            [this](uint64_t rev) { this->onWindowsRevision(rev); });
    }
    // Settings — use SettingsService Listener (revision + changedKeys)
    if (impl_->host->settings()) {
        impl_->settingsListener = impl_->host->settings()->addListener(
            [this](uint64_t rev, const std::vector<std::string>&) { this->onSettingsRevision(rev); });
    }
    // System
    if (impl_->host->system()) {
        impl_->systemListener = impl_->host->system()->addListener(
            [this](platform::system::SystemSnapshot snap) { this->onSystemSnapshot(snap); });
    }
    // Scale
    if (impl_->host->scale()) {
        impl_->scaleListener = impl_->host->scale()->addListener(
            [this](uint64_t) {
                // Scale change -> re-render task/workspace/status as needed.
                if (impl_->taskView) impl_->taskView->invalidate();
                if (impl_->workspaceView) impl_->workspaceView->invalidate();
            });
    }
}

void Shell::unsubscribe() {
    if (!impl_->host)
        return;
    if (impl_->panelsListener && impl_->host->panels()) {
        impl_->host->panels()->removeListener(impl_->panelsListener);
        impl_->panelsListener = 0;
    } else if (impl_->panelsListener) {
        impl_->panelsListener = 0;
    }
    if (impl_->displaysListener && impl_->host->displays()) {
        impl_->host->displays()->removeListener(impl_->displaysListener);
        impl_->displaysListener = 0;
    } else if (impl_->displaysListener) {
        impl_->displaysListener = 0;
    }
    if (impl_->workspacesListener && impl_->host->workspaces()) {
        impl_->host->workspaces()->removeListener(impl_->workspacesListener);
        impl_->workspacesListener = 0;
    } else if (impl_->workspacesListener) {
        impl_->workspacesListener = 0;
    }
    if (impl_->windowsListener && impl_->host->windows()) {
        impl_->host->windows()->removeListener(impl_->windowsListener);
        impl_->windowsListener = 0;
    } else if (impl_->windowsListener) {
        impl_->windowsListener = 0;
    }
    if (impl_->settingsListener && impl_->host->settings()) {
        impl_->host->settings()->removeListener(impl_->settingsListener);
        impl_->settingsListener = 0;
    } else if (impl_->settingsListener) {
        impl_->settingsListener = 0;
    }
    if (impl_->systemListener && impl_->host->system()) {
        impl_->host->system()->removeListener(impl_->systemListener);
        impl_->systemListener = 0;
    } else if (impl_->systemListener) {
        impl_->systemListener = 0;
    }
    if (impl_->scaleListener && impl_->host->scale()) {
        impl_->host->scale()->removeListener(impl_->scaleListener);
        impl_->scaleListener = 0;
    } else if (impl_->scaleListener) {
        impl_->scaleListener = 0;
    }
}

void Shell::onPanelsRevision(uint64_t) {
    if (!impl_->taskModel || !impl_->host || !impl_->host->panels())
        return;
    api::PanelsSnapshot ps = impl_->host->panels()->snapshot();
    impl_->taskModel->setSnapshot(ps);
    if (impl_->panelHost)
        impl_->panelHost->applyPanelSnapshot(ps);
    if (impl_->taskView)
        impl_->taskView->render();
    if (impl_->panelHost) {
        // Keep Start state in sync with PanelsSnapshot startOpen/startOutput.
        if (ps.startOpen)
            impl_->panelHost->setStartOpen(true, ps.startOutput);
        else
            impl_->panelHost->closeStart();
    }
    if (impl_->startView) {
        if (ps.startOpen)
            impl_->startView->open(ps.startOutput);
        else
            impl_->startView->close();
    }
    if (impl_->panelHost) impl_->panelHost->invalidateViews();
    if (impl_->popovers)
        impl_->popovers->reanchorVisible();
}

void Shell::onDisplaysGeneration(uint64_t) {
    if (!impl_->host || !impl_->host->displays())
        return;
    api::DisplaySnapshot ds = impl_->host->displays()->snapshot();
    if (impl_->host->panels())
        impl_->host->panels()->onDisplaySnapshot(ds);
    if (impl_->panelHost)
        impl_->panelHost->sync(ds);
    if (impl_->host->panels() && impl_->trayHost)
        impl_->trayHost->adopt(impl_->host->panels()->snapshot().trayOwner);
    if (impl_->panelHost)
        impl_->panelHost->bindViews(impl_->startView, impl_->taskView,
                                    impl_->workspaceView, impl_->statusArea,
                                    impl_->trayHost, impl_->clockView,
                                    impl_->host->reactor() ? impl_->host->reactor()->port() : 0,
                                    impl_->host->applications(), impl_->host->windows(),
                                    impl_->host->workspaces(), impl_->host->system());
    if (impl_->host->scale())
        impl_->host->scale()->onDisplayGeneration(ds.generation);
    if (impl_->popovers)
        impl_->popovers->reanchorVisible();
}

void Shell::onWorkspacesRevision(uint64_t) {
    if (!impl_->workspaceView || !impl_->host || !impl_->host->workspaces())
        return;
    api::WorkspaceSnapshot ws = impl_->host->workspaces()->snapshot();
    impl_->workspaceView->setSnapshot(ws);
    impl_->workspaceView->render();
    if (impl_->panelHost) impl_->panelHost->invalidateViews();
}

void Shell::onWindowsRevision(uint64_t) {
    if (!impl_->host || !impl_->host->windows())
        return;
    std::vector<api::WindowSnapshot> wins = impl_->host->windows()->snapshot();
    if (impl_->host->panels())
        impl_->host->panels()->onWindowSnapshots(wins);
    // Task reconciliation is driven by PanelService tasks; keep model in sync.
    if (impl_->taskModel && impl_->host->panels()) {
        impl_->taskModel->setSnapshot(impl_->host->panels()->snapshot());
        if (impl_->taskView)
            impl_->taskView->render();
        if (impl_->panelHost) impl_->panelHost->invalidateViews();
    }
}

void Shell::onSettingsRevision(uint64_t) {
    if (!impl_->host || !impl_->host->settings() || !impl_->host->panels())
        return;
    api::SettingsSnapshot ss = impl_->host->settings()->snapshot();
    impl_->host->panels()->onSettingsSnapshot(ss);
    // Settings may affect clock format / appearance; re-render lightweight views.
    if (impl_->clockView)
        impl_->clockView->invalidate();
    if (impl_->statusArea)
        impl_->statusArea->invalidate();
    if (impl_->panelHost) impl_->panelHost->invalidateViews();
}

void Shell::onSystemSnapshot(const platform::system::SystemSnapshot& snap) {
    if (!impl_->statusArea)
        return;
    impl_->statusArea->setSnapshot(snap);
    impl_->statusArea->render();
    if (impl_->panelHost) impl_->panelHost->invalidateViews();
}

} // namespace shell
} // namespace flamewm
