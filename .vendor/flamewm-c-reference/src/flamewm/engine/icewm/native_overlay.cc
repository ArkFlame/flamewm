#include "flamewm/engine/icewm/native_overlay.h"

#if __has_include(<X11/Xlib.h>)
#include <X11/Xlib.h>
#include <X11/Xatom.h>
#define FLAMEWM_HAS_X11 1
#if __has_include(<X11/extensions/shape.h>)
#define Status int
#include <X11/extensions/shape.h>
#undef Status
#define FLAMEWM_HAS_SHAPE 1
#else
#define FLAMEWM_HAS_SHAPE 0
#endif
#if __has_include(<X11/extensions/Xfixes.h>)
#include <X11/extensions/Xfixes.h>
#define FLAMEWM_HAS_XFIXES 1
#else
#define FLAMEWM_HAS_XFIXES 0
#endif
#ifdef Status
#undef Status
#endif
#else
#define FLAMEWM_HAS_X11 0
#define FLAMEWM_HAS_SHAPE 0
#define FLAMEWM_HAS_XFIXES 0
#endif

namespace flamewm {
namespace engine {
namespace icewm {

NativeOverlay::NativeOverlay()
    : display_(nullptr)
#if FLAMEWM_HAS_X11
    , window_(None)
#else
    , window_(0)
#endif
    , visible_(false)
    , geometry_()
    , hasDisplay_(false)
    , accentColor_(0xe53935U), fillColor_(0x7f1d1dU), opacity_(20), inset_(2) {}

NativeOverlay::NativeOverlay(void* display)
    : display_(display)
#if FLAMEWM_HAS_X11
    , window_(None)
#else
    , window_(0)
#endif
    , visible_(false)
    , geometry_()
    , hasDisplay_(display != nullptr)
    , accentColor_(0xe53935U), fillColor_(0x7f1d1dU), opacity_(20), inset_(2) {}

NativeOverlay::~NativeOverlay() {
    destroy();
}

void NativeOverlay::setDisplay(void* display) {
    if (visible_) {
        hide();
        destroy();
    }
    display_ = display;
    hasDisplay_ = (display != nullptr);
}

void* NativeOverlay::display() const {
    return display_;
}

bool NativeOverlay::isVisible() const {
    return visible_;
}

api::Rect NativeOverlay::geometry() const {
    return geometry_;
}

void NativeOverlay::setAccentColor(uint32_t color) { accentColor_ = color & 0xffffffU; }
void NativeOverlay::setFillColor(uint32_t color) { fillColor_ = color & 0xffffffU; }
void NativeOverlay::setOpacity(unsigned percent) { opacity_ = percent > 60 ? 60 : percent; }
void NativeOverlay::setInset(unsigned inset) { inset_ = inset; }
uint32_t NativeOverlay::accentColor() const { return accentColor_; }
uint32_t NativeOverlay::fillColor() const { return fillColor_; }
unsigned NativeOverlay::opacity() const { return opacity_; }
unsigned NativeOverlay::inset() const { return inset_; }

#if FLAMEWM_HAS_X11
static unsigned long overlayPixel(uint32_t color) {
    return static_cast<unsigned long>(color & 0xffffffU);
}
#endif

bool NativeOverlay::createWindow(const api::Rect& rect) {
#if FLAMEWM_HAS_X11
    if (!hasDisplay_ || display_ == nullptr) {
        // Keep lifecycle testable and harmless when X11 display unavailable.
        window_ = static_cast<Window>(1);
        return true;
    }
#endif
    if (!rect.valid()) {
        return false;
    }
#if FLAMEWM_HAS_X11
    Display* dpy = static_cast<Display*>(display_);
    Window root = DefaultRootWindow(dpy);
    int screen = DefaultScreen(dpy);
    unsigned long bg = 0x3a000000UL; // translucent dark without compositor dep
    // Lightweight: override_redirect, no decorations, no WM management.
    XSetWindowAttributes attrs;
    attrs.override_redirect = True;
    attrs.background_pixel = overlayPixel(fillColor_);
    attrs.border_pixel = 0;
    attrs.colormap = DefaultColormap(dpy, screen);
    // Do not select input events — pointer-transparent.
    attrs.event_mask = ExposureMask | StructureNotifyMask;
    attrs.do_not_propagate_mask = ButtonPressMask | ButtonReleaseMask |
                                  PointerMotionMask | KeyPressMask | KeyReleaseMask;

    unsigned long mask = CWOverrideRedirect | CWBackPixel | CWBorderPixel |
                         CWEventMask | CWDontPropagate;

    Window w = XCreateWindow(dpy, root,
                             rect.x, rect.y,
                             static_cast<unsigned int>(rect.w),
                             static_cast<unsigned int>(rect.h),
                             0, CopyFromParent, InputOutput, CopyFromParent,
                             mask, &attrs);
    if (w == None) {
        return false;
    }

    // Pointer-transparent: empty input shape so clicks fall through.
#if FLAMEWM_HAS_SHAPE
    // Shape extension may be absent at runtime; guard with query.
    int shapeEvent = 0, shapeError = 0;
    if (XShapeQueryExtension(dpy, &shapeEvent, &shapeError)) {
        // Empty input shape.
        XShapeCombineMask(dpy, w, ShapeInput, 0, 0, None, ShapeSet);
    }
#endif
#if FLAMEWM_HAS_XFIXES
    // XFixes input shape keeps overlay pass-through when Shape is unavailable.
    int fixesEvent = 0, fixesError = 0;
    if (XFixesQueryExtension(dpy, &fixesEvent, &fixesError)) {
        XserverRegion empty = XFixesCreateRegion(dpy, nullptr, 0);
        XFixesSetWindowShapeRegion(dpy, w, 2, 0, 0, empty);
        XFixesDestroyRegion(dpy, empty);
    }
#endif
    // Compositor-independent: no ARGB / composite requirement. Plain window
    // with background; opacity handled via XRender if available, otherwise
    // solid fill is acceptable fallback.

    window_ = w;
    XSetWindowBorder(dpy, window_, overlayPixel(accentColor_));
    XSetWindowBorderWidth(dpy, window_, inset_ == 0 ? 1 : inset_);
    Atom opacityAtom = XInternAtom(dpy, "_NET_WM_WINDOW_OPACITY", False);
    unsigned long opacityValue = static_cast<unsigned long>(
        (static_cast<unsigned long long>(opacity_) * 0xffffffffULL) / 100ULL);
    XChangeProperty(dpy, window_, opacityAtom, XA_CARDINAL, 32, PropModeReplace,
                    reinterpret_cast<unsigned char*>(&opacityValue), 1);
    return true;
#else
    (void)rect;
    // Headless stub: track without real window.
    window_ = 1;
    return true;
#endif
}

void NativeOverlay::applyGeometry(const api::Rect& rect) {
#if FLAMEWM_HAS_X11
    if (window_ != None && display_ != nullptr) {
        Display* dpy = static_cast<Display*>(display_);
        XMoveResizeWindow(dpy, window_,
                          rect.x, rect.y,
                          static_cast<unsigned int>(rect.w),
                          static_cast<unsigned int>(rect.h));
        XSetWindowBorder(dpy, window_, overlayPixel(accentColor_));
        XSetWindowBorderWidth(dpy, window_, inset_ == 0 ? 1 : inset_);
        XFlush(dpy);
    }
#else
    (void)rect;
#endif
}

bool NativeOverlay::show(const api::Rect& rect) {
    if (!rect.valid()) {
        return false;
    }
#if FLAMEWM_HAS_X11
    if (window_ == None) {
        if (!createWindow(rect)) {
            return false;
        }
    } else {
        applyGeometry(rect);
    }
    if (display_ != nullptr) {
        Display* dpy = static_cast<Display*>(display_);
        XSetWindowBackground(dpy, window_, overlayPixel(fillColor_));
        XClearWindow(dpy, window_);
        XMapRaised(dpy, window_);
        XFlush(dpy);
    }
#else
    if (window_ == 0) {
        if (!createWindow(rect)) {
            return false;
        }
    }
#endif
    geometry_ = rect;
    visible_ = true;
    return true;
}

void NativeOverlay::move(const api::Rect& rect) {
    if (!visible_) {
        return;
    }
    if (!rect.valid()) {
        return;
    }
    geometry_ = rect;
    applyGeometry(rect);
}

void NativeOverlay::hide() {
    if (!visible_) {
        return;
    }
#if FLAMEWM_HAS_X11
    if (window_ != None && display_ != nullptr) {
        Display* dpy = static_cast<Display*>(display_);
        XUnmapWindow(dpy, window_);
        XFlush(dpy);
    }
#endif
    visible_ = false;
}

void NativeOverlay::destroy() {
#if FLAMEWM_HAS_X11
    if (window_ != None && display_ != nullptr) {
        Display* dpy = static_cast<Display*>(display_);
        XDestroyWindow(dpy, window_);
        XFlush(dpy);
    }
    window_ = None;
#else
    window_ = 0;
#endif
    visible_ = false;
    geometry_ = api::Rect();
}

} // namespace icewm
} // namespace engine
} // namespace flamewm
