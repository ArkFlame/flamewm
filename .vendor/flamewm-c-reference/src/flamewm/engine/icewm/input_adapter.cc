#include "flamewm/engine/icewm/input_adapter.h"

#include "flamewm/api/errors.h"

#if __has_include(<X11/Xlib.h>)
#include <X11/Xlib.h>
#define FLAMEWM_HAS_X11 1
#if __has_include("yxapp.h") && __has_include(<X11/extensions/Xrender.h>)
#include "yxapp.h"
#define FLAMEWM_HAS_YXAPP 1
#else
#define FLAMEWM_HAS_YXAPP 0
#endif
// All X headers (Xlib.h -> yxapp.h -> ypixmap.h -> Xrender.h -> Xutil.h)
// expect Status as macro/int. Undefine only after X parsing is complete
// so api::Status (from ports.h, included via input_adapter.h before X)
#ifdef Status
#undef Status
#endif
#else
#define FLAMEWM_HAS_X11 0
#define FLAMEWM_HAS_YXAPP 0
#endif



namespace flamewm {
namespace engine {
namespace icewm {

InputAdapter::InputAdapter() : display_(nullptr) {}

InputAdapter::InputAdapter(void* display) : display_(display) {}

InputAdapter::~InputAdapter() {}

void InputAdapter::setDisplay(void* display) {
    display_ = display;
}

void* InputAdapter::display() const {
    return display_;
}

namespace {
#if FLAMEWM_HAS_X11
api::Result<api::PointerPosition> queryPointer(Display* dpy, Window root) {
    Window child = None;
    Window rootRet = None;
    int rootX = 0;
    int rootY = 0;
    int winX = 0;
    int winY = 0;
    unsigned int mask = 0;
    if (dpy == nullptr || root == None)
        return api::Result<api::PointerPosition>::Err(
            api::Status::make(api::Error::Unavailable, "rootPointer: no display/root"));
    Bool ok = XQueryPointer(dpy, root, &rootRet, &child,
                            &rootX, &rootY, &winX, &winY, &mask);
    if (ok == True) {
        api::PointerPosition pos;
        pos.root = api::Point(rootX, rootY);
        // Output resolution requires RandR/EngineAccess; leave empty until
        // display topology is wired. Caller maps point->output.
        return api::Result<api::PointerPosition>::Ok(pos);
    }
    return api::Result<api::PointerPosition>::Err(
        api::Status::make(api::Error::Unavailable, "rootPointer: XQueryPointer failed"));
}
#endif
} // namespace

api::Result<api::PointerPosition> InputAdapter::rootPointer() {
#if FLAMEWM_HAS_X11
    // 1) Explicit Display injected via Bootstrap/setDisplay.
    if (display_ != nullptr) {
        Display* dpy = static_cast<Display*>(display_);
        Window root = DefaultRootWindow(dpy);
        api::Result<api::PointerPosition> r = queryPointer(dpy, root);
        if (r.ok())
            return r;
        return r;
    }
    // 2) Global YApplication (xapp) owns Display* — use it when Bootstrap
    //    hasn't injected a Display* explicitly. Pure X path, no D-Bus in
    //    hot path. xapp is valid only after YXApplication construction
    //    (IceWM main) and before teardown.
#if FLAMEWM_HAS_YXAPP
    if (::xapp != nullptr && ::xapp->display() != nullptr) {
        Display* dpy = ::xapp->display();
        Window root = ::xapp->root();
        if (root == None)
            root = DefaultRootWindow(dpy);
        api::Result<api::PointerPosition> r = queryPointer(dpy, root);
        if (r.ok())
            return r;
        return r;
    }
    if (::xapp != nullptr) {
        return api::Result<api::PointerPosition>::Err(
            api::Status::make(api::Error::Unavailable, "rootPointer: xapp has no display"));
    }
#endif
    return api::Result<api::PointerPosition>::Err(
        api::Status::make(api::Error::Unavailable, "rootPointer: no display (setDisplay/xapp)"));
#else
    (void)display_;
    return api::Result<api::PointerPosition>::Err(
        api::Status::make(api::Error::Unavailable, "rootPointer unavailable (no X11)"));
#endif
}

} // namespace icewm
} // namespace engine
} // namespace flamewm
