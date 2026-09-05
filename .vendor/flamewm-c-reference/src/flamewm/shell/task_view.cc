#include "flamewm/shell/task_view.h"
#include "flamewm/platform/windows/service.h"
#include "flamewm/platform/applications/service.h"
#include "flamewm/ui/backend.h"
#include "flamewm/ui/shellstyle.h"
#include <algorithm>
#include <cmath>

namespace flamewm {
namespace shell {

static bool isHorizontalEdge(api::PanelEdge e) {
    return e == api::PanelEdge::Bottom || e == api::PanelEdge::Top;
}

static int taskSlotSize() {
    return flamewm::ShellStyle::defaults().taskButtonWidth;
}

TaskView::TaskView(TaskModel* m)
    : model_(m), windowService_(nullptr), applicationService_(nullptr), container_(nullptr)
    , edge_(api::PanelEdge::Bottom), previewActive_(false), popover_(nullptr), menuList_(nullptr)
    , closingPopover_(false) {}

TaskView::~TaskView() {
    clearButtons();
    if (menuList_) { delete menuList_; menuList_ = nullptr; }
    if (popover_) { delete popover_; popover_ = nullptr; }
}

void TaskView::setModel(TaskModel* m) { model_ = m; }
TaskModel* TaskView::model() const { return model_; }

void TaskView::setWindowService(platform::windows::WindowService* service) { windowService_ = service; }
void TaskView::setApplicationService(platform::applications::ApplicationService* service) { applicationService_ = service; }
void TaskView::setContainer(flamewm::ui::Window* c) { container_ = c; }
void TaskView::setPanelEdge(api::PanelEdge e) { edge_ = e; }
api::PanelEdge TaskView::panelEdge() const { return edge_; }

void TaskView::clearButtons() {
    for (size_t i = 0; i < buttons_.size(); ++i) delete buttons_[i];
    buttons_.clear();
    buttonIds_.clear();
    buttonStates_.clear();
}

flamewm::ui::Button* TaskView::buttonFor(const api::TaskEntryId& id) const {
    for (size_t i = 0; i < buttons_.size(); ++i) {
        if (i < buttonIds_.size() && buttons_[i] && buttonIds_[i] == id) return buttons_[i];
    }
    return nullptr;
}

int TaskView::indexOf(const api::TaskEntryId& id) const {
    if (!model_) return -1;
    std::vector<api::TaskEntry> ts = model_->orderedTasks();
    for (size_t i = 0; i < ts.size(); ++i) if (ts[i].id == id) return static_cast<int>(i);
    return -1;
}

void TaskView::applyVisualState(flamewm::ui::Button* button,
                                const TaskVisualState& state) const {
    if (!button) return;

    unsigned visualState = 0;
    if (state.running) visualState |= flamewm::TaskVisualRunning;
    if (state.focused) visualState |= flamewm::TaskVisualFocused;
    if (state.minimized) visualState |= flamewm::TaskVisualMinimized;
    if (state.hovered) visualState |= flamewm::TaskVisualHovered;
    if (state.pressed) visualState |= flamewm::TaskVisualPressed;
    button->setVisualState(visualState);

    unsigned controlState = 0;
    if (state.focused) controlState |= static_cast<unsigned>(flamewm::ui::style::VisualFocused);
    if (state.hovered) controlState |= static_cast<unsigned>(flamewm::ui::style::VisualHovered);
    if (state.pressed) controlState |= static_cast<unsigned>(flamewm::ui::style::VisualPressed);
    button->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleTask,
                                                  controlState));
}

api::WindowSnapshot TaskView::windowSnapshotFor(const api::TaskEntry& e) const {
    api::WindowSnapshot ws;
    if (!windowService_ || !e.hasWindow) return ws;
    api::Result<api::WindowSnapshot> r = windowService_->get(e.window);
    if (r.ok()) ws = r.value();
    return ws;
}

