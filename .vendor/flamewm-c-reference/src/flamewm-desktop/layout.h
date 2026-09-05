#ifndef FLAMEWM_DESKTOP_LAYOUT_H
#define FLAMEWM_DESKTOP_LAYOUT_H

#include <string>
#include <vector>
#include <set>
#include <map>

namespace flamewm {
namespace desktop {

struct WorkArea {
    int x; int y; int w; int h;
    WorkArea():x(0),y(0),w(1920),h(1080){}
    WorkArea(int X,int Y,int W,int H):x(X),y(Y),w(W),h(H){}
    bool contains(int px,int py) const { return px>=x && py>=y && px<x+w && py<y+h; }
};

struct GridConfig {
    int cellW; // physical pixels per cell
    int cellH;
    int cols;
    int rows;
    int scalePct; // 100..200
    WorkArea workArea;
    GridConfig():cellW(96),cellH(96),cols(0),rows(0),scalePct(100){}
};

// Pure layout engine: no X, no IO
class LayoutEngine {
public:
    // Compute grid from work area + scale. Bounded: at least 1x1.
    static GridConfig compute(const WorkArea& wa, int scalePct);
    // Logical cell -> pixel top-left (cell origin)
    static void cellToPixel(const GridConfig& g, int col, int row, int* px, int* py);
    // Pixel -> cell (floor)
    static void pixelToCell(const GridConfig& g, int px, int py, int* col, int* row);
    // Clamp cell into grid
    static void clampCell(const GridConfig& g, int* col, int* row);
    // Bounding rect of set of cells (inclusive)
    struct CellRect { int minCol, minRow, maxCol, maxRow; };
    static CellRect boundingRect(const std::map<std::string, std::pair<int,int> >& cells);

    // Group drag transaction helper (pure, no IO).
    // Inputs: selected paths each with current (col,row), unselected occupied set,
    //         delta (dCol,dRow), grid bounds.
    // Returns: true if whole group can move (clamped bounding rect inside grid, no collision with unselected).
    // Out: new positions for selected (clamped delta applied uniformly).
    // Document: compute delta once, preserve offsets, clamp whole group bounding rect, reject collision with unselected, commit together.
    struct DragItem { std::string path; int col; int row; };
    static bool groupDragTransaction(const std::vector<DragItem>& selected,
                                     const std::set< std::pair<int,int> >& unselectedOccupied,
                                     int dCol, int dRow,
                                     const GridConfig& grid,
                                     std::map<std::string, std::pair<int,int> >* outNewPos,
                                     std::string* error);
};

} // namespace desktop
} // namespace flamewm

#endif
