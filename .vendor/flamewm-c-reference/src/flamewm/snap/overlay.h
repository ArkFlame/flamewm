#ifndef FLAMEWM_SNAP_OVERLAY_H
#define FLAMEWM_SNAP_OVERLAY_H

#include "types.h"
#include <string>

namespace flamewm {
namespace snap {

// Stub snap preview surface controller — no X window creation yet.
// When wired, this will manage an override-redirect, pointer-transparent
// X window with accent border + muted fill.
//
// Geometry+alpha interface only — pure and testable.
//
// V5 opacity key: WindowSnapPreviewFillOpacity 0..60 inclusive.
// Border remains visible at 0% fill.
//
// Fail/edge docs: no compositor/blur dependency; override-redirect so WM
// does not manage it; Input transparent so pointer events pass through.
class SnapOverlay {
public:
    static const int kOpacityMin = 0;
    static const int kOpacityMax = 60;
    static const int kOpacityDefault = 20;

    SnapOverlay() : visible_(false), rect_(), fillOpacity_(kOpacityDefault), borderVisible_(true) {}
    explicit SnapOverlay(int fillOpacity) : visible_(false), rect_(), fillOpacity_(fillOpacity), borderVisible_(true) {
        if (fillOpacity_ < kOpacityMin) fillOpacity_ = kOpacityMin;
        if (fillOpacity_ > kOpacityMax) fillOpacity_ = kOpacityMax;
    }

    bool visible() const { return visible_; }
    const Rect& rect() const { return rect_; }
    int fillOpacity() const { return fillOpacity_; }
    bool borderVisible() const { return borderVisible_; }

    // Clamp opacity to 0..60. Border stays visible even at 0.
    void setFillOpacity(int pct) {
        if (pct < kOpacityMin) pct = kOpacityMin;
        if (pct > kOpacityMax) pct = kOpacityMax;
        fillOpacity_ = pct;
    }

    // Effective alpha 0.0..1.0 derived from fillOpacity 0..60
    double fillAlpha() const { return fillOpacity_ / 100.0; }

    // Geometry is always from geometryFor (preview==commit)
    void show(const Rect& r) { rect_ = r; visible_ = true; }
    void update(const Rect& r) { rect_ = r; }
    void hide() { visible_ = false; }

    // No X creation yet — when implemented:
    //   create(): XCreateWindow override-redirect, SelectInput None,
    //             shape/transparent input, draw accent border + muted fill.
    //   destroy(): XDestroyWindow.
    bool needsXCreate() const { return false; }

private:
    bool visible_;
    Rect rect_;
    int fillOpacity_;
    bool borderVisible_;
};

} // namespace snap
} // namespace flamewm
#endif
