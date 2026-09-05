#include "service.h"

#include <map>

namespace flamewm {
namespace platform {
namespace snap {

namespace {

static const int kEdgeThreshold = 16;
static const int kCornerThreshold = 32;
static const uint64_t kSideDwellMs = 400;

bool nearLeft(int x, const api::Rect& o) {
    return x >= o.x && x < o.x + kEdgeThreshold;
}
bool nearRight(int x, const api::Rect& o) {
    return x >= o.x + o.w - kEdgeThreshold && x < o.x + o.w;
}
bool nearTop(int y, const api::Rect& o) {
    return y >= o.y && y < o.y + kCornerThreshold;
}
bool nearBottom(int y, const api::Rect& o) {
    return y >= o.y + o.h - kCornerThreshold && y < o.y + o.h;
}

SnapTarget detectTarget(const api::Point& p, const api::Rect& out) {
    bool left = nearLeft(p.x, out);
    bool right = nearRight(p.x, out);
    bool top = nearTop(p.y, out);
    bool bottom = nearBottom(p.y, out);
    bool midX = !left && !right;

    if (left && top) return SnapTarget::TopLeftQuarter;
    if (right && top) return SnapTarget::TopRightQuarter;
    if (left && bottom) return SnapTarget::BottomLeftQuarter;
    if (right && bottom) return SnapTarget::BottomRightQuarter;
    if (top && midX) return SnapTarget::TopMaximize;
    if (bottom && midX) return SnapTarget::None;
    if (left) return SnapTarget::LeftHalf;
    if (right) return SnapTarget::RightHalf;
    return SnapTarget::None;
}

api::Rect geometryFor(SnapTarget t, const api::Rect& work) {
    int mx = work.x;
    int my = work.y;
    int Mx = work.x + work.w;
    int My = work.y + work.h;
    int W = Mx - mx;
    int H = My - my;
    api::Rect g;
    switch (t) {
        case SnapTarget::LeftHalf:
            g.x = mx; g.y = my; g.w = W / 2; g.h = H;
            break;
        case SnapTarget::RightHalf:
            g.x = mx + W / 2; g.y = my; g.w = W - W / 2; g.h = H;
            break;
        case SnapTarget::TopMaximize:
            g.x = mx; g.y = my; g.w = W; g.h = H;
            break;
        case SnapTarget::TopLeftQuarter:
            g.x = mx; g.y = my; g.w = W / 2; g.h = H / 2;
            break;
        case SnapTarget::TopRightQuarter:
            g.x = mx + W / 2; g.y = my; g.w = W - W / 2; g.h = H / 2;
            break;
        case SnapTarget::BottomLeftQuarter:
            g.x = mx; g.y = my + H / 2; g.w = W / 2; g.h = H - H / 2;
            break;
        case SnapTarget::BottomRightQuarter:
            g.x = mx + W / 2; g.y = my + H / 2; g.w = W - W / 2; g.h = H - H / 2;
            break;
        default:
            g.x = 0; g.y = 0; g.w = 0; g.h = 0;
            break;
    }
    return g;
}

} // namespace

struct SnapService::Impl {
    api::WindowPort* window;
    api::MainLoopPort* loop;
    std::map<api::WindowRef, SnapSession> sessions;
    api::WindowRef active;
    std::function<void(SnapTarget)> workspaceDwellCallback;

    Impl(api::WindowPort* w, api::MainLoopPort* l)
        : window(w), loop(l), active(), workspaceDwellCallback() {
        active.id = 0;
        active.generation = 0;
    }

    SnapSession* find(api::WindowRef win) {
        std::map<api::WindowRef, SnapSession>::iterator it = sessions.find(win);
        if (it == sessions.end()) return 0;
        return &it->second;
    }
    const SnapSession* findConst(api::WindowRef win) const {
        std::map<api::WindowRef, SnapSession>::const_iterator it = sessions.find(win);
        if (it == sessions.end()) return 0;
        return &it->second;
    }

    void disarm(SnapSession& s) {
        if (s.dwellArmed && loop && s.dwellHandle.id != 0) {
            loop->removeTimer(s.dwellHandle);
        }
        s.dwellArmed = false;
        s.dwellHandle.id = 0;
        s.dwellTarget = SnapTarget::None;
    }

