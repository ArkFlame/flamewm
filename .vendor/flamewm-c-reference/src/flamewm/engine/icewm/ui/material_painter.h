#ifndef FLAMEWM_ENGINE_ICEWM_UI_MATERIAL_PAINTER_H
#define FLAMEWM_ENGINE_ICEWM_UI_MATERIAL_PAINTER_H

#include "flamewm/ui/style/box.h"
#include "flamewm/ui/style/theme.h"
#include "yimage.h"
#include "yicon.h"
#include "ypaint.h"
#include "base.h"
#include "yrect.h"

namespace flamewm {
namespace engine {
namespace icewm {
namespace ui {

// Small X11 drawing bridge. State selection stays with controls; painting
// remains shared so fallback glyphs and image scaling cannot diverge.
class MaterialPainter {
public:
    static void fill(Graphics& g, const YRect& rect, const flamewm::ui::style::Color& color) {
        fill(g, rect.x(), rect.y(), rect.width(), rect.height(), color);
    }

    static void fill(Graphics& g, int x, int y, unsigned width, unsigned height,
                     const flamewm::ui::style::Color& color);

    static void material(Graphics& g, const YRect& rect,
                         const flamewm::ui::style::Material& material);

    static void material(Graphics& g, const YRect& rect,
                         const flamewm::ui::style::Theme& theme,
                         flamewm::ui::style::VisualRole role,
                         flamewm::ui::style::VisualStateMask state = 0);

    static void box(Graphics& g, const flamewm::ui::style::BoxStyle& style,
                    const YRect& rect);

    static void image(Graphics& g, ref<YImage> image, int x, int y,
                      unsigned width, unsigned height) {
        if (image == null || width == 0 || height == 0) return;
        g.drawImage(image, x, y, width, height, 0, 0);
    }

    static bool icon(Graphics& g, ref<YIcon> iconImage, int x, int y,
                     unsigned size) {
        return iconImage != null && iconImage->draw(g, x, y, size);
    }

    static void text(Graphics& g, YFont font, const flamewm::ui::style::Color& color,
                     const char* value, int x, int y) {
        if (!font || !value) return;
        g.setFont(font);
        if (!color.empty()) g.setColor(YColor(color.value.c_str()));
        g.drawString(x, y, value);
    }
};

} // namespace ui
} // namespace icewm
} // namespace engine
} // namespace flamewm

#endif
