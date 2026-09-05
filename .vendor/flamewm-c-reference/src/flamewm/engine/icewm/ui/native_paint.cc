#include "native_paint.h"

#include "ypaint.h"
#include "ywindow.h"
#include "yrect.h"

namespace flamewm {
namespace engine {
namespace icewm {
namespace ui {

void NativePaint::repaint(YWindow& window) {
    if (window.destroyed() || window.width() < 2 || window.height() < 2) return;
    GraphicsBuffer(&window).paint();
}

void NativePaint::repaint(YWindow& window, const YRect& rect) {
    if (window.destroyed() || window.width() < 2 || window.height() < 2 ||
        !rect.nonempty()) return;
    GraphicsBuffer(&window).paint(rect);
}

} // namespace ui
} // namespace icewm
} // namespace engine
} // namespace flamewm
