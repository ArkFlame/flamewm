#include "controller.h"

namespace flamewm {
namespace snap {

bool SnapController::nearLeft(int x, const Rect& o) { return x >= o.x && x < o.x + kEdgeThreshold; }
bool SnapController::nearRight(int x, const Rect& o) { return x >= o.x + o.w - kEdgeThreshold && x < o.x + o.w; }
bool SnapController::nearTop(int y, const Rect& o) { return y >= o.y && y < o.y + kCornerThreshold; }
bool SnapController::nearBottom(int y, const Rect& o) { return y >= o.y + o.h - kCornerThreshold && y < o.y + o.h; }

SnapTarget SnapController::updateTarget(int rootX, int rootY, const Rect& outputRect) {
    bool left = nearLeft(rootX, outputRect);
    bool right = nearRight(rootX, outputRect);
    bool top = nearTop(rootY, outputRect);
    bool bottom = nearBottom(rootY, outputRect);

    // Bottom center -> none (no snap, preserves drag for workspace edge logic)
    // Use bottom stripe without left/right.
    bool midX = !left && !right;

    SnapTarget t = SnapNone;

    if (left && top) t = SnapTopLeft;
    else if (right && top) t = SnapTopRight;
    else if (left && bottom) t = SnapBottomLeft;
    else if (right && bottom) t = SnapBottomRight;
    else if (top && midX) {
        // Top center -> maximize (full work area)
        t = SnapMaximize;
    } else if (bottom && midX) {
        t = SnapNone;
    } else if (left) {
        t = SnapLeftHalf;
    } else if (right) {
        t = SnapRightHalf;
    } else {
        t = SnapNone;
    }
    lastTarget_ = t;
    return t;
}

Rect SnapController::previewGeometry(SnapTarget t, const Rect& workArea, int curW, int curH) const {
    return toRect(geometryFor(t, workArea, curW, curH));
}

Rect SnapController::commitGeometry(SnapTarget t, const Rect& workArea, int curW, int curH) const {
    // Identical to preview — must not diverge.
    return toRect(geometryFor(t, workArea, curW, curH));
}

bool SnapController::shouldDwellSideEdge(SnapTarget t, int rootX, const Rect& outputRect, int elapsedMs) const {
    if (t != SnapLeftHalf && t != SnapRightHalf) return false;
    // Must still be at extreme side edge.
    bool atEdge = (t == SnapLeftHalf && nearLeft(rootX, outputRect)) ||
                  (t == SnapRightHalf && nearRight(rootX, outputRect));
    if (!atEdge) return false;
    return elapsedMs >= kSideDwellMs;
}

} // namespace snap
} // namespace flamewm
