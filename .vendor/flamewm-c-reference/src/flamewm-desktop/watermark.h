#ifndef FLAMEWM_DESKTOP_WATERMARK_H
#define FLAMEWM_DESKTOP_WATERMARK_H

#include "layout.h"

namespace flamewm {
namespace desktop {

// Passive desktop-layer visual, pointer-transparent, behind normal windows,
// aligned to usable work area bottom-right, adapts to taskbar edge/output, subtle.
struct WatermarkGeometry {
    int x; int y; int w; int h;
};

class Watermark {
public:
    // Compute bottom-right position inside work area with padding, given watermark size and scale
    static WatermarkGeometry compute(const WorkArea& wa, int wmW, int wmH, int padding);
    // Visibility: hidden during fullscreen (caller supplies flag)
    static bool shouldShow(bool isFullscreen, bool userEnabled);
    static void draw(void* display, unsigned long drawable, void* gc,
                     const WorkArea& wa, bool isFullscreen, bool userEnabled);
};

} // namespace desktop
} // namespace flamewm

#endif