    api::Status validateWindow(api::WindowRef win) const {
        if (!win.valid()) {
            return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
        }
        if (!window) {
            return api::Status::make(api::Error::Unavailable, "WindowPort unavailable");
        }
        api::Result<api::WindowSnapshot> cur = window->get(win);
        if (!cur.ok()) {
            return cur.status();
        }
        if (cur.value().ref.generation != win.generation) {
            return api::Status::make(api::Error::InvalidArgument, "stale WindowRef generation");
        }
        return api::Status::Ok();
    }
};

SnapService::SnapService(api::WindowPort* window, api::MainLoopPort* loop)
    : impl_(new Impl(window, loop)) {}

SnapService::~SnapService() {
    if (impl_) {
        // Disarm all timers before destruction — tolerate loop already gone.
        for (std::map<api::WindowRef, SnapSession>::iterator it = impl_->sessions.begin();
             it != impl_->sessions.end(); ++it) {
            impl_->disarm(it->second);
        }
        delete impl_;
        impl_ = 0;
    }
}

api::Status SnapService::moveBegin(api::WindowRef win, api::Rect floatingGeo) {
    if (!floatingGeo.valid()) {
        return api::Status::make(api::Error::InvalidArgument, "invalid floating geometry");
    }
    api::Status v = impl_->validateWindow(win);
    if (!v.ok()) return v;

    std::map<api::WindowRef, SnapSession>::iterator it = impl_->sessions.find(win);
    if (it != impl_->sessions.end()) {
        // Existing session — entering new drag; preserve snap state but reset preview/dwell.
        impl_->disarm(it->second);
        it->second.floatingGeometry = floatingGeo;
        // Keep unsnappedOuter/currentTarget/hasUnsnapped for restore/commit logic.
        it->second.previewTarget = SnapTarget::None;
        it->second.dragging = true;
        it->second.generation = win.generation;
    } else {
        SnapSession s;
        s.window = win;
        s.floatingGeometry = floatingGeo;
        s.unsnappedOuter = api::Rect();
        s.previewTarget = SnapTarget::None;
        s.currentTarget = SnapTarget::None;
        s.dragging = true;
        s.hasUnsnapped = false;
        s.generation = win.generation;
        s.dwellHandle.id = 0;
        s.dwellArmed = false;
        s.dwellTriggered = false;
        s.dwellTarget = SnapTarget::None;
        impl_->sessions[win] = s;
    }
    impl_->active = win;
    return api::Status::Ok();
}

api::Status SnapService::moveMotion(api::WindowRef win, api::Point pointer, api::Rect outputRect) {
    api::Status v = impl_->validateWindow(win);
    if (!v.ok()) return v;
    SnapSession* s = impl_->find(win);
    if (!s) {
        return api::Status::make(api::Error::NotFound, "no snap session for window");
    }
    if (!s->dragging) {
        return api::Status::make(api::Error::Conflict, "not dragging");
    }
    SnapTarget t = detectTarget(pointer, outputRect);
    s->previewTarget = t;
    impl_->active = win;

    // 400ms side-edge dwell via MainLoopPort — corners/maximize never dwell.
    bool isSide = (t == SnapTarget::LeftHalf || t == SnapTarget::RightHalf);
    if (isSide && impl_->loop) {
        if (s->dwellTarget != t)
            s->dwellTriggered = false;
        if (s->dwellTriggered && s->dwellTarget == t)
            return api::Status::Ok();
        if (s->dwellArmed && s->dwellTarget == t) {
            // Keep existing timer.
            return api::Status::Ok();
        }
        impl_->disarm(*s);
        s->dwellTarget = t;
        // Arm dwell timer. Workspace traversal owns dwell cancellation; SnapService must
        // retain candidate until that owner explicitly changes the motion target.
        api::WindowRef capturedWin = win;
        Impl* implPtr = impl_;
        api::MainLoopPort::TimerHandle h = impl_->loop->addTimer(kSideDwellMs, [implPtr, capturedWin]() {
            if (!implPtr) return;
            SnapSession* ss = implPtr->find(capturedWin);
            if (!ss) return;
            if (!ss->dragging) return;
            if (!ss->dwellArmed) return;
            ss->dwellArmed = false;
            ss->dwellHandle.id = 0;
            ss->dwellTriggered = true;
            if (implPtr->workspaceDwellCallback)
                implPtr->workspaceDwellCallback(ss->dwellTarget);
        }, false);
        s->dwellHandle = h;
        s->dwellArmed = true;
    } else {
        impl_->disarm(*s);
        s->dwellTriggered = false;
    }
    return api::Status::Ok();
}

void SnapService::setWorkspaceDwellCallback(
    const std::function<void(SnapTarget)>& callback) {
    impl_->workspaceDwellCallback = callback;
}

api::Result<api::Rect> SnapService::moveEnd(api::WindowRef win) {
    api::Status v = impl_->validateWindow(win);
    if (!v.ok()) return api::Result<api::Rect>::Err(v);
    SnapSession* s = impl_->find(win);
    if (!s) {
        return api::Result<api::Rect>::Err(api::Error::NotFound, "no snap session for window");
    }
    if (!s->dragging) {
        return api::Result<api::Rect>::Err(api::Error::Conflict, "not dragging");
    }
    impl_->disarm(*s);
    if (s->previewTarget == SnapTarget::None) {
        s->dragging = false;
        s->previewTarget = SnapTarget::None;
        return api::Result<api::Rect>::Err(api::Error::NotFound, "no snap target");
    }
    if (!impl_->window) {
        return api::Result<api::Rect>::Err(api::Error::Unavailable, "WindowPort unavailable");
    }
    api::Result<api::Rect> waRes = impl_->window->workArea(win);
    if (!waRes.ok()) {
        return api::Result<api::Rect>::Err(waRes.status());
    }
    api::Rect geo = geometryFor(s->previewTarget, waRes.value());
    if (!geo.valid()) {
        return api::Result<api::Rect>::Err(api::Error::InternalFailure, "invalid snap geometry");
    }
    // Capture unsnapped only on first floating->snap; snap->snap preserves.
    SnapTarget committed = s->previewTarget;
    bool wasSnapped = (s->currentTarget != SnapTarget::None);
    if (!wasSnapped && !s->hasUnsnapped) {
        s->unsnappedOuter = s->floatingGeometry;
        s->hasUnsnapped = true;
    }
    api::Status st = impl_->window->setOuterGeometry(win, geo);
    if (!st.ok()) {
        return api::Result<api::Rect>::Err(st);
    }
    s->currentTarget = committed;
    s->previewTarget = SnapTarget::None;
    s->dragging = false;
    impl_->active = win;
    return api::Result<api::Rect>::Ok(geo);
}

api::Status SnapService::moveCancel(api::WindowRef win) {
    api::Status v = impl_->validateWindow(win);
    if (!v.ok()) return v;
    SnapSession* s = impl_->find(win);
    if (!s) {
        return api::Status::make(api::Error::NotFound, "no snap session for window");
    }
    impl_->disarm(*s);
    s->previewTarget = SnapTarget::None;
    s->dragging = false;
    return api::Status::Ok();
}

api::Status SnapService::onManualResize(api::WindowRef win) {
    api::Status v = impl_->validateWindow(win);
    if (!v.ok()) return v;
    SnapSession* s = impl_->find(win);
    if (!s) {
        return api::Status::make(api::Error::NotFound, "no snap session for window");
    }
    impl_->disarm(*s);
    s->currentTarget = SnapTarget::None;
    s->hasUnsnapped = false;
    s->unsnappedOuter = api::Rect();
    s->previewTarget = SnapTarget::None;
    // dragging stays as-is; manual resize during drag is still exit.
    return api::Status::Ok();
}

api::Result<api::Rect> SnapService::dragAway(api::WindowRef win) {
    api::Status v = impl_->validateWindow(win);
    if (!v.ok()) return api::Result<api::Rect>::Err(v);
    SnapSession* s = impl_->find(win);
    if (!s) {
        return api::Result<api::Rect>::Err(api::Error::NotFound, "no snap session for window");
    }
    if (!s->hasUnsnapped || s->currentTarget == SnapTarget::None) {
        return api::Result<api::Rect>::Err(api::Error::NotFound, "not snapped");
    }
    if (!impl_->window) {
        return api::Result<api::Rect>::Err(api::Error::Unavailable, "WindowPort unavailable");
    }
    api::Result<api::Rect> waRes = impl_->window->workArea(win);
    if (!waRes.ok()) {
        return api::Result<api::Rect>::Err(waRes.status());
    }
    api::Rect clamped = clampToWorkArea(s->unsnappedOuter, waRes.value());
    if (!clamped.valid()) {
        return api::Result<api::Rect>::Err(api::Error::InternalFailure, "invalid restore geometry");
    }
    api::Status st = impl_->window->setOuterGeometry(win, clamped);
    if (!st.ok()) {
        return api::Result<api::Rect>::Err(st);
    }
    // Exit snap state after restore.
    impl_->disarm(*s);
    s->currentTarget = SnapTarget::None;
    s->hasUnsnapped = false;
    s->unsnappedOuter = api::Rect();
    s->previewTarget = SnapTarget::None;
    return api::Result<api::Rect>::Ok(clamped);
}

void SnapService::windowRemoved(api::WindowRef win) {
    // Remove all generations for this id. Disarm timers before erase.
    for (std::map<api::WindowRef, SnapSession>::iterator it = impl_->sessions.begin();
         it != impl_->sessions.end();) {
        if (it->first.id == win.id) {
            impl_->disarm(it->second);
            bool isActive = (impl_->active == it->first);
            it = impl_->sessions.erase(it);
            if (isActive) {
                impl_->active.id = 0;
                impl_->active.generation = 0;
            }
        } else {
            ++it;
        }
    }
    // If active still present but points to removed id, clear it.
    if (impl_->active.valid()) {
        if (impl_->sessions.find(impl_->active) == impl_->sessions.end()) {
            impl_->active.id = 0;
            impl_->active.generation = 0;
        }
    }
}

api::Rect SnapService::previewGeometry(api::Rect workArea, SnapTarget target) const {
    return geometryFor(target, workArea);
}

api::Result<api::Rect> SnapService::previewGeometryForWindow(api::WindowRef win, SnapTarget target) const {
    if (!win.valid()) {
        return api::Result<api::Rect>::Err(api::Error::InvalidArgument, "invalid WindowRef");
    }
    if (!impl_->window) {
        return api::Result<api::Rect>::Err(api::Error::Unavailable, "WindowPort unavailable");
    }
    api::Status v = impl_->validateWindow(win);
    if (!v.ok()) return api::Result<api::Rect>::Err(v);
    if (target == SnapTarget::None) {
        return api::Result<api::Rect>::Err(api::Error::InvalidArgument, "no snap target");
    }
    api::Result<api::Rect> waRes = impl_->window->workArea(win);
    if (!waRes.ok()) return api::Result<api::Rect>::Err(waRes.status());
    api::Rect g = geometryFor(target, waRes.value());
    if (!g.valid()) return api::Result<api::Rect>::Err(api::Error::InternalFailure, "invalid geometry");
    return api::Result<api::Rect>::Ok(g);
}

api::Status SnapService::commitForWindow(api::WindowRef win, SnapTarget target) {
    api::Result<api::Rect> g = previewGeometryForWindow(win, target);
    if (!g.ok()) return g.status();
    return impl_->window->setOuterGeometry(win, g.value());
}

bool SnapService::hasPreview(api::WindowRef win) const {
    const SnapSession* s = impl_->findConst(win);
    return s && s->previewTarget != SnapTarget::None;
}

SnapTarget SnapService::previewTarget(api::WindowRef win) const {
    const SnapSession* s = impl_->findConst(win);
    if (!s) return SnapTarget::None;
    return s->previewTarget;
}

bool SnapService::isSnapped(api::WindowRef win) const {
    const SnapSession* s = impl_->findConst(win);
    return s && s->currentTarget != SnapTarget::None;
}

SnapTarget SnapService::currentTarget(api::WindowRef win) const {
    const SnapSession* s = impl_->findConst(win);
    if (!s) return SnapTarget::None;
    return s->currentTarget;
}

bool SnapService::hasPreview() const {
    if (!impl_->active.valid()) return false;
    return hasPreview(impl_->active);
}

SnapTarget SnapService::previewTarget() const {
    if (!impl_->active.valid()) return SnapTarget::None;
    return previewTarget(impl_->active);
}

bool SnapService::shouldDwellSideEdge(SnapTarget target, api::Point pointer,
                                      api::Rect outputWorkArea, int elapsedMs) const {
    if (target != SnapTarget::LeftHalf && target != SnapTarget::RightHalf) return false;
    bool atEdge = false;
    if (target == SnapTarget::LeftHalf) atEdge = nearLeft(pointer.x, outputWorkArea);
    else atEdge = nearRight(pointer.x, outputWorkArea);
    if (!atEdge) return false;
    return elapsedMs >= static_cast<int>(kSideDwellMs);
}

api::Rect SnapService::clampToWorkArea(const api::Rect& rect, const api::Rect& workArea) {
    api::Rect out = rect;
    if (out.w > workArea.w) out.w = workArea.w;
    if (out.h > workArea.h) out.h = workArea.h;
    if (out.x < workArea.x) out.x = workArea.x;
    if (out.y < workArea.y) out.y = workArea.y;
    if (out.x + out.w > workArea.x + workArea.w)
        out.x = workArea.x + workArea.w - out.w;
    if (out.y + out.h > workArea.y + workArea.h)
        out.y = workArea.y + workArea.h - out.h;
    return out;
}

} // namespace snap
} // namespace platform
} // namespace flamewm
