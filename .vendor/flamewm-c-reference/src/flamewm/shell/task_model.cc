#include "flamewm/shell/task_model.h"
#if defined(__has_include)
#if __has_include("flamewm/platform/panels/service.h")
#include "flamewm/platform/panels/service.h"
#endif
#endif
#include <algorithm>

namespace flamewm {
namespace shell {

TaskModel::TaskModel() : snapshot_(), service_(nullptr) {}
TaskModel::~TaskModel() {}

void TaskModel::setSnapshot(const api::PanelsSnapshot& s) { snapshot_ = s; }
api::PanelsSnapshot TaskModel::snapshot() const { return snapshot_; }

std::vector<api::DesktopAppId> TaskModel::pinnedApps() const { return snapshot_.pinnedApps; }

bool TaskModel::isPinned(const api::DesktopAppId& app) const {
    if (!app.valid()) return false;
    for (size_t i = 0; i < snapshot_.pinnedApps.size(); ++i) {
        if (snapshot_.pinnedApps[i] == app) return true;
    }
    return false;
}

void TaskModel::setPanelService(platform::panels::PanelService* svc) { service_ = svc; }
platform::panels::PanelService* TaskModel::panelService() const { return service_; }

int TaskModel::taskIndex(const api::TaskEntryId& id) const {
    for (size_t i = 0; i < snapshot_.tasks.size(); ++i) if (snapshot_.tasks[i].id == id) return static_cast<int>(i);
    return -1;
}
int TaskModel::pinnedIndex(const api::DesktopAppId& app) const {
    for (size_t i = 0; i < snapshot_.pinnedApps.size(); ++i) if (snapshot_.pinnedApps[i] == app) return static_cast<int>(i);
    return -1;
}
void TaskModel::bumpRevision() { ++snapshot_.revision; }

api::Status TaskModel::pin(const api::DesktopAppId& app) {
    if (!app.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid app");
    if (isPinned(app)) return api::Status::make(api::Error::Conflict, "already pinned");
    if (service_) {
        api::Status st = service_->pin(app, snapshot_.revision);
        if (!st.ok()) return st;
        snapshot_ = service_->snapshot();
        return api::Status::Ok();
    }
    snapshot_.pinnedApps.push_back(app);
    api::TaskEntry e;
    e.id = api::TaskEntryId::pinned(app.value);
    e.isPinned = true;
    e.hasWindow = false;
    e.appId = app;
    e.orderIndex = static_cast<int>(snapshot_.tasks.size());
    snapshot_.tasks.push_back(e);
    bumpRevision();
    for (size_t i = 0; i < snapshot_.tasks.size(); ++i) snapshot_.tasks[i].orderIndex = static_cast<int>(i);
    return api::Status::Ok();
}

api::Status TaskModel::unpin(const api::DesktopAppId& app) {
    if (!app.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid app");
    int idx = pinnedIndex(app);
    if (idx < 0) return api::Status::make(api::Error::NotFound, "not pinned");
    if (service_) {
        api::Status st = service_->unpin(app, snapshot_.revision);
        if (!st.ok()) return st;
        snapshot_ = service_->snapshot();
        return api::Status::Ok();
    }
    snapshot_.pinnedApps.erase(snapshot_.pinnedApps.begin() + idx);
    api::TaskEntryId pid = api::TaskEntryId::pinned(app.value);
    int ti = taskIndex(pid);
    if (ti >= 0) {
        // If pinned entry has a window, convert to window entry? For minimal model, keep entry but mark unpinned if it has window.
        if (snapshot_.tasks[static_cast<size_t>(ti)].hasWindow) {
            snapshot_.tasks[static_cast<size_t>(ti)].isPinned = false;
        } else {
            snapshot_.tasks.erase(snapshot_.tasks.begin() + ti);
        }
    }
    bumpRevision();
    for (size_t i = 0; i < snapshot_.tasks.size(); ++i) snapshot_.tasks[i].orderIndex = static_cast<int>(i);
    return api::Status::Ok();
}

std::vector<api::TaskEntry> TaskModel::orderedTasks() const { return snapshot_.tasks; }

const api::TaskEntry* TaskModel::find(const api::TaskEntryId& id) const {
    if (!id.valid()) return nullptr;
    for (size_t i = 0; i < snapshot_.tasks.size(); ++i) if (snapshot_.tasks[i].id == id) return &snapshot_.tasks[i];
    return nullptr;
}

api::Status TaskModel::move(const api::TaskEntryId& id, int newIndex) {
    if (!id.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid entryId");
    int from = taskIndex(id);
    if (from < 0) return api::Status::make(api::Error::NotFound, "entry not found");
    if (snapshot_.tasks.empty()) return api::Status::make(api::Error::InvalidArgument, "no tasks");
    if (newIndex < 0 || newIndex > static_cast<int>(snapshot_.tasks.size())) return api::Status::make(api::Error::InvalidArgument, "invalid insertionIndex");
    if (from == newIndex) return api::Status::Ok();
    if (service_) {
        api::Status st = service_->reorder(id, newIndex, snapshot_.revision);
        if (!st.ok()) return st;
        snapshot_ = service_->snapshot();
        return api::Status::Ok();
    }
    api::TaskEntry e = snapshot_.tasks[static_cast<size_t>(from)];
    snapshot_.tasks.erase(snapshot_.tasks.begin() + from);
    int target = newIndex;
    if (from < target) --target;
    if (target < 0) target = 0;
    if (target > static_cast<int>(snapshot_.tasks.size())) target = static_cast<int>(snapshot_.tasks.size());
    snapshot_.tasks.insert(snapshot_.tasks.begin() + target, e);
    for (size_t i = 0; i < snapshot_.tasks.size(); ++i) snapshot_.tasks[i].orderIndex = static_cast<int>(i);
    // Keep pinnedApps order in sync when moving pinned entries.
    if (e.id.isPinned()) {
        std::vector<api::DesktopAppId> ordered;
        ordered.reserve(snapshot_.pinnedApps.size());
        for (size_t i = 0; i < snapshot_.tasks.size(); ++i) {
            if (snapshot_.tasks[i].id.isPinned()) {
                api::DesktopAppId app(snapshot_.tasks[i].id.inner());
                bool seen = false;
                for (size_t k = 0; k < ordered.size(); ++k) if (ordered[k] == app) { seen = true; break; }
                if (!seen) ordered.push_back(app);
            }
        }
        for (size_t i = 0; i < snapshot_.pinnedApps.size(); ++i) {
            bool found = false;
            for (size_t k = 0; k < ordered.size(); ++k) if (ordered[k] == snapshot_.pinnedApps[i]) { found = true; break; }
            if (!found) ordered.push_back(snapshot_.pinnedApps[i]);
        }
        snapshot_.pinnedApps = ordered;
    }
    bumpRevision();
    return api::Status::Ok();
}

void TaskModel::setOrder(const std::vector<api::TaskEntryId>& order) {
    if (order.size() != snapshot_.tasks.size()) return;
    // Validate all ids exist and unique.
    for (size_t i = 0; i < order.size(); ++i) {
        if (!order[i].valid()) return;
        if (taskIndex(order[i]) < 0) return;
        for (size_t j = 0; j < i; ++j) if (order[j] == order[i]) return;
    }
    std::vector<api::TaskEntry> reordered;
    reordered.reserve(order.size());
    for (size_t i = 0; i < order.size(); ++i) {
        int idx = taskIndex(order[i]);
        reordered.push_back(snapshot_.tasks[static_cast<size_t>(idx)]);
    }
    for (size_t i = 0; i < reordered.size(); ++i) reordered[i].orderIndex = static_cast<int>(i);
    snapshot_.tasks = reordered;
    // Sync pinnedApps to task order.
    std::vector<api::DesktopAppId> ordered;
    ordered.reserve(snapshot_.pinnedApps.size());
    for (size_t i = 0; i < snapshot_.tasks.size(); ++i) {
        if (snapshot_.tasks[i].id.isPinned()) {
            api::DesktopAppId app(snapshot_.tasks[i].id.inner());
            bool seen = false;
            for (size_t k = 0; k < ordered.size(); ++k) if (ordered[k] == app) { seen = true; break; }
            if (!seen) ordered.push_back(app);
        }
    }
    for (size_t i = 0; i < snapshot_.pinnedApps.size(); ++i) {
        bool found = false;
        for (size_t k = 0; k < ordered.size(); ++k) if (ordered[k] == snapshot_.pinnedApps[i]) { found = true; break; }
        if (!found) ordered.push_back(snapshot_.pinnedApps[i]);
    }
    snapshot_.pinnedApps = ordered;
    bumpRevision();
    if (service_) {
        service_->updateTasks(snapshot_.tasks);
        snapshot_ = service_->snapshot();
    }
}

api::Status TaskModel::bindWindow(const api::TaskEntryId& pinId, const api::WindowRef& win) {
    if (!pinId.valid() || !pinId.isPinned()) return api::Status::make(api::Error::InvalidArgument, "pinId must be pin: namespace");
    if (!win.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid window");
    int idx = taskIndex(pinId);
    if (idx < 0) return api::Status::make(api::Error::NotFound, "pin entry not found");
    api::TaskEntry& e = snapshot_.tasks[static_cast<size_t>(idx)];
    if (e.hasWindow) return api::Status::make(api::Error::Conflict, "already bound");
    // Check window not bound elsewhere
    for (size_t i = 0; i < snapshot_.tasks.size(); ++i) if (snapshot_.tasks[i].hasWindow && snapshot_.tasks[i].window == win) return api::Status::make(api::Error::Conflict, "window already bound");
    e.hasWindow = true;
    e.window = win;
    bumpRevision();
    if (service_) {
        service_->updateTasks(snapshot_.tasks);
        snapshot_ = service_->snapshot();
    }
    return api::Status::Ok();
}

api::Status TaskModel::unbindWindow(const api::TaskEntryId& id) {
    if (!id.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid id");
    int idx = taskIndex(id);
    if (idx < 0) return api::Status::make(api::Error::NotFound, "entry not found");
    api::TaskEntry& e = snapshot_.tasks[static_cast<size_t>(idx)];
    if (!e.hasWindow) return api::Status::make(api::Error::Conflict, "no window bound");
    // If pinned entry with window, unbinding leaves pinned launcher without window.
    e.hasWindow = false;
    e.window = api::WindowRef();
    bumpRevision();
    if (service_) {
        service_->updateTasks(snapshot_.tasks);
        snapshot_ = service_->snapshot();
    }
    return api::Status::Ok();
}

size_t TaskModel::size() const { return snapshot_.tasks.size(); }
bool TaskModel::empty() const { return snapshot_.tasks.empty(); }
void TaskModel::clear() { snapshot_ = api::PanelsSnapshot(); }
uint64_t TaskModel::revision() const { return snapshot_.revision; }

} // namespace shell
} // namespace flamewm
