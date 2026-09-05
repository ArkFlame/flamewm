#ifndef FLAMEWM_SHELL_PANEL_HOST_H
#define FLAMEWM_SHELL_PANEL_HOST_H

#include "flamewm/api/display.h"
#include "flamewm/api/geometry.h"
#include "flamewm/api/ids.h"
#include "flamewm/api/panels.h"
#include "flamewm/api/ports.h"
#include "flamewm/shell/panel_surface.h"

#include <map>
#include <vector>

#if defined(__has_include)
#if __has_include("flamewm/engine/icewm/display_adapter.h")
#include "flamewm/engine/icewm/display_adapter.h"
#endif
#endif

namespace flamewm {
namespace platform {
namespace applications { class ApplicationService; }
namespace system { class SystemService; }
namespace windows { class WindowService; }
namespace workspaces { class WorkspaceService; }
}
namespace shell {

class StartView;
class TaskView;
class WorkspaceView;
class StatusArea;
class ClockView;
class TrayHost;

class PanelHost {
public:
    explicit PanelHost(api::WorkAreaPort* workArea);
    PanelHost(api::WorkAreaPort* workArea, api::TrayPort* tray);
    ~PanelHost();

    // Non-copyable.
    PanelHost(const PanelHost&) = delete;
    PanelHost& operator=(const PanelHost&) = delete;

    // Sync to active outputs — creates/removes PanelSurface per output.
    // Geometry comes from DisplaySnapshot output geometries; one panel per
    // connected output. Removes stale, creates new, updates geometry for
    // existing. Handles one-open Start close on removed output and tray
    // owner selection. Guarded against re-entrancy (no infinite feedback).
    void sync(const api::DisplaySnapshot& snapshot);
    void handleHotplug(const api::DisplaySnapshot& snapshot);
    // Apply persisted/runtime panel presentation settings after display sync.
    void applyPanelSnapshot(const api::PanelsSnapshot& snapshot);
    void bindViews(StartView* start, TaskView* tasks, WorkspaceView* pager,
                   StatusArea* status, TrayHost* tray, ClockView* clock,
                   api::MainLoopPort* loop,
                   platform::applications::ApplicationService* applications,
                   platform::windows::WindowService* windows,
                   platform::workspaces::WorkspaceService* workspaces,
                   platform::system::SystemService* system);
    void invalidateViews();
    void stopViewTimers();

    PanelSurface* panelFor(const api::OutputId& output);
    const PanelSurface* panelFor(const api::OutputId& output) const;

    std::vector<PanelSurface*> panels();
    std::vector<const PanelSurface*> panels() const;

    size_t count() const;
    bool empty() const;
    void clear();

    // Apply struts for all panels via WorkAreaPort (aggregated).
    void requestStruts();

    api::WorkAreaPort* workArea() const;
    api::TrayPort* tray() const;

    // Tray owner: exactly one owner across all panels (primary > first).
    api::OutputId trayOwner() const;

    // One-open Start: at most one Start open globally.
    bool isStartOpen() const;
    api::OutputId startOutput() const;
    // Open Start on output (closes previous). Returns false if output invalid.
    bool setStartOpen(bool open, const api::OutputId& output);
    bool toggleStart(const api::OutputId& output);
    void closeStart();

private:
    struct Presentation;
    std::vector<std::pair<api::OutputId, api::Rect> > buildReservations() const;

    api::WorkAreaPort* workArea_;
    api::TrayPort* tray_;
    std::map<api::OutputId, PanelSurface*> surfaces_;
    std::map<api::OutputId, Presentation*> presentations_;
    bool syncing_;
    bool startOpen_;
    api::OutputId startOutput_;
    api::OutputId trayOwner_;
    api::OutputId primaryOutput_;
    bool requestingStruts_;
};

} // namespace shell
} // namespace flamewm

#endif // FLAMEWM_SHELL_PANEL_HOST_H
