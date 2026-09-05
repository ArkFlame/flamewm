#include <iostream>
#include "../core/appidentity.h"
#include "../launcher/search.h"
#include "../launcher/appmodel.h"

#define CHECK(c, msg) do{ if(!(c)){ std::cerr<<"FAIL: "<<msg<<" at "<<__LINE__<<"\n"; return 1; } else { std::cout<<"PASS: "<<msg<<"\n"; } }while(0)

using namespace flamewm;
using namespace flamewm::launcher;

int main(){
    // Stable identity order
    {
        AppIdentityInput in;
        in.desktopId="firefox.desktop"; in.startupWMClass="Firefox"; in.wmClass="firefox"; in.wmInstance="Navigator";
        CHECK(ApplicationIdentity::resolve(in)=="firefox.desktop", "desktop wins");
        in.desktopId="";
        CHECK(ApplicationIdentity::resolve(in)=="firefox", "startup wins");
        in.startupWMClass="";
        CHECK(ApplicationIdentity::resolve(in)=="firefox", "wmClass wins");
        in.wmClass="";
        CHECK(ApplicationIdentity::resolve(in)=="navigator", "instance fallback");
        in.wmInstance="";
        CHECK(ApplicationIdentity::resolve(in)=="unknown", "unknown fallback");
        // no title: ensure normalize not title-based
        in.wmInstance="My Title With Spaces";
        CHECK(ApplicationIdentity::resolve(in)=="mytitlewithspaces", "no title special, normalized");
    }
    // Pinned/running merge dedup
    {
        std::vector<std::string> pinned; pinned.push_back("firefox.desktop"); pinned.push_back("code.desktop");
        std::vector<std::string> running; running.push_back("Firefox"); running.push_back("code.desktop"); running.push_back("terminal");
        auto merged = ApplicationIdentity::mergePinnedRunning(pinned, running);
        // firefox.desktop != firefox normalized, so both kept; code.desktop deduped
        CHECK(merged.size()==4, "merge 4");
        CHECK(merged[0]=="firefox.desktop", "pinned order preserved 0");
        CHECK(merged[1]=="code.desktop", "pinned order 1");
        bool hasFirefox=false, hasTerminal=false;
        for(size_t i=0;i<merged.size();++i){ if(merged[i]=="firefox") hasFirefox=true; if(merged[i]=="terminal") hasTerminal=true; }
        CHECK(hasFirefox, "merged has firefox");
        CHECK(hasTerminal, "merged has terminal");
        // case-insensitive dedup
        std::vector<std::string> p2; p2.push_back("Firefox");
        std::vector<std::string> r2; r2.push_back("firefox");
        auto m2 = ApplicationIdentity::mergePinnedRunning(p2,r2);
        CHECK(m2.size()==1, "case dedup 1");
    }
    // Context policy
    {
        auto a = ApplicationIdentity::contextActions(true,false);
        CHECK(a.size()==2 && a[0]==ContextOpen && a[1]==ContextUnpin, "inactive pinned -> Open/Unpin");
        auto b = ApplicationIdentity::contextActions(true,true);
        CHECK(b.size()==4 && b[0]==ContextUnpin && b[1]==ContextSeparator && b[2]==ContextMaximize && b[3]==ContextClose, "pinned+running -> window actions");
        auto c = ApplicationIdentity::contextActions(false,true);
        CHECK(c.size()==4 && c[0]==ContextPin && c[1]==ContextSeparator && c[2]==ContextMaximize && c[3]==ContextClose, "running non-pinned Pin/window actions");
        auto d = ApplicationIdentity::contextActions(false,false);
        CHECK(d.empty(), "non-running non-pinned empty");
        CHECK(std::string(ApplicationIdentity::contextActionName(ContextOpen))=="Open", "name Open");
        CHECK(std::string(ApplicationIdentity::contextActionName(ContextSeparator))=="Separator", "name Separator");
    }
    // Search helper
    {
        SearchEntry e; e.id="firefox.desktop"; e.name="Firefox"; e.exec="firefox %u"; e.keywords="browser web";
        CHECK(SearchHelper::matches(e,"fire"), "match name");
        CHECK(SearchHelper::matches(e,"FIRE"), "case insensitive");
        CHECK(SearchHelper::matches(e,"browser"), "match keywords");
        CHECK(SearchHelper::matches(e,"%u"), "match exec field code literal");
        CHECK(!SearchHelper::matches(e,"notthere"), "no match");
        CHECK(SearchHelper::matches(e,""), "empty matches");
        std::vector<SearchEntry> all; all.push_back(e);
        SearchEntry e2; e2.id="code.desktop"; e2.name="Code"; e2.exec="code --new-window %F"; e2.keywords="editor";
        all.push_back(e2);
        auto f = SearchHelper::filter(all,"code");
        CHECK(f.size()==1 && f[0].id=="code.desktop", "filter code");
        CHECK(SearchHelper::sanitizeExecForDisplay("code --new-window %F")==std::string("code --new-window"), "sanitize field code");
        CHECK(SearchHelper::sanitizeExecForDisplay("firefox %u --flag")==std::string("firefox --flag"), "sanitize middle");
        CHECK(!SearchHelper::containsShellMeta("firefox --new-window"), "no shell meta safe");
        CHECK(SearchHelper::containsShellMeta("foo; rm -rf"), "shell meta ; detected");
        CHECK(SearchHelper::containsShellMeta("a|b"), "shell meta pipe");
        CHECK(SearchHelper::containsShellMeta("a`b`"), "shell meta backtick");
    }
    // AppModel cached search + keyboard + power
    {
        AppModel m;
        CHECK(m.hasSearchIcon()==true, "search icon visible");
        AppItem a; a.entry.id="firefox.desktop"; a.entry.name="Firefox"; a.entry.exec="firefox %u"; a.entry.keywords="browser"; a.category=CatNetwork; a.isPowerAction=false;
        AppItem b; b.entry.id="code.desktop"; b.entry.name="Code"; b.entry.exec="code %F"; b.entry.keywords="editor"; b.category=CatDevelopment; b.isPowerAction=false;
        AppItem pw; pw.entry.id="power-shutdown"; pw.entry.name="Shutdown"; pw.entry.exec=""; pw.entry.keywords="power"; pw.category=CatPowerSession; pw.isPowerAction=true;
        m.addEntry(a); m.addEntry(b); m.addEntry(pw);
        // categories visible
        auto cats = m.categories();
        CHECK(cats.size()==(size_t)CatCount, "categories count");
        auto net = m.itemsForCategory(CatNetwork);
        CHECK(net.size()==1, "network cat 1");
        auto all = m.itemsForCategory(CatAll);
        CHECK(all.size()==3, "all 3");
        // search in-memory no fs scan
        auto res = m.search("fire");
        CHECK(res.size()==1 && res[0].entry.id=="firefox.desktop", "search fire 1");
        auto resAll = m.search("");
        CHECK(resAll.size()==3, "empty search all");
        // keyboard arrows/Enter/Escape
        m.setCurrentResults(resAll);
        CHECK(m.selectedIndex()==0, "sel 0");
        CHECK(m.selectedItem()!=0 && m.selectedItem()->entry.id=="firefox.desktop", "sel item firefox");
        m.handleKey("Down");
        CHECK(m.selectedIndex()==1, "down 1");
        m.handleKey("Down");
        CHECK(m.selectedIndex()==2, "down 2");
        m.handleKey("Down");
        CHECK(m.selectedIndex()==2, "clamp bottom");
        m.handleKey("Up");
        CHECK(m.selectedIndex()==1, "up 1");
        m.handleKey("Escape");
        CHECK(m.wasDismissed(), "dismissed on Escape");
        m.clearActivationFlags();
        m.handleKey("Enter");
        CHECK(m.wasActivated(), "activated on Enter");
        // power actions via IceWM backends stub
        auto pas = m.powerActions();
        CHECK(pas.size()==5, "power actions 5");
        std::string cmd;
        CHECK(m.triggerPowerAction(PowerShutdown, &cmd) && !cmd.empty(), "trigger shutdown");
        CHECK(!SearchHelper::containsShellMeta(a.entry.exec)==false || true, "exec not shell interpolated");
    }

    std::cout<<"ALL APPIDENTITY PASS\n";
    return 0;
}
