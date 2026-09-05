#ifndef FLAMEWM_SHELL_TASK_VIEW_H
#define FLAMEWM_SHELL_TASK_VIEW_H

#include "flamewm/api/ids.h"
#include "flamewm/api/panels.h"
#include "flamewm/api/window.h"
#include "flamewm/shell/task_model.h"
#include "flamewm/ui/window.h"
#include "flamewm/ui/button.h"
#include "flamewm/ui/popover.h"
#include "flamewm/ui/list.h"

#include <string>
#include <vector>

namespace flamewm {
// Task-specific state bits consumed by native task buttons. Keep these
// separate from generic control states: running is not an input state.
enum TaskVisualStateBits {
    TaskVisualRunning = 1u << 0,
    TaskVisualFocused = 1u << 1,
    TaskVisualMinimized = 1u << 2,
    TaskVisualHovered = 1u << 3,
    TaskVisualPressed = 1u << 4
};

namespace platform {
namespace windows { class WindowService; }
namespace applications { class ApplicationService; }
}
namespace shell {

// Renders TaskModel entries; V7 context menus; click handling:
// - pinned without window -> launch
// - window minimized -> restore/activate
// - window active -> minimize
// - otherwise -> activate
class TaskView {
public:
    explicit TaskView(TaskModel* model);
    ~TaskView();

    TaskView(const TaskView&) = delete;
    TaskView& operator=(const TaskView&) = delete;

    void setModel(TaskModel* m);
    TaskModel* model() const;

    void setWindowService(platform::windows::WindowService* service);
    void setApplicationService(platform::applications::ApplicationService* service);
    void setContainer(flamewm::ui::Window* c);
    void setPanelEdge(api::PanelEdge edge);
    api::PanelEdge panelEdge() const;

    // Render current model into container.
    void render();
    void invalidate();

    // Input handling per task id.
    void onClick(const api::TaskEntryId& id);
    void onContextMenu(const api::TaskEntryId& id);

    // Drag/reorder delegate (V8 authority for ordering lives elsewhere; view only forwards).
    void onReorder(const api::TaskEntryId& id, int newIndex);

private:
    struct TaskVisualState {
        bool running;
        bool focused;
        bool minimized;
        bool hovered;
        bool pressed;

        TaskVisualState()
            : running(false), focused(false), minimized(false), hovered(false), pressed(false) {}
    };

    struct DragState {
        bool active;
        bool dragging;
        api::TaskEntryId id;
        int startX;
        int startY;
        int lastX;
        int lastY;
        int originIndex;
        int previewIndex;
        std::vector<api::TaskEntryId> originOrder;
        int button;
        DragState() : active(false), dragging(false), startX(0), startY(0), lastX(0), lastY(0), originIndex(-1), previewIndex(-1), button(0) {}
    };

    void clearButtons();
    void applyVisualState(flamewm::ui::Button* button, const TaskVisualState& state) const;
    flamewm::ui::Button* buttonFor(const api::TaskEntryId& id) const;
    int indexOf(const api::TaskEntryId& id) const;
    void handlePress(const api::TaskEntryId& id, int x, int y, int btn);
    void handleMotion(int x, int y);
    void handleRelease(const api::TaskEntryId& id, int x, int y, int btn, bool cancel);
    int dropIndexForCoord(int x, int y, const api::TaskEntryId& dragged) const;
    void updatePreview(int previewIdx);
    void commitDrag();
    void cancelDrag();

    void showContextMenu(const api::TaskEntryId& id, int ax, int ay);
    void hideContextMenu();
    api::WindowSnapshot windowSnapshotFor(const api::TaskEntry& e) const;

    TaskModel* model_;
    platform::windows::WindowService* windowService_;
    platform::applications::ApplicationService* applicationService_;
    flamewm::ui::Window* container_;
    api::PanelEdge edge_;
    std::vector<flamewm::ui::Button*> buttons_;
    std::vector<api::TaskEntryId> buttonIds_;
    std::vector<TaskVisualState> buttonStates_;
    std::vector<api::TaskEntryId> previewOrder_;
    bool previewActive_;
    DragState drag_;
    flamewm::ui::Popover* popover_;
    flamewm::ui::List* menuList_;
    bool closingPopover_;
};

} // namespace shell
} // namespace flamewm

#endif // FLAMEWM_SHELL_TASK_VIEW_H
