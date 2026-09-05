#include <iostream>
#include <cassert>
#include "../snap/types.h"
#include "../snap/state.h"
#include "../snap/controller.h"
#include "../snap/overlay.h"
#include "../platform/snap/service.h"
#include "../engine/icewm/snap_bridge.h"
#include "../api/ports.h"

#define CHECK(c, msg) do{ if(!(c)){ std::cerr<<"FAIL: "<<msg<<" at "<<__LINE__<<"\n"; return 1; } else { std::cout<<"PASS: "<<msg<<"\n"; } }while(0)

using namespace flamewm::snap;

namespace {
class SnapWindowPort : public flamewm::api::WindowPort {
public:
    flamewm::api::WindowRef ref;
    flamewm::api::Rect work;
    flamewm::api::Rect committed;

    SnapWindowPort()
        : ref(7, 1), work(0, 0, 1920, 1080), committed() {}

    flamewm::api::Result<flamewm::api::WindowSnapshot> get(flamewm::api::WindowRef win) override {
        flamewm::api::WindowSnapshot snapshot;
        snapshot.ref = win;
        return flamewm::api::Result<flamewm::api::WindowSnapshot>::Ok(snapshot);
    }
    std::vector<flamewm::api::WindowSnapshot> snapshot() override { return std::vector<flamewm::api::WindowSnapshot>(); }
    flamewm::api::Status activate(flamewm::api::WindowRef) override { return flamewm::api::Status::Ok(); }
    flamewm::api::Status minimize(flamewm::api::WindowRef) override { return flamewm::api::Status::Ok(); }
    flamewm::api::Status maximize(flamewm::api::WindowRef) override { return flamewm::api::Status::Ok(); }
    flamewm::api::Status restore(flamewm::api::WindowRef) override { return flamewm::api::Status::Ok(); }
    flamewm::api::Status close(flamewm::api::WindowRef) override { return flamewm::api::Status::Ok(); }
    flamewm::api::Status setOuterGeometry(flamewm::api::WindowRef, flamewm::api::Rect geo) override {
        committed = geo;
        return flamewm::api::Status::Ok();
    }
    flamewm::api::Result<flamewm::api::Rect> workArea(flamewm::api::WindowRef) override {
        return flamewm::api::Result<flamewm::api::Rect>::Ok(work);
    }
    flamewm::api::Result<flamewm::api::OutputId> output(flamewm::api::WindowRef) override {
        return flamewm::api::Result<flamewm::api::OutputId>::Ok(flamewm::api::OutputId("test"));
    }
};

class SnapMainLoop : public flamewm::api::MainLoopPort {
public:
    TimerCallback timer;
    int nextId;

    SnapMainLoop() : timer(), nextId(1) {}
    FdHandle addPoll(int, int, FdCallback) override { FdHandle h = { nextId++ }; return h; }
    void removePoll(FdHandle) override {}
    TimerHandle addTimer(uint64_t, TimerCallback cb, bool) override {
        timer = cb;
        TimerHandle h = { nextId++ };
        return h;
    }
    void removeTimer(TimerHandle) override {}
    TimerHandle defer(TimerCallback cb) override {
        cb();
        TimerHandle h = { nextId++ };
        return h;
    }
};
}

