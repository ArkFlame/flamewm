#include "flamewm/engine/icewm/window_adapter.h"

#include "flamewm/api/errors.h"
#include "flamewm/engine/icewm/access.h"

#include <cstdint>
#include <map>
#include <set>
#include <string>
#include <vector>

#ifdef HAVE_X11
#include <X11/Xlib.h>
#if __has_include(<X11/extensions/Xrender.h>)
#define FLAMEWM_HAS_X11 1
#if __has_include("wmframe.h")
#include "wmframe.h"
#define FLAMEWM_HAS_WMFRAME 1
#else
#define FLAMEWM_HAS_WMFRAME 0
#endif
#if __has_include("wmmgr.h")
#include "wmmgr.h"
#define FLAMEWM_HAS_WMMGR 1
#else
#define FLAMEWM_HAS_WMMGR 0
#endif
#if __has_include("WinMgr.h")
#include "WinMgr.h"
#endif
#if __has_include("ywindow.h")
#include "ywindow.h"
#endif
#if __has_include("mstring.h")
#include "mstring.h"
#endif
#ifdef Status
#undef Status
#endif
#else
#define FLAMEWM_HAS_X11 0
#define FLAMEWM_HAS_WMFRAME 0
#define FLAMEWM_HAS_WMMGR 0
#endif
#else
#define FLAMEWM_HAS_X11 0
#define FLAMEWM_HAS_WMFRAME 0
#define FLAMEWM_HAS_WMMGR 0
#endif

