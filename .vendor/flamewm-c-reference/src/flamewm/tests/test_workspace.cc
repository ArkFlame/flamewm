#include <iostream>
#include <vector>
#include <string>
#include "../workspace/topology.h"
#include "../workspace/transaction.h"

#define CHECK(c, msg) do{ if(!(c)){ std::cerr<<"FAIL: "<<msg<<" at "<<__LINE__<<"\n"; return 1; } else { std::cout<<"PASS: "<<msg<<"\n"; } }while(0)

using namespace flamewm::workspace;

int main(){
    // ---- topology index<->row,col two-row ----
    {
        CHECK(TwoRowTopology::columnsFor(4)==2, "cols 4->2");
        CHECK(TwoRowTopology::columnsFor(5)==3, "cols 5->3");
        CHECK(TwoRowTopology::columnsFor(1)==1, "cols 1->1");
        CHECK(TwoRowTopology::columnsFor(0)==1, "cols 0->1");
        // index->pos
        IndexPos p = TwoRowTopology::indexToPos(0,5);
        CHECK(p.row==0&&p.col==0, "idx0");
        p=TwoRowTopology::indexToPos(1,5); CHECK(p.row==1&&p.col==0, "idx1");
        p=TwoRowTopology::indexToPos(2,5); CHECK(p.row==0&&p.col==1, "idx2");
        p=TwoRowTopology::indexToPos(3,5); CHECK(p.row==1&&p.col==1, "idx3");
        p=TwoRowTopology::indexToPos(4,5); CHECK(p.row==0&&p.col==2, "idx4");
        // rowColToIndex alias
        CHECK(TwoRowTopology::rowColToIndex(0,0,5)==0, "rowCol 0,0");
        CHECK(TwoRowTopology::rowColToIndex(1,0,5)==1, "rowCol 1,0");
        CHECK(TwoRowTopology::rowColToIndex(0,2,5)==4, "rowCol 0,2");
        // odd tail
        CHECK(TwoRowTopology::posToIndex(1,2,5)==4, "odd tail clamp");
        CHECK(TwoRowTopology::rowColToIndex(1,2,5)==4, "rowCol alias odd tail");
        // indexToRowCol alias
        IndexPos pr = TwoRowTopology::indexToRowCol(3,5);
        CHECK(pr.row==1&&pr.col==1, "alias indexToRowCol");
        // round-trip bijective for valid indices
        for(int total=1; total<=6; ++total){
            for(int i=0;i<total;++i){
                IndexPos pp = TwoRowTopology::indexToPos(i,total);
                int back = TwoRowTopology::posToIndex(pp.row, pp.col, total);
                CHECK(back==i, "round-trip");
            }
        }
    }
    // ---- directional Ctrl+Super+Arrow using same model ----
    {
        CHECK(TwoRowTopology::moveLeft(2,5)==0, "left 2->0");
        CHECK(TwoRowTopology::moveRight(0,5)==2, "right 0->2");
        CHECK(TwoRowTopology::moveDown(0,5)==1, "down 0->1");
        CHECK(TwoRowTopology::moveUp(1,5)==0, "up 1->0");
        CHECK(TwoRowTopology::moveLeft(0,5)==0, "left edge stay");
        CHECK(TwoRowTopology::moveDown(4,5)==4, "down odd last stay");
        CHECK(TwoRowTopology::moveRight(4,5)==4, "right edge stay");
        CHECK(TwoRowTopology::moveUp(0,5)==0, "up stay");
        // odd last column transition from row1 col1 to col2
        CHECK(TwoRowTopology::moveRight(1,5)==3, "right 1->3");
        CHECK(TwoRowTopology::moveRight(3,5)==4, "right 3->4");
        CHECK(TwoRowTopology::moveLeft(4,5)==2, "left 4->2 uses row0");
        // single workspace
        CHECK(TwoRowTopology::moveLeft(0,1)==0, "single left stay");
        CHECK(TwoRowTopology::moveDown(0,1)==0, "single down stay");
        // invariant: moveLeft/moveRight use same posToIndex as paint
        for(int total=2; total<=7; ++total){
            for(int i=0;i<total;++i){
                int l = TwoRowTopology::moveLeft(i,total);
                int r = TwoRowTopology::moveRight(i,total);
                // l and r must be valid indices
                CHECK(l>=0 && l<total, "left valid");
                CHECK(r>=0 && r<total, "right valid");
            }
        }
    }

    // ---- transaction insert first/middle/last ----
    {
        std::vector<std::string> names = {"A","B","C"};
        std::vector<int> frames = {0,1,2,2,1,0,-1};
        // insert at 0 (first)
        auto ins0 = WorkspaceTransaction::insertWorkspace(3,0,names,frames,1,2);
        CHECK(ins0.valid && ins0.newCount==4, "insert first valid count");
        CHECK(ins0.newNames[0]=="Workspace 1", "insert first name");
        CHECK(ins0.newNames[1]=="A", "shift A");
        CHECK(ins0.frameNewWs[0]==1 && ins0.frameNewWs[1]==2, "frame shift after insert first");
        CHECK(ins0.frameNewWs[6]==-1, "sticky stays -1");
        CHECK(ins0.newActive==2 && ins0.newLast==3, "active/last shift after insert first");
        // insert middle (1)
        auto ins1 = WorkspaceTransaction::insertWorkspace(3,1,names,frames,0,2);
        CHECK(ins1.valid, "insert middle valid");
        CHECK(ins1.newNames[0]=="A" && ins1.newNames[1]=="Workspace 2" && ins1.newNames[2]=="B", "insert middle names");
        CHECK(ins1.frameNewWs[0]==0, "frame 0 stays 0");
        CHECK(ins1.frameNewWs[1]==2, "frame 1 ->2 (was 1 >=1)");
        CHECK(ins1.frameNewWs[2]==3, "frame 2 ->3");
        CHECK(ins1.newActive==0, "active 0 stays 0 before insert point");
        CHECK(ins1.newLast==3, "last 2 ->3");
        // insert last (3)
        auto insLast = WorkspaceTransaction::insertWorkspace(3,3,names,frames,2,0);
        CHECK(insLast.valid && insLast.newNames[3]=="Workspace 4", "insert last");
        CHECK(insLast.frameNewWs[0]==0, "frame 0 stays after insert last");
        CHECK(insLast.frameNewWs[2]==2, "frame 2 stays 2 after insert last");
        CHECK(insLast.newActive==2, "active stays before last insert");
    }

    // ---- transaction remove first/middle/last, windows migrate ----
    {
        std::vector<std::string> names = {"A","B","C","D"};
        std::vector<int> frames = {0,1,2,3,1,2,0};
        // remove middle (1 => B)
        auto rem1 = WorkspaceTransaction::removeWorkspace(4,1,names,frames,1,3);
        CHECK(rem1.valid && rem1.newCount==3, "remove middle count");
        CHECK(rem1.newNames[0]=="A" && rem1.newNames[1]=="C", "remove middle names");
        CHECK(rem1.removedMigrateTarget==1, "migrate target 1");
        // frames on removed workspace 1 migrate to 1 (now C)
        CHECK(rem1.frameNewWs[1]==1 && rem1.frameNewWs[4]==1, "windows on removed migrate to nearest");
        CHECK(rem1.frameNewWs[2]==1 && rem1.frameNewWs[5]==1, "2->1");
        CHECK(rem1.frameNewWs[3]==2, "3->2");
        CHECK(rem1.newActive==1, "active on removed -> migrate");
        CHECK(rem1.newLast==2, "last 3->2");
        // remove first (0)
        auto rem0 = WorkspaceTransaction::removeWorkspace(4,0,names,frames,0,3);
        CHECK(rem0.valid, "remove first valid");
        CHECK(rem0.newNames[0]=="B", "remove first names");
        CHECK(rem0.frameNewWs[0]==0, "0 migrates to 0");
        CHECK(rem0.frameNewWs[6]==0, "0 duplicate migrates");
        CHECK(rem0.frameNewWs[1]==0, "1->0 after remove first");
        CHECK(rem0.newActive==0, "active 0->0 migrate");
        CHECK(rem0.newLast==2, "last 3->2");
        // remove last (3)
        auto remLast = WorkspaceTransaction::removeWorkspace(4,3,names,frames,2,3);
        CHECK(remLast.valid, "remove last valid");
        CHECK(remLast.newNames.size()==3 && remLast.newNames[2]=="C", "remove last names");
        CHECK(remLast.frameNewWs[3]==2, "3 migrates to 2");
        CHECK(remLast.newActive==2, "active 2 stays 2 (not last)");
        CHECK(remLast.newLast==2, "last 3 migrates to 2");
        // sticky stays
        std::vector<int> framesSticky = {0,-1,1};
        auto remS = WorkspaceTransaction::removeWorkspace(2,0,{"A","B"},framesSticky,0,0);
        CHECK(remS.frameNewWs[1]==-1, "sticky stays after remove");
    }

    // ---- only-workspace removal rejected ----
    {
        auto remOnly = WorkspaceTransaction::removeWorkspace(1,0,{"Only"},{0},0,0);
        CHECK(!remOnly.valid, "only workspace removal rejected");
        // also with min>=1 explicit
        auto remOnly2 = WorkspaceTransaction::removeWorkspace(1,0,{"Only"},{},0,0,1);
        CHECK(!remOnly2.valid, "only workspace min1 rejected");
        // count below min invalid
        std::string err;
        CHECK(!WorkspaceTransaction::isValidCount(0,1,&err), "count 0 invalid");
    }

    // ---- EWMH conceptual fields: newCount/names/current reflect model ----
    {
        // After insert+remove round-trip names count matches newCount
        std::vector<std::string> names = {"Ws1","Ws2"};
        auto ins = WorkspaceTransaction::insertWorkspace(2,1,names,{0,1},0,1);
        CHECK(ins.valid && (int)ins.newNames.size()==ins.newCount, "EWMH names size == count after insert");
        auto rem = WorkspaceTransaction::removeWorkspace(ins.newCount,1,ins.newNames,ins.frameNewWs,ins.newActive,ins.newLast);
        CHECK(rem.valid && (int)rem.newNames.size()==rem.newCount, "EWMH names size == count after remove");
        // viewport/workarea conceptual: newCount drives _NET_NUMBER_OF_DESKTOPS etc.
        CHECK(rem.newCount==2, "round-trip count restores");
    }

    std::cout<<"ALL WORKSPACE PASS\n";
    return 0;
}
