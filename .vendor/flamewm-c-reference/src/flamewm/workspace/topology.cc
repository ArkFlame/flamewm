#include "topology.h"

namespace flamewm {
namespace workspace {

int TwoRowTopology::columnsFor(int total){
    if(total<=0) return 1;
    return (total+1)/2;
}
IndexPos TwoRowTopology::indexToPos(int index, int total){
    IndexPos p; p.row=0; p.col=0;
    if(total<=0) return p;
    if(index<0) index=0;
    if(index>=total) index=total-1;
    p.row = index % 2; // 0,1,0,1...
    p.col = index / 2;
    // For odd total, last col has only row0; row1 invalid col already clamped by caller via move logic
    return p;
}
int TwoRowTopology::posToIndex(int row, int col, int total){
    if(total<=0) return 0;
    if(col<0) col=0;
    int cols=columnsFor(total);
    if(col>=cols) col=cols-1;
    if(row<0) row=0;
    if(row>1) row=1;
    int idx = col*2 + row;
    if(idx>=total) {
        // last column odd: row1 invalid -> clamp to last valid in that col (row0)
        idx = col*2;
        if(idx>=total) idx=total-1;
    }
    return idx;
}
int TwoRowTopology::moveLeft(int index, int total){
    IndexPos p=indexToPos(index,total);
    if(p.col==0) return index; // clamp at left edge (deterministic, no wrap)
    return posToIndex(p.row, p.col-1, total);
}
int TwoRowTopology::moveRight(int index, int total){
    IndexPos p=indexToPos(index,total);
    int cols=columnsFor(total);
    if(p.col+1>=cols) return index;
    // if moving right from odd tail, ensure target exists
    int target = posToIndex(p.row, p.col+1, total);
    // if target col is last odd col and p.row==1 -> posToIndex would have clamped to row0 col;
    // but we want to preserve row if possible; if row1 invalid then stay (no move) or fall to row0?
    // spec: deterministic. For p.row==1 moving into last col that has only row0, go to that row0.
    // That's what posToIndex does; keep it.
    return target;
}
int TwoRowTopology::moveUp(int index, int total){
    IndexPos p=indexToPos(index,total);
    if(p.row==0) return index;
    return posToIndex(0, p.col, total);
}
int TwoRowTopology::moveDown(int index, int total){
    IndexPos p=indexToPos(index,total);
    if(p.row==1) return index;
    // down only if row1 exists in this col
    int cols=columnsFor(total);
    // col last with odd total has no row1
    if(total%2==1 && p.col==cols-1) return index;
    return posToIndex(1, p.col, total);
}

} // namespace workspace
} // namespace flamewm
