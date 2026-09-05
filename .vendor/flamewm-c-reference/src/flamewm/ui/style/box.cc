#include "box.h"

namespace flamewm {
namespace ui {
namespace style {

Box::Box() : x(0), y(0), width(0), height(0) {
}

Box::Box(int x_, int y_, int width_, int height_)
    : x(x_), y(y_), width(width_), height(height_) {
}

bool Box::empty() const {
    return width <= 0 || height <= 0;
}

bool Box::contains(int px, int py) const {
    return px >= x && py >= y && px < x + width && py < y + height;
}

Box Box::inset(int amount) const {
    return Box(x + amount, y + amount, width - 2 * amount, height - 2 * amount);
}

} // namespace style
} // namespace ui
} // namespace flamewm
