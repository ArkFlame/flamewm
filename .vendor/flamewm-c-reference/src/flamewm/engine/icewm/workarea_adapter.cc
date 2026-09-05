#include "config.h"
#include "base.h"

#include "flamewm/engine/icewm/workarea_adapter.h"

#include "flamewm/engine/icewm/access.h"

#if __has_include(<X11/Xlib.h>) && __has_include(<X11/extensions/Xrender.h>)
#include <X11/Xlib.h>
#define FLAMEWM_HAS_X11 1
#if __has_include("wmmgr.h")
#include "wmmgr.h"
#define FLAMEWM_HAS_WMGR 1
#else
#define FLAMEWM_HAS_WMGR 0
#endif
#if __has_include("ywindow.h")
#include "ywindow.h"
#define FLAMEWM_HAS_YWINDOW 1
#else
#define FLAMEWM_HAS_YWINDOW 0
#endif
#ifdef Status
#undef Status
#endif
#else
#define FLAMEWM_HAS_X11 0
#define FLAMEWM_HAS_WMGR 0
#define FLAMEWM_HAS_YWINDOW 0
#endif

#include <algorithm>

// Lightweight forward decls for Xrender-less host syntax check.
// When FLAMEWM_HAS_WMGR is 0 we avoid pulling IceWM headers but still
// compile; baseWorkAreas falls back to empty (no manager truth).
#if !FLAMEWM_HAS_WMGR && !FLAMEWM_HAS_YWINDOW
class YFrameWindow;
class YWindowManager {
public:
    void getWorkArea(int*, int*, int*, int*, int);
    void getWorkArea(const YFrameWindow*, int*, int*, int*, int, int = -1);
    int activeWorkspace() const;
    void updateWorkArea();
};
class YDesktop {
public:
    int getScreenCount() const;
    void getScreenGeometry(int*, int*, unsigned*, unsigned*, int = -1);
};
extern YDesktop* desktop;
#endif