void TaskView::render() {
    if (!model_) return;
    clearButtons();
    previewActive_ = false;
    previewOrder_.clear();
    drag_ = DragState();
    std::vector<api::TaskEntry> tasks = model_->orderedTasks();
    for (size_t i = 0; i < tasks.size(); ++i) {
        const api::TaskEntry& e = tasks[i];
        flamewm::ui::Button* b = flamewm::ui::UiBackend::createButton(container_);
        if (!b) continue;
        std::string iconName;
        if (applicationService_) {
            api::Result<api::DesktopApplication> app = applicationService_->findById(e.appId);
            if (app.ok()) iconName = app.value().iconName;
        }
        b->setIconName(iconName);
        // Retain the app id fallback when desktop metadata is unavailable.
        b->setText(iconName.empty() ? e.appId.value : iconName);
        TaskVisualState state;
        state.running = e.hasWindow;
        if (e.hasWindow) {
            api::WindowSnapshot ws = windowSnapshotFor(e);
            state.focused = ws.focused;
            state.minimized = ws.minimized;
        }
        applyVisualState(b, state);
        b->setIconRole(state.focused ? flamewm::IconRoleTaskbarStatus
                                     : flamewm::IconRoleTaskbar);
        // Short release activates; right-click uses the context menu.
        api::TaskEntryId bid = e.id;
        b->setOnContextMenu([this, bid](int x, int y) {
            showContextMenu(bid, x, y);
         });
        b->setOnPress([this, bid](int x, int y, int btn) {
            for (size_t i = 0; i < buttonIds_.size(); ++i) {
                if (buttonIds_[i] == bid && i < buttonStates_.size()) {
                    buttonStates_[i].pressed = true;
                    applyVisualState(buttons_[i], buttonStates_[i]);
                    break;
                }
            }
            handlePress(bid, x, y, btn);
        });
        b->setOnRelease([this, bid](int x, int y, int btn) {
            for (size_t i = 0; i < buttonIds_.size(); ++i) {
                if (buttonIds_[i] == bid && i < buttonStates_.size()) {
                    buttonStates_[i].pressed = false;
                    applyVisualState(buttons_[i], buttonStates_[i]);
                    break;
                }
            }
            handleRelease(bid, x, y, btn, false);
        });
        b->setOnMotion([this, bid](int x, int y) {
            for (size_t i = 0; i < buttonIds_.size(); ++i) {
                if (i >= buttonStates_.size()) continue;
                buttonStates_[i].hovered = buttonIds_[i] == bid;
                applyVisualState(buttons_[i], buttonStates_[i]);
            }
            handleMotion(x, y);
        });
        if (container_) {
            // Simple linear layout: horizontal uses x, vertical uses y.
            const flamewm::ShellStyle style = flamewm::ShellStyle::defaults();
            int sz = taskSlotSize();
            int gap = 2;
            api::Rect cr = container_->geometry();
            (void)cr;
            if (isHorizontalEdge(edge_)) b->setGeometry(api::Rect(static_cast<int>(i) * (sz + gap), 0, sz, style.panelHeight));
            else b->setGeometry(api::Rect(0, static_cast<int>(i) * (sz + gap), style.panelHeight, sz));
        }
        b->show();
        buttons_.push_back(b);
        buttonIds_.push_back(e.id);
        buttonStates_.push_back(state);
    }
}

void TaskView::invalidate() { render(); }

void TaskView::onClick(const api::TaskEntryId& id) {
    if (!model_) return;
    const api::TaskEntry* e = model_->find(id);
    if (!e) return;
    if (model_->isPinned(e->appId) && !e->hasWindow) {
        if (applicationService_) applicationService_->launch(e->appId);
        return;
    }
    if (!e->hasWindow) return;
    if (!windowService_) return;
    api::WindowSnapshot ws = windowSnapshotFor(*e);
    // If we cannot query, just activate.
    if (ws.minimized) {
        windowService_->restore(e->window);
        windowService_->activate(e->window);
        return;
    }
    if (ws.focused && !ws.minimized) {
        windowService_->minimize(e->window);
    } else {
        windowService_->activate(e->window);
    }
}

void TaskView::onContextMenu(const api::TaskEntryId& id) {
    if (!model_) return;
    const api::TaskEntry* e = model_->find(id);
    if (!e) return;
    showContextMenu(id, 0, 0);
}

void TaskView::onReorder(const api::TaskEntryId& id, int newIndex) {
    if (!model_) return;
    model_->move(id, newIndex);
    render();
}

// Drag helpers

