#ifndef FLAMEWM_SNAP_CONTROLLER_H
#define FLAMEWM_SNAP_CONTROLLER_H

#include "types.h"
#include "state.h"
#include <stdint.h>

namespace flamewm {
namespace snap {

// Pure logic snap controller — no X / IceWM calls.
// Insert comment pointing to integration hook:
//
// Integration proposal (no edit to movesize.cc/wmframe/wmmgr yet):
//   In YFrameWindow::handleMotion (movingWindow branch) before moveWindow,
//   query manager->getWorkArea(...) for workArea and desktop screen info
//   for outputRect, call controller.updateTarget(rootPointer, outputRect).
//   Branch on result for preview vs commit; preview and commit share
//   controller.previewGeometry/commitGeometry (identical call). Dwell
//   handling uses shouldDwellSideEdge and is gated on side targets only
//   — corners always snap per WINDOWS.md. All generation checks use
//   SnapState.frameGen. See FAILURES.md dwell/frame-lifetime rows.
//
class SnapController {
public:
    // thresholds for pointer zone detection (pixels from output edge)
    static const int kEdgeThreshold = 16;
    static const int kCornerThreshold = 32;
    // dwell before side-edge workspace switch while dragging (ms conceptual)
    static const int kSideDwellMs = 400;

    SnapController() : state_(0), lastTarget_(SnapNone), dwellMs_(0), generation_(0) {}
    explicit SnapController(uint64_t gen) : state_(gen), lastTarget_(SnapNone), dwellMs_(0), generation_(gen) {}

    SnapState& state() { return state_; }
    const SnapState& state() const { return state_; }

    uint64_t generation() const { return generation_; }
    void setGeneration(uint64_t g) { generation_ = g; state_.frameGen = g; }

    // Pure pointer -> SnapTarget mapping.
    // outputRect: physical output rect (RandR/Xinerama) for detection.
    // V4 targets:
    //   corners -> quarter, side edges -> half, top center -> maximize, bottom center -> none.
    SnapTarget updateTarget(int rootX, int rootY, const Rect& outputRect);

    // Preview and commit share identical geometry via geometryFor.
    // workArea: manager work area (strut-aware) for commit/preview.
    Rect previewGeometry(SnapTarget t, const Rect& workArea, int curW, int curH) const;
    Rect commitGeometry(SnapTarget t, const Rect& workArea, int curW, int curH) const;

    // Side-edge dwell: only for LeftHalf/RightHalf side targets.
    // Returns true when dwell threshold exceeded while pointer remains at
    // extreme output edge during active drag. Corners never trigger dwell.
    // elapsedMs: time pointer has dwelled at edge since entering side zone.
    bool shouldDwellSideEdge(SnapTarget t, int rootX, const Rect& outputRect, int elapsedMs) const;

    // Stale frame guard: callers capture generation before async/dwell callbacks.
    bool isStaleGeneration(uint64_t captured) const { return captured != generation_; }

    void resetDwell() { dwellMs_ = 0; }
    SnapTarget lastTarget() const { return lastTarget_; }

private:
    SnapState state_;
    SnapTarget lastTarget_;
    int dwellMs_;
    uint64_t generation_;

    static bool nearLeft(int x, const Rect& o);
    static bool nearRight(int x, const Rect& o);
    static bool nearTop(int y, const Rect& o);
    static bool nearBottom(int y, const Rect& o);
};

} // namespace snap
} // namespace flamewm
#endif
