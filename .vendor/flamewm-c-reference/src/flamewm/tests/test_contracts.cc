#include <cassert>
#include <iostream>
#include <cstdio>
#include "../core/types.h"
#include "../ui/palette.h"
#include "../ui/metrics.h"
#include "../core/scalemanager.h"
#include "../core/config.h"
#include "../core/shortcutregistry.h"
#include "../core/appidentity.h"
#include "../snap/types.h"
#include "../workspace/topology.h"
#include "../core/productmetadata.h"
#include "../core/runtime.h"

#define CHECK(c, msg) do{ if(!(c)){ std::cerr<<"FAIL: "<<msg<<" at "<<__LINE__<<"\n"; return 1; } else { std::cout<<"PASS: "<<msg<<"\n"; } }while(0)

int main(){
    // 1) scale bucket validation
    CHECK(flamewm::isSupportedScale(100), "100 supported");
    CHECK(flamewm::isSupportedScale(125), "125 supported");
    CHECK(!flamewm::isSupportedScale(110), "110 not supported");
    CHECK(flamewm::nearestSupportedScale(112)==100, "nearest 112 ->100");
    CHECK(flamewm::nearestSupportedScale(130)==125, "nearest 130 ->125");
    CHECK(flamewm::FlameMetrics::logicalToPhysical(48,150)==72, "48@150=72");
    // ScaleManager
    {
        flamewm::ScaleManager sm;
        flamewm::OutputId id = flamewm::ScaleManager::makeOutputId("HDMI-1","");
        CHECK(sm.scaleFor(id)==100, "default 100");
        CHECK(sm.setScale(id,150), "set 150 ok");
        CHECK(sm.scaleFor(id)==150, "get 150");
        CHECK(!sm.setScale(id,110), "reject 110");
        CHECK(sm.scaleFor(id)==150, "still 150 after bad set");
        CHECK(sm.toPhysical(48,id)==72, "toPhysical via manager");
    }

    // 2) topology index<->row/col two-row
    {
        using flamewm::workspace::TwoRowTopology;
        CHECK(TwoRowTopology::columnsFor(4)==2, "cols 4->2");
        CHECK(TwoRowTopology::columnsFor(5)==3, "cols 5->3");
        // index->pos: 0->(0,0) 1->(1,0) 2->(0,1) 3->(1,1) 4->(0,2)
        flamewm::workspace::IndexPos p = TwoRowTopology::indexToPos(0,5);
        CHECK(p.row==0&&p.col==0, "idx0");
        p=TwoRowTopology::indexToPos(1,5); CHECK(p.row==1&&p.col==0, "idx1");
        p=TwoRowTopology::indexToPos(2,5); CHECK(p.row==0&&p.col==1, "idx2");
        p=TwoRowTopology::indexToPos(3,5); CHECK(p.row==1&&p.col==1, "idx3");
        p=TwoRowTopology::indexToPos(4,5); CHECK(p.row==0&&p.col==2, "idx4");
        CHECK(TwoRowTopology::posToIndex(0,0,5)==0, "pos00");
        CHECK(TwoRowTopology::posToIndex(1,0,5)==1, "pos10");
        CHECK(TwoRowTopology::posToIndex(0,2,5)==4, "pos02");
        // odd tail: pos(1,2) with total5 => clamps to 4
        CHECK(TwoRowTopology::posToIndex(1,2,5)==4, "odd tail clamp");
        // directional
        CHECK(TwoRowTopology::moveLeft(2,5)==0, "left 2->0");
        CHECK(TwoRowTopology::moveRight(0,5)==2, "right 0->2");
        CHECK(TwoRowTopology::moveDown(0,5)==1, "down 0->1");
        CHECK(TwoRowTopology::moveUp(1,5)==0, "up 1->0");
        // edge clamps
        CHECK(TwoRowTopology::moveLeft(0,5)==0, "left edge stay");
        CHECK(TwoRowTopology::moveDown(4,5)==4, "down odd last stay");
    }

    // 3) snap geometry preview==commit same func
    {
        using flamewm::snap::geometryFor;
        using flamewm::snap::SnapLeftHalf;
        using flamewm::snap::SnapCenter;
        int mx=10, my=20, Mx=1010, My=620;
        flamewm::snap::SnapGeometry a = geometryFor(SnapLeftHalf, mx,my,Mx,My, 400,300);
        flamewm::snap::SnapGeometry b = geometryFor(SnapLeftHalf, mx,my,Mx,My, 400,300);
        CHECK(a.x==b.x&&a.y==b.y&&a.w==b.w&&a.h==b.h, "snap preview==commit left");
        flamewm::snap::SnapGeometry c = geometryFor(SnapCenter, mx,my,Mx,My, 400,300);
        // center formula: x = mx + (Mx - mx - w)/2
        int expX = mx + (Mx - mx - 400)/2;
        int expY = my + (My - my - 300)/2;
        CHECK(c.x==expX && c.y==expY, "center tile formula non-zero origin");
        // negative origin
        mx=-1920; Mx=0; my=0; My=1080;
        c = geometryFor(SnapCenter, mx,my,Mx,My, 400,300);
        expX = mx + (Mx - mx - 400)/2;
        CHECK(c.x==expX, "center negative origin");
    }

    // 4) hotkey null -> Not assigned, Escape clears, conflict detection
    {
        flamewm::ShortcutRegistry reg;
        CHECK(reg.bindingFor(flamewm::ActionWindowMinimize).display()=="Not assigned", "minimize Not assigned");
        // Escape clears capture helper
        CHECK(flamewm::ShortcutRegistry::isEscape("Escape"), "Escape detection");
        std::string err;
        // conflict: WorkspaceLeft already Ctrl+Super+Left
        bool ok = reg.setBinding(flamewm::ActionWorkspaceRight, "Ctrl+Super+Left", &err);
        CHECK(!ok, "conflict rejected");
        // clear then set should succeed
        reg.clearBinding(flamewm::ActionWorkspaceLeft);
        CHECK(reg.bindingFor(flamewm::ActionWorkspaceLeft).display()=="Not assigned", "cleared -> Not assigned");
        ok = reg.setBinding(flamewm::ActionWorkspaceRight, "Ctrl+Super+Left", &err);
        // now it conflicts with the cleared left? left is now not assigned, so should succeed
        CHECK(ok, "after clear no conflict");
        // reset per-row defaults
        reg.resetToDefaults();
        CHECK(reg.bindingFor(flamewm::ActionWorkspaceLeft).display()=="Ctrl+Super+Left", "reset defaults");
    }

    // 5) config revision monotonic
    {
        flamewm::ConfigStore store;
        flamewm::SettingsSnapshot snap;
        std::string err;
        CHECK(store.revision()==1, "initial revision 1");
        // stale: revision 1 again -> should be stale
        snap = store.current();
        snap.revision = 1;
        CHECK(store.isStale(snap), "stale 1");
        CHECK(!store.applySnapshot(snap, &err), "apply stale false");
        // newer revision 2
        snap.revision = 2;
        snap.desktopSelectionFillOpacity = 40;
        snap.windowSnapPreviewFillOpacity = 20;
        CHECK(store.applySnapshot(snap, &err), "apply rev 2");
        CHECK(store.revision()==2, "now 2");
        // try to apply rev 1 again -> ignored
        flamewm::SettingsSnapshot old = flamewm::SettingsSnapshot::defaults();
        old.revision = 1;
        CHECK(store.isStale(old), "old stale after 2");
        // invalid opacity
        snap.revision = 3;
        snap.desktopSelectionFillOpacity = 99;
        CHECK(!store.applySnapshot(snap, &err), "invalid opacity rejected");
        CHECK(store.revision()==2, "still 2 after invalid");
        // unknown keys preserved via parse
        flamewm::SettingsSnapshot parsed;
        std::string text = "revision=3\naccent=#FF0000\nDesktopSelectionFillOpacity=10\nWindowSnapPreviewFillOpacity=20\nfutureKey=keepme\n";
        CHECK(store.parseSnapshot(text, parsed, &err), "parse with unknown");
        CHECK(parsed.unknownKeys.count("futureKey")==1, "unknown preserved");
        CHECK(store.applySnapshot(parsed, &err), "apply parsed 3");
        CHECK(store.current().unknownKeys.count("futureKey")==1, "unknown in current");
    }

    // 6) app identity order desktopId -> StartupWMClass -> WM_CLASS -> normalized fallback, no title
    {
        flamewm::AppIdentityInput in;
        in.desktopId="firefox.desktop"; in.startupWMClass="Firefox"; in.wmClass="firefox"; in.wmInstance="Navigator";
        CHECK(flamewm::ApplicationIdentity::resolve(in)=="firefox.desktop", "desktopId wins");
        in.desktopId="";
        CHECK(flamewm::ApplicationIdentity::resolve(in)=="firefox", "startup wins");
        in.startupWMClass="";
        CHECK(flamewm::ApplicationIdentity::resolve(in)=="firefox", "wmClass wins");
        in.wmClass="";
        CHECK(flamewm::ApplicationIdentity::resolve(in)=="navigator", "instance fallback normalized lower");
        in.wmInstance="";
        CHECK(flamewm::ApplicationIdentity::resolve(in)=="unknown", "unknown fallback");
    }

    // palette defaults sanity
    {
        auto p = flamewm::FlamePalette::defaults();
        CHECK(p.accent=="#EF4048", "palette accent");
        CHECK(p.surface=="#202326", "palette surface");
    }
    // product metadata URLs
    {
        CHECK(std::string(flamewm::ProductMetadata::donateUrl())=="https://paypal.me/LinsaFTW", "donate url");
        CHECK(std::string(flamewm::ProductMetadata::sourceUrl())=="https://github.com/arkflame/flamewm", "source url");
        CHECK(std::string(flamewm::ProductMetadata::websiteUrl())=="https://wm.arkflame.com", "website url");
        CHECK(flamewm::SafeUriLauncher::isSafeUri("https://paypal.me/LinsaFTW"), "safe https");
        CHECK(!flamewm::SafeUriLauncher::isSafeUri("file:///etc/passwd"), "unsafe file");
        CHECK(!flamewm::SafeUriLauncher::isSafeUri("https://example.com; rm -rf /"), "uri with space still https but size check");
        // argv vector no shell
        auto v = flamewm::SafeUriLauncher::buildXdgOpenArgv("https://example.com");
        CHECK(v.size()==2 && v[0]=="xdg-open", "argv vector");
    }
    // runtime lifecycle skeleton
    {
        flamewm::Runtime rt;
        CHECK(!rt.isStopping(), "not stopping initially");
        CHECK(!rt.isInitialized(), "not initialized initially");
        uint64_t g0 = rt.generation();
        // initialize with null should fail, still not stopping generation unchanged
        CHECK(!rt.initialize(nullptr), "init null fails");
        // shutdown bumps generation
        rt.shutdown();
        CHECK(rt.generation()==g0+1, "generation bump on shutdown");
        CHECK(!rt.isCallbackCurrent(g0), "stale callback rejected");
        rt.shutdown();
        CHECK(rt.generation()==g0+1, "shutdown idempotent");
    }

    std::cout<<"ALL PASS\n";
    return 0;
}
