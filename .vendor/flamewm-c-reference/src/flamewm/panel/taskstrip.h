#ifndef FLAMEWM_PANEL_TASKSTRIP_H
#define FLAMEWM_PANEL_TASKSTRIP_H

#include "pinnedlauncher.h"
#include <string>
#include <vector>

class TaskPane;
class TaskButton;

namespace flamewm { namespace panel {

enum TaskEntryKind { TaskPinnedLauncher = 0, TaskWindow = 1 };

struct TaskStripEntry {
    TaskEntryKind kind;
    std::string appId;
    std::string windowId;
    TaskStripEntry() : kind(TaskPinnedLauncher) {}
    static TaskStripEntry launcher(const std::string& app) {
        TaskStripEntry e; e.kind = TaskPinnedLauncher; e.appId = app; return e;
    }
    static TaskStripEntry window(const std::string& app, const std::string& window) {
        TaskStripEntry e; e.kind = TaskWindow; e.appId = app; e.windowId = window; return e;
    }
};

class FlameTaskStrip {
public:
    void setPinned(const std::vector<std::string>& apps);
    void addWindow(const std::string& appId, const std::string& windowId);
    bool removeWindow(const std::string& windowId);
    bool pinWindow(const std::string& windowId);
    bool unpinWindow(const std::string& windowId);
    bool reorder(size_t from, size_t to);
    const std::vector<TaskStripEntry>& entries() const { return entries_; }
    bool isPinned(const std::string& appId) const;
    const std::vector<std::string>& pinned() const { return pinned_; }

    // Native TaskPane bridge: create/destroy PinnedLauncherButton instances for current pinned set.
    // Must be called on correct server thread when TaskPane exists. No second window registry.
    // Each launcher button is a real TaskPane child (compatible with TaskPane drag/reorder).
    void syncLaunchersToPane(TaskPane* pane);
    // Collect current launcher appIds directly from pane for verification / persistence.
    static std::vector<std::string> launcherIdsFromPane(TaskPane* pane);

private:
    size_t findWindow(const std::string& windowId) const;
    size_t findApp(const std::string& appId) const;
    std::vector<std::string> pinned_;
    std::vector<TaskStripEntry> entries_;
};

} }
#endif