namespace flamewm {
namespace engine {
namespace icewm {

struct WindowAdapter::Impl {
    std::map<uint64_t, uint64_t> genById;
    std::set<uint64_t> prevLive;
};

WindowAdapter* WindowAdapter::active_ = nullptr;

WindowAdapter::WindowAdapter() : impl_(new Impl()) { active_ = this; }
WindowAdapter::~WindowAdapter() {
    if (active_ == this)
        active_ = nullptr;
    delete impl_;
}

#if FLAMEWM_HAS_X11 && FLAMEWM_HAS_WMFRAME && FLAMEWM_HAS_WMMGR

namespace {

static uint64_t rawIdForFrame(YFrameWindow* f) {
    if (f == nullptr)
        return 0;
    YFrameClient* c = f->client();
    Window w = c != nullptr ? c->handle() : None;
    if (w == None) {
        w = f->handle();
        if (w == None)
            return 0;
    }
    return static_cast<uint64_t>(static_cast<unsigned long>(w));
}

static void syncGenerations(WindowAdapter::Impl* impl, YWindowManager* mgr) {
    if (impl == nullptr || mgr == nullptr)
        return;
    std::set<uint64_t> curLive;
    for (int layer = 0; layer < WinLayerCount; ++layer) {
        for (YFrameWindow* f = mgr->top(layer); f != nullptr; f = f->next()) {
            uint64_t id = rawIdForFrame(f);
            if (id == 0)
                continue;
            curLive.insert(id);
            std::map<uint64_t, uint64_t>::iterator it = impl->genById.find(id);
            if (it == impl->genById.end()) {
                impl->genById[id] = 1;
            }
        }
    }
    if (curLive.empty()) {
        YFrameIter it = mgr->focusedIterator();
        while (++it) {
            YFrameWindow* f = it;
            if (f == nullptr)
                continue;
            uint64_t id = rawIdForFrame(f);
            if (id == 0)
                continue;
            curLive.insert(id);
            if (impl->genById.find(id) == impl->genById.end()) {
                impl->genById[id] = 1;
            }
        }
    }
    for (std::set<uint64_t>::const_iterator it = impl->prevLive.begin();
         it != impl->prevLive.end(); ++it) {
        uint64_t id = *it;
        if (curLive.find(id) == curLive.end()) {
            std::map<uint64_t, uint64_t>::iterator git = impl->genById.find(id);
            if (git != impl->genById.end()) {
                git->second += 1;
            } else {
                impl->genById[id] = 1;
            }
        }
    }
    impl->prevLive = curLive;
}

static api::WindowSnapshot toSnapshot(YFrameWindow* f, uint64_t id, uint64_t generation, uint64_t focusedId) {
    api::WindowSnapshot s;
    s.ref = api::WindowRef(id, generation);
    s.stateGeneration = generation;
    if (f->getTitle().c_str() != nullptr) {
        s.title = std::string(f->getTitle().c_str());
    } else {
        s.title = std::string();
    }
    YFrameClient* c = f->client();
    if (c != nullptr && c->classHint() != nullptr) {
        ClassHint* ch = c->classHint();
        if (ch->res_class != nullptr && ch->res_class[0] != '\0') {
            s.appId = std::string(ch->res_class);
        } else if (ch->res_name != nullptr && ch->res_name[0] != '\0') {
            s.appId = std::string(ch->res_name);
        } else {
            s.appId = std::string();
        }
    } else {
        s.appId = std::string();
    }
    YRect g = f->geometry();
    s.outerGeometry = api::Rect(g.x(), g.y(), static_cast<int>(g.width()), static_cast<int>(g.height()));
    s.restoreGeometry = s.outerGeometry;
    s.minimized = f->isMinimized();
    s.maximized = f->isMaximized();
    s.fullscreen = f->isFullscreen();
    s.sticky = f->isSticky();
    s.focused = id == focusedId && f->visible();
    int ws = f->getWorkspace();
    s.workspace = api::WorkspaceRef(ws, 0);
    int screen = f->getScreen();
    if (screen >= 0) {
        s.output = api::OutputId(std::to_string(screen));
    } else {
        s.output = api::OutputId();
    }
    return s;
}

static YFrameWindow* resolveFrame(YWindowManager* mgr, uint64_t id) {
    if (mgr == nullptr || id == 0)
        return nullptr;
    Window w = static_cast<Window>(static_cast<unsigned long>(id));
    return mgr->findFrame(w);
}

} // namespace

api::WindowRef WindowAdapter::currentRef(uint64_t id) {
    if (active_ == nullptr || id == 0)
        return api::WindowRef();
    YWindowManager* mgr = EngineAccess::managerTyped();
    if (mgr == nullptr)
        return api::WindowRef();
    syncGenerations(active_->impl_, mgr);
    std::map<uint64_t, uint64_t>::const_iterator it = active_->impl_->genById.find(id);
    if (it == active_->impl_->genById.end() || resolveFrame(mgr, id) == nullptr)
        return api::WindowRef();
    return api::WindowRef(id, it->second);
}

api::Result<api::WindowSnapshot> WindowAdapter::get(api::WindowRef ref) {
    if (!ref.valid()) {
        return api::Result<api::WindowSnapshot>::Err(api::Error::InvalidArgument, "invalid WindowRef");
    }
    YWindowManager* mgr = EngineAccess::managerTyped();
    if (mgr == nullptr) {
        return api::Result<api::WindowSnapshot>::Err(api::Error::Unavailable, "WindowManager unavailable");
    }
    syncGenerations(impl_, mgr);
    std::map<uint64_t, uint64_t>::const_iterator git = impl_->genById.find(ref.id);
    if (git == impl_->genById.end()) {
        return api::Result<api::WindowSnapshot>::Err(api::Error::NotFound, "unknown WindowRef id");
    }
    if (git->second != ref.generation) {
        return api::Result<api::WindowSnapshot>::Err(api::Error::StaleRevision, "stale WindowRef generation");
    }
    YFrameWindow* f = resolveFrame(mgr, ref.id);
    if (f == nullptr) {
        return api::Result<api::WindowSnapshot>::Err(api::Error::NotFound, "window not found");
    }
    uint64_t curId = rawIdForFrame(f);
    if (curId != ref.id) {
        return api::Result<api::WindowSnapshot>::Err(api::Error::NotFound, "window id mismatch");
    }
    api::WindowSnapshot s = toSnapshot(f, ref.id, git->second, rawIdForFrame(mgr->getFocus()));
    return api::Result<api::WindowSnapshot>::Ok(s);
}

std::vector<api::WindowSnapshot> WindowAdapter::snapshot() {
    std::vector<api::WindowSnapshot> out;
    YWindowManager* mgr = EngineAccess::managerTyped();
    if (mgr == nullptr)
        return out;
    syncGenerations(impl_, mgr);
    uint64_t focusedId = rawIdForFrame(mgr->getFocus());
    for (int layer = 0; layer < WinLayerCount; ++layer) {
        for (YFrameWindow* f = mgr->top(layer); f != nullptr; f = f->next()) {
            uint64_t id = rawIdForFrame(f);
            if (id == 0)
                continue;
            std::map<uint64_t, uint64_t>::const_iterator git = impl_->genById.find(id);
            if (git == impl_->genById.end())
                continue;
            out.push_back(toSnapshot(f, id, git->second, focusedId));
        }
    }
    if (out.empty()) {
        YFrameIter it = mgr->focusedIterator();
        while (++it) {
            YFrameWindow* f = it;
            if (f == nullptr)
                continue;
            uint64_t id = rawIdForFrame(f);
            if (id == 0)
                continue;
            bool already = false;
            for (std::size_t i = 0; i < out.size(); ++i) {
                if (out[i].ref.id == id) { already = true; break; }
            }
            if (already)
                continue;
            std::map<uint64_t, uint64_t>::const_iterator git = impl_->genById.find(id);
            if (git == impl_->genById.end())
                continue;
            out.push_back(toSnapshot(f, id, git->second, focusedId));
        }
    }
    return out;
}

api::Status WindowAdapter::activate(api::WindowRef ref) {
    if (!ref.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
    YWindowManager* mgr = EngineAccess::managerTyped();
    if (mgr == nullptr)
        return api::Status::make(api::Error::Unavailable, "WindowManager unavailable");
    syncGenerations(impl_, mgr);
    std::map<uint64_t, uint64_t>::const_iterator git = impl_->genById.find(ref.id);
    if (git == impl_->genById.end())
        return api::Status::make(api::Error::NotFound, "unknown WindowRef id");
    if (git->second != ref.generation)
        return api::Status::make(api::Error::StaleRevision, "stale WindowRef generation");
    YFrameWindow* f = resolveFrame(mgr, ref.id);
    if (f == nullptr)
        return api::Status::make(api::Error::NotFound, "window not found");
    f->activateWindow(true, true);
    return api::Status::Ok();
}

api::Status WindowAdapter::minimize(api::WindowRef ref) {
    if (!ref.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
    YWindowManager* mgr = EngineAccess::managerTyped();
    if (mgr == nullptr)
        return api::Status::make(api::Error::Unavailable, "WindowManager unavailable");
    syncGenerations(impl_, mgr);
    std::map<uint64_t, uint64_t>::const_iterator git = impl_->genById.find(ref.id);
    if (git == impl_->genById.end())
        return api::Status::make(api::Error::NotFound, "unknown WindowRef id");
    if (git->second != ref.generation)
        return api::Status::make(api::Error::StaleRevision, "stale WindowRef generation");
    YFrameWindow* f = resolveFrame(mgr, ref.id);
    if (f == nullptr)
        return api::Status::make(api::Error::NotFound, "window not found");
    if (!f->isMinimized()) {
        f->wmMinimize();
    }
    return api::Status::Ok();
}

api::Status WindowAdapter::maximize(api::WindowRef ref) {
    if (!ref.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
    YWindowManager* mgr = EngineAccess::managerTyped();
    if (mgr == nullptr)
        return api::Status::make(api::Error::Unavailable, "WindowManager unavailable");
    syncGenerations(impl_, mgr);
    std::map<uint64_t, uint64_t>::const_iterator git = impl_->genById.find(ref.id);
    if (git == impl_->genById.end())
        return api::Status::make(api::Error::NotFound, "unknown WindowRef id");
    if (git->second != ref.generation)
        return api::Status::make(api::Error::StaleRevision, "stale WindowRef generation");
    YFrameWindow* f = resolveFrame(mgr, ref.id);
    if (f == nullptr)
        return api::Status::make(api::Error::NotFound, "window not found");
    if (!f->isMaximized()) {
        f->wmMaximize();
    }
    return api::Status::Ok();
}

api::Status WindowAdapter::restore(api::WindowRef ref) {
    if (!ref.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
    YWindowManager* mgr = EngineAccess::managerTyped();
    if (mgr == nullptr)
        return api::Status::make(api::Error::Unavailable, "WindowManager unavailable");
    syncGenerations(impl_, mgr);
    std::map<uint64_t, uint64_t>::const_iterator git = impl_->genById.find(ref.id);
    if (git == impl_->genById.end())
        return api::Status::make(api::Error::NotFound, "unknown WindowRef id");
    if (git->second != ref.generation)
        return api::Status::make(api::Error::StaleRevision, "stale WindowRef generation");
    YFrameWindow* f = resolveFrame(mgr, ref.id);
    if (f == nullptr)
        return api::Status::make(api::Error::NotFound, "window not found");
    if (f->isMinimized()) {
        f->wmMinimize();
    } else if (f->isMaximized() || f->isFullscreen()) {
        f->wmRestore();
    }
    return api::Status::Ok();
}

api::Status WindowAdapter::close(api::WindowRef ref) {
    if (!ref.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
    YWindowManager* mgr = EngineAccess::managerTyped();
    if (mgr == nullptr)
        return api::Status::make(api::Error::Unavailable, "WindowManager unavailable");
    syncGenerations(impl_, mgr);
    std::map<uint64_t, uint64_t>::const_iterator git = impl_->genById.find(ref.id);
    if (git == impl_->genById.end())
        return api::Status::make(api::Error::NotFound, "unknown WindowRef id");
    if (git->second != ref.generation)
        return api::Status::make(api::Error::StaleRevision, "stale WindowRef generation");
    YFrameWindow* f = resolveFrame(mgr, ref.id);
    if (f == nullptr)
        return api::Status::make(api::Error::NotFound, "window not found");
    f->wmClose();
    return api::Status::Ok();
}

api::Status WindowAdapter::setOuterGeometry(api::WindowRef ref, api::Rect rect) {
    if (!ref.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
    if (!rect.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid geometry");
    YWindowManager* mgr = EngineAccess::managerTyped();
    if (mgr == nullptr)
        return api::Status::make(api::Error::Unavailable, "WindowManager unavailable");
    syncGenerations(impl_, mgr);
    std::map<uint64_t, uint64_t>::const_iterator git = impl_->genById.find(ref.id);
    if (git == impl_->genById.end())
        return api::Status::make(api::Error::NotFound, "unknown WindowRef id");
    if (git->second != ref.generation)
        return api::Status::make(api::Error::StaleRevision, "stale WindowRef generation");
    YFrameWindow* f = resolveFrame(mgr, ref.id);
    if (f == nullptr)
        return api::Status::make(api::Error::NotFound, "window not found");
    f->setCurrentGeometryOuter(YRect(rect.x, rect.y, static_cast<unsigned>(rect.w), static_cast<unsigned>(rect.h)));
    return api::Status::Ok();
}

api::Result<api::Rect> WindowAdapter::workArea(api::WindowRef ref) {
    if (!ref.valid())
        return api::Result<api::Rect>::Err(api::Error::InvalidArgument, "invalid WindowRef");
    YWindowManager* mgr = EngineAccess::managerTyped();
    if (mgr == nullptr)
        return api::Result<api::Rect>::Err(api::Error::Unavailable, "WindowManager unavailable");
    syncGenerations(impl_, mgr);
    std::map<uint64_t, uint64_t>::const_iterator git = impl_->genById.find(ref.id);
    if (git == impl_->genById.end())
        return api::Result<api::Rect>::Err(api::Error::NotFound, "unknown WindowRef id");
    if (git->second != ref.generation)
        return api::Result<api::Rect>::Err(api::Error::StaleRevision, "stale WindowRef generation");
    YFrameWindow* f = resolveFrame(mgr, ref.id);
    if (f == nullptr)
        return api::Result<api::Rect>::Err(api::Error::NotFound, "window not found");
    int mx = 0;
    int my = 0;
    int Mx = 0;
    int My = 0;
    int screen = f->getScreen();
    mgr->getWorkArea(f, &mx, &my, &Mx, &My, screen);
    api::Rect r(mx, my, Mx - mx, My - my);
    return api::Result<api::Rect>::Ok(r);
}

api::Result<api::OutputId> WindowAdapter::output(api::WindowRef ref) {
    if (!ref.valid())
        return api::Result<api::OutputId>::Err(api::Error::InvalidArgument, "invalid WindowRef");
    YWindowManager* mgr = EngineAccess::managerTyped();
    if (mgr == nullptr)
        return api::Result<api::OutputId>::Err(api::Error::Unavailable, "WindowManager unavailable");
    syncGenerations(impl_, mgr);
    std::map<uint64_t, uint64_t>::const_iterator git = impl_->genById.find(ref.id);
    if (git == impl_->genById.end())
        return api::Result<api::OutputId>::Err(api::Error::NotFound, "unknown WindowRef id");
    if (git->second != ref.generation)
        return api::Result<api::OutputId>::Err(api::Error::StaleRevision, "stale WindowRef generation");
    YFrameWindow* f = resolveFrame(mgr, ref.id);
    if (f == nullptr)
        return api::Result<api::OutputId>::Err(api::Error::NotFound, "window not found");
    int screen = f->getScreen();
    if (screen < 0)
        return api::Result<api::OutputId>::Err(api::Error::NotFound, "no output for window");
    api::OutputId out(std::to_string(screen));
    return api::Result<api::OutputId>::Ok(out);
}

#else // !FLAMEWM_HAS_X11

api::WindowRef WindowAdapter::currentRef(uint64_t) {
    return api::WindowRef();
}

api::Result<api::WindowSnapshot> WindowAdapter::get(api::WindowRef ref) {
    if (!ref.valid()) {
        return api::Result<api::WindowSnapshot>::Err(api::Error::InvalidArgument, "invalid WindowRef");
    }
    (void)impl_;
    return api::Result<api::WindowSnapshot>::Err(api::Error::Unavailable, "WindowManager unavailable (no X11)");
}

std::vector<api::WindowSnapshot> WindowAdapter::snapshot() {
    return std::vector<api::WindowSnapshot>();
}

api::Status WindowAdapter::activate(api::WindowRef ref) {
    if (!ref.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
    return api::Status::make(api::Error::Unavailable, "WindowManager unavailable (no X11)");
}

api::Status WindowAdapter::minimize(api::WindowRef ref) {
    if (!ref.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
    return api::Status::make(api::Error::Unavailable, "WindowManager unavailable (no X11)");
}

api::Status WindowAdapter::maximize(api::WindowRef ref) {
    if (!ref.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
    return api::Status::make(api::Error::Unavailable, "WindowManager unavailable (no X11)");
}

api::Status WindowAdapter::restore(api::WindowRef ref) {
    if (!ref.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
    return api::Status::make(api::Error::Unavailable, "WindowManager unavailable (no X11)");
}

api::Status WindowAdapter::close(api::WindowRef ref) {
    if (!ref.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
    return api::Status::make(api::Error::Unavailable, "WindowManager unavailable (no X11)");
}

api::Status WindowAdapter::setOuterGeometry(api::WindowRef ref, api::Rect rect) {
    if (!ref.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid WindowRef");
    if (!rect.valid())
        return api::Status::make(api::Error::InvalidArgument, "invalid geometry");
    return api::Status::make(api::Error::Unavailable, "WindowManager unavailable (no X11)");
}

api::Result<api::Rect> WindowAdapter::workArea(api::WindowRef ref) {
    if (!ref.valid())
        return api::Result<api::Rect>::Err(api::Error::InvalidArgument, "invalid WindowRef");
    return api::Result<api::Rect>::Err(api::Error::Unavailable, "WindowManager unavailable (no X11)");
}

api::Result<api::OutputId> WindowAdapter::output(api::WindowRef ref) {
    if (!ref.valid())
        return api::Result<api::OutputId>::Err(api::Error::InvalidArgument, "invalid WindowRef");
    return api::Result<api::OutputId>::Err(api::Error::Unavailable, "WindowManager unavailable (no X11)");
}

#endif

} // namespace icewm
} // namespace engine
} // namespace flamewm
