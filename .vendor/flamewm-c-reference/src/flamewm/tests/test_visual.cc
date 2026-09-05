#include <iostream>
#include <string>
#include "../ui/palette.h"
#include "../ui/metrics.h"
#include "../ui/iconroles.h"
#include "../ui/iconresolver.h"
#include "../ui/popoveranchor.h"
#include "../core/scalemanager.h"

#define CHECK(c, msg) do{ if(!(c)){ std::cerr<<"FAIL: "<<msg<<" at "<<__LINE__<<"\n"; return 1; } else { std::cout<<"PASS: "<<msg<<"\n"; } }while(0)

int main() {
    // Palette: single source of Flame red, pitch-black surfaces
    {
        flamewm::FlamePalette p = flamewm::FlamePalette::defaults();
        flamewm::FlamePalette p2 = flamewm::FlamePalette::defaultInstance();
        CHECK(p.accent == "#EF4048", "palette accent Flame red");
        CHECK(p2.accent == "#EF4048", "defaultInstance same");
        CHECK(p.surface == "#202326", "surface background");
        CHECK(p.surfaceRaised == "#25282B", "surfaceRaised");
        CHECK(p.accentHover != p.accent, "hover derived not raw");
        CHECK(p.accentPressed != p.accent, "pressed derived not raw");
        CHECK(p.accentMuted != p.accent, "muted derived not raw");
        CHECK(flamewm::FlamePalette::isFlameRed("#EF4048"), "isFlameRed true");
        CHECK(!flamewm::FlamePalette::isFlameRed("#FF0000"), "isFlameRed false for generic red");
        // Hover/pressed/muted must be distinct family not scattered raw outside
        CHECK(p.accentHover == "#F2575E", "hover value");
        CHECK(p.accentPressed == "#D9363E", "pressed value");
        CHECK(p.accentMuted == "#6C3034", "muted value");
        // border/danger not raw red confusion
        CHECK(p.border == "#3A3F44", "border");
        CHECK(p.danger == "#E53935", "danger distinct from accent");
    }

    // Metrics: logical -> physical 100/150 and per-output via ScaleManager (no global mutation)
    {
        CHECK(flamewm::FlameMetrics::logicalToPhysical(48, 100) == 48, "48@100=48");
        CHECK(flamewm::FlameMetrics::logicalToPhysical(48, 150) == 72, "48@150=72");
        CHECK(flamewm::FlameMetrics::logicalToPhysical(4, 150) == 6, "4@150=6");
        CHECK(flamewm::FlameMetrics::logicalToPhysical(8, 125) == 10, "8@125=10");
        // Spacing baseline tokens
        flamewm::FlameMetrics m = flamewm::FlameMetrics::defaults();
        CHECK(m.spacing4 == 4 && m.spacing8 == 8 && m.spacing12 == 12, "spacing tokens");
        CHECK(m.visualBorder == 1, "visual border 1");
        CHECK(m.resizeHit == 6, "resize hit 6");
        CHECK(m.controlRadius == 4, "control radius 4");
        CHECK(m.popupRadius == 8, "popup radius 8");
        // Scaled metrics per-output bucket (no global mutation)
        flamewm::FlameMetrics s150 = m.scaled(150);
        CHECK(s150.panelSize == 72, "scaled panel 48->72");
        CHECK(s150.spacing4 == 6, "scaled spacing4 4->6");
        // Per-output manager: different outputs have different scales
        flamewm::ScaleManager sm;
        flamewm::OutputId a = flamewm::ScaleManager::makeOutputId("HDMI-1", "hashA");
        flamewm::OutputId b = flamewm::ScaleManager::makeOutputId("eDP-1", "hashB");
        sm.setScale(a, 100);
        sm.setScale(b, 150);
        CHECK(sm.toPhysical(48, a) == 48, "per-output a 100");
        CHECK(sm.toPhysical(48, b) == 72, "per-output b 150");
    }

    // Scale bucket bounds
    {
        CHECK(flamewm::isSupportedScale(100), "bucket 100");
        CHECK(flamewm::isSupportedScale(125), "bucket 125");
        CHECK(flamewm::isSupportedScale(150), "bucket 150");
        CHECK(flamewm::isSupportedScale(175), "bucket 175");
        CHECK(flamewm::isSupportedScale(200), "bucket 200");
        CHECK(!flamewm::isSupportedScale(110), "bucket 110 not supported");
        CHECK(!flamewm::isSupportedScale(99), "bucket 99 not");
        CHECK(!flamewm::isSupportedScale(201), "bucket 201 not");
        CHECK(flamewm::nearestSupportedScale(112) == 100, "nearest 112->100");
        CHECK(flamewm::nearestSupportedScale(130) == 125, "nearest 130->125");
        CHECK(flamewm::nearestSupportedScale(160) == 150, "nearest 160->150");
        CHECK(flamewm::clampToSupportedScale(90) == 100, "clamp low");
        CHECK(flamewm::clampToSupportedScale(300) == 200, "clamp high");
    }

    // Icon fallback chain: FlameOverride -> Breeze -> system -> packaged -> builtin
    {
        flamewm::SimpleIconResolver r;
        // Breeze path when available
        r.setBreezeAvailable(true);
        r.setSystemThemeAvailable(true);
        r.setPackagedFallbackAvailable(true);
        flamewm::IconLookupResult res = r.resolve(flamewm::IconRoleSearch, 100, 0);
        CHECK(res.source == flamewm::IconSourceBreeze, "search -> Breeze");
        CHECK(res.renderMode == flamewm::RenderSymbolic, "search symbolic");
        CHECK(res.physicalSize == 16, "search 16@100");

        // Check FullColor preserved for Launcher
        flamewm::IconLookupResult lr = r.resolve(flamewm::IconRoleLauncher, 100, 0);
        CHECK(lr.renderMode == flamewm::RenderFullColor, "launcher FullColor preserved");
        // Check 150 scaling
        flamewm::IconLookupResult sr150 = r.resolve(flamewm::IconRoleSearch, 150, 0);
        CHECK(sr150.physicalSize == 24, "search 16@150=24");

        // Flame override takes precedence
        r.setFlameOverride(flamewm::IconRoleSearch, "/flame/custom/search.svg");
        res = r.resolve(flamewm::IconRoleSearch, 100, 0);
        CHECK(res.source == flamewm::IconSourceFlameOverride, "override wins");
        r.clearFlameOverride(flamewm::IconRoleSearch);
        res = r.resolve(flamewm::IconRoleSearch, 100, 0);
        CHECK(res.source == flamewm::IconSourceBreeze, "after clear back to Breeze");

        // Disable Breeze -> System
        r.setBreezeAvailable(false);
        res = r.resolve(flamewm::IconRoleSearch, 100, 0);
        CHECK(res.source == flamewm::IconSourceSystemTheme, "Breeze off -> System");

        // Disable System -> Packaged
        r.setSystemThemeAvailable(false);
        res = r.resolve(flamewm::IconRoleSearch, 100, 0);
        CHECK(res.source == flamewm::IconSourcePackagedFallback, "System off -> Packaged");

        // Disable Packaged -> Builtin
        r.setPackagedFallbackAvailable(false);
        res = r.resolve(flamewm::IconRoleSearch, 100, 0);
        CHECK(res.source == flamewm::IconSourceBuiltinGlyph, "all off -> Builtin");
        CHECK(res.pathOrGlyph == "[Q]" || !res.pathOrGlyph.empty(), "builtin glyph present");

        // RenderMode symbolic recolor vs preserve: Symbolic recolor flag, Brand/FullColor preserve
        // Already checked Launcher FullColor; check Settings categories are Symbolic
        r.setBreezeAvailable(true);
        r.setSystemThemeAvailable(true);
        r.setPackagedFallbackAvailable(true);
        flamewm::IconLookupResult ar = r.resolve(flamewm::IconRoleAppearance, 100, 0);
        CHECK(ar.renderMode == flamewm::RenderSymbolic, "Appearance Symbolic->recolor");

        // Registry metadata present
        size_t n = 0;
        const flamewm::IconFallbackEntry* reg = flamewm::iconFallbackRegistry(&n);
        CHECK(n >= 6, "registry has at least 6");
        CHECK(reg != 0, "registry non-null");
    }

    // Popover anchor: clamped on negative origins, shared across popovers
    {
        // Negative origin output (left monitor)
        flamewm::Rect output(-1920, 0, 1920, 1080);
        flamewm::Rect anchor(-1900, 1040, 32, 40); // on bottom-edge panel
        // Bottom edge: popover above anchor, left-aligned, clamped within output
        flamewm::PopoverPlacement pl = flamewm::anchorPopover(flamewm::PanelEdgeBottom, output, anchor, 200, 300, 48, 4);
        CHECK(pl.x >= output.x && pl.x + pl.w <= output.x + output.w, "anchor bottom clamped X within output with negative origin");
        CHECK(pl.y >= output.y && pl.y + pl.h <= output.y + output.h, "anchor bottom clamped Y");
        // Negative origin still positive placement calculation
        CHECK(pl.x >= -1920, "negative origin not underflow");

        // Top edge case
        flamewm::Rect output2(0, 0, 1920, 1080);
        flamewm::Rect anchor2(10, 0, 32, 40);
        flamewm::PopoverPlacement pl2 = flamewm::anchorPopover(flamewm::PanelEdgeTop, output2, anchor2, 200, 300, 48, 4);
        CHECK(pl2.y == anchor2.y + anchor2.h + 4, "top edge below anchor");

        // Right edge
        flamewm::Rect anchor3(1880, 100, 40, 32);
        flamewm::PopoverPlacement pl3 = flamewm::anchorPopover(flamewm::PanelEdgeRight, output2, anchor3, 200, 300, 48, 4);
        CHECK(pl3.x == anchor3.x - 4 - 200, "right edge left of anchor");

        // Left edge
        flamewm::Rect anchor4(0, 100, 40, 32);
        flamewm::PopoverPlacement pl4 = flamewm::anchorPopover(flamewm::PanelEdgeLeft, output2, anchor4, 200, 300, 48, 4);
        CHECK(pl4.x == anchor4.x + anchor4.w + 4, "left edge right of anchor");

        // Clamp when popover would overflow right side
        flamewm::Rect anchorFar(1800, 100, 32, 32);
        flamewm::PopoverPlacement pl5 = flamewm::anchorPopover(flamewm::PanelEdgeBottom, output2, anchorFar, 400, 200, 48, 4);
        CHECK(pl5.x + pl5.w <= output2.x + output2.w, "clamp overflow right");
        CHECK(pl5.clampedX, "clampedX flagged");

        // Popover larger than output -> pinned to output origin
        flamewm::PopoverPlacement pl6 = flamewm::anchorPopover(flamewm::PanelEdgeBottom, output2, anchor2, 3000, 3000, 48, 4);
        CHECK(pl6.x == output2.x && pl6.y == output2.y, "oversized pinned to origin");
    }

    // ScaleManager cache: font/icon keys by role+scale+themeGeneration, bounded retirement
    {
        flamewm::ScaleManager sm;
        // Populate cache for generations 0..5
        for (int gen = 0; gen < 6; ++gen) {
            for (int scale = 100; scale <= 200; scale += 25) if (flamewm::isSupportedScale(scale)) {
                sm.cachedPhysicalSize(16, scale, gen);
                sm.cachedPhysicalSize(32, scale, gen);
            }
        }
        sm.setThemeGeneration(5);
        size_t after = sm.cacheSize();
        // With kMax=3, should keep only generations 3,4,5
        CHECK(after <= 3 * 5 * 2 + 5, "cache bounded after retirement");
        // Current generation still retrievable
        CHECK(sm.cachedPhysicalSize(16, 150, 5) == flamewm::FlameMetrics::logicalToPhysical(16, 150), "cached 16@150 gen5");
        // Older generation evicted -> recompute still correct
        sm.setThemeGeneration(10);
        CHECK(sm.cachedPhysicalSize(16, 100, 10) == 16, "gen10 cache after retirement");
    }

    std::cout << "ALL VISUAL PASS\n";
    return 0;
}
