#ifndef FLAMEWM_DESKTOP_ITEM_H
#define FLAMEWM_DESKTOP_ITEM_H

#include "model.h"
#include "layout.h"

namespace flamewm {
namespace desktop {

// Visual hit rect for an item (pixel space)
struct ItemRect {
    int x; int y; int w; int h;
    bool intersects(const ItemRect& o) const {
        return !(x+w <= o.x || o.x+o.w <= x || y+h <= o.y || o.y+o.h <= y);
    }
    bool contains(int px,int py) const { return px>=x&&py>=y&&px<x+w&&py<y+h; }
};

inline ItemRect itemRectForCell(const GridConfig& g, int col, int row) {
    ItemRect r; r.w=g.cellW; r.h=g.cellH;
    LayoutEngine::cellToPixel(g,col,row,&r.x,&r.y);
    // inset slightly for visual padding
    return r;
}

} // namespace desktop
} // namespace flamewm

#endif
