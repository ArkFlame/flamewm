#ifndef FLAMEWM_SHELL_SHELL_H
#define FLAMEWM_SHELL_SHELL_H

#include <cstdint>
#include <string>

namespace flamewm {
namespace platform {
class PlatformHost;
namespace system { struct SystemSnapshot; }
}
namespace engine { namespace icewm { class Bridge; } }
namespace control { class ControlServer; }
namespace shell {

class PanelHost;
class TaskModel;
class TaskView;
class StartView;
class WorkspaceView;
class StatusArea;
class ClockView;
class TrayHost;
class Popovers;

// Shell: one object graph composing all Flame panel/shell views atop
// PlatformHost. Owns PanelHost, TaskModel/View, StartView, WorkspaceView,
// StatusArea, ClockView, TrayHost, Popovers. Subscribes to PlatformHost
// services (WindowService, WorkspaceService, DisplayService, PanelService,
// ApplicationService, ScaleService, SettingsService, SystemService, etc.)
// via snapshot/revision listeners. Drives startup/shutdown generation and
// integrates with a borrowed ControlServer when available. Header is
// X11-free and C++11-compatible.
class Shell {
public:
    explicit Shell(platform::PlatformHost* host);
    explicit Shell(platform::PlatformHost* host, control::ControlServer* bus);
    ~Shell();

    Shell(const Shell&) = delete;
    Shell& operator=(const Shell&) = delete;

    // Lifecycle: subscribe to snapshots/revisions, perform initial sync from
    // PlatformHost services, bump generation. stop() clears subscriptions,
    // hides panels/popovers, bumps generation, and is idempotent.
    bool start();
    void stop();

    bool isStarted() const;
    uint64_t generation() const;

    // Toggle one global Start surface, selecting focused output first.
    bool toggleStart(int keyCode, unsigned state);

    platform::PlatformHost* host() const;
    control::ControlServer* controlServer() const;
    void setControlServer(control::ControlServer* bus);

    // Owned graph — never null after construction, null after stop() still
    // valid pointer until destruction but views are cleared.
    PanelHost* panelHost() const;
    TaskModel* taskModel() const;
    TaskView* taskView() const;
    StartView* startView() const;
    WorkspaceView* workspaceView() const;
    StatusArea* statusArea() const;
    ClockView* clockView() const;
    TrayHost* trayHost() const;
    Popovers* popovers() const;

private:
    void subscribe();
    void unsubscribe();
    void syncInitial();
    void bumpGeneration();

    void onPanelsRevision(uint64_t rev);
    void onDisplaysGeneration(uint64_t gen);
    void onWorkspacesRevision(uint64_t rev);
    void onWindowsRevision(uint64_t rev);
    void onSettingsRevision(uint64_t rev);
    void onSystemSnapshot(const platform::system::SystemSnapshot& snap);

    struct Impl;
    Impl* impl_;
};

} // namespace shell
} // namespace flamewm

#endif // FLAMEWM_SHELL_SHELL_H
