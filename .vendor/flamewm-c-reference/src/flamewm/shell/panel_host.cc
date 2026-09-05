#include "flamewm/shell/panel_host.h"
#include "flamewm/shell/start_view.h"
#include "flamewm/shell/task_view.h"
#include "flamewm/shell/workspace_view.h"
#include "flamewm/shell/status_area.h"
#include "flamewm/shell/tray_host.h"
#include "flamewm/shell/clock_view.h"

#include <set>

namespace flamewm {
namespace shell {

struct PanelHost::Presentation {
    StartView* start;
    TaskView* tasks;
    WorkspaceView* pager;
    StatusArea* status;
    ClockView* clock;
    bool owned;

    Presentation()
        : start(0), tasks(0), pager(0), status(0), clock(0), owned(false) {}
};

PanelHost::PanelHost(api::WorkAreaPort* wa)
    : workArea_(wa)
    , tray_(0)
    , syncing_(false)
    , startOpen_(false)
    , startOutput_()
    , trayOwner_()
    , primaryOutput_()
    , requestingStruts_(false) {}

PanelHost::PanelHost(api::WorkAreaPort* wa, api::TrayPort* tray)
    : workArea_(wa)
    , tray_(tray)
    , syncing_(false)
    , startOpen_(false)
    , startOutput_()
    , trayOwner_()
    , primaryOutput_()
    , requestingStruts_(false) {}

PanelHost::~PanelHost() { clear(); }

void PanelHost::sync(const api::DisplaySnapshot& snapshot) {
    if (syncing_) return;
    syncing_ = true;

    // Build set of active (connected) output ids
    std::set<api::OutputId> active;
    for (size_t i = 0; i < snapshot.outputs.size(); ++i) {
        const api::OutputSnapshot& o = snapshot.outputs[i];
        if (!o.connected) continue;
        active.insert(o.id);
    }

    // Shared views follow the authoritative display primary, not map order.
    primaryOutput_ = api::OutputId();
    for (size_t i = 0; i < snapshot.outputs.size(); ++i) {
        const api::OutputSnapshot& o = snapshot.outputs[i];
        if (o.connected && o.primary) {
            primaryOutput_ = o.id;
            break;
        }
    }
    if (!primaryOutput_.valid()) {
        for (size_t i = 0; i < snapshot.outputs.size(); ++i) {
            if (snapshot.outputs[i].connected) {
                primaryOutput_ = snapshot.outputs[i].id;
                break;
            }
        }
    }

    // Remove panels whose output is no longer active
    std::vector<api::OutputId> toRemove;
    for (std::map<api::OutputId, PanelSurface*>::iterator it = surfaces_.begin();
         it != surfaces_.end(); ++it) {
        if (active.find(it->first) == active.end()) {
            toRemove.push_back(it->first);
        }
    }
    for (size_t i = 0; i < toRemove.size(); ++i) {
        std::map<api::OutputId, PanelSurface*>::iterator it = surfaces_.find(toRemove[i]);
        if (it != surfaces_.end()) {
            std::map<api::OutputId, Presentation*>::iterator pit = presentations_.find(toRemove[i]);
            if (pit != presentations_.end()) {
                Presentation* p = pit->second;
                if (p) {
                    if (p->owned) {
                        delete p->start;
                        delete p->tasks;
                        delete p->pager;
                        delete p->status;
                        delete p->clock;
                    }
                    delete p;
                }
                presentations_.erase(pit);
            }
            delete it->second;
            surfaces_.erase(it);
        }
    }

    if (trayOwner_.valid() && active.find(trayOwner_) == active.end())
        trayOwner_ = api::OutputId();

    // Close Start if its output vanished
    if (startOpen_ && !startOutput_.empty()) {
        if (active.find(startOutput_) == active.end()) {
            startOpen_ = false;
            startOutput_ = api::OutputId();
        }
    }

    // Create/update panels for active outputs
    for (size_t i = 0; i < snapshot.outputs.size(); ++i) {
        const api::OutputSnapshot& o = snapshot.outputs[i];
        if (!o.connected) continue;
        std::map<api::OutputId, PanelSurface*>::iterator it = surfaces_.find(o.id);
        if (it == surfaces_.end()) {
            PanelSurface* ps = new PanelSurface(o.id, api::PanelEdge::Bottom, o.geometry);
            ps->configure();
            ps->show();
            surfaces_[o.id] = ps;
        } else {
            // Update geometry if changed (handles mode/resolution change)
            if (it->second->outputGeometry() != o.geometry) {
                it->second->setOutputGeometry(o.geometry);
                it->second->configure();
            }
            if (!it->second->visible()) it->second->show();
        }
    }

    syncing_ = false;
}

void PanelHost::handleHotplug(const api::DisplaySnapshot& s) { sync(s); }

void PanelHost::applyPanelSnapshot(const api::PanelsSnapshot& snapshot) {
    if (syncing_) return;
    syncing_ = true;
    for (size_t i = 0; i < snapshot.panels.size(); ++i) {
        const api::PanelSnapshot& saved = snapshot.panels[i];
        PanelSurface* panel = panelFor(saved.output);
        if (!panel) continue;
        panel->setEdge(saved.edge);
        if (saved.logicalSize > 0) panel->setThickness(saved.logicalSize);
        if (saved.visible) {
            panel->configure();
            panel->show();
        } else {
            panel->hide();
        }
        std::map<api::OutputId, Presentation*>::iterator pit = presentations_.find(saved.output);
        Presentation* p = pit == presentations_.end() ? 0 : pit->second;
        if (p) {
            if (p->start) {
                p->start->setPanelEdge(panel->edge());
                p->start->setOutputRect(panel->outputGeometry());
            }
            if (p->tasks) p->tasks->setPanelEdge(panel->edge());
            if (p->pager) p->pager->setVertical(panel->isVertical());
        }
    }
    syncing_ = false;
    invalidateViews();
}

void PanelHost::bindViews(StartView* start, TaskView* tasks, WorkspaceView* pager,
                           StatusArea* status, TrayHost* tray, ClockView* clock,
                           api::MainLoopPort* loop,
                           platform::applications::ApplicationService* applications,
                           platform::windows::WindowService* windows,
                           platform::workspaces::WorkspaceService* workspaces,
                           platform::system::SystemService* system) {
    // The supplied views are the shared/controller presentation. Keep that
    // presentation stable; tray ownership is independent and may move.
    const api::OutputId primary = primaryOutput_;
    std::map<api::OutputId, Presentation*>::iterator sharedIt = presentations_.end();
    for (std::map<api::OutputId, Presentation*>::iterator it = presentations_.begin();
         it != presentations_.end(); ++it) {
        if (it->second && !it->second->owned) {
            sharedIt = it;
            break;
        }
    }
    if (sharedIt != presentations_.end() && primary.valid() &&
        sharedIt->first != primary) {
        Presentation*& target = presentations_[primary];
        if (!target) target = new Presentation();
        if (target->owned) {
            delete target->start;
            delete target->tasks;
            delete target->pager;
            delete target->status;
            delete target->clock;
        }
        Presentation* old = sharedIt->second;
        target->start = old->start;
        target->tasks = old->tasks;
        target->pager = old->pager;
        target->status = old->status;
        target->clock = old->clock;
        target->owned = false;
        old->start = new StartView();
        old->tasks = new TaskView(tasks ? tasks->model() : 0);
        old->pager = new WorkspaceView(0);
        old->status = new StatusArea();
        old->clock = new ClockView();
        old->owned = true;
    }
    for (std::map<api::OutputId, PanelSurface*>::iterator it = surfaces_.begin();
         it != surfaces_.end(); ++it) {
        Presentation*& p = presentations_[it->first];
        if (!p) p = new Presentation();
        if (!p->start) {
            p->start = it->first == primary ? start : new StartView();
            p->tasks = it->first == primary ? tasks : new TaskView(tasks ? tasks->model() : 0);
            p->pager = it->first == primary ? pager : new WorkspaceView(0);
            p->status = it->first == primary ? status : new StatusArea();
            p->clock = it->first == primary ? clock : new ClockView();
            p->owned = it->first != primary;
        }
        PanelSurface* panel = it->second;
        if (p->start) {
            p->start->setOutputId(it->first);
            p->start->setContainer(panel->container(PanelSurface::PanelSlot::Start));
            p->start->setApplicationService(applications);
            p->start->setPanelEdge(panel->edge());
            p->start->setOutputRect(panel->outputGeometry());
        }
        if (p->tasks) {
            p->tasks->setContainer(panel->container(PanelSurface::PanelSlot::Tasks));
            p->tasks->setWindowService(windows);
            p->tasks->setApplicationService(applications);
            p->tasks->setPanelEdge(panel->edge());
        }
        if (p->pager) {
            p->pager->setContainer(panel->container(PanelSurface::PanelSlot::WorkspacePager));
            p->pager->setWorkspaceService(workspaces);
            p->pager->setVertical(panel->isVertical());
            if (pager) p->pager->setSnapshot(pager->snapshot());
        }
        if (p->status) {
            p->status->setContainer(panel->container(PanelSurface::PanelSlot::MediaAudioNetwork));
            p->status->setSystemService(system);
            p->status->refreshFromService();
        }
        if (p->clock) {
            p->clock->setContainer(panel->container(PanelSurface::PanelSlot::Clock));
            p->clock->setMainLoop(loop);
            if (clock) {
                p->clock->setFormat(clock->format());
                p->clock->setDateFormat(clock->dateFormat());
            }
        }
    }
    if (tray) {
        trayOwner_ = tray->owner();
        PanelSurface* ownerPanel = panelFor(trayOwner_);
        tray->setContainer(ownerPanel ? ownerPanel->container(PanelSurface::PanelSlot::Tray) : 0);
    }
    invalidateViews();
}

void PanelHost::invalidateViews() {
    for (std::map<api::OutputId, Presentation*>::iterator it = presentations_.begin();
         it != presentations_.end(); ++it) {
        Presentation* p = it->second;
        if (!p) continue;
        if (p->start) p->start->syncListRows();
        if (p->tasks) p->tasks->render();
        if (p->pager) p->pager->render();
        if (p->status) p->status->render();
        if (p->clock) p->clock->render();
    }
}

void PanelHost::stopViewTimers() {
    for (std::map<api::OutputId, Presentation*>::iterator it = presentations_.begin();
         it != presentations_.end(); ++it)
        if (it->second && it->second->clock) it->second->clock->setMainLoop(0);
}

std::vector<std::pair<api::OutputId, api::Rect> > PanelHost::buildReservations() const {
    std::vector<std::pair<api::OutputId, api::Rect> > out;
    for (std::map<api::OutputId, PanelSurface*>::const_iterator it = surfaces_.begin();
         it != surfaces_.end(); ++it) {
        if (!it->second) continue;
        if (!it->second->visible()) continue;
        api::Rect r = it->second->strutRect();
        if (!r.valid()) continue;
        out.push_back(std::make_pair(it->first, r));
    }
    return out;
}

PanelSurface* PanelHost::panelFor(const api::OutputId& o) {
    std::map<api::OutputId, PanelSurface*>::iterator it = surfaces_.find(o);
    return it == surfaces_.end() ? 0 : it->second;
}
const PanelSurface* PanelHost::panelFor(const api::OutputId& o) const {
    std::map<api::OutputId, PanelSurface*>::const_iterator it = surfaces_.find(o);
    return it == surfaces_.end() ? 0 : it->second;
}

std::vector<PanelSurface*> PanelHost::panels() {
    std::vector<PanelSurface*> out;
    out.reserve(surfaces_.size());
    for (std::map<api::OutputId, PanelSurface*>::iterator it = surfaces_.begin(); it != surfaces_.end(); ++it)
        out.push_back(it->second);
    return out;
}
std::vector<const PanelSurface*> PanelHost::panels() const {
    std::vector<const PanelSurface*> out;
    out.reserve(surfaces_.size());
    for (std::map<api::OutputId, PanelSurface*>::const_iterator it = surfaces_.begin(); it != surfaces_.end(); ++it)
        out.push_back(it->second);
    return out;
}

size_t PanelHost::count() const { return surfaces_.size(); }
bool PanelHost::empty() const { return surfaces_.empty(); }

void PanelHost::clear() {
    for (std::map<api::OutputId, Presentation*>::iterator it = presentations_.begin(); it != presentations_.end(); ++it) {
        Presentation* p = it->second;
        if (!p) continue;
        if (p->owned) {
            delete p->start;
            delete p->tasks;
            delete p->pager;
            delete p->status;
            delete p->clock;
        }
        delete p;
    }
    presentations_.clear();
    for (std::map<api::OutputId, PanelSurface*>::iterator it = surfaces_.begin(); it != surfaces_.end(); ++it)
        delete it->second;
    surfaces_.clear();
    startOpen_ = false;
    startOutput_ = api::OutputId();
    trayOwner_ = api::OutputId();
    primaryOutput_ = api::OutputId();
}

void PanelHost::requestStruts() {
    // PanelService is the sole owner of work-area side effects.
}

api::WorkAreaPort* PanelHost::workArea() const { return workArea_; }
api::TrayPort* PanelHost::tray() const { return tray_; }
api::OutputId PanelHost::trayOwner() const { return trayOwner_; }

bool PanelHost::isStartOpen() const { return startOpen_; }
api::OutputId PanelHost::startOutput() const { return startOutput_; }

bool PanelHost::setStartOpen(bool open, const api::OutputId& output) {
    if (open) {
        if (!output.valid()) return false;
        if (surfaces_.find(output) == surfaces_.end()) return false;
        if (startOpen_ && startOutput_ == output) return true;
        startOpen_ = true;
        startOutput_ = output;
        for (std::map<api::OutputId, Presentation*>::iterator it = presentations_.begin();
             it != presentations_.end(); ++it) {
            Presentation* p = it->second;
            if (p && p->start) {
                if (it->first == output) p->start->open(output);
                else p->start->close();
            }
        }
        return true;
    } else {
        if (!startOpen_) return true;
        startOpen_ = false;
        startOutput_ = api::OutputId();
        for (std::map<api::OutputId, Presentation*>::iterator it = presentations_.begin();
             it != presentations_.end(); ++it)
            if (it->second && it->second->start) it->second->start->close();
        return true;
    }
}

bool PanelHost::toggleStart(const api::OutputId& output) {
    if (!output.valid()) return false;
    if (surfaces_.find(output) == surfaces_.end()) return false;
    if (startOpen_ && startOutput_ == output) {
        setStartOpen(false, output);
        return false;
    }
    setStartOpen(true, output);
    return true;
}

void PanelHost::closeStart() {
    setStartOpen(false, api::OutputId());
}

} // namespace shell
} // namespace flamewm
