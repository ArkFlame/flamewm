#ifndef FLAMEWM_UI_STYLE_BOX_H
#define FLAMEWM_UI_STYLE_BOX_H

#include "theme.h"

namespace flamewm {
namespace ui {
namespace style {

struct Box {
    int x;
    int y;
    int width;
    int height;

    Box();
    Box(int x_, int y_, int width_, int height_);

    bool empty() const;
    bool contains(int px, int py) const;
    Box inset(int amount) const;
};

// Engine-neutral material token shared by cards, controls and popovers.
struct Material {
    Color fill;
    Color border;
    Color shadow;
    int radius;
    int borderWidth;
    Material() : radius(0), borderWidth(0) {}
    Material(const Color& fill_, const Color& border_, const Color& shadow_,
             int radius_, int borderWidth_)
        : fill(fill_), border(border_), shadow(shadow_), radius(radius_), borderWidth(borderWidth_) {}
};

struct BoxStyle {
    Visual visual;
    Color fill;
    Color border;
    int radius;
    int borderWidth;
    Material material;

    BoxStyle() : radius(0), borderWidth(0) {}
    bool hasFill() const { return !fill.empty() || !material.fill.empty(); }
    bool hasBorder() const {
        return (!border.empty() || !material.border.empty()) &&
               (borderWidth > 0 || material.borderWidth > 0);
    }
};

struct OverlayRectStyle {
    Visual visual;
    Box rect;
    Color fill;
    Color border;
    float opacity;
    int radius;
    int borderWidth;
    Material material;

    OverlayRectStyle()
        : opacity(1.0f), radius(0), borderWidth(0) {}
    bool visible() const { return !rect.empty() && opacity > 0.0f; }
};

} // namespace style
} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_STYLE_BOX_H
