#include "flamewm/platform/panels/service.h"

#include <algorithm>
#include <map>
#include <utility>
#include <vector>

namespace flamewm {
namespace platform {
namespace panels {

struct PanelService::Impl {
    api::WorkAreaPort* workArea;
    api::TrayPort* tray;

    uint64_t revision;
    std::vector<api::PanelSnapshot> panels;
    std::vector<api::DesktopAppId> pinnedApps;
    std::vector<api::TaskEntry> tasks;
    bool startOpen;
    api::OutputId startOutput;
    api::OutputId trayOwner;

    // Snapshots consumed explicitly (no direct IceWM)
    api::DisplaySnapshot displaySnap;
    std::vector<api::WindowSnapshot> windowSnaps;
    api::SettingsSnapshot settingsSnap;

    int nextListenerId;
    std::map<int, std::function<void(uint64_t)> > listeners;

    explicit Impl(api::WorkAreaPort* wa, api::TrayPort* t)
        : workArea(wa)
        , tray(t)
        , revision(1)
        , panels()
        , pinnedApps()
        , tasks()
        , startOpen(false)
        , startOutput()
        , trayOwner()
        , displaySnap()
        , windowSnaps()
        , settingsSnap()
        , nextListenerId(1) {}

    void bump() { ++revision; }

    void notify() {
        std::map<int, std::function<void(uint64_t)> > copy = listeners;
        for (std::map<int, std::function<void(uint64_t)> >::iterator it = copy.begin(); it != copy.end(); ++it) {
            if (it->second) it->second(revision);
        }
    }

    api::PanelSnapshot* findPanel(const api::OutputId& output) {
        for (size_t i = 0; i < panels.size(); ++i) {
            if (panels[i].output == output) return &panels[i];
        }
        return 0;
    }

    const api::PanelSnapshot* findPanelConst(const api::OutputId& output) const {
        for (size_t i = 0; i < panels.size(); ++i) {
            if (panels[i].output == output) return &panels[i];
        }
        return 0;
    }

    bool pinnedContains(const api::DesktopAppId& app) const {
        for (size_t i = 0; i < pinnedApps.size(); ++i) {
            if (pinnedApps[i] == app) return true;
        }
        return false;
    }

    int pinnedIndex(const api::DesktopAppId& app) const {
        for (size_t i = 0; i < pinnedApps.size(); ++i) {
            if (pinnedApps[i] == app) return static_cast<int>(i);
        }
        return -1;
    }

    int taskIndex(const api::TaskEntryId& id) const {
        for (size_t i = 0; i < tasks.size(); ++i) {
            if (tasks[i].id == id) return static_cast<int>(i);
        }
        return -1;
    }

    bool outputIsActive(const api::OutputId& id) const {
        for (size_t i = 0; i < displaySnap.outputs.size(); ++i) {
            if (displaySnap.outputs[i].id == id && displaySnap.outputs[i].connected) return true;
        }
        return false;
    }

    const api::OutputSnapshot* findDisplayOutput(const api::OutputId& id) const {
        for (size_t i = 0; i < displaySnap.outputs.size(); ++i) {
            if (displaySnap.outputs[i].id == id) return &displaySnap.outputs[i];
        }
        return 0;
    }

    // Compute strut rect for a panel given its output geometry and edge/size.
    // Thickness == logicalSize (logical pixels; scale omitted for reservation shape).
    static api::Rect strutFor(const api::OutputSnapshot& out, api::PanelEdge edge, int logicalSize) {
        const api::Rect& g = out.geometry;
        if (!g.valid()) return api::Rect();
        switch (edge) {
            case api::PanelEdge::Bottom: return api::Rect(g.x, g.y + g.h - logicalSize, g.w, logicalSize);
            case api::PanelEdge::Top:    return api::Rect(g.x, g.y, g.w, logicalSize);
            case api::PanelEdge::Left:   return api::Rect(g.x, g.y, logicalSize, g.h);
            case api::PanelEdge::Right:  return api::Rect(g.x + g.w - logicalSize, g.y, logicalSize, g.h);
            default: return api::Rect();
        }
    }

    std::vector<std::pair<api::OutputId, api::Rect> > buildReservations() const {
        std::vector<std::pair<api::OutputId, api::Rect> > out;
        out.reserve(panels.size());
        for (size_t i = 0; i < panels.size(); ++i) {
            const api::PanelSnapshot& p = panels[i];
            if (!p.visible) continue;
            const api::OutputSnapshot* ds = findDisplayOutput(p.output);
            if (!ds) continue;
            api::Rect r = strutFor(*ds, p.edge, p.logicalSize);
            if (r.valid()) out.push_back(std::make_pair(p.output, r));
        }
        return out;
    }

