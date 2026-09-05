#ifndef FLAMEWM_DESKTOP_RENDER_MATERIAL_X11_H
#define FLAMEWM_DESKTOP_RENDER_MATERIAL_X11_H

#include <X11/Xlib.h>

namespace flamewm {
namespace desktop {

// Blend before Xlib draw. Keeps desktop rendering correct without requiring
// desktop helper to become a compositor.
class MaterialX11 {
public:
    static unsigned long blend(unsigned long foreground, unsigned long background,
                               int opacity) {
        if (opacity < 0) opacity = 0;
        if (opacity > 100) opacity = 100;
        const unsigned f = static_cast<unsigned>(opacity);
        const unsigned b = 100U - f;
        const unsigned fr = (foreground >> 16) & 0xffU;
        const unsigned fg = (foreground >> 8) & 0xffU;
        const unsigned fb = foreground & 0xffU;
        const unsigned br = (background >> 16) & 0xffU;
        const unsigned bg = (background >> 8) & 0xffU;
        const unsigned bb = background & 0xffU;
        return (((fr * f + br * b) / 100U) << 16) |
               (((fg * f + bg * b) / 100U) << 8) |
               ((fb * f + bb * b) / 100U);
    }

    static void fill(Display* display, Drawable drawable, GC gc,
                     int x, int y, unsigned width, unsigned height,
                     unsigned long color) {
        XSetForeground(display, gc, color);
        XFillRectangle(display, drawable, gc, x, y, width, height);
    }
};

} // namespace desktop
} // namespace flamewm

#endif
