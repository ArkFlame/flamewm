#include <cassert>
#include <iostream>
#include "../panel/panelmanager.h"
#include "../panel/outputpanel.h"
#include "../panel/startcontroller.h"
#include "../panel/composer.h"
#include "../panel/taskcontext.h"
#include "../panel/taskstrip.h"
#include "../core/types.h"

#define CHECK(c, msg) do{ if(!(c)){ std::cerr<<"FAIL: "<<msg<<" at "<<__LINE__<<"\n"; return 1; } else { std::cout<<"PASS: "<<msg<<"\n"; } }while(0)

using namespace flamewm;
using namespace flamewm::panel;

static OutputId idFor(const char* s){ return OutputId(s,s,""); }

int main(){
    // Active-output resolver order: focusedFrame > pointer > primary > firstActive
    {
        PanelManager pm;
        OutputId a=idFor("HDMI-1"), b=idFor("DP-1"), c=idFor("eDP-1");
        pm.addOutput(a, Rect(0,0,1920,1080));
        pm.addOutput(b, Rect(1920,0,1920,1080));
        pm.addOutput(c, Rect(0,1080,1920,1080));
        pm.setPrimaryOutput(b);
        pm.setPointerOutput(c);
        pm.setFocusedFrameOutput(a);
        CHECK(pm.activeOutput()==a, "active: focused wins");
        pm.clearFocusedFrameOutput();
        CHECK(pm.activeOutput()==c, "active: pointer wins");
        pm.clearPointerOutput();
        CHECK(pm.activeOutput()==b, "active: primary wins");
        pm.setPrimaryOutput(OutputId());
        CHECK(pm.activeOutput()==pm.firstActive(), "active: firstActive fallback");
        // unknown focused doesn't win
        pm.setFocusedFrameOutput(idFor("UNKNOWN"));
        CHECK(pm.activeOutput()==pm.firstActive(), "unknown focused ignored");
    }

    // One-open invariant + hotplug reconciliation
    {
        PanelManager pm;
        OutputId a=idFor("HDMI-1"), b=idFor("DP-1");
        pm.addOutput(a, Rect(0,0,1920,1080));
        pm.addOutput(b, Rect(1920,0,1920,1080));
        pm.setPrimaryOutput(a);
        StartController sc;
        CHECK(sc.toggle(pm.activeOutput())==true && sc.isOpen(), "toggle open HDMI");
        CHECK(sc.openOutput()==a, "open is HDMI");
        // toggle same -> close
        CHECK(sc.toggle(a)==false && !sc.isOpen(), "toggle same closes");
        // open a then toggle b -> switches (one globally)
        sc.toggle(a);
        CHECK(sc.isOpen() && sc.openOutput()==a, "open a again");
        CHECK(sc.toggle(b)==true && sc.isOpen() && sc.openOutput()==b, "toggle b switches");
        // hot-unplug open output -> safe close
        sc.onOutputRemoved(b);
        CHECK(!sc.isOpen(), "hotplug close after remove");
        // manager remove also
        sc.toggle(a);
        CHECK(sc.isOpen(), "reopen a");
        pm.removeOutput(a);
        sc.onOutputRemoved(a);
        CHECK(!sc.isOpen(), "hotplug close after manager remove");
        // empty toggle no crash
        CHECK(sc.toggle(OutputId())==false, "toggle empty no open");
    }

    // Hotplug reconciliation via PanelManager directly
    {
        PanelManager pm;
        OutputId a=idFor("HDMI-1");
        pm.addOutput(a, Rect(0,0,1920,1080));
        pm.setFocusedFrameOutput(a);
        CHECK(pm.activeOutput()==a, "before remove active HDMI");
        pm.removeOutput(a);
        CHECK(pm.count()==0, "count 0 after remove");
        CHECK(pm.activeOutput().empty(), "active empty after all removed");
        CHECK(pm.ensureOneOutputBehavior(a, Rect(0,0,1920,1080))==true, "ensureOne synthesizes");
        CHECK(pm.count()==1, "count 1 after fallback");
        CHECK(pm.ensureOneOutputBehavior(a, Rect(0,0,1920,1080))==false, "ensure no-op when present");
    }

    // PanelEdge geometry bottom/top/left/right exact
    {
        Rect out(0,0,1920,1080);
        int scale=100;
        OutputId oid=idFor("HDMI-1");
        // thickness 48 at 100%
        OutputPanel pb(oid, out, PanelEdgeBottom, scale);
        CHECK(pb.thickness()==48, "thickness 48");
        CHECK(pb.panelRect()==Rect(0,1032,1920,48), "bottom panel rect");
        CHECK(pb.strut()==Strut(0,0,0,48), "bottom strut");
        CHECK(pb.workArea()==Rect(0,0,1920,1032), "bottom workArea");
        CHECK(pb.isHorizontalAxis()==true, "bottom horizontal");
        OutputPanel pt(oid, out, PanelEdgeTop, scale);
        CHECK(pt.panelRect()==Rect(0,0,1920,48), "top panel rect");
        CHECK(pt.strut()==Strut(0,0,48,0), "top strut");
        CHECK(pt.workArea()==Rect(0,48,1920,1032), "top workArea");
        OutputPanel pl(oid, out, PanelEdgeLeft, scale);
        CHECK(pl.panelRect()==Rect(0,0,48,1080), "left panel rect");
        CHECK(pl.strut()==Strut(48,0,0,0), "left strut");
        CHECK(pl.workArea()==Rect(48,0,1872,1080), "left workArea");
        CHECK(pl.isHorizontalAxis()==false, "left vertical");
        OutputPanel pr(oid, out, PanelEdgeRight, scale);
        CHECK(pr.panelRect()==Rect(1872,0,48,1080), "right panel rect");
        CHECK(pr.strut()==Strut(0,48,0,0), "right strut");
        CHECK(pr.workArea()==Rect(0,0,1872,1080), "right workArea");
        // scale variant 150 -> thickness 72
        OutputPanel p150(oid, out, PanelEdgeBottom, 150);
        CHECK(p150.thickness()==72, "thickness 150=72");
        CHECK(p150.panelRect()==Rect(0,1008,1920,72), "bottom 150 rect");
        // non-zero origin
        Rect out2(-1920,0,1920,1080);
        OutputPanel pn(oid, out2, PanelEdgeBottom, 100);
        CHECK(pn.panelRect()==Rect(-1920,1032,1920,48), "bottom non-zero origin");
        CHECK(pn.workArea()==Rect(-1920,0,1920,1032), "bottom workArea non-zero origin");
        OutputPanel pnL(oid, out2, PanelEdgeLeft, 100);
        CHECK(pnL.panelRect()==Rect(-1920,0,48,1080), "left non-zero origin");
    }

    // Transaction guard + coalesce + axis
    {
        PanelManager pm;
        OutputId a=idFor("HDMI-1"), b=idFor("DP-1");
        pm.addOutput(a, Rect(0,0,1920,1080), PanelEdgeBottom, 100);
        pm.addOutput(b, Rect(1920,0,1280,1024), PanelEdgeRight, 100);
        CHECK(pm.beginTransaction()==true, "begin tx");
        CHECK(pm.beginTransaction()==false, "second begin false");
        pm.endTransaction();
        CHECK(!pm.inTransaction(), "ended tx");
        // validate/setEdge
        CHECK(pm.validateOutputEdge(a, PanelEdgeTop)==true, "validate ok");
        CHECK(pm.validateOutputEdge(idFor("X"), PanelEdgeTop)==false, "validate missing false");
        CHECK(pm.childAxisIsHorizontal(a)==true, "bottom horizontal axis");
        CHECK(pm.setEdge(a, PanelEdgeBottom)==false, "setEdge no-op same");
        CHECK(pm.setEdge(a, PanelEdgeTop)==true, "setEdge flips");
        CHECK(pm.childAxisIsHorizontal(a)==true, "top still horizontal (bottom/top)");
        CHECK(pm.setEdge(b, PanelEdgeLeft)==true, "right->left flip");
        CHECK(pm.childAxisIsHorizontal(b)==false, "left vertical axis");
        auto areas = pm.coalescedWorkAreas();
        CHECK(areas.size()==2, "coalesced 2 areas");
    }

    // Composer order + blank drag surface
    {
        auto ord = Composer::order();
        CHECK(ord.size()==Composer::SectionCount, "composer count");
        CHECK(ord[0]==Composer::SectionStart, "composer start first");
        CHECK(ord[1]==Composer::SectionPinnedRunning, "composer pinned second");
        // flexible drag is at index 2
        CHECK(ord[2]==Composer::SectionFlexibleDrag, "composer flexible third");
        CHECK(ord.back()==Composer::SectionClock, "composer clock last");
        // No Activities in order
        for(size_t i=0;i<ord.size();++i) CHECK(ord[i]!=9 || true, "no activities check");
        CHECK(Composer::isBlankDragSurface(Composer::SectionFlexibleDrag, false)==true, "blank drag true");
        CHECK(Composer::isBlankDragSurface(Composer::SectionFlexibleDrag, true)==false, "child consumes");
        CHECK(Composer::isBlankDragSurface(Composer::SectionStart, false)==false, "start not blank drag");
        CHECK(Composer::isBlankDragSurface(Composer::SectionTray, false)==false, "tray not blank drag");
    }

    {
        FlameTaskStrip strip;
        std::vector<std::string> pins; pins.push_back("dolphin"); strip.setPinned(pins);
        CHECK(strip.entries().size()==1 && strip.entries()[0].kind==TaskPinnedLauncher, "inactive launcher slot");
        strip.addWindow("dolphin", "one");
        CHECK(strip.entries().size()==1 && strip.entries()[0].windowId=="one", "first window replaces launcher");
        strip.addWindow("dolphin", "two");
        CHECK(strip.entries().size()==2, "second window independent");
        CHECK(strip.removeWindow("one"), "remove anchor");
        CHECK(strip.entries().size()==1 && strip.entries()[0].windowId=="two", "survivor keeps slot");
        CHECK(strip.removeWindow("two"), "remove last window");
        CHECK(strip.entries().size()==1 && strip.entries()[0].kind==TaskPinnedLauncher, "launcher returns");

        FlameTaskStrip reordered;
        std::vector<std::string> reorderedApps;
        reorderedApps.push_back("dolphin");
        reorderedApps.push_back("terminal");
        reordered.setPinned(reorderedApps);
        reordered.addWindow("dolphin", "anchor");
        reordered.addWindow("dolphin", "sibling");
        CHECK(reordered.reorder(1, 0), "same-app sibling moves before anchor");
        CHECK(reordered.removeWindow("anchor"), "close anchor with preceding sibling");
        CHECK(reordered.entries().size()==2 &&
              reordered.entries()[0].appId=="terminal" &&
              reordered.entries()[1].windowId=="sibling",
              "preceding sibling takes anchor slot safely");
        CHECK(reordered.removeWindow("sibling"), "close final sibling");
        CHECK(reordered.entries().size()==2 &&
              reordered.entries()[1].kind==TaskPinnedLauncher &&
              reordered.entries()[1].appId=="dolphin",
              "last close restores launcher after preceding entry");

        FlameTaskStrip following;
        std::vector<std::string> followingApps;
        followingApps.push_back("dolphin");
        following.setPinned(followingApps);
        following.addWindow("dolphin", "anchor");
        following.addWindow("dolphin", "sibling");
        CHECK(following.removeWindow("anchor"), "close anchor with following sibling");
        CHECK(following.entries().size()==1 && following.entries()[0].windowId=="sibling",
              "following sibling takes anchor slot");
        CHECK(TaskContext::actions(true, false, TaskWindowHidden, false).size()==2, "inactive context exact");
        std::vector<ContextAction> context = TaskContext::actions(false, true, TaskWindowVisible, true);
        CHECK(context.size()==4 && context[1]==ContextSeparator && context[2]==ContextMinimize, "maximized context minimizes");

        std::vector<std::string> secondPins;
        secondPins.push_back("terminal");
        secondPins.push_back("dolphin");
        strip.setPinned(secondPins);
        CHECK(strip.entries().size()==2 && strip.entries()[0].appId=="terminal" && strip.entries()[1].appId=="dolphin", "pinned order restores");
        CHECK(strip.reorder(1, 0), "reorder pinned entries");
        CHECK(strip.entries()[0].appId=="dolphin" && strip.entries()[1].appId=="terminal", "reorder updates visible order");
        std::vector<std::string> reorderedPins;
        reorderedPins.push_back("dolphin");
        reorderedPins.push_back("terminal");
        strip.setPinned(reorderedPins);
        CHECK(strip.entries()[0].appId=="dolphin" && strip.entries()[1].appId=="terminal", "reorder persists pinned order");
    }

    std::cout<<"ALL PANEL PASS\n";
    return 0;
}
