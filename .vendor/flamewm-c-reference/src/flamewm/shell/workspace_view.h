#ifndef FLAMEWM_SHELL_WORKSPACE_VIEW_H
#define FLAMEWM_SHELL_WORKSPACE_VIEW_H

#include "flamewm/api/ports.h"
#include "flamewm/api/workspace.h"
#include "flamewm/ui/window.h"
#include "flamewm/workspace/topology.h"

#include <string>
#include <vector>

namespace flamewm {
namespace ui { class Toggle; }
namespace platform { namespace workspaces { class WorkspaceService; } }
namespace shell {

// Pager driven by TwoRowTopology; takes zero space when single workspace.
class WorkspaceView {
public:
    explicit WorkspaceView(api::WorkspacePort* port);
    ~WorkspaceView();

    WorkspaceView(const WorkspaceView&) = delete;
    WorkspaceView& operator=(const WorkspaceView&) = delete;

    void setContainer(flamewm::ui::Window* c);
    // Vertical panels use the same two-row topology with compact portrait cells.
    void setVertical(bool vertical);
    void setWorkspacePort(api::WorkspacePort* p);
    void setWorkspaceService(platform::workspaces::WorkspaceService* svc);

    bool shouldShow() const;
    void setSnapshot(const api::WorkspaceSnapshot& s);
    api::WorkspaceSnapshot snapshot() const;

    // Pull snapshot from WorkspaceService when available, else from port.
    void refreshFromService();

    void render();
    void invalidate();

    // TwoRowTopology delegates (single source; also used for paint).
    static workspace::IndexPos indexToPos(int idx, int total);
    static int posToIndex(int row, int col, int total);

    void activate(int index);
    void onClick(int index);

private:
    void ensureButtons();
    void clearButtons();
    void syncButtons();

    api::WorkspacePort* port_;
    platform::workspaces::WorkspaceService* service_;
    flamewm::ui::Window* container_;
    bool vertical_;
    api::WorkspaceSnapshot snapshot_;
    std::vector<flamewm::ui::Toggle*> buttons_;
};

} // namespace shell
} // namespace flamewm

#endif // FLAMEWM_SHELL_WORKSPACE_VIEW_H
