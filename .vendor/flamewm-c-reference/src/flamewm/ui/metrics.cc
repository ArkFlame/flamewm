#include "metrics.h"

namespace flamewm {

FlameMetrics FlameMetrics::defaults() {
    FlameMetrics m;
    m.panelSize       = 48;
    m.titlebarSize    = 32;
    m.menuRowHeight   = 34;
    m.taskHitTarget   = 44;
    m.statusHitTarget = 40;
    m.visualBorder    = 1;
    m.resizeHit       = 6;
    m.controlRadius   = 4;
    m.popupRadius     = 8;
    m.floatRadius     = 6;
    m.spacing4        = 4;
    m.spacing8        = 8;
    m.spacing12       = 12;
    m.spacing16       = 16;
    m.spacing24       = 24;
    return m;
}

int FlameMetrics::logicalToPhysical(int logical, int scalePercent) {
    // round half up: (logical*scale + 50)/100 not quite; use (logical*scale+50)/100? simpler (logical*scale+50)/...
    // Use integer arithmetic with rounding: (logical * pct + 50)/100
    return (logical * scalePercent + 50) / 100;
}

int FlameMetrics::physicalToLogical(int physical, int scalePercent) {
    if (scalePercent == 0) return physical;
    return (physical * 100 + scalePercent / 2) / scalePercent;
}

FlameMetrics::Variant FlameMetrics::panelVariants()      { return Variant{40,48,56}; }
FlameMetrics::Variant FlameMetrics::titlebarVariants()   { return Variant{28,32,36}; }
FlameMetrics::Variant FlameMetrics::menuRowVariants()    { return Variant{30,34,38}; }
FlameMetrics::Variant FlameMetrics::taskHitVariants()    { return Variant{36,44,52}; }
FlameMetrics::Variant FlameMetrics::statusHitVariants()  { return Variant{32,40,48}; }

FlameMetrics FlameMetrics::scaled(int scalePercent) const {
    FlameMetrics out = *this;
    out.panelSize       = logicalToPhysical(panelSize, scalePercent);
    out.titlebarSize    = logicalToPhysical(titlebarSize, scalePercent);
    out.menuRowHeight   = logicalToPhysical(menuRowHeight, scalePercent);
    out.taskHitTarget   = logicalToPhysical(taskHitTarget, scalePercent);
    out.statusHitTarget = logicalToPhysical(statusHitTarget, scalePercent);
    out.visualBorder    = logicalToPhysical(visualBorder, scalePercent);
    out.resizeHit       = logicalToPhysical(resizeHit, scalePercent);
    out.controlRadius   = logicalToPhysical(controlRadius, scalePercent);
    out.popupRadius     = logicalToPhysical(popupRadius, scalePercent);
    out.floatRadius     = logicalToPhysical(floatRadius, scalePercent);
    out.spacing4        = logicalToPhysical(spacing4, scalePercent);
    out.spacing8        = logicalToPhysical(spacing8, scalePercent);
    out.spacing12       = logicalToPhysical(spacing12, scalePercent);
    out.spacing16       = logicalToPhysical(spacing16, scalePercent);
    out.spacing24       = logicalToPhysical(spacing24, scalePercent);
    return out;
}

bool isSupportedScale(int pct) {
    return pct == 100 || pct == 125 || pct == 150 || pct == 175 || pct == 200;
}

int clampToSupportedScale(int pct) {
    if (pct <= 100) return 100;
    if (pct <= 125) return 125;
    if (pct <= 150) return 150;
    if (pct <= 175) return 175;
    return 200;
}

int nearestSupportedScale(int pct) {
    const int buckets[] = {100,125,150,175,200};
    int best = buckets[0];
    int bestDist = 1000;
    for (int i = 0; i < 5; ++i) {
        int d = pct >= buckets[i] ? pct - buckets[i] : buckets[i] - pct;
        if (d < bestDist) { bestDist = d; best = buckets[i]; }
    }
    return best;
}

} // namespace flamewm
