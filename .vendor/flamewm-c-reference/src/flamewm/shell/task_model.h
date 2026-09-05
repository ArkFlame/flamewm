#ifndef FLAMEWM_SHELL_TASK_MODEL_H
#define FLAMEWM_SHELL_TASK_MODEL_H

#ifdef Status
#pragma push_macro("Status")
#undef Status
#define FLAMEWM_TASK_MODEL_RS
#endif

#include "flamewm/api/ids.h"
#include "flamewm/api/panels.h"
#include "flamewm/api/errors.h"

#include <string>
#include <vector>

namespace flamewm {
namespace platform { namespace panels { class PanelService; } }
namespace shell {

// TaskModel per C-PANEL-01:
// - pin:<app> and win:<id> are distinct namespaces (TaskEntryId).
// - Separate pin vs window identity: a pinned app persists without window;
//   a window task may be pinned or unpinned.
// - Session task order: stable orderIndex across restarts for pinned set.
class TaskModel {
public:
    TaskModel();
    ~TaskModel();

    TaskModel(const TaskModel&) = delete;
    TaskModel& operator=(const TaskModel&) = delete;

    // Full snapshot sync.
    void setSnapshot(const api::PanelsSnapshot& snap);
    api::PanelsSnapshot snapshot() const;

    // Pinned apps (DesktopAppId.value without prefix).
    std::vector<api::DesktopAppId> pinnedApps() const;
    bool isPinned(const api::DesktopAppId& app) const;
    api::Status pin(const api::DesktopAppId& app);
    api::Status unpin(const api::DesktopAppId& app);

    // Tasks in session order (pinned first by orderIndex, then windows).
    std::vector<api::TaskEntry> orderedTasks() const;
    const api::TaskEntry* find(const api::TaskEntryId& id) const;

    // Ordering mutation (session order persistence).
    api::Status move(const api::TaskEntryId& id, int newIndex);
    void setOrder(const std::vector<api::TaskEntryId>& order);

    // Window binding.
    api::Status bindWindow(const api::TaskEntryId& pinId, const api::WindowRef& win);
    api::Status unbindWindow(const api::TaskEntryId& id);

    size_t size() const;
    bool empty() const;
    void clear();

    uint64_t revision() const;

    void setPanelService(platform::panels::PanelService* svc);
    platform::panels::PanelService* panelService() const;

private:
    int taskIndex(const api::TaskEntryId& id) const;
    int pinnedIndex(const api::DesktopAppId& app) const;
    void bumpRevision();

    api::PanelsSnapshot snapshot_;
    platform::panels::PanelService* service_;
};

} // namespace shell
} // namespace flamewm

#endif // FLAMEWM_SHELL_TASK_MODEL_H

#ifdef FLAMEWM_TASK_MODEL_RS
#pragma pop_macro("Status")
#undef FLAMEWM_TASK_MODEL_RS
#endif
