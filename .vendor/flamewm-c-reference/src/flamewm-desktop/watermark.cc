#include "watermark.h"
#include "ypaint.h"
#include "yimage.h"
#include "yicon.h"
#include <X11/Xlib.h>
#include <unistd.h>
#include <cstdlib>

namespace {
std::string installedWordmarkPath() {
    const char* dataDir = std::getenv("FLAMEWM_DATADIR");
    if (dataDir && dataDir[0]) {
        const std::string path = std::string(dataDir) + "/flamewm/branding/flamewm-wordmark.png";
        if (access(path.c_str(), R_OK) == 0) return path;
    }
    const char* paths[] = {
        "/usr/share/flamewm/branding/flamewm-wordmark.png",
        "/usr/local/share/flamewm/branding/flamewm-wordmark.png",
        "lib/flamewm/branding/flamewm-wordmark.png"
    };
    for (size_t i = 0; i < sizeof(paths) / sizeof(paths[0]); ++i)
        if (access(paths[i], R_OK) == 0) return paths[i];
    return std::string();
}
}

namespace flamewm {
namespace desktop {

WatermarkGeometry Watermark::compute(const WorkArea& wa,int wmW,int wmH,int padding){
    WatermarkGeometry g;
    g.w=wmW; g.h=wmH;
    g.x = wa.x + wa.w - wmW - padding;
    g.y = wa.y + wa.h - wmH - padding;
    if(g.x < wa.x) g.x = wa.x;
    if(g.y < wa.y) g.y = wa.y;
    return g;
}
bool Watermark::shouldShow(bool isFullscreen,bool userEnabled){
    if(!userEnabled) return false;
    if(isFullscreen) return false;
    return true;
}
void Watermark::draw(void* d, unsigned long drawable, void* rawGc,
                     const WorkArea& wa, bool isFullscreen, bool userEnabled){
    if (!shouldShow(isFullscreen, userEnabled)) return;
    Display* display = static_cast<Display*>(d);
    GC gc = static_cast<GC>(rawGc);
    const WatermarkGeometry geometry = compute(wa, 220, 73, 24);
    XWindowAttributes attributes;
    if (!XGetWindowAttributes(display, drawable, &attributes)) return;
    Graphics graphics(static_cast<Drawable>(drawable), attributes.width,
                      attributes.height, attributes.depth);
    const std::string path = installedWordmarkPath();
    if (!path.empty()) {
        // Load directly so the non-square wordmark keeps its aspect ratio.
        ref<YImage> image = YImage::load(upath(path.c_str()));
        if (image != null)
            graphics.drawImage(image, geometry.x, geometry.y,
                               static_cast<unsigned>(geometry.w),
                               static_cast<unsigned>(geometry.h), 0, 0);
    } else {
        ref<YIcon> icon = YIcon::getIcon("flamewm-wordmark");
        if (icon == null) return;
        icon->draw(graphics, geometry.x, geometry.y, geometry.w);
    }
    (void)gc;
}

} // namespace desktop
} // namespace flamewm
