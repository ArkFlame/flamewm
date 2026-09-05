#ifndef FLAMEWM_UI_METRICS_H
#define FLAMEWM_UI_METRICS_H

namespace flamewm {

// Logical metrics at 100% scale (design tokens).
// Physical = logical * scalePercent / 100.
struct FlameMetrics {
    // baseline tokens at 100%
    int panelSize;      // default 48
    int titlebarSize;   // default 32
    int menuRowHeight;  // default 34
    int taskHitTarget;  // default 44
    int statusHitTarget;// default 40

    int visualBorder;   // 1
    int resizeHit;      // 6
    int controlRadius;  // 4
    int popupRadius;    // 8
    int floatRadius;    // 6-8

    int spacing4;
    int spacing8;
    int spacing12;
    int spacing16;
    int spacing24;

    static FlameMetrics defaults();

    // Scale helpers
    static int logicalToPhysical(int logical, int scalePercent);
    static int physicalToLogical(int physical, int scalePercent);

    // Compact/Default/Large variants
    struct Variant { int compact; int def; int large; };
    static Variant panelVariants();
    static Variant titlebarVariants();
    static Variant menuRowVariants();
    static Variant taskHitVariants();
    static Variant statusHitVariants();

    // Apply scale to this metric set (returns physical values struct)
    FlameMetrics scaled(int scalePercent) const;
};

// Supported discrete buckets: 100/125/150/175/200
bool isSupportedScale(int pct);
int clampToSupportedScale(int pct);
int nearestSupportedScale(int pct);

} // namespace flamewm
#endif // FLAMEWM_UI_METRICS_H