int TaskView::dropIndexForCoord(int x, int y, const api::TaskEntryId& dragged) const {
    if (!model_) return 0;
    // Determine insertion index based on previewOrder_ or model order.
    std::vector<api::TaskEntryId> order;
    if (previewActive_ && !previewOrder_.empty()) order = previewOrder_;
    else {
        std::vector<api::TaskEntry> ts = model_->orderedTasks();
        order.reserve(ts.size());
        for (size_t i = 0; i < ts.size(); ++i) order.push_back(ts[i].id);
    }
    // Build list without dragged.
    std::vector<api::TaskEntryId> siblings;
    siblings.reserve(order.size());
    for (size_t i = 0; i < order.size(); ++i) if (!(order[i] == dragged)) siblings.push_back(order[i]);

    if (siblings.empty()) return 0;

    bool horiz = isHorizontalEdge(edge_);
    int coord = horiz ? x : y;

    // Use button rects if available to find insertion point, otherwise linear index.
    // Try geometry-based: iterate siblings buttons.
    for (size_t i = 0; i < siblings.size(); ++i) {
        flamewm::ui::Button* b = nullptr;
        for (size_t k = 0; k < buttons_.size(); ++k) {
            if (k < buttonIds_.size() && buttons_[k] && buttonIds_[k] == siblings[i]) { b = buttons_[k]; break; }
        }
        if (!b) continue;
        api::Rect r = b->geometry();
        int center = horiz ? (r.x + r.w / 2) : (r.y + r.h / 2);
        if (coord < center) return static_cast<int>(i);
    }
    // Fallback: uniform slots.
    int slot = taskSlotSize() + 2;
    int idx = coord / slot;
    if (idx < 0) idx = 0;
    if (idx > static_cast<int>(siblings.size())) idx = static_cast<int>(siblings.size());
    // If we had geometry hit, we already returned; fallback index based on coord.
    if (coord < 0) return 0;
    // Use linear mapping for fallback.
    return idx > static_cast<int>(siblings.size()) ? static_cast<int>(siblings.size()) : idx;
}

void TaskView::updatePreview(int previewIdx) {
    if (!drag_.active) return;
    std::vector<api::TaskEntryId> base = drag_.originOrder;
    // Remove dragged
    std::vector<api::TaskEntryId> siblings;
    siblings.reserve(base.size());
    for (size_t i = 0; i < base.size(); ++i) if (!(base[i] == drag_.id)) siblings.push_back(base[i]);
    if (previewIdx < 0) previewIdx = 0;
    if (previewIdx > static_cast<int>(siblings.size())) previewIdx = static_cast<int>(siblings.size());
    std::vector<api::TaskEntryId> reordered;
    reordered.reserve(siblings.size() + 1);
    for (int i = 0; i < previewIdx; ++i) reordered.push_back(siblings[static_cast<size_t>(i)]);
    reordered.push_back(drag_.id);
    for (size_t i = static_cast<size_t>(previewIdx); i < siblings.size(); ++i) reordered.push_back(siblings[i]);
    previewOrder_ = reordered;
    previewActive_ = true;
    drag_.previewIndex = previewIdx;
    // Reorder buttons to reflect preview (visual).
    std::vector<flamewm::ui::Button*> newBtns;
    std::vector<api::TaskEntryId> newIds;
    std::vector<TaskVisualState> newStates;
    newBtns.reserve(buttons_.size());
    newIds.reserve(buttonIds_.size());
    newStates.reserve(buttonStates_.size());
    for (size_t i = 0; i < reordered.size(); ++i) {
        for (size_t k = 0; k < buttons_.size(); ++k) {
            if (k < buttonIds_.size() && buttons_[k] && buttonIds_[k] == reordered[i]) {
                newBtns.push_back(buttons_[k]);
                newIds.push_back(buttonIds_[k]);
                if (k < buttonStates_.size()) newStates.push_back(buttonStates_[k]);
                break;
            }
        }
    }
    // Append any missing (should not happen)
    for (size_t k = 0; k < buttons_.size(); ++k) {
        bool found = false;
        for (size_t j = 0; j < newBtns.size(); ++j) if (newBtns[j] == buttons_[k]) { found = true; break; }
        if (!found) {
            newBtns.push_back(buttons_[k]);
            newIds.push_back(buttonIds_[k]);
            if (k < buttonStates_.size()) newStates.push_back(buttonStates_[k]);
        }
    }
    buttons_ = newBtns;
    buttonIds_ = newIds;
    buttonStates_ = newStates;
    // Relayout to show before/between/after insertion visually.
    for (size_t i = 0; i < buttons_.size(); ++i) {
        if (!buttons_[i]) continue;
        const flamewm::ShellStyle style = flamewm::ShellStyle::defaults();
        int sz = taskSlotSize(); int gap = 2;
        if (isHorizontalEdge(edge_)) buttons_[i]->setGeometry(api::Rect(static_cast<int>(i) * (sz + gap), 0, sz, style.panelHeight));
        else buttons_[i]->setGeometry(api::Rect(0, static_cast<int>(i) * (sz + gap), style.panelHeight, sz));
    }
}

