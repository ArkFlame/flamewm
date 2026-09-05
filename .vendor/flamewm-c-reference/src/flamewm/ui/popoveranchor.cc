#include "popoveranchor.h"

namespace flamewm {

PopoverPlacement anchorPopover(
    PanelEdge edge,
    const Rect& outputRect,
    const Rect& anchorRect,
    int popoverW,
    int popoverH,
    int /*panelThickness*/,
    int gapPx) {

    PopoverPlacement out;
    out.w = popoverW;
    out.h = popoverH;
    out.edge = edge;
    out.clampedX = false;
    out.clampedY = false;

    // Clamp output rect is respected even with negative origins.
    // Compute desired position per edge, then clamp within output.

    int desiredX = 0;
    int desiredY = 0;

    switch (edge) {
        case PanelEdgeBottom:
            // Above panel, left-aligned to anchor
            desiredX = anchorRect.x;
            desiredY = (anchorRect.y - gapPx - popoverH);
            // Fallback: if anchor is near top of output and bottom edge but popover would go off,
            // clamp will handle. Keep simple.
            break;
        case PanelEdgeTop:
            desiredX = anchorRect.x;
            desiredY = anchorRect.y + anchorRect.h + gapPx;
            break;
        case PanelEdgeLeft:
            desiredX = anchorRect.x + anchorRect.w + gapPx;
            desiredY = anchorRect.y;
            break;
        case PanelEdgeRight:
            desiredX = anchorRect.x - gapPx - popoverW;
            desiredY = anchorRect.y;
            break;
        default:
            desiredX = anchorRect.x;
            desiredY = anchorRect.y - gapPx - popoverH;
            break;
    }

    // Clamp to output rect
    int minX = outputRect.x;
    int maxX = outputRect.x + outputRect.w - popoverW;
    int minY = outputRect.y;
    int maxY = outputRect.y + outputRect.h - popoverH;

    // If popover larger than output, pin to output origin
    if (popoverW >= outputRect.w) { minX = outputRect.x; maxX = outputRect.x; }
    if (popoverH >= outputRect.h) { minY = outputRect.y; maxY = outputRect.y; }

    int clampedX = desiredX;
    int clampedY = desiredY;
    if (clampedX < minX) { clampedX = minX; out.clampedX = (desiredX != clampedX); }
    if (clampedX > maxX) { clampedX = maxX; out.clampedX = true; }
    if (clampedY < minY) { clampedY = minY; out.clampedY = (desiredY != clampedY); }
    if (clampedY > maxY) { clampedY = maxY; out.clampedY = true; }

    out.x = clampedX;
    out.y = clampedY;
    return out;
}

} // namespace flamewm