    // Sync panels to active outputs. Returns true if model changed.
    bool syncPanelsToDisplay() {
        bool changed = false;
        // Remove panels whose output no longer active
        for (size_t i = 0; i < panels.size(); ) {
            if (!outputIsActive(panels[i].output)) {
                const api::OutputId removedOutput = panels[i].output;
                panels.erase(panels.begin() + static_cast<int>(i));
                changed = true;
                // If start was open on removed output, close it
                if (startOpen && startOutput == removedOutput) {
                    // will handle below
                }
            } else {
                ++i;
            }
        }
        // Close start if its output vanished
        if (startOpen && !outputIsActive(startOutput)) {
            startOpen = false;
            startOutput = api::OutputId();
            changed = true;
        }
        // Ensure one panel per active output
        for (size_t i = 0; i < displaySnap.outputs.size(); ++i) {
            const api::OutputSnapshot& o = displaySnap.outputs[i];
            if (!o.connected) continue;
            if (!findPanelConst(o.id)) {
                api::PanelSnapshot np;
                np.output = o.id;
                np.edge = api::PanelEdge::Bottom;
                np.logicalSize = 44;
                np.visible = true;
                np.geometry = strutFor(o, np.edge, np.logicalSize);
                panels.push_back(np);
                changed = true;
            } else {
                api::PanelSnapshot* existing = findPanel(o.id);
                api::Rect geometry = strutFor(o, existing->edge, existing->logicalSize);
                if (existing->geometry != geometry) {
                    existing->geometry = geometry;
                    changed = true;
                }
            }
        }
        // If tray owner invalid or not active, pick new owner (primary or first active)
        api::OutputId desiredTray;
        if (!trayOwner.valid() || !outputIsActive(trayOwner)) {
            // prefer primary
            for (size_t i = 0; i < displaySnap.outputs.size(); ++i) {
                if (displaySnap.outputs[i].connected && displaySnap.outputs[i].primary) {
                    desiredTray = displaySnap.outputs[i].id;
                    break;
                }
            }
            if (!desiredTray.valid()) {
                for (size_t i = 0; i < displaySnap.outputs.size(); ++i) {
                    if (displaySnap.outputs[i].connected) { desiredTray = displaySnap.outputs[i].id; break; }
                }
            }
            if (desiredTray.valid() && desiredTray != trayOwner) {
                // effect via TrayPort — only publish after success
                if (tray) {
                    api::Status st = tray->setOwner(desiredTray);
                    if (!st.ok()) {
                        // do not update model if port rejected
                    } else {
                        trayOwner = desiredTray;
                        changed = true;
                    }
                } else {
                    trayOwner = desiredTray;
                    changed = true;
                }
            } else if (!desiredTray.valid() && trayOwner.valid()) {
                // no active outputs => clear tray owner
                if (tray) {
                    api::Status st = tray->setOwner(api::OutputId());
                    if (st.ok()) {
                        trayOwner = api::OutputId();
                        changed = true;
                    }
                } else {
                    trayOwner = api::OutputId();
                    changed = true;
                }
            }
        }
        // Push workarea reservations if changed or topology changed
        if (changed && workArea) {
            std::vector<std::pair<api::OutputId, api::Rect> > res = buildReservations();
            workArea->applyFlameReservations(res);
            workArea->requestRecompute();
        }
        return changed;
    }

    // Apply reservations for current panels (after edge/size change)
    api::Status applyWorkAreaReservations() {
        if (!workArea) return api::Status::Ok();
        std::vector<std::pair<api::OutputId, api::Rect> > res = buildReservations();
        // WorkAreaPort mutators are void in current api; treat as always succeeding.
        // If a future impl returns Status and fails, caller should not bump.
        workArea->applyFlameReservations(res);
        workArea->requestRecompute();
        return api::Status::Ok();
    }