void TaskView::commitDrag() {
    if (!drag_.active || !drag_.dragging) return;
    if (!model_) return;
    if (previewActive_ && !previewOrder_.empty()) {
        // Use setOrder if available; otherwise move.
        model_->setOrder(previewOrder_);
    } else if (drag_.previewIndex >= 0) {
        model_->move(drag_.id, drag_.previewIndex);
    }
    previewActive_ = false;
    previewOrder_.clear();
    render();
}

void TaskView::cancelDrag() {
    if (!drag_.active) return;
    // Restore exact original order.
    if (previewActive_) {
        previewActive_ = false;
        previewOrder_ = drag_.originOrder;
        // Rebuild button order to origin
        std::vector<flamewm::ui::Button*> newBtns;
        std::vector<api::TaskEntryId> newIds;
        std::vector<TaskVisualState> newStates;
        newBtns.reserve(buttons_.size());
        newIds.reserve(buttonIds_.size());
        for (size_t i = 0; i < drag_.originOrder.size(); ++i) {
            for (size_t k = 0; k < buttons_.size(); ++k) {
                if (k < buttonIds_.size() && buttons_[k] && buttonIds_[k] == drag_.originOrder[i]) {
                    newBtns.push_back(buttons_[k]);
                    newIds.push_back(buttonIds_[k]);
                    if (k < buttonStates_.size()) newStates.push_back(buttonStates_[k]);
                    break;
                }
            }
        }
        for (size_t k = 0; k < buttons_.size(); ++k) {
            bool found = false;
            for (size_t j = 0; j < newBtns.size(); ++j) if (newBtns[j] == buttons_[k]) { found = true; break; }
            if (!found) {
                newBtns.push_back(buttons_[k]);
                newIds.push_back(buttonIds_[k]);
                if (k < buttonStates_.size()) newStates.push_back(buttonStates_[k]);
            }
        }
        buttons_ = newBtns;
        buttonIds_ = newIds;
        buttonStates_ = newStates;
        for (size_t i = 0; i < buttons_.size(); ++i) {
            if (!buttons_[i]) continue;
            const flamewm::ShellStyle style = flamewm::ShellStyle::defaults();
            int sz = taskSlotSize(); int gap = 2;
            if (isHorizontalEdge(edge_)) buttons_[i]->setGeometry(api::Rect(static_cast<int>(i) * (sz + gap), 0, sz, style.panelHeight));
            else buttons_[i]->setGeometry(api::Rect(0, static_cast<int>(i) * (sz + gap), style.panelHeight, sz));
        }
        previewActive_ = false;
        previewOrder_.clear();
    }
}

void TaskView::handlePress(const api::TaskEntryId& id, int x, int y, int btn) {
    if (btn != 1) return;
    if (drag_.active) return;
    drag_.active = true;
    drag_.dragging = false;
    drag_.id = id;
    drag_.startX = x;
    drag_.startY = y;
    drag_.lastX = x;
    drag_.lastY = y;
    drag_.button = btn;
    drag_.originIndex = indexOf(id);
    drag_.previewIndex = drag_.originIndex;
    if (model_) {
        std::vector<api::TaskEntry> ts = model_->orderedTasks();
        drag_.originOrder.clear();
        drag_.originOrder.reserve(ts.size());
        for (size_t i = 0; i < ts.size(); ++i) drag_.originOrder.push_back(ts[i].id);
    } else {
        drag_.originOrder.clear();
    }
}

void TaskView::handleMotion(int x, int y) {
    if (!drag_.active) return;
    drag_.lastX = x;
    drag_.lastY = y;
    int dx = x - drag_.startX;
    int dy = y - drag_.startY;
    double dist = std::sqrt(static_cast<double>(dx*dx + dy*dy));
    if (!drag_.dragging) {
        if (dist < 6.0) return;
        drag_.dragging = true;
    }
    int idx = dropIndexForCoord(x, y, drag_.id);
    updatePreview(idx);
}

void TaskView::handleRelease(const api::TaskEntryId& id, int x, int y, int btn, bool cancel) {
    (void)id; (void)x; (void)y; (void)btn;
    if (!drag_.active) {
        // Click already handled via onClick; ignore.
        return;
    }
    bool wasDragging = drag_.dragging;
    if (cancel) {
        cancelDrag();
        drag_ = DragState();
        render();
        return;
    }
    if (!wasDragging) {
        // Threshold not exceeded -> treat as click
        drag_ = DragState();
        onClick(id);
        return;
    }
    // Dragging -> commit on release
    commitDrag();
    drag_ = DragState();
}

