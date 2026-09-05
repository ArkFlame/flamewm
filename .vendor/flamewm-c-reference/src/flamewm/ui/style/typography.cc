#include "typography.h"

namespace flamewm {
namespace ui {
namespace style {

Typography Typography::defaults() {
    Typography typography;
    typography.family = "IBM Plex Sans";
    typography.size = 13;
    typography.sizeOffset = 0;
    typography.bold = false;
    typography.lineHeight = 16;
    typography.letterSpacing = 0;
    return typography;
}

} // namespace style
} // namespace ui
} // namespace flamewm
