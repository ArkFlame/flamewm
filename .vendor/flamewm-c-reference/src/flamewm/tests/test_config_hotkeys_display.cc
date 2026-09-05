#include <cassert>
#include <iostream>
#include <cstdio>
#include <unistd.h>
#include "../core/config.h"
#include "../core/shortcutregistry.h"
#include "../core/display.h"
#include "../core/productmetadata.h"
#include "../core/scalemanager.h"
#include "../../flamewm-settings/pages/desktop.h"
#include "../../flamewm-settings/pages/hotkeys.h"
#include "../../flamewm-settings/pages/about.h"
#include "../../flamewm-settings/pages/displays.h"
#include "../../flamewm-settings/pages/fonts.h"
#include "../../flamewm-settings/widgets/keycapture.h"

#define CHECK(c,msg) do{ if(!(c)){ std::cerr<<"FAIL: "<<msg<<" at "<<__LINE__<<"\n"; return 1;} else { std::cout<<"PASS: "<<msg<<"\n"; }}while(0)

static bool grabFailForCtrlSuperLeft(const std::string& ks, void*){
    if(ks=="Ctrl+Super+Left") return false;
    return true;
}
static bool grabAlways(const std::string& ks, void*){ (void)ks; return true; }

int main(){
    // invalid snapshot rollback
    {
        flamewm::ConfigStore store;
        flamewm::SettingsSnapshot snap = store.current();
        snap.revision=2; snap.desktopSelectionFillOpacity=70; // invalid >60
        std::string err;
        CHECK(!store.applySnapshot(snap,&err), "invalid snapshot rollback");
        CHECK(store.revision()==1, "still rev1 after invalid");
        CHECK(store.current().desktopSelectionFillOpacity==20, "opacity unchanged after rollback");
    }
    // stale revision ignored
    {
        flamewm::ConfigStore store;
        flamewm::SettingsSnapshot s = store.current(); s.revision=2; s.desktopSelectionFillOpacity=10;
        std::string err; CHECK(store.applySnapshot(s,&err), "apply rev2");
        flamewm::SettingsSnapshot old = flamewm::SettingsSnapshot::defaults(); old.revision=1;
        CHECK(store.isStale(old), "stale 1 after 2");
        CHECK(!store.applySnapshot(old,&err), "apply stale ignored");
        CHECK(store.revision()==2, "still 2 after stale");
        CHECK(!err.empty(), "stale apply reports error");
    }
    // strict config scalar and durable-key validation
    {
        flamewm::ConfigStore store;
        flamewm::SettingsSnapshot parsed;
        std::string err;
        CHECK(!store.parseSnapshot("revision=-1\n", parsed, &err), "negative revision rejected");
        CHECK(!store.parseSnapshot("revision=18446744073709551616\n", parsed, &err), "overflow revision rejected");
        CHECK(!store.parseSnapshot("revision=2\naccent=red\n", parsed, &err), "noncanonical accent rejected");
        CHECK(!store.parseSnapshot("revision=2\nfontSizeOffset=5\n", parsed, &err), "font offset upper bound rejected");
        CHECK(!store.parseSnapshot("revision=2\nscale.=125\n", parsed, &err), "empty output ID rejected");
        CHECK(!store.parseSnapshot("revision=2\nhotkey.Unknown=Alt+X\n", parsed, &err), "unknown hotkey action rejected");
        flamewm::SettingsSnapshot duplicate = store.current();
        duplicate.revision = 2;
        duplicate.unknownKeys["accent"] = "#000000";
        CHECK(!store.applySnapshot(duplicate, &err), "stable key duplicate rejected");
        CHECK(store.current().accent == "#EF4048", "invalid apply preserves accent");
    }
    // unknown keys preserved
    {
        flamewm::ConfigStore store;
        std::string err; flamewm::SettingsSnapshot parsed;
        CHECK(store.parseSnapshot("revision=2\nfutureKey=keepme\n", parsed, &err), "parse unknown");
        CHECK(parsed.unknownKeys.count("futureKey")==1, "unknown preserved parse");
        CHECK(store.applySnapshot(parsed,&err), "apply with unknown");
        CHECK(store.current().unknownKeys.count("futureKey")==1, "unknown in current");
        // unrelated override unchanged: change accent, futureKey still there
        flamewm::SettingsSnapshot s2 = store.current(); s2.revision=3; s2.accent="#FF0000";
        CHECK(store.applySnapshot(s2,&err), "apply accent change");
        CHECK(store.current().unknownKeys.count("futureKey")==1, "unrelated override unchanged");
    }
    // persist atomic tmp+rename
    {
        flamewm::ConfigStore store;
        char tmpl[]="/tmp/flame_test_XXXXXX";
        int fd=mkstemp(tmpl); close(fd);
        std::string path=tmpl;
        flamewm::SettingsSnapshot s=store.current(); s.revision=5;
        std::string err;
        CHECK(store.persistToFile(path,s,&err), "persist atomic");
        FILE* f=fopen(path.c_str(),"r"); CHECK(f!=0, "file exists after persist");
        char buf[4096]={0}; fread(buf,1,sizeof(buf)-1,f); fclose(f);
        CHECK(std::string(buf).find("revision=5")!=std::string::npos, "persist content");
        unlink(path.c_str()); unlink((path+".tmp").c_str());
    }
    // invalid persist leaves target and temp file untouched
    {
        flamewm::ConfigStore store;
        char tmpl[]="/tmp/flame_invalid_persist_XXXXXX";
        int fd=mkstemp(tmpl);
        CHECK(fd>=0, "create invalid persist target");
        const char original[]="existing config\n";
        CHECK(write(fd, original, sizeof(original)-1)==static_cast<ssize_t>(sizeof(original)-1), "seed invalid persist target");
        close(fd);
        std::string path=tmpl;
        std::string err;
        flamewm::SettingsSnapshot invalid=store.current();
        invalid.desktopSelectionFillOpacity=61;
        CHECK(!store.persistToFile(path, invalid, &err), "invalid persist rejected");
        CHECK(err.find("invalid snapshot:")!=std::string::npos, "invalid persist reports reason");
        FILE* f=fopen(path.c_str(), "rb"); CHECK(f!=0, "invalid persist keeps target");
        char buf[64]={0}; fread(buf,1,sizeof(buf)-1,f); fclose(f);
        CHECK(std::string(buf)==original, "invalid persist preserves target content");
        CHECK(access((path+".tmp").c_str(), F_OK)!=0, "invalid persist does not create temp");
        unlink(path.c_str()); unlink((path+".tmp").c_str());
    }
    // font failure rollback model
    {
        flamewm::settings::FontsPage fp;
        fp.family="IBM Plex Sans"; fp.globalBold=false; fp.sizeOffset=0;
        std::string err;
        CHECK(!fp.apply("__fail__", true, 1, &err), "font failure returns false");
        CHECK(fp.family=="IBM Plex Sans", "font rollback family unchanged");
        CHECK(fp.globalBold==false, "font rollback bold unchanged");
        CHECK(fp.sizeOffset==0, "font rollback offset unchanged");
        // success path
        CHECK(fp.apply("Noto Sans", true, 2, &err), "font success");
        CHECK(fp.family=="Noto Sans", "font after success");
    }
    // Escape->null , null->Not assigned
    {
        flamewm::ShortcutRegistry reg;
        CHECK(reg.bindingFor(flamewm::ActionWindowMinimize).display()=="Not assigned", "null->Not assigned");
        flamewm::settings::KeyCaptureWidget w; w.begin();
        CHECK(w.onKey("Escape"), "Escape captured");
        CHECK(w.isNull(), "Escape->null");
        CHECK(w.display()=="Not assigned", "Escape display Not assigned");
        // HotkeysPage Escape clears (Control-only API, zero-arg ctor)
        flamewm::settings::HotkeysPage hp;
        hp.beginCapture(flamewm::api::ActionWorkspaceLeft);
        CHECK(hp.isCapturing(), "hp capturing after beginCapture");
        CHECK(hp.captureAction==flamewm::api::ActionWorkspaceLeft, "hp captureAction set");
        CHECK(flamewm::settings::HotkeysPage::isEscape("Escape"), "HotkeysPage::isEscape");
        CHECK(flamewm::settings::HotkeysPage::displayFor(flamewm::api::KeyBinding())=="Not assigned", "HotkeysPage displayFor unassigned");
        // handleCaptureKey now requires ControlClient*; with null client it clears capture but fails to commit
        {
            std::string perr;
            hp.handleCaptureKey((flamewm::control::ControlClient*)0, "Escape", &perr);
            CHECK(!hp.isCapturing(), "hp not capturing after Escape");
        }
        // Escape never stored (core registry still rejects)
        std::string err;
        CHECK(!reg.setBinding(flamewm::ActionWorkspaceRight, "Escape", &err), "Escape never stored");
        // commitCapture without client also fails gracefully
        {
            flamewm::settings::HotkeysPage hp2;
            hp2.beginCapture(flamewm::api::ActionWorkspaceLeft);
            std::string e2;
            CHECK(!hp2.commitCapture((flamewm::control::ControlClient*)0, "Alt+Left", &e2), "commitCapture without control fails");
        }
    }
    // per-row reset only one
    {
        flamewm::ShortcutRegistry reg;
        reg.setBinding(flamewm::ActionWorkspaceLeft, std::string("Alt+Left"), 0);
        reg.setBinding(flamewm::ActionWorkspaceRight, std::string("Alt+Right"), 0);
        reg.resetOneRow(flamewm::ActionWorkspaceLeft);
        CHECK(reg.bindingFor(flamewm::ActionWorkspaceLeft).display()=="Ctrl+Super+Left", "reset one restores that row");
        CHECK(reg.bindingFor(flamewm::ActionWorkspaceRight).display()=="Alt+Right", "reset one leaves other");
    }
    // conflict path
    {
        flamewm::ShortcutRegistry reg;
        std::string err;
        CHECK(!reg.setBinding(flamewm::ActionWorkspaceRight, "Ctrl+Super+Left", &err), "conflict rejected");
        CHECK(err.find("conflict")!=std::string::npos, "conflict error");
    }
    // modifier-only rejection unless action allows
    {
        flamewm::ShortcutRegistry reg;
        std::string err;
        CHECK(!reg.setBinding(flamewm::ActionWorkspaceLeft, "Ctrl", &err), "modifier-only rejected");
        CHECK(reg.setBinding(flamewm::ActionToggleStartMenu, "Super", &err), "Super allowed for ToggleStart");
        CHECK(reg.setBinding(flamewm::ActionToggleStartMenu, "Super_L", &err), "Super_L allowed");
    }
    // failed grab path: old set kept, persist only effective
    {
        flamewm::ShortcutRegistry reg;
        std::string err;
        // old: WorkspaceLeft Ctrl+Super+Left
        std::map<flamewm::ShortcutAction,std::string> desired;
        desired[flamewm::ActionWorkspaceLeft]="Alt+Left"; // would conflict if grab fails simulate
        // use grab that fails for this keysym
        bool ok = reg.applyWithGrabStage(desired, grabFailForCtrlSuperLeft, 0, &err);
        // Actually grabFail fails on Ctrl+Super+Left but desired is Alt+Left, so it should succeed; need to craft failing case
        // Now test failing: set desired to keep old which includes Ctrl+Super+Left -> grab fails
        flamewm::ShortcutRegistry reg2;
        std::map<flamewm::ShortcutAction,std::string> d2; // empty desired -> will attempt grab of existing Ctrl+Super+Left
        // Add a no-op but grab will fail on existing left binding
        CHECK(!reg2.applyWithGrabStage(d2, grabFailForCtrlSuperLeft, 0, &err), "failed grab keeps old");
        CHECK(reg2.bindingFor(flamewm::ActionWorkspaceLeft).display()=="Ctrl+Super+Left", "old kept after grab fail");
        // successful grab
        flamewm::ShortcutRegistry reg3;
        CHECK(reg3.applyWithGrabStage(d2, grabAlways, 0, &err), "grab success");
    }
    // output identity stability
    {
        flamewm::OutputInfo o; o.connector="HDMI-1"; o.edidId="abc123";
        CHECK(o.durableKey()=="abc123:HDMI-1", "edid+connector key");
        flamewm::OutputInfo o2; o2.connector="HDMI-1"; o2.edidId="";
        CHECK(o2.durableKey()=="HDMI-1", "connector fallback");
        // durableKey stable across generation
        flamewm::OutputId id = flamewm::ScaleManager::makeOutputId("DP-1","hash1");
        CHECK(id.durableId=="hash1:DP-1", "makeOutputId durable");
        CHECK(flamewm::ScaleManager::durableKey(id)=="hash1:DP-1", "durableKey");
    }
    // selected-output UI model (Control-only: DisplaysPage zero-arg, ControlClient gated)
    {
        // Core DisplayManager durable identity still Control-independent — keep direct DM checks
        flamewm::DisplayManager dm;
        flamewm::DisplaySnapshot snap; snap.generation=1;
        flamewm::OutputInfo a; a.connector="HDMI-1"; a.edidId="h1"; a.connected=true; a.enabled=true;
        a.availableModes.push_back(flamewm::DisplayMode(1,1920,1080,60000));
        a.currentMode = a.availableModes[0]; a.shellScale=100;
        flamewm::OutputInfo b; b.connector="DP-1"; b.edidId="h2"; b.connected=true; b.enabled=true;
        b.availableModes.push_back(flamewm::DisplayMode(2,2560,1440,60000));
        b.currentMode=b.availableModes[0]; b.shellScale=100;
        snap.outputs.push_back(a); snap.outputs.push_back(b);
        dm.injectSnapshot(snap);
        dm.setSelectedOutputId(a.durableKey());
        CHECK(dm.selectedOutputId()==a.durableKey(), "selectedOutputId UI state");
        CHECK(dm.selectedOutput()!=0 && dm.selectedOutput()->connector=="HDMI-1", "selectedOutput resolves");
        // DisplaysPage is now Control-only zero-arg; selection UI state lives in page, scale via ControlClient
        flamewm::settings::DisplaysPage page;
        CHECK(page.selectedOutputId().empty(), "page initial no selection");
        CHECK(!page.hasSnapshot(), "page initial no snapshot");
        page.selectOutput(a.durableKey());
        CHECK(page.selectedOutputId()==a.durableKey(), "page selectedOutputId UI state");
        CHECK(page.selectedOutput()==0, "page selectedOutput null without snapshot");
        CHECK(page.modesForSelected().empty(), "page modesForSelected empty without snapshot");
        CHECK(page.scaleOptions().size()==5, "page scaleOptions 5 buckets");
        std::vector<int> opts = flamewm::settings::DisplaysPage::scaleOptions();
        bool has100=false, has125=false, has200=false;
        for(size_t i=0;i<opts.size();++i){ if(opts[i]==100) has100=true; if(opts[i]==125) has125=true; if(opts[i]==200) has200=true; }
        CHECK(has100 && has125 && has200, "scaleOptions contains 100/125/200");
        std::string err;
        // setScaleForSelected now requires ControlClient*; null client fails with error, does not mutate DM
        CHECK(!page.setScaleForSelected((flamewm::control::ControlClient*)0, 150, &err), "scale for selected without control fails");
        CHECK(!err.empty(), "scale without control reports error");
        CHECK(dm.snapshot().findByDurable(a.durableKey())->shellScale==100, "DM unmodified without Control");
        CHECK(dm.snapshot().findByDurable(b.durableKey())->shellScale==100, "unselected unchanged without Control");
        CHECK(!page.isPendingActive(), "page no pending without snapshot");
        std::string err2;
        CHECK(!page.keep((flamewm::control::ControlClient*)0, &err2), "keep without control fails");
        CHECK(!page.revert((flamewm::control::ControlClient*)0, &err2), "revert without control fails");
        CHECK(!page.setScaleForSelected((flamewm::control::ControlClient*)0, 110, &err2), "invalid scale without control fails");
    }
    // scale bounds 100-200 buckets only
    {
        flamewm::DisplayManager dm;
        flamewm::DisplaySnapshot snap; snap.generation=1;
        flamewm::OutputInfo o; o.connector="HDMI-1"; o.edidId="h1"; o.connected=true; o.enabled=true;
        snap.outputs.push_back(o); dm.injectSnapshot(snap);
        std::string k=o.durableKey(); std::string err;
        CHECK(!dm.setShellScale(k, 110, &err), "scale 110 rejected");
        CHECK(dm.setShellScale(k,125,&err), "125 ok");
        CHECK(dm.setShellScale(k,200,&err), "200 ok");
        CHECK(!dm.setShellScale(k,250,&err), "250 rejected");
    }
    // resolution tx model/timeout/crash/hotplug + do not persist unconfirmed
    {
        flamewm::DisplayManager dm;
        flamewm::DisplaySnapshot snap; snap.generation=10;
        flamewm::OutputInfo o; o.connector="HDMI-1"; o.edidId="h1"; o.connected=true; o.enabled=true;
        o.availableModes.push_back(flamewm::DisplayMode(1,1920,1080,60000));
        o.availableModes.push_back(flamewm::DisplayMode(2,2560,1440,60000));
        o.currentMode=o.availableModes[0];
        snap.outputs.push_back(o); dm.injectSnapshot(snap);
        dm.setSelectedOutputId(o.durableKey());
        std::string err;
        CHECK(dm.beginResolutionTx(o.durableKey(), o.availableModes[1], 1000, 5000, &err), "begin tx");
        CHECK(dm.pendingTx()!=0, "pending exists");
        CHECK(dm.pendingTx()->candidateMode.id==2, "candidate 2");
        // before confirm, persistedScales not for mode; check current snapshot still old mode (tx not auto-applied to snapshot until observer)
        // timeout
        CHECK(!dm.onTimeoutCheck(2000), "not timed out at 2000");
        CHECK(dm.onTimeoutCheck(7000), "timed out at 7000 reverted");
        CHECK(dm.pendingTx()==0, "no pending after timeout");
        // crash revert
        CHECK(dm.beginResolutionTx(o.durableKey(), o.availableModes[1], 8000, 5000, &err), "begin second tx");
        dm.onSettingsCrash();
        CHECK(dm.pendingTx()==0, "crash reverted");
        // hotplug stale: output disappeared
        CHECK(dm.beginResolutionTx(o.durableKey(), o.availableModes[1], 9000, 5000, &err), "begin third tx");
        flamewm::DisplaySnapshot fresh; fresh.generation=11;
        // fresh has no HDMI-1 (unplugged)
        dm.onHotplug(fresh);
        CHECK(dm.pendingTx()==0, "hotplug revert when output gone");
    }
    // independent opacities
    {
        flamewm::ConfigStore store;
        std::string err; flamewm::SettingsSnapshot s = store.current(); s.revision=2;
        s.desktopSelectionFillOpacity=10; s.windowSnapPreviewFillOpacity=50;
        CHECK(store.applySnapshot(s,&err), "independent opacities apply");
        CHECK(store.current().desktopSelectionFillOpacity==10, "desktop 10");
        CHECK(store.current().windowSnapPreviewFillOpacity==50, "snap 50");
        // also via DesktopPage distinct
        flamewm::settings::DesktopPage dp;
        CHECK(dp.setDesktopOpacity(0, &err), "desktop 0 ok borders strong at 0");
        CHECK(dp.setSnapOpacity(60, &err), "snap 60 ok");
        CHECK(dp.borderStrongAtZero(), "border strong at 0");
        CHECK(dp.desktopSelectionFillOpacity==0 && dp.windowSnapPreviewFillOpacity==60, "desktop page independent");
    }
    // exact About URLs central via productmetadata
    {
        CHECK(std::string(flamewm::ProductMetadata::donateUrl())=="https://paypal.me/LinsaFTW", "donate url exact");
        CHECK(std::string(flamewm::ProductMetadata::sourceUrl())=="https://github.com/arkflame/flamewm", "source url exact");
        CHECK(std::string(flamewm::ProductMetadata::websiteUrl())=="https://wm.arkflame.com", "website url exact");
        CHECK(flamewm::settings::AboutPage::donateUrl()=="https://paypal.me/LinsaFTW", "about donate via productmetadata");
    }
    // safe URI argv no shell interpolation
    {
        auto v = flamewm::SafeUriLauncher::buildXdgOpenArgv("https://example.com");
        CHECK(v.size()==2 && v[0]=="xdg-open" && v[1]=="https://example.com", "xdg-open argv vector");
        CHECK(!flamewm::SafeUriLauncher::isSafeUri("https://example.com; rm -rf /"), "semicolon uri rejected");
        CHECK(!flamewm::SafeUriLauncher::isSafeUri("https://example.com && evil"), "shell metachars rejected");
        // ensure no system() concatenation: buildXdgOpenArgv separates args
        std::string evil="https://example.com; echo pwned";
        auto v2 = flamewm::SafeUriLauncher::buildXdgOpenArgv(evil);
        CHECK(v2[1]==evil && v2.size()==2, "argv keeps uri as single arg not shell");
        CHECK(!flamewm::SafeUriLauncher::isSafeUri(evil), "evil uri isSafe false even though argv would contain it");
    }

    std::cout<<"ALL PASS\n";
    return 0;
}
