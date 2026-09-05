#ifndef FLAMEWM_WORKSPACE_TOPOLOGY_H
#define FLAMEWM_WORKSPACE_TOPOLOGY_H

namespace flamewm {
namespace workspace {

// Two-row topology — single mapping drives pager paint and keyboard nav.
//
// Invariant: index <-> row,col is bijective for valid indices; one set of
// functions drives BOTH paint and Ctrl+Super+Arrow. Do not duplicate math.
//
// Layout (per WINDOWS.md / INDEX.md):
//   two rows, ceil(total/2) columns
//   index = col*2 + row   (0->(0,0) 1->(1,0) 2->(0,1) 3->(1,1) 4->(0,2)...)
//   Odd total: last column has only row0; row1 there is invalid and clamped.
//
struct IndexPos { int row; int col; };

class TwoRowTopology {
public:
    // total workspaces >=1
    static IndexPos indexToPos(int index, int total);
    static int posToIndex(int row, int col, int total);
    // alias per spec name
    static inline IndexPos indexToRowCol(int index, int total) { return indexToPos(index, total); }
    static inline int rowColToIndex(int row, int col, int total) { return posToIndex(row, col, total); }
    static int columnsFor(int total); // ceil(total/2)
    // directional navigation: returns new index, clamps at edges (deterministic, no wrap)
    static int moveLeft(int index, int total);
    static int moveRight(int index, int total);
    static int moveUp(int index, int total);
    static int moveDown(int index, int total);
};

} // namespace workspace
} // namespace flamewm
#endif
