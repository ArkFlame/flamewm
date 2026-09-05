#ifndef FLAMEWM_PANEL_ANCHOR_H
#define FLAMEWM_PANEL_ANCHOR_H

#include "../core/types.h"
#include <algorithm>

namespace flamewm {
namespace panel {

// Work area rect from IceWM YStrut / manager — supplied by caller to avoid hard X dep.
struct WorkArea {
    int x, y, w, h;
    WorkArea() : x(0), y(0), w(0), h(0) {}
    WorkArea(int _x,int _y,int _w,int _h):x(_x),y(_y),w(_w),h(_h){}
};

// Panel rect on its output
struct PanelRect {
    int x, y, w, h;
    PanelRect() : x(0), y(0), w(0), h(0) {}
    PanelRect(int _x,int _y,int _w,int _h):x(_x),y(_y),w(_w),h(_h){}
};

struct PopoverPlacement {
    int x, y, w, h;
    PopoverPlacement():x(0),y(0),w(0),h(0){}
    PopoverPlacement(int _x,int _y,int _w,int _h):x(_x),y(_y),w(_w),h(_h){}
};

// One edge-aware anchor helper using actual output work area + scale + negative-origin clamp.
// Used by clock/calendar, network, media, audio popovers and any future popover.
class PopoverAnchor {
public:
    // Compute popover rect clamped into work area + visible on output.
    // iconRect: clicked icon/status rect in root coords
    // popoverSize: desired content size (before clamp)
    // edge: panel edge owning the icon
    // workArea: output work area (after strut)
    // gap: semantic gap between panel and popover (e.g. 4px logical scaled)
    static PopoverPlacement anchor(const PanelRect& iconRect,
                                   int popoverW, int popoverH,
                                   PanelEdge edge,
                                   const WorkArea& workArea,
                                   int gap) {
        PopoverPlacement out;
        out.w = popoverW;
        out.h = popoverH;

        // Clamp size to work area if larger
        if (out.w > workArea.w) out.w = workArea.w;
        if (out.h > workArea.h) out.h = workArea.h;

        switch (edge) {
            case PanelEdgeBottom:
                out.x = iconRect.x; // left aligns clicked icon left
                out.y = workArea.y + workArea.h - iconRect.h - gap - out.h;
                // actually panel sits at bottom of workArea; compute above panel
                // iconRect.y is panel.y; so popover y = panel.y - gap - popoverH
                out.y = iconRect.y - gap - out.h;
                break;
            case PanelEdgeTop:
                out.x = iconRect.x;
                out.y = iconRect.y + iconRect.h + gap;
                break;
            case PanelEdgeLeft:
                out.x = iconRect.x + iconRect.w + gap;
                out.y = iconRect.y;
                break;
            case PanelEdgeRight:
                out.x = iconRect.x - gap - out.w;
                out.y = iconRect.y;
                break;
            default:
                out.x = iconRect.x;
                out.y = iconRect.y - gap - out.h;
                break;
        }

        // Horizontal clamp to work area (handles negative origins)
        if (out.x < workArea.x) out.x = workArea.x;
        if (out.x + out.w > workArea.x + workArea.w) out.x = workArea.x + workArea.w - out.w;
        if (out.y < workArea.y) out.y = workArea.y;
        if (out.y + out.h > workArea.y + workArea.h) out.y = workArea.y + workArea.h - out.h;

        return out;
    }

    // Calendar today cell: square then rounded 50% -> circle, not oval.
    // Returns diameter to use for highlight (square side).
    static int todayCellDiameter(int cellW, int cellH) {
        int d = std::min(cellW, cellH);
        return d;
    }
};

} // namespace panel
} // namespace flamewm

#endif
