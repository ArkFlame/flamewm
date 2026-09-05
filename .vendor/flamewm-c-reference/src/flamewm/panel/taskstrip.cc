#include "taskstrip.h"

#if !defined(TASKSTRIP_PURE_FALLBACK)
#if defined(__has_include)
#  if __has_include(<X11/extensions/Xrender.h>)
#    if __has_include("atasks.h")
#      include "atasks.h"
#    endif
#    if __has_include("pinnedlauncher.h")
#      include "pinnedlauncher.h"
#    endif
#  endif
#endif
#endif
#ifndef TASKSTRIP_PURE_FALLBACK
#include "pinnedlauncher.h"
#endif

namespace flamewm { namespace panel {

size_t FlameTaskStrip::findWindow(const std::string& id) const {
    for (size_t i = 0; i < entries_.size(); ++i)
        if (entries_[i].kind == TaskWindow && entries_[i].windowId == id) return i;
    return entries_.size();
}

size_t FlameTaskStrip::findApp(const std::string& id) const {
    for (size_t i = 0; i < pinned_.size(); ++i) if (pinned_[i] == id) return i;
    return pinned_.size();
}

bool FlameTaskStrip::isPinned(const std::string& id) const { return findApp(id) != pinned_.size(); }

void FlameTaskStrip::setPinned(const std::vector<std::string>& apps) {
    pinned_ = apps;
    for (size_t i = 0; i < pinned_.size(); ++i) {
        bool present = false;
        for (size_t j = 0; j < entries_.size(); ++j)
            if (entries_[j].appId == pinned_[i]) { present = true; break; }
        if (!present) entries_.push_back(TaskStripEntry::launcher(pinned_[i]));
    }
    for (size_t i = 0; i < entries_.size();) {
        if (entries_[i].kind == TaskPinnedLauncher && !isPinned(entries_[i].appId)) entries_.erase(entries_.begin() + i);
        else ++i;
    }
    for (size_t target = 0; target < pinned_.size(); ++target) {
        size_t pos = entries_.size();
        for (size_t i = target; i < entries_.size(); ++i) {
            if (isPinned(entries_[i].appId) && entries_[i].appId == pinned_[target]) { pos = i; break; }
        }
        if (pos == entries_.size() || pos == target) continue;
        TaskStripEntry entry = entries_[pos];
        entries_.erase(entries_.begin() + pos);
        entries_.insert(entries_.begin() + target, entry);
    }
}

void FlameTaskStrip::addWindow(const std::string& app, const std::string& window) {
    if (findWindow(window) != entries_.size()) return;
    size_t slot = entries_.size();
    for (size_t i = 0; i < entries_.size(); ++i)
        if (entries_[i].kind == TaskPinnedLauncher && entries_[i].appId == app) { slot = i; break; }
    TaskStripEntry e = TaskStripEntry::window(app, window);
    if (slot == entries_.size()) entries_.push_back(e);
    else entries_[slot] = e;
}

bool FlameTaskStrip::removeWindow(const std::string& window) {
    size_t pos = findWindow(window);
    if (pos == entries_.size()) return false;
    std::string app = entries_[pos].appId;
    entries_.erase(entries_.begin() + pos);
    if (isPinned(app)) {
        size_t survivor = entries_.size();
        for (size_t i = 0; i < entries_.size(); ++i)
            if (entries_[i].kind == TaskWindow && entries_[i].appId == app) { survivor = i; break; }
        if (survivor == entries_.size()) entries_.insert(entries_.begin() + pos, TaskStripEntry::launcher(app));
        else if (survivor != pos) {
            TaskStripEntry replacement = entries_[survivor];
            entries_.erase(entries_.begin() + survivor);
            size_t replacementPos = pos < entries_.size() ? pos : entries_.size();
            entries_.insert(entries_.begin() + replacementPos, replacement);
        }
    }
    return true;
}

bool FlameTaskStrip::pinWindow(const std::string& window) {
    size_t pos = findWindow(window);
    if (pos == entries_.size()) return false;
    std::string app = entries_[pos].appId;
    if (isPinned(app)) return true;
    pinned_.push_back(app);
    entries_.insert(entries_.begin() + pos, TaskStripEntry::launcher(app));
    return true;
}

bool FlameTaskStrip::unpinWindow(const std::string& window) {
    size_t pos = findWindow(window);
    if (pos == entries_.size() || !isPinned(entries_[pos].appId)) return false;
    std::string app = entries_[pos].appId;
    pinned_.erase(pinned_.begin() + findApp(app));
    return true;
}

bool FlameTaskStrip::reorder(size_t from, size_t to) {
    if (from >= entries_.size() || to >= entries_.size() || from == to) return false;
    TaskStripEntry e = entries_[from];
    entries_.erase(entries_.begin() + from);
    entries_.insert(entries_.begin() + to, e);
    if (isPinned(e.appId)) {
        std::vector<std::string> order;
        for (size_t i = 0; i < entries_.size(); ++i) {
            if (!isPinned(entries_[i].appId)) continue;
            bool seen = false;
            for (size_t j = 0; j < order.size(); ++j) if (order[j] == entries_[i].appId) { seen = true; break; }
            if (!seen) order.push_back(entries_[i].appId);
        }
        pinned_ = order;
    }
    return true;
}

#if defined(ATASKS_H_) && !defined(TASKSTRIP_PURE_FALLBACK)
std::vector<std::string> FlameTaskStrip::launcherIdsFromPane(TaskPane* pane) {
    std::vector<std::string> out;
    if (!pane) return out;
    int n = pane->buttonCount();
    for (int i = 0; i < n; ++i) {
        TaskButton* b = pane->buttonAt(i);
        if (!b) continue;
        flamewm::panel::PinnedLauncherButton* pl = dynamic_cast<flamewm::panel::PinnedLauncherButton*>(b);
        if (pl) out.push_back(pl->launcherAppId());
    }
    return out;
}
void FlameTaskStrip::syncLaunchersToPane(TaskPane* pane) {
    if (!pane) return;
    int n = pane->buttonCount();
    for (int i = n - 1; i >= 0; --i) {
        TaskButton* b = pane->buttonAt(i);
        flamewm::panel::PinnedLauncherButton* pl = dynamic_cast<flamewm::panel::PinnedLauncherButton*>(b);
        if (!pl) continue;
        const std::string& id = pl->launcherAppId();
        bool pinned = isPinned(id);
        if (!pinned) pane->remove(b);
    }
    std::vector<std::string> existing = launcherIdsFromPane(pane);
    auto hasExisting = [&](const std::string& id) { for (size_t k = 0; k < existing.size(); ++k) if (existing[k] == id) return true; return false; };
    for (size_t pi = 0; pi < pinned_.size(); ++pi) {
        const std::string& id = pinned_[pi];
        if (hasExisting(id)) continue;
        TaskButton* btn = createPinnedLauncherButton(pane, id, id);
        if (!btn) continue;
        size_t li = entries_.size();
        for (size_t ei = 0; ei < entries_.size(); ++ei) if (entries_[ei].kind == TaskPinnedLauncher && entries_[ei].appId == id) { li = ei; break; }
        int ins = (int)li; if (ins < 0) ins = 0; if (ins > pane->buttonCount()) ins = pane->buttonCount();
        pane->insertButtonAt(btn, ins);
        existing.push_back(id);
    }
    for (size_t pi = 0; pi < pinned_.size(); ++pi) {
        for (int i = 0; i < pane->buttonCount(); ++i) {
            PinnedLauncherButton* pl = dynamic_cast<PinnedLauncherButton*>(pane->buttonAt(i));
            if (!pl || pl->launcherAppId() != pinned_[pi]) continue;
            pl->setLauncherOrder((int)pi);
            size_t li = entries_.size();
            for (size_t ei = 0; ei < entries_.size(); ++ei)
                if (entries_[ei].kind == TaskPinnedLauncher && entries_[ei].appId == pinned_[pi]) { li = ei; break; }
            if (li != entries_.size()) pane->moveButtonTo(pl, (int)li);
            break;
        }
    }
    pane->relayout(true);
}
#else
std::vector<std::string> FlameTaskStrip::launcherIdsFromPane(TaskPane*) { return std::vector<std::string>(); }
void FlameTaskStrip::syncLaunchersToPane(TaskPane*) {}
#endif

} }
