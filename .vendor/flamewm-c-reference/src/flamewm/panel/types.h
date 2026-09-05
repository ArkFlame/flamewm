#ifndef FLAMEWM_PANEL_TYPES_H
#define FLAMEWM_PANEL_TYPES_H

// Reuses PanelEdge from core/types.h.
// PanelManager concept: map<OutputId, OutputPanel*> where each OutputPanel
// owns a placeholder panel window handle, edge, Start button and local views.
// Workspace/window/task EWMH truth stays IceWM global (wmmgr/atasks/objbar).
// Flame panels are presentation only — do not duplicate stacking/workArea
// authority. Strut/workArea values here are pure model; actual X configure
// is applied via IceWM wmmgr/workArea hooks in a single transaction.

#include "../core/types.h"
#include <map>
#include <string>

namespace flamewm {
namespace panel {

// Lightweight rect for pure geometry model (no X dependency).
struct Rect {
    int x;
    int y;
    int w;
    int h;
    Rect() : x(0), y(0), w(0), h(0) {}
    Rect(int x_, int y_, int w_, int h_) : x(x_), y(y_), w(w_), h(h_) {}
    bool empty() const { return w <= 0 || h <= 0; }
    bool operator==(const Rect& o) const { return x==o.x && y==o.y && w==o.w && h==o.h; }
    bool operator!=(const Rect& o) const { return !(*this==o); }
};

// EWMH strut reservation (partial strut simplified to thickness per edge).
// For Bottom/Top only top/bottom matter; for Left/Right only left/right.
// Kept explicit for verification.
struct Strut {
    int left;
    int right;
    int top;
    int bottom;
    Strut() : left(0), right(0), top(0), bottom(0) {}
    Strut(int l,int r,int t,int b) : left(l), right(r), top(t), bottom(b) {}
    bool operator==(const Strut& o) const {
        return left==o.left && right==o.right && top==o.top && bottom==o.bottom;
    }
};

// Forward declaration; concrete type in outputpanel.h.
// Placed here so PanelManager concept compiles without pulling X headers.
class OutputPanel;

// Hook contract (proposed, no edits to IceWM owners):
// - src/wmtaskbar.* : host YWindow remains IceWM-owned. Flame PanelManager
//   subscribes to RandR topology changes (YXApplication/XRandR listener) and
//   drives ensureOneOutputBehavior + add/removeOutput. A single transaction
//   owner stages geometry then calls wmmgr to coalesce workArea and configure
//   per-output YWindow frames (one window per output). Legacy single-taskbar
//   path preserved behind ensureOneOutputBehavior fallback.
// - src/wmmgr.* : IceWM workArea/EWMH authority remains. Flame provides
//   coalesced workArea rects per output; wmmgr applies _NET_WORKAREA and
//   configures clients. No duplicate truth.
// - src/atasks.* / src/objbar.* : IceWM owns TaskBarApp/TaskPane truth.
//   Flame composer presents pinned+running via stable ApplicationIdentity;
//   no title-based grouping.
// - src/yxtray.* : XEmbed selection has a single owner (one manager window).
//   Flame presents tray visuals on appropriate panel (primary or active) but
//   does NOT create duplicate tray owners per output.

} // namespace panel
} // namespace flamewm

#endif // FLAMEWM_PANEL_TYPES_H
