#include "material_painter.h"

#include <cctype>

namespace {

bool opaqueHexColor(const std::string& value) {
    if (value.size() != 7 || value[0] != '#') return false;
    for (size_t i = 1; i < value.size(); ++i) {
        if (!std::isxdigit(static_cast<unsigned char>(value[i]))) return false;
    }
    return true;
}

void paintColor(Graphics& g, int x, int y, unsigned width, unsigned height,
                const flamewm::ui::style::Color& color, unsigned radius) {
    if (width == 0 || height == 0 || !opaqueHexColor(color.value)) return;
    g.setColor(YColor(color.value.c_str()));
    g.fillRect(x, y, width, height, radius);
}

}

namespace flamewm {
namespace engine {
namespace icewm {
namespace ui {

void MaterialPainter::fill(Graphics& g, int x, int y, unsigned width, unsigned height,
                           const flamewm::ui::style::Color& color) {
    paintColor(g, x, y, width, height, color, 0);
}

void MaterialPainter::material(Graphics& g, const YRect& rect,
                               const flamewm::ui::style::Material& material) {
    if (!rect.nonempty()) return;
    const unsigned radius = material.radius > 0 ? static_cast<unsigned>(material.radius) : 0;
    paintColor(g, rect.x(), rect.y(), rect.width(), rect.height(), material.fill, radius);
    if (material.borderWidth <= 0 || !opaqueHexColor(material.border.value)) return;
    g.setColor(YColor(material.border.value.c_str()));
    g.setWideLines(static_cast<unsigned>(material.borderWidth));
    g.drawRect(rect.x(), rect.y(), rect.width() - 1, rect.height() - 1);
}

void MaterialPainter::box(Graphics& g, const flamewm::ui::style::BoxStyle& style,
                          const YRect& rect) {
    flamewm::ui::style::Material material = style.material;
    if (!style.fill.empty()) material.fill = style.fill;
    if (!style.border.empty()) material.border = style.border;
    if (style.radius > 0) material.radius = style.radius;
    if (style.borderWidth > 0) material.borderWidth = style.borderWidth;
    MaterialPainter::material(g, rect, material);
}

void MaterialPainter::material(Graphics& g, const YRect& rect,
                               const flamewm::ui::style::Theme& theme,
                               flamewm::ui::style::VisualRole role,
                               flamewm::ui::style::VisualStateMask state) {
    flamewm::ui::style::Material material;
    material.fill = theme.color(role, state);
    material.border = theme.border;
    material.radius = theme.settings.cardRadius;
    material.borderWidth = 0;
    MaterialPainter::material(g, rect, material);
}

} // namespace ui
} // namespace icewm
} // namespace engine
} // namespace flamewm
