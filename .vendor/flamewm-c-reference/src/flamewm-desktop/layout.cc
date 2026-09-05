#include "layout.h"
#include <algorithm>

namespace flamewm {
namespace desktop {

GridConfig LayoutEngine::compute(const WorkArea& wa, int scalePct) {
    GridConfig g;
    g.workArea = wa;
    g.scalePct = scalePct;
    int baseW = 96, baseH = 96;
    // scale cell size linearly? keep physical size stable: logical 96 * scale/100
    // But spec: grid recomputed from work area+scale => scale enlarges cells
    g.cellW = (baseW * scalePct + 50) / 100;
    if (g.cellW < 48) g.cellW = 48;
    g.cellH = (baseH * scalePct + 50) / 100;
    if (g.cellH < 48) g.cellH = 48;
    g.cols = wa.w / g.cellW;
    g.rows = wa.h / g.cellH;
    if (g.cols < 1) g.cols = 1;
    if (g.rows < 1) g.rows = 1;
    return g;
}
void LayoutEngine::cellToPixel(const GridConfig& g,int col,int row,int* px,int* py){
    if(px) *px = g.workArea.x + col * g.cellW;
    if(py) *py = g.workArea.y + row * g.cellH;
}
void LayoutEngine::pixelToCell(const GridConfig& g,int px,int py,int* col,int* row){
    if(col) *col = (px - g.workArea.x) / g.cellW;
    if(row) *row = (py - g.workArea.y) / g.cellH;
}
void LayoutEngine::clampCell(const GridConfig& g,int* col,int* row){
    if(col){ if(*col<0) *col=0; if(*col>=g.cols) *col=g.cols-1; }
    if(row){ if(*row<0) *row=0; if(*row>=g.rows) *row=g.rows-1; }
}
LayoutEngine::CellRect LayoutEngine::boundingRect(const std::map<std::string,std::pair<int,int> >& cells){
    CellRect r; r.minCol=9999; r.minRow=9999; r.maxCol=-9999; r.maxRow=-9999;
    for(std::map<std::string,std::pair<int,int> >::const_iterator it=cells.begin(); it!=cells.end(); ++it){
        if(it->second.first < r.minCol) r.minCol=it->second.first;
        if(it->second.second < r.minRow) r.minRow=it->second.second;
        if(it->second.first > r.maxCol) r.maxCol=it->second.first;
        if(it->second.second > r.maxRow) r.maxRow=it->second.second;
    }
    if(r.minCol==9999){ r.minCol=0; r.minRow=0; r.maxCol=0; r.maxRow=0; }
    return r;
}

// Group drag transaction: pure helper
bool LayoutEngine::groupDragTransaction(const std::vector<DragItem>& selected,
                                        const std::set<std::pair<int,int> >& unselectedOccupied,
                                        int dCol,int dRow,
                                        const GridConfig& grid,
                                        std::map<std::string,std::pair<int,int> >* outNewPos,
                                        std::string* error) {
    if (selected.empty()) { if(error) *error="empty selection"; return false; }
    // 1) compute bounding rect of selected
    int minC=selected[0].col, minR=selected[0].row, maxC=minC, maxR=minR;
    for(size_t i=1;i<selected.size();++i){ if(selected[i].col<minC)minC=selected[i].col; if(selected[i].row<minR)minR=selected[i].row; if(selected[i].col>maxC)maxC=selected[i].col; if(selected[i].row>maxR)maxR=selected[i].row; }
    // 2) clamp whole group bounding rect: adjust delta so rect stays in bounds
    // Compute allowed delta range
    int clampedDCol=dCol, clampedDRow=dRow;
    if(minC + clampedDCol < 0) clampedDCol = -minC;
    if(maxC + clampedDCol >= grid.cols) clampedDCol = grid.cols -1 - maxC;
    if(minR + clampedDRow < 0) clampedDRow = -minR;
    if(maxR + clampedDRow >= grid.rows) clampedDRow = grid.rows -1 - maxR;
    // If clamped delta would still be out of bounds -> reject (shouldn't happen after clamp, but check)
    if(minC+clampedDCol<0 || maxC+clampedDCol>=grid.cols || minR+clampedDRow<0 || maxR+clampedDRow>=grid.rows){
        if(error) *error="group out of bounds";
        return false;
    }
    // 3) compute candidate new positions preserving offsets
    std::map<std::string,std::pair<int,int> > cand;
    std::set<std::pair<int,int> > candSet;
    for(size_t i=0;i<selected.size();++i){
        int nc = selected[i].col + clampedDCol;
        int nr = selected[i].row + clampedDRow;
        cand[selected[i].path]=std::make_pair(nc,nr);
        candSet.insert(std::make_pair(nc,nr));
    }
    // Detect internal duplicate after move (should not happen if offsets preserved, but check)
    if(candSet.size()!=selected.size()){ if(error) *error="internal collision"; return false; }
    // 4) reject overlap with unselected
    for(std::map<std::string,std::pair<int,int> >::iterator it=cand.begin(); it!=cand.end(); ++it){
        if(unselectedOccupied.find(it->second)!=unselectedOccupied.end()){
            if(error) *error="collision with unselected at "+it->first;
            return false;
        }
    }
    if(outNewPos) *outNewPos=cand;
    return true;
}

} // namespace desktop
} // namespace flamewm
