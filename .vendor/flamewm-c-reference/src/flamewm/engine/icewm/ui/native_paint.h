#ifndef FLAMEWM_ENGINE_ICEWM_UI_NATIVE_PAINT_H
#define FLAMEWM_ENGINE_ICEWM_UI_NATIVE_PAINT_H

class YRect;
class YWindow;

namespace flamewm {
namespace engine {
namespace icewm {
namespace ui {

class NativePaint {
public:
    static void repaint(YWindow& window);
    static void repaint(YWindow& window, const YRect& rect);
};

} // namespace ui
} // namespace icewm
} // namespace engine
} // namespace flamewm

#endif