    api::Status applyTrayOwner(const api::OutputId& desired) {
        if (!tray) return api::Status::Ok();
        api::Status st = tray->setOwner(desired);
        return st;
    }
};

PanelService::PanelService(api::WorkAreaPort* workArea, api::TrayPort* tray)
    : impl_(new Impl(workArea, tray)) {}
PanelService::~PanelService() { delete impl_; }

api::PanelsSnapshot PanelService::snapshot() const {
    api::PanelsSnapshot s;
    s.revision = impl_->revision;
    s.panels = impl_->panels;
    s.tasks = impl_->tasks;
    s.pinnedApps = impl_->pinnedApps;
    s.trayOwner = impl_->trayOwner;
    s.startOpen = impl_->startOpen;
    s.startOutput = impl_->startOutput;
    return s;
}

uint64_t PanelService::revision() const {
    return impl_->revision;
}

void PanelService::onDisplaySnapshot(const api::DisplaySnapshot& snap) {
    impl_->displaySnap = snap;
    bool changed = impl_->syncPanelsToDisplay();
    if (changed) {
        impl_->bump();
        impl_->notify();
    }
}

void PanelService::onWindowSnapshots(const std::vector<api::WindowSnapshot>& windows) {
    impl_->windowSnaps = windows;
    // No auto-bump; tasks reconciled via updateTasks from window layer.
}

void PanelService::onSettingsSnapshot(const api::SettingsSnapshot& snap) {
    impl_->settingsSnap = snap;
    // Settings may carry panel.pinnedApps or panel.* keys; for now just store.
    // If settings affect edge/size, callers should issue SetEdge/SetSize via ports.
}

void PanelService::clearSubscriptions() {
    impl_->listeners.clear();
}

api::Status PanelService::setEdge(const api::OutputId& output, api::PanelEdge edge, uint64_t expectedRevision) {
    if (!output.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid output");
    int ei = static_cast<int>(edge);
    if (ei < 0 || ei > 3) return api::Status::make(api::Error::InvalidArgument, "invalid edge");
    if (expectedRevision != impl_->revision) return api::Status::make(api::Error::StaleRevision, "stale revision");
    if (!impl_->outputIsActive(output) && impl_->displaySnap.outputs.size() != 0) {
        // If we have display topology, output must be active
        if (!impl_->findPanelConst(output)) {
            return api::Status::make(api::Error::NotFound, "output not found");
        }
    }
    api::PanelSnapshot* p = impl_->findPanel(output);
    if (!p) {
        // No display topology yet — allow creating panel for edge command
        if (impl_->displaySnap.outputs.empty()) {
            api::PanelSnapshot np;
            np.output = output;
            np.edge = edge;
            np.logicalSize = 44;
            np.visible = true;
            np.geometry = api::Rect();
            // validate+effect via WorkAreaPort before publishing
            // Temporarily push to compute reservation
            impl_->panels.push_back(np);
            api::Status st = impl_->applyWorkAreaReservations();
            if (!st.ok()) {
                impl_->panels.pop_back();
                return st;
            }
            impl_->bump();
            impl_->notify();
            return api::Status::Ok();
        }
        return api::Status::make(api::Error::NotFound, "panel not found");
    }
    if (p->edge == edge) return api::Status::Ok();
    api::PanelEdge old = p->edge;
    api::Rect oldGeometry = p->geometry;
    p->edge = edge;
    const api::OutputSnapshot* display = impl_->findDisplayOutput(output);
    if (display) p->geometry = impl_->strutFor(*display, p->edge, p->logicalSize);
    api::Status st = impl_->applyWorkAreaReservations();
    if (!st.ok()) {
        p->edge = old;
        p->geometry = oldGeometry;
        return st;
    }
    impl_->bump();
    impl_->notify();
    return api::Status::Ok();
}

api::Status PanelService::setSize(const api::OutputId& output, int logicalSize, uint64_t expectedRevision) {
    if (!output.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid output");
    if (logicalSize < 34 || logicalSize > 72) return api::Status::make(api::Error::InvalidArgument, "logicalSize must be 34..72");
    if (expectedRevision != impl_->revision) return api::Status::make(api::Error::StaleRevision, "stale revision");
    if (!impl_->outputIsActive(output) && impl_->displaySnap.outputs.size() != 0) {
        if (!impl_->findPanelConst(output)) {
            return api::Status::make(api::Error::NotFound, "output not found");
        }
    }
    api::PanelSnapshot* p = impl_->findPanel(output);
    if (!p) {
        if (impl_->displaySnap.outputs.empty()) {
            api::PanelSnapshot np;
            np.output = output;
            np.edge = api::PanelEdge::Bottom;
            np.logicalSize = logicalSize;
            np.visible = true;
            np.geometry = api::Rect();
            impl_->panels.push_back(np);
            api::Status st = impl_->applyWorkAreaReservations();
            if (!st.ok()) {
                impl_->panels.pop_back();
                return st;
            }
            impl_->bump();
            impl_->notify();
            return api::Status::Ok();
        }
        return api::Status::make(api::Error::NotFound, "panel not found");
    }
    if (p->logicalSize == logicalSize) return api::Status::Ok();
    int old = p->logicalSize;
    api::Rect oldGeometry = p->geometry;
    p->logicalSize = logicalSize;
    const api::OutputSnapshot* display = impl_->findDisplayOutput(output);
    if (display) p->geometry = impl_->strutFor(*display, p->edge, p->logicalSize);
    api::Status st = impl_->applyWorkAreaReservations();
    if (!st.ok()) {
        p->logicalSize = old;
        p->geometry = oldGeometry;
        return st;
    }
    impl_->bump();
    impl_->notify();
    return api::Status::Ok();
}

api::Status PanelService::pin(const api::DesktopAppId& app, uint64_t expectedRevision) {
    if (!app.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid app");
    if (expectedRevision != impl_->revision) return api::Status::make(api::Error::StaleRevision, "stale revision");
    if (impl_->pinnedContains(app)) return api::Status::make(api::Error::Conflict, "already pinned");
    impl_->pinnedApps.push_back(app);
    impl_->bump();
    impl_->notify();
    return api::Status::Ok();
}

api::Status PanelService::unpin(const api::DesktopAppId& app, uint64_t expectedRevision) {
    if (!app.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid app");
    if (expectedRevision != impl_->revision) return api::Status::make(api::Error::StaleRevision, "stale revision");
    int idx = impl_->pinnedIndex(app);
    if (idx < 0) return api::Status::make(api::Error::NotFound, "not pinned");
    impl_->pinnedApps.erase(impl_->pinnedApps.begin() + idx);
    impl_->bump();
    impl_->notify();
    return api::Status::Ok();
}

api::Status PanelService::reorder(const api::TaskEntryId& entryId, int insertionIndex, uint64_t expectedRevision) {
    if (!entryId.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid entryId");
    if (expectedRevision != impl_->revision) return api::Status::make(api::Error::StaleRevision, "stale revision");
    if (impl_->tasks.empty()) return api::Status::make(api::Error::InvalidArgument, "invalid insertionIndex");
    if (insertionIndex < 0 || insertionIndex > static_cast<int>(impl_->tasks.size())) {
        return api::Status::make(api::Error::InvalidArgument, "invalid insertionIndex");
    }
    if (insertionIndex == static_cast<int>(impl_->tasks.size())) {
        insertionIndex = static_cast<int>(impl_->tasks.size()) - 1;
    }
    int from = impl_->taskIndex(entryId);
    if (from < 0) return api::Status::make(api::Error::NotFound, "entry not found");

    if (from == insertionIndex) return api::Status::Ok();

    api::TaskEntry entry = impl_->tasks[static_cast<size_t>(from)];
    impl_->tasks.erase(impl_->tasks.begin() + from);
    int target = insertionIndex;
    if (from < target) target -= 1;
    if (target < 0) target = 0;
    if (target > static_cast<int>(impl_->tasks.size())) target = static_cast<int>(impl_->tasks.size());
    impl_->tasks.insert(impl_->tasks.begin() + target, entry);

    for (size_t i = 0; i < impl_->tasks.size(); ++i) impl_->tasks[i].orderIndex = static_cast<int>(i);

    if (entry.id.isPinned()) {
        std::vector<api::DesktopAppId> ordered;
        ordered.reserve(impl_->pinnedApps.size());
        for (size_t i = 0; i < impl_->tasks.size(); ++i) {
            if (impl_->tasks[i].id.isPinned()) {
                api::DesktopAppId app(impl_->tasks[i].id.inner());
                bool seen = false;
                for (size_t k = 0; k < ordered.size(); ++k) if (ordered[k] == app) { seen = true; break; }
                if (!seen) ordered.push_back(app);
            }
        }
        for (size_t i = 0; i < impl_->pinnedApps.size(); ++i) {
            bool found = false;
            for (size_t k = 0; k < ordered.size(); ++k) if (ordered[k] == impl_->pinnedApps[i]) { found = true; break; }
            if (!found) ordered.push_back(impl_->pinnedApps[i]);
        }
        impl_->pinnedApps = ordered;
    }

    impl_->bump();
    impl_->notify();
    return api::Status::Ok();
}

void PanelService::updateTasks(const std::vector<api::TaskEntry>& tasks) {
    std::map<api::TaskEntryId, int> prevOrder;
    for (size_t i = 0; i < impl_->tasks.size(); ++i) {
        prevOrder[impl_->tasks[i].id] = static_cast<int>(i);
    }

    std::vector<api::TaskEntry> next = tasks;
    std::vector<api::TaskEntry> withPrev;
    std::vector<api::TaskEntry> withoutPrev;
    withPrev.reserve(next.size());
    withoutPrev.reserve(next.size());
    for (size_t i = 0; i < next.size(); ++i) {
        if (prevOrder.find(next[i].id) != prevOrder.end()) withPrev.push_back(next[i]);
        else withoutPrev.push_back(next[i]);
    }
    std::sort(withPrev.begin(), withPrev.end(), [&prevOrder](const api::TaskEntry& a, const api::TaskEntry& b) {
        return prevOrder[a.id] < prevOrder[b.id];
    });
    std::vector<api::TaskEntry> composed;
    composed.reserve(next.size());
    for (size_t i = 0; i < withPrev.size(); ++i) composed.push_back(withPrev[i]);
    for (size_t i = 0; i < withoutPrev.size(); ++i) composed.push_back(withoutPrev[i]);

    std::vector<api::DesktopAppId> missing;
    for (size_t i = 0; i < impl_->pinnedApps.size(); ++i) {
        bool found = false;
        for (size_t j = 0; j < composed.size(); ++j) {
            if (composed[j].appId == impl_->pinnedApps[i]) { found = true; break; }
        }
        if (!found) missing.push_back(impl_->pinnedApps[i]);
    }
    for (size_t i = 0; i < missing.size(); ++i) {
        api::TaskEntry pinEntry;
        pinEntry.id = api::TaskEntryId::pinned(missing[i].value);
        pinEntry.isPinned = true;
        pinEntry.hasWindow = false;
        pinEntry.appId = missing[i];
        pinEntry.orderIndex = 0;
        int pinnedPos = -1;
        for (size_t k = 0; k < impl_->pinnedApps.size(); ++k) if (impl_->pinnedApps[k] == missing[i]) { pinnedPos = static_cast<int>(k); break; }
        int insertAt = 0;
        for (int k = 0; k < pinnedPos; ++k) {
            for (size_t j = 0; j < composed.size(); ++j) if (composed[j].appId == impl_->pinnedApps[static_cast<size_t>(k)]) { insertAt++; break; }
        }
        for (size_t k = 0; k < i; ++k) insertAt++;
        if (insertAt < 0) insertAt = 0;
        if (insertAt > static_cast<int>(composed.size())) insertAt = static_cast<int>(composed.size());
        composed.insert(composed.begin() + insertAt, pinEntry);
    }

    for (size_t i = 0; i < composed.size(); ++i) composed[i].orderIndex = static_cast<int>(i);

    bool changed = false;
    if (composed.size() != impl_->tasks.size()) changed = true;
    else {
        for (size_t i = 0; i < composed.size(); ++i) {
            if (composed[i].id != impl_->tasks[i].id ||
                composed[i].appId != impl_->tasks[i].appId ||
                composed[i].isPinned != impl_->tasks[i].isPinned ||
                composed[i].hasWindow != impl_->tasks[i].hasWindow ||
                composed[i].window != impl_->tasks[i].window) { changed = true; break; }
        }
    }
    if (!changed) return;
    impl_->tasks = composed;
    impl_->bump();
    impl_->notify();
}

api::Status PanelService::setStartOpen(bool open, const api::OutputId& output, uint64_t expectedRevision) {
    if (expectedRevision != impl_->revision) return api::Status::make(api::Error::StaleRevision, "stale revision");
    if (open) {
        if (!output.valid()) return api::Status::make(api::Error::InvalidArgument, "invalid output");
        if (!impl_->outputIsActive(output) && !impl_->displaySnap.outputs.empty()) {
            return api::Status::make(api::Error::NotFound, "output not found");
        }
        if (impl_->startOpen && impl_->startOutput == output) return api::Status::Ok();
        impl_->startOpen = true;
        impl_->startOutput = output;
        impl_->bump();
        impl_->notify();
        return api::Status::Ok();
    } else {
        if (!impl_->startOpen) return api::Status::Ok();
        impl_->startOpen = false;
        impl_->startOutput = api::OutputId();
        impl_->bump();
        impl_->notify();
        return api::Status::Ok();
    }
}

int PanelService::addListener(std::function<void(uint64_t)> cb) {
    if (!cb) return 0;
    int id = impl_->nextListenerId++;
    impl_->listeners[id] = cb;
    return id;
}

void PanelService::removeListener(int listenerId) {
    std::map<int, std::function<void(uint64_t)> >::iterator it = impl_->listeners.find(listenerId);
    if (it != impl_->listeners.end()) impl_->listeners.erase(it);
}

} // namespace panels
} // namespace platform
} // namespace flamewm
