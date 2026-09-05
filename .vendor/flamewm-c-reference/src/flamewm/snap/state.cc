#include "state.h"

namespace flamewm {
namespace snap {

void SnapState::onSnap(SnapTarget target, const Rect& curOuter) {
    if (target == SnapNone) return;
    if (!isSnapped()) {
        // First floating->snap: capture.
        if (!hasUnsnapped) {
            unsnappedOuter = curOuter;
            hasUnsnapped = true;
        }
        current = target;
    } else {
        // Snap->snap: preserve unsnappedOuter.
        onSnapToSnap(target);
    }
}

void SnapState::onSnapToSnap(SnapTarget target) {
    if (target == SnapNone) {
        // Should not happen in snap->snap but handle.
        current = SnapNone;
        return;
    }
    current = target;
}

void SnapState::onManualResize() {
    current = SnapNone;
    hasUnsnapped = false;
    unsnappedOuter = Rect();
}

Rect SnapState::onDragAway() const {
    if (!hasUnsnapped) return Rect();
    return unsnappedOuter;
}

Rect SnapState::consumeDragAway() {
    Rect r = onDragAway();
    onManualResize(); // exits snap state after restore
    return r;
}

Rect SnapState::clampToWorkArea(const Rect& rect, const Rect& workArea) {
    // Keep size, clamp origin so rect stays within workArea when possible.
    // If rect larger than workArea, pin to workArea origin.
    Rect out = rect;
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
} // namespace flamewm
