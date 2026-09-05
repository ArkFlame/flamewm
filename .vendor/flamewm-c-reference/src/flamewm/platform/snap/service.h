#ifndef FLAMEWM_PLATFORM_SNAP_SERVICE_H
#define FLAMEWM_PLATFORM_SNAP_SERVICE_H

#ifdef None
#pragma push_macro("None")
#undef None
#define FLAMEWM_SNAP_RN
#endif
#ifdef Status
#pragma push_macro("Status")
#undef Status
#define FLAMEWM_SNAP_RS
#endif

#include "flamewm/api/errors.h"
#include "flamewm/api/geometry.h"
#include "flamewm/api/ids.h"
#include "flamewm/api/ports.h"

#include <cstdint>
#include <functional>

namespace flamewm {
namespace platform {
namespace snap {

enum class SnapTarget {
    None = 0,
    LeftHalf,
    RightHalf,
    TopMaximize,
    TopLeftQuarter,
    TopRightQuarter,
    BottomLeftQuarter,
    BottomRightQuarter
};

struct SnapSession {
    api::WindowRef window;
    api::Rect floatingGeometry;
    api::Rect unsnappedOuter;
    SnapTarget previewTarget;
    SnapTarget currentTarget;
    bool dragging;
    bool hasUnsnapped;
    uint64_t generation;
    api::MainLoopPort::TimerHandle dwellHandle;
    bool dwellArmed;
    bool dwellTriggered;
    SnapTarget dwellTarget;

    SnapSession()
        : window()
        , floatingGeometry()
        , unsnappedOuter()
        , previewTarget(SnapTarget::None)
        , currentTarget(SnapTarget::None)
        , dragging(false)
        , hasUnsnapped(false)
        , generation(0)
        , dwellHandle()
        , dwellArmed(false)
        , dwellTriggered(false)
        , dwellTarget(SnapTarget::None) {
        dwellHandle.id = 0;
    }
};

class SnapService {
public:
    explicit SnapService(api::WindowPort* window, api::MainLoopPort* loop);
    ~SnapService();

    SnapService(const SnapService&) = delete;
    SnapService& operator=(const SnapService&) = delete;

    // Begin interactive move for window. Captures floating geometry for restore.
    api::Status moveBegin(api::WindowRef win, api::Rect floatingGeo);

    // Motion during drag. outputRect is physical output rect for edge detection.
    // Committed geometry uses workArea (via WindowPort::workArea) on moveEnd.
    api::Status moveMotion(api::WindowRef win, api::Point pointer, api::Rect outputRect);
    void setWorkspaceDwellCallback(const std::function<void(SnapTarget)>& callback);

    // End drag — commits preview if any via WindowPort::setOuterGeometry.
    // Returns committed geometry on snap, or NotFound if no preview.
    api::Result<api::Rect> moveEnd(api::WindowRef win);
    api::Status moveCancel(api::WindowRef win);

    // Manual resize exits snap state (clears hasUnsnapped/current).
    api::Status onManualResize(api::WindowRef win);

    // Drag-away restore: returns clamped unsnapped geometry and exits snap.
    api::Result<api::Rect> dragAway(api::WindowRef win);

    void windowRemoved(api::WindowRef win);

    // Pure geometry — preview==commit invariant.
    api::Rect previewGeometry(api::Rect workArea, SnapTarget target) const;
    api::Rect commitGeometry(api::Rect workArea, SnapTarget target) const {
        return previewGeometry(workArea, target);
    }

    // Port-delegating helpers — fetch work area via WindowPort.
    api::Result<api::Rect> previewGeometryForWindow(api::WindowRef win, SnapTarget target) const;
    api::Status commitForWindow(api::WindowRef win, SnapTarget target);

    bool hasPreview(api::WindowRef win) const;
    SnapTarget previewTarget(api::WindowRef win) const;
    bool isSnapped(api::WindowRef win) const;
    SnapTarget currentTarget(api::WindowRef win) const;

    // Back-compat active-window shims (operate on last active session, if any)
    bool hasPreview() const;
    SnapTarget previewTarget() const;

    // 400ms side-edge dwell — only LeftHalf/RightHalf, requires still at edge.
    bool shouldDwellSideEdge(SnapTarget target, api::Point pointer,
                             api::Rect outputWorkArea, int elapsedMs) const;

    static api::Rect clampToWorkArea(const api::Rect& rect, const api::Rect& workArea);

private:
    struct Impl;
    Impl* impl_;
};

} // namespace snap
} // namespace platform
} // namespace flamewm

#ifdef FLAMEWM_SNAP_RS
#pragma pop_macro("Status")
#undef FLAMEWM_SNAP_RS
#endif
#ifdef FLAMEWM_SNAP_RN
#pragma pop_macro("None")
#undef FLAMEWM_SNAP_RN
#endif

#endif // FLAMEWM_PLATFORM_SNAP_SERVICE_H