namespace flamewm {
namespace engine {
namespace icewm {

struct WorkAreaAdapter::Impl {
    std::vector<std::pair<api::OutputId, api::Rect> > reservations;
    bool pendingRecompute;
    bool inRecompute;
    Impl() : pendingRecompute(false), inRecompute(false) {}
};

WorkAreaAdapter::WorkAreaAdapter() : impl_(new Impl()) {}
WorkAreaAdapter::~WorkAreaAdapter() { delete impl_; }

std::vector<api::Rect> WorkAreaAdapter::baseWorkAreas() {
#if FLAMEWM_HAS_X11 && FLAMEWM_HAS_WMGR
    YWindowManager* mgr = EngineAccess::managerTyped();
    if (mgr != NULL) {
        std::vector<api::Rect> out;
        // Prefer per-output work areas for the active workspace.
        // getWorkArea(frame=null, xiscreen=s) returns clamped work area for
        // that output, which is exactly the IceWM base before Flame struts.
        // Fallback to single-workspace query if screen count unavailable.
#if FLAMEWM_HAS_YWINDOW
        if (::desktop != NULL) {
            int n = ::desktop->getScreenCount();
            if (n > 0) {
                out.reserve(static_cast<size_t>(n));
                for (int s = 0; s < n; ++s) {
                    int mx = 0, my = 0, Mx = 0, My = 0;
                    mgr->getWorkArea(static_cast<const YFrameWindow*>(NULL), &mx, &my, &Mx, &My, s);
                    int w = Mx - mx;
                    int h = My - my;
                    if (w > 0 && h > 0)
                        out.push_back(api::Rect(mx, my, w, h));
                    else {
                        // Degenerate — fall back to screen geometry.
                        int sx = 0, sy = 0;
                        unsigned sw = 0, sh = 0;
                        ::desktop->getScreenGeometry(&sx, &sy, &sw, &sh, s);
                        out.push_back(api::Rect(sx, sy, static_cast<int>(sw), static_cast<int>(sh)));
                    }
                }
                return out;
            }
        }
#endif
        // No desktop/screen info — single work area for active workspace.
        {
            int mx = 0, my = 0, Mx = 0, My = 0;
            mgr->getWorkArea(&mx, &my, &Mx, &My, mgr->activeWorkspace());
            int w = Mx - mx;
            int h = My - my;
            if (w > 0 && h > 0)
                out.push_back(api::Rect(mx, my, w, h));
            return out;
        }
    }
#endif
    // No manager context (unit-test / vanilla build): no IceWM truth to
    // return. Returning empty preserves "source wins" — caller must handle
    // empty base and rely on cached reservations only when manager absent.
    // The effective work areas (base minus output-local struts) are
    // observable via the manager path; this fallback avoids fabricating
    // geometry that would diverge from IceWM.
    (void)impl_;
    return std::vector<api::Rect>();
}

void WorkAreaAdapter::applyFlameReservations(
    const std::vector<std::pair<api::OutputId, api::Rect> >& reservations) {
    // Normalize: keep only valid rects, output-local deduplication
    // (last wins per OutputId). This ensures each output's strut is
    // independent and a repeated call from PanelService does not accumulate
    // duplicates.
    std::vector<std::pair<api::OutputId, api::Rect> > norm;
    norm.reserve(reservations.size());
    for (size_t i = 0; i < reservations.size(); ++i) {
        const api::OutputId& oid = reservations[i].first;
        const api::Rect& rc = reservations[i].second;
        if (!oid.valid())
            continue;
        if (!rc.valid())
            continue;
        bool replaced = false;
        for (size_t k = 0; k < norm.size(); ++k) {
            if (norm[k].first == oid) {
                norm[k].second = rc;
                replaced = true;
                break;
            }
        }
        if (!replaced)
            norm.push_back(std::make_pair(oid, rc));
    }

    // No-op guard — identical reservations must not mark dirty or
    // trigger recomputation, preventing feedback loops where
    // panel configure -> strut -> workarea -> panel configure.
    if (norm.size() == impl_->reservations.size()) {
        bool same = true;
        for (size_t i = 0; i < norm.size(); ++i) {
            if (norm[i].first != impl_->reservations[i].first ||
                norm[i].second != impl_->reservations[i].second) {
                same = false;
                break;
            }
        }
        if (same)
            return;
    }

    impl_->reservations = norm;
    // Do not recompute here — caller must explicitly call requestRecompute().
    // This keeps applyFlameReservations caching-only and lets PanelService
    // batch apply + recompute as one transaction.
}

void WorkAreaAdapter::requestRecompute() {
    // Guard against infinite feedback: if IceWM's updateWorkArea triggers
    // Hooks::reserveWorkAreas which re-enters this adapter, coalesce into
    // a single pending flag instead of recursing.
    if (impl_->inRecompute) {
        impl_->pendingRecompute = true;
        return;
    }
    impl_->inRecompute = true;

#if FLAMEWM_HAS_X11 && FLAMEWM_HAS_WMGR
    YWindowManager* mgr = EngineAccess::managerTyped();
    if (mgr != NULL) {
        // Trigger IceWM work-area recomputation/EWMH _NET_WORKAREA update.
        // updateWorkArea() internally:
        //   - rebuilds fWorkArea from xiInfo
        //   - applies struts/doNotCover via updateWorkAreaInner
        //   - calls announceWorkArea() if changed (publishes _NET_WORKAREA)
        //   - resizes affected windows
        // When FLAMEWM product hooks are present, reserveWorkAreas will be
        // called inside updateWorkAreaInner to apply impl_->reservations
        // output-locally. Until then, this at least ensures EWMH stays in
        // sync with current IceWM truth.
        //
        // Use the public lock/unlock coalescing pattern if already locked
        // (updateWorkArea handles fWorkAreaLock/fWorkAreaUpdate internally).
        mgr->updateWorkArea();
    } else {
        // No manager — nothing to recompute; reservations are still cached
        // for the next manager attachment. Mark pending false.
        impl_->pendingRecompute = false;
    }
#else
    (void)impl_;
#endif

    impl_->inRecompute = false;
    // Coalesce one extra recompute if a re-entrant request arrived while
    // we were inside updateWorkArea. This is bounded to one extra pass
    // to avoid oscillation; if it re-sets pending again, it will be
    // handled on the next explicit requestRecompute call.
    if (impl_->pendingRecompute) {
        impl_->pendingRecompute = false;
#if FLAMEWM_HAS_X11 && FLAMEWM_HAS_WMGR
        YWindowManager* mgr2 = EngineAccess::managerTyped();
        if (mgr2 != NULL)
            mgr2->updateWorkArea();
#endif
    }
}

} // namespace icewm
} // namespace engine
} // namespace flamewm