void TaskView::showContextMenu(const api::TaskEntryId& id, int ax, int ay) {
    if (!model_) return;
    const api::TaskEntry* e = model_->find(id);
    if (!e) return;
    hideContextMenu();
    if (!popover_) popover_ = flamewm::ui::UiBackend::createPopover(container_);
    if (popover_ && !menuList_) menuList_ = flamewm::ui::UiBackend::createList(popover_);
    if (!popover_ || !menuList_) return;

    // Pin state is application identity, not a property of one live window.
    bool pinned = model_->isPinned(e->appId);
    bool hasWin = e->hasWindow;

    std::vector<flamewm::ui::ListRow> rows;
    if (!hasWin) {
        if (pinned) rows.push_back(flamewm::ui::ListRow("open", "Open"));
        if (pinned) rows.push_back(flamewm::ui::ListRow("unpin", "Unpin"));
    } else {
        rows.push_back(flamewm::ui::ListRow(pinned ? "unpin" : "pin",
                                            pinned ? "Unpin" : "Pin"));
        rows.push_back(flamewm::ui::ListRow("sep1", ""));
        api::WindowSnapshot ws = windowSnapshotFor(*e);
        rows.push_back(flamewm::ui::ListRow(
            ws.maximized && !ws.minimized ? "minimize" : "maximize",
            ws.maximized && !ws.minimized ? "Minimize" : "Maximize"));
        rows.push_back(flamewm::ui::ListRow("close", "Close"));
    }

    // Filter separators for backend that may not render them; keep as disabled rows handled via callback ignore.
    menuList_->setRows(rows);

    // Anchor at button or fallback
    api::Rect anchor(0,0,0,0);
    for (size_t i = 0; i < buttons_.size(); ++i) {
        if (i < buttonIds_.size() && buttons_[i] && buttonIds_[i] == id) { anchor = buttons_[i]->geometry(); break; }
    }
    if (!anchor.valid()) anchor = api::Rect(ax, ay, 1, 1);

    const ShellStyle style = ShellStyle::defaults();
    const int width = style.startSubmenuWidth;
    const int height = style.popoverPadding * 2 + static_cast<int>(rows.size()) * 32;
    menuList_->setGeometry(api::Rect(style.popoverPadding, style.popoverPadding,
                                     std::max(0, width - style.popoverPadding * 2),
                                     std::max(0, height - style.popoverPadding * 2)));
    menuList_->show();

    // Wire selection
    flamewm::ui::List* list = menuList_;
    flamewm::ui::Popover* pop = popover_;
    TaskModel* m = model_;
    platform::windows::WindowService* windowService = windowService_;
    platform::applications::ApplicationService* applicationService = applicationService_;
    api::TaskEntry entry = *e;

    list->setOnActivated([this, list, m, windowService, applicationService, entry](int idx) {
        std::vector<flamewm::ui::ListRow> rs = list->rows();
        if (idx < 0 || idx >= static_cast<int>(rs.size())) { hideContextMenu(); return; }
        std::string rid = rs[static_cast<size_t>(idx)].id;
        if (rid == "sep1" || rid.rfind("sep", 0) == 0) return;
        if (rid == "open") {
            if (applicationService) applicationService->launch(entry.appId);
        } else if (rid == "pin") {
            if (m) m->pin(entry.appId);
            render();
        } else if (rid == "unpin") {
            if (m) m->unpin(entry.appId);
            render();
        } else if (rid == "maximize") {
            if (windowService && entry.hasWindow) windowService->maximize(entry.window);
        } else if (rid == "minimize") {
            if (windowService && entry.hasWindow) windowService->minimize(entry.window);
        } else if (rid == "close") {
            if (windowService && entry.hasWindow) windowService->close(entry.window);
        }
        hideContextMenu();
    });

    pop->setOnClosed([this]() { hideContextMenu(); });
    pop->setGeometry(api::Rect(0, 0, width, height));
    pop->showAt(anchor);
}

void TaskView::hideContextMenu() {
    if (!popover_ || closingPopover_) return;
    closingPopover_ = true;
    popover_->close();
    closingPopover_ = false;
    // Keep objects for reuse; do not delete immediately to avoid callback use-after-free.
}

} // namespace shell
} // namespace flamewm