int main(){
    // ---- geometryFor all origins ----
    struct Origin { int mx,my,Mx,My; const char* label; };
    Origin origins[] = {
        {0,0,1920,1080,"origin 0,0"},
        {100,50,2020,1130,"positive offset"},
        {-1920,0,0,1080,"negative x"},
        {-100,-100,1820,980,"negative both"},
        {1920,0,3840,1080,"second output"},
    };
    for (auto &o: origins){
        int W=o.Mx-o.mx, H=o.My-o.my;
        SnapGeometry g;
        g=geometryFor(SnapLeftHalf,o.mx,o.my,o.Mx,o.My,0,0);
        CHECK(g.x==o.mx && g.y==o.my && g.w==W/2 && g.h==H, std::string("LeftHalf ")+o.label);
        g=geometryFor(SnapRightHalf,o.mx,o.my,o.Mx,o.My,0,0);
        CHECK(g.x==o.mx+W/2 && g.w==W-W/2 && g.h==H, std::string("RightHalf ")+o.label);
        g=geometryFor(SnapTopLeft,o.mx,o.my,o.Mx,o.My,0,0);
        CHECK(g.x==o.mx && g.y==o.my && g.w==W/2 && g.h==H/2, std::string("TopLeft ")+o.label);
        g=geometryFor(SnapTopRight,o.mx,o.my,o.Mx,o.My,0,0);
        CHECK(g.x==o.mx+W/2 && g.w==W-W/2 && g.h==H/2, std::string("TopRight ")+o.label);
        g=geometryFor(SnapBottomLeft,o.mx,o.my,o.Mx,o.My,0,0);
        CHECK(g.w==W/2 && g.h==H-H/2 && g.y==o.my+H/2, std::string("BottomLeft ")+o.label);
        g=geometryFor(SnapBottomRight,o.mx,o.my,o.Mx,o.My,0,0);
        CHECK(g.w==W-W/2 && g.h==H-H/2 && g.x==o.mx+W/2, std::string("BottomRight ")+o.label);
        g=geometryFor(SnapMaximize,o.mx,o.my,o.Mx,o.My,0,0);
        CHECK(g.x==o.mx && g.y==o.my && g.w==W && g.h==H, std::string("Maximize ")+o.label);
        // center formula non-zero origin correctness
        g=geometryFor(SnapCenter,o.mx,o.my,o.Mx,o.My,400,300);
        int ex = o.mx + (o.Mx - o.mx - 400)/2;
        int ey = o.my + (o.My - o.my - 300)/2;
        CHECK(g.x==ex && g.y==ey, std::string("Center ")+o.label);
        // Rect overload preview==commit
        Rect wa(o.mx,o.my,W,H);
        SnapGeometry g2 = geometryFor(SnapLeftHalf, wa, 0,0);
        CHECK(g2.x==geometryFor(SnapLeftHalf,o.mx,o.my,o.Mx,o.My,0,0).x, std::string("Rect overload ")+o.label);
        // outputRect overload documented: commit uses workArea, not outputRect
        Rect output(o.mx,o.my,W,H);
        SnapGeometry g3 = geometryFor(SnapRightHalf, wa, output, 0,0);
        CHECK(g3.x==geometryFor(SnapRightHalf,o.mx,o.my,o.Mx,o.My,0,0).x, std::string("outputRect overload ")+o.label);
    }

    // ---- preview == commit via controller ----
    {
        SnapController ctrl(1);
        Rect workArea(100,50,1820,1030);
        Rect output(100,50,1820,1080);
        for(int t=SnapLeftHalf; t<=SnapMaximize; ++t){
            SnapTarget tt=(SnapTarget)t;
            if(tt==SnapCenter) continue;
            if(tt==SnapTopHalf || tt==SnapBottomHalf) continue;
            Rect pre = ctrl.previewGeometry(tt, workArea, 500,400);
            Rect com = ctrl.commitGeometry(tt, workArea, 500,400);
            CHECK(pre==com, "preview==commit");
        }
        // also direct geometryFor equality
        for(int t=SnapLeftHalf; t<=SnapMaximize; ++t){
            SnapTarget tt=(SnapTarget)t;
            if(tt==SnapCenter) continue;
            if(tt==SnapTopHalf || tt==SnapBottomHalf) continue;
            Rect a = toRect(geometryFor(tt, workArea, 500,400));
            Rect b = toRect(geometryFor(tt, workArea, output, 500,400));
            CHECK(a==b, "workArea vs workArea+output identical");
        }
    }

    // ---- controller updateTarget detection ----
    {
        SnapController ctrl(42);
        Rect out(0,0,1920,1080);
        CHECK(ctrl.updateTarget(2, 540, out)==SnapLeftHalf, "left edge -> half");
        CHECK(ctrl.updateTarget(1918, 540, out)==SnapRightHalf, "right edge -> half");
        CHECK(ctrl.updateTarget(10,10, out)==SnapTopLeft, "top-left corner quarter");
        CHECK(ctrl.updateTarget(1910,10, out)==SnapTopRight, "top-right quarter");
        CHECK(ctrl.updateTarget(5, 1070, out)==SnapBottomLeft, "bottom-left quarter");
        CHECK(ctrl.updateTarget(1915,1070, out)==SnapBottomRight, "bottom-right quarter");
        CHECK(ctrl.updateTarget(960,2, out)==SnapMaximize, "top center maximize");
        CHECK(ctrl.updateTarget(960,1075, out)==SnapNone, "bottom center none");
        CHECK(ctrl.updateTarget(960,540, out)==SnapNone, "center none");
        // negative origin output
        Rect outNeg(-1920,0,1920,1080);
        CHECK(ctrl.updateTarget(-1918,540,outNeg)==SnapLeftHalf, "negative origin left");
        CHECK(ctrl.updateTarget(-5,540,outNeg)==SnapRightHalf, "negative origin right");
        CHECK(ctrl.updateTarget(-1910,10,outNeg)==SnapTopLeft, "negative origin corner");
    }

    // ---- shouldDwellSideEdge ----
    {
        SnapController ctrl(1);
        Rect out(0,0,1920,1080);
        CHECK(!ctrl.shouldDwellSideEdge(SnapTopLeft, 2, out, 1000), "corner never dwell");
        CHECK(!ctrl.shouldDwellSideEdge(SnapBottomRight, 1918, out, 1000), "corner never dwell BR");
        CHECK(!ctrl.shouldDwellSideEdge(SnapMaximize, 960, out, 1000), "maximize never dwell");
        CHECK(!ctrl.shouldDwellSideEdge(SnapLeftHalf, 2, out, 100), "side not yet elapsed");
        CHECK(ctrl.shouldDwellSideEdge(SnapLeftHalf, 2, out, 400), "side dwell true at threshold");
        CHECK(ctrl.shouldDwellSideEdge(SnapRightHalf, 1918, out, 500), "right dwell true");
        CHECK(!ctrl.shouldDwellSideEdge(SnapLeftHalf, 500, out, 500), "side leaves edge -> false");
    }

    // ---- live SnapService dwell retains uncommitted candidate ----
    {
        SnapWindowPort window;
        SnapMainLoop loop;
        flamewm::platform::snap::SnapService service(&window, &loop);
        flamewm::api::WindowRef win = window.ref;
        CHECK(service.moveBegin(win, flamewm::api::Rect(200, 200, 800, 600)).ok(), "service move begin");
        CHECK(service.moveMotion(win, flamewm::api::Point(2, 540),
                                 flamewm::api::Rect(0, 0, 1920, 1080)).ok(), "service side preview");
        CHECK(static_cast<bool>(loop.timer), "service dwell armed");
        loop.timer();
        CHECK(service.hasPreview(win) &&
              service.previewTarget(win) == flamewm::platform::snap::SnapTarget::LeftHalf,
              "dwell expiry retains preview");
        flamewm::api::Result<flamewm::api::Rect> committed = service.moveEnd(win);
        CHECK(committed.ok() && window.committed == flamewm::api::Rect(0, 0, 960, 1080),
               "retained preview commits");
    }

    // ---- live SnapService edge/corner/top commit and cancellation paths ----
    {
        SnapWindowPort window;
        SnapMainLoop loop;
        flamewm::platform::snap::SnapService service(&window, &loop);
        flamewm::api::WindowRef win = window.ref;
        flamewm::api::Rect floating(200, 200, 800, 600);
        CHECK(service.moveBegin(win, floating).ok(), "right edge move begin");
        CHECK(service.moveMotion(win, flamewm::api::Point(1918, 540),
                                 flamewm::api::Rect(0, 0, 1920, 1080)).ok(),
              "right edge preview");
        CHECK(service.previewTarget(win) == flamewm::platform::snap::SnapTarget::RightHalf,
              "right edge target");
        CHECK(window.committed == flamewm::api::Rect(),
              "dwell preview does not commit geometry");
        CHECK(static_cast<bool>(loop.timer), "right dwell armed");
        loop.timer();
        CHECK(window.committed == flamewm::api::Rect(),
              "dwell expiry does not commit geometry");
        flamewm::api::Result<flamewm::api::Rect> committed = service.moveEnd(win);
        CHECK(committed.ok() && window.committed == flamewm::api::Rect(960, 0, 960, 1080),
              "right edge commits on move end");
    }

    {
        SnapWindowPort window;
        SnapMainLoop loop;
        flamewm::platform::snap::SnapService service(&window, &loop);
        flamewm::api::WindowRef win = window.ref;
        CHECK(service.moveBegin(win, flamewm::api::Rect(200, 200, 800, 600)).ok(),
              "move-away begin");
        CHECK(service.moveMotion(win, flamewm::api::Point(2, 540),
                                 flamewm::api::Rect(0, 0, 1920, 1080)).ok(),
              "move-away side preview");
        CHECK(static_cast<bool>(loop.timer), "move-away dwell armed");
        CHECK(service.moveMotion(win, flamewm::api::Point(960, 540),
                                 flamewm::api::Rect(0, 0, 1920, 1080)).ok(),
              "move-away center motion");
        CHECK(!service.hasPreview(win), "move-away cancels preview");
        loop.timer();
        CHECK(window.committed == flamewm::api::Rect(),
              "cancelled dwell cannot commit geometry");
        flamewm::api::Result<flamewm::api::Rect> cancelled = service.moveEnd(win);
        CHECK(!cancelled.ok() && cancelled.status().code == flamewm::api::Error::NotFound,
              "move-away ends without commit");
    }

    {
        struct LiveTarget {
            flamewm::api::Point pointer;
            flamewm::platform::snap::SnapTarget target;
            flamewm::api::Rect geometry;
            const char* label;
        } cases[] = {
            { flamewm::api::Point(2, 10), flamewm::platform::snap::SnapTarget::TopLeftQuarter,
              flamewm::api::Rect(0, 0, 960, 540), "top-left" },
            { flamewm::api::Point(1918, 10), flamewm::platform::snap::SnapTarget::TopRightQuarter,
              flamewm::api::Rect(960, 0, 960, 540), "top-right" },
            { flamewm::api::Point(2, 1070), flamewm::platform::snap::SnapTarget::BottomLeftQuarter,
              flamewm::api::Rect(0, 540, 960, 540), "bottom-left" },
            { flamewm::api::Point(1918, 1070), flamewm::platform::snap::SnapTarget::BottomRightQuarter,
              flamewm::api::Rect(960, 540, 960, 540), "bottom-right" },
            { flamewm::api::Point(960, 10), flamewm::platform::snap::SnapTarget::TopMaximize,
              flamewm::api::Rect(0, 0, 1920, 1080), "top" }
        };
        for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); ++i) {
            SnapWindowPort window;
            SnapMainLoop loop;
            flamewm::platform::snap::SnapService service(&window, &loop);
            flamewm::api::WindowRef win = window.ref;
            CHECK(service.moveBegin(win, flamewm::api::Rect(200, 200, 800, 600)).ok(),
                  std::string(cases[i].label) + " move begin");
            CHECK(service.moveMotion(win, cases[i].pointer,
                                     flamewm::api::Rect(0, 0, 1920, 1080)).ok(),
                  std::string(cases[i].label) + " preview");
            CHECK(service.previewTarget(win) == cases[i].target,
                  std::string(cases[i].label) + " target");
            CHECK(!static_cast<bool>(loop.timer),
                  std::string(cases[i].label) + " no dwell timer");
            CHECK(window.committed == flamewm::api::Rect(),
                  std::string(cases[i].label) + " preview does not commit");
            flamewm::api::Result<flamewm::api::Rect> committed = service.moveEnd(win);
            CHECK(committed.ok() && window.committed == cases[i].geometry,
                  std::string(cases[i].label) + " commits on move end");
        }
    }

    // ---- SnapState unsnapped restore sequences ----
    {
        SnapState st(10);
        Rect floating(200,200,800,600);
        Rect wa(0,0,1920,1080);
        // floating -> snap captures
        st.onSnap(SnapLeftHalf, floating);
        CHECK(st.hasUnsnapped && st.unsnappedOuter==floating && st.current==SnapLeftHalf, "capture on first snap");
        // snap -> snap preserves
        Rect duringSnap(0,0,960,1080);
        st.onSnap(SnapRightHalf, duringSnap);
        CHECK(st.unsnappedOuter==floating && st.current==SnapRightHalf, "snap->snap preserves");
        st.onSnap(SnapTopLeft, duringSnap);
        CHECK(st.unsnappedOuter==floating, "second snap->snap still preserves");
        // manual resize exits
        st.onManualResize();
        CHECK(!st.hasUnsnapped && st.current==SnapNone, "manual resize exits");
        // re-snap after manual resize captures new floating
        Rect floating2(100,100,700,500);
        st.onSnap(SnapBottomRight, floating2);
        CHECK(st.unsnappedOuter==floating2, "re-capture after exit");
        // drag-away restores exact
        Rect restore = st.onDragAway();
        CHECK(restore==floating2, "drag-away restores exact");
        // consume drag-away exits state
        Rect r2 = st.consumeDragAway();
        CHECK(r2==floating2 && !st.hasUnsnapped && st.current==SnapNone, "consume exits");
        // clamp to surviving output
        Rect bigOuter(3000,0,800,600);
        SnapState st2(11);
        st2.onSnap(SnapLeftHalf, bigOuter);
        Rect surviving(0,0,1920,1080);
        Rect clamped = SnapState::clampToWorkArea(st2.onDragAway(), surviving);
        CHECK(clamped.x + clamped.w <= surviving.x+surviving.w && clamped.y >= surviving.y, "clamped to surviving");
        // overflow rect larger than workarea pinned
        Rect huge(-100,-100,4000,3000);
        Rect clamped2 = SnapState::clampToWorkArea(huge, surviving);
        CHECK(clamped2.w <= surviving.w && clamped2.h <= surviving.h, "huge clamped size");
    }

    // ---- generation guards ----
    {
        SnapController ctrl(100);
        CHECK(!ctrl.isStaleGeneration(100), "current gen not stale");
        CHECK(ctrl.isStaleGeneration(99), "old gen stale");
        ctrl.setGeneration(101);
        CHECK(ctrl.isStaleGeneration(100), "after bump old stale");
        CHECK(!ctrl.isStaleGeneration(101), "new gen valid");
        // SnapState invalidation bumps
        SnapState st(50);
        CHECK(st.isValidGeneration(50), "state gen valid");
        st.invalidate();
        CHECK(!st.isValidGeneration(50), "after invalidate stale");
        CHECK(st.isValidGeneration(51), "after invalidate new gen");
    }

    // ---- native bridge preview lifecycle ----
    {
        SnapWindowPort window;
        SnapMainLoop loop;
        flamewm::platform::snap::SnapService service(&window, &loop);
        flamewm::engine::icewm::SnapBridge& bridge =
            flamewm::engine::icewm::SnapBridge::instance();
        CHECK(bridge.begin(&service, nullptr, window.ref,
                          flamewm::api::Rect(200, 200, 800, 600)),
              "bridge begin");
        CHECK(bridge.motion(flamewm::api::Point(2, 540),
                            flamewm::api::Rect(0, 0, 1920, 1080)),
              "bridge preview motion");
        CHECK(bridge.previewVisible() &&
              bridge.previewGeometry() == flamewm::api::Rect(0, 0, 960, 1080),
              "bridge preview geometry");
        flamewm::api::Result<flamewm::api::Rect> result = bridge.end(false);
        CHECK(result.ok() && result.value() == window.committed &&
              result.value() == bridge.previewGeometry(),
              "bridge commit geometry");
        CHECK(!bridge.active() && !bridge.previewVisible(), "bridge cleanup");
    }

    // ---- overlay opacity 0..60, border visible at 0 ----
    {
        SnapOverlay ov;
        CHECK(ov.fillOpacity()==20, "default opacity 20");
        CHECK(ov.borderVisible(), "border visible default");
        ov.setFillOpacity(0);
        CHECK(ov.fillOpacity()==0 && ov.borderVisible(), "border visible at 0% fill");
        CHECK(ov.fillAlpha()==0.0, "alpha 0");
        ov.setFillOpacity(60);
        CHECK(ov.fillAlpha()==0.6, "alpha 60%");
        ov.setFillOpacity(100);
        CHECK(ov.fillOpacity()==60, "clamp high to 60");
        ov.setFillOpacity(-5);
        CHECK(ov.fillOpacity()==0, "clamp low to 0");
        ov.show(Rect(0,0,960,1080));
        CHECK(ov.visible() && ov.rect()==Rect(0,0,960,1080), "show");
        ov.hide();
        CHECK(!ov.visible(), "hide");
    }

    std::cout<<"ALL SNAP PASS\n";
    return 0;
}
