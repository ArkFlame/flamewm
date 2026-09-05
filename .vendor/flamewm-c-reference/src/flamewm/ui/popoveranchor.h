#ifndef FLAMEWM_UI_POPOVERANCHOR_H
#define FLAMEWM_UI_POPOVERANCHOR_H

#include "../core/types.h"

namespace flamewm {

// Simple rect in physical pixels (may have negative origin for multi-monitor)
struct Rect {
    int x; int y; int w; int h;
    Rect(): x(0), y(0), w(0), h(0) {}
    Rect(int _x,int _y,int _w,int _h): x(_x), y(_y), w(_w), h(_h) {}
    int x2() const { return x + w; }
    int y2() const { return y + h; }
    bool empty() const { return w <= 0 || h <= 0; }
};

struct PopoverPlacement {
    int x; int y; int w; int h;
    PanelEdge edge;
    bool clampedX;
    bool clampedY;
};

// Helper shared across popovers: compute popover position from PanelEdge + output rect + anchor.
// - anchor: icon/button rect in screen coords (physical)
// - popoverSize: desired w/h (physical)
// - panelThickness: thickness of panel on this edge (physical)
// - outputRect: monitor geometry (may have negative origin)
// - gap: logical gap converted to physical (e.g. 4 at 100% -> scaled outside, caller passes physical)
PopoverPlacement anchorPopover(
    PanelEdge edge,
    const Rect& outputRect,
    const Rect& anchorRect,
    int popoverW,
    int popoverH,
    int panelThickness,
    int gapPx);

inline PopoverPlacement anchorPopover(PanelEdge edge, const Rect& outputRect, const Rect& anchorRect,
                                      int popoverW, int popoverH) {
    return anchorPopover(edge, outputRect, anchorRect, popoverW, popoverH, 0, 4);
}

} // namespace flamewm
#endif
