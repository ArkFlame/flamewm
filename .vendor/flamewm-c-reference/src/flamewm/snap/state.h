#ifndef FLAMEWM_SNAP_STATE_H
#define FLAMEWM_SNAP_STATE_H

#include "types.h"
#include <stdint.h>

namespace flamewm {
namespace snap {

// Per-frame transient snap state.
// Owned by frame controller / YFrameWindow hook; not duplicated in IceWM.
//
// Rules (WINDOWS.md + FAILURES.md):
//  - Capture floating only on first floating->snap. Snap->snap preserves.
//  - Manual resize exits snap state (clears hasUnsnapped/current).
//  - Drag-away restores exact saved unsnappedOuter clamped to surviving output.
//  - Destruction invalidates (generation check).
//  - Generation guards stale callbacks/timers holding raw frame pointer.
struct SnapState {
    SnapTarget current;
    bool hasUnsnapped;
    Rect unsnappedOuter;
    uint64_t frameGen;

    SnapState() : current(SnapNone), hasUnsnapped(false), unsnappedOuter(), frameGen(0) {}
    explicit SnapState(uint64_t gen) : current(SnapNone), hasUnsnapped(false), unsnappedOuter(), frameGen(gen) {}

    bool isSnapped() const { return current != SnapNone; }

    // Invalidate on frame destruction: clears state and bumps generation externally.
    void invalidate() {
        current = SnapNone;
        hasUnsnapped = false;
        unsnappedOuter = Rect();
        frameGen++;
    }

    // Generation validation for callbacks.
    bool isValidGeneration(uint64_t gen) const { return gen == frameGen; }

    // Called when window transitions floating -> snap.
    // Captures unsnappedOuter only if not already snapped.
    // curOuter: current outer geometry before snap.
    void onSnap(SnapTarget target, const Rect& curOuter);

    // Called when snapping from one snap target to another (snap->snap).
    // Preserves unsnappedOuter, only updates current.
    void onSnapToSnap(SnapTarget target);

    // Called on manual resize (user resize handle or keyboard resize).
    // Exits snap state: clears current and unsnapped.
    void onManualResize();

    // Called on drag-away from snapped state: returns geometry to restore.
    // Caller clamps result to surviving output work area.
    // After restore, exits snap state.
    Rect onDragAway() const;

    // Consume drag-away: retrieve and exit.
    Rect consumeDragAway();

    // Clamp rect to stay within bounds (work area of surviving output).
    // Pure helper, no IceWM dependency.
    static Rect clampToWorkArea(const Rect& rect, const Rect& workArea);
};

} // namespace snap
} // namespace flamewm
#endif
