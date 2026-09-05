#include "flamewm/engine/icewm/display_adapter.h"

#include <cstdint>
#include <string>
#include <vector>

#if __has_include("config.h")
#include "config.h"
#endif

#if defined(CONFIG_XRANDR)
#if __has_include(<X11/Xlib.h>)
#include <X11/Xlib.h>
#endif
#if __has_include(<X11/extensions/Xrandr.h>)
#include <X11/extensions/Xrandr.h>
#define FLAMEWM_DA_HAS_RANDR 1
#else
#define FLAMEWM_DA_HAS_RANDR 0
#endif
#if __has_include("yxapp.h")
#include "yxapp.h"
#define FLAMEWM_DA_HAS_XAPP 1
#else
#define FLAMEWM_DA_HAS_XAPP 0
#endif
#else
#define FLAMEWM_DA_HAS_RANDR 0
#define FLAMEWM_DA_HAS_XAPP 0
#endif

#ifdef Status
#undef Status
#endif

namespace flamewm {
namespace engine {
namespace icewm {

struct DisplayAdapter::Impl {
    uint64_t generation;
    uint64_t nextTxId;
    struct Captured {
        bool valid;
        api::TransactionId tx;
        std::string outputKey;
        int x;
        int y;
        unsigned w;
        unsigned h;
        uint64_t oldMode;
        int oldRotation;
        Captured()
            : valid(false)
            , tx()
            , outputKey()
            , x(0)
            , y(0)
            , w(0)
            , h(0)
            , oldMode(0)
            , oldRotation(0) {}
    } captured;
    Impl() : generation(0), nextTxId(1), captured() {}
};

DisplayAdapter::DisplayAdapter() : impl_(new Impl()) {}
DisplayAdapter::~DisplayAdapter() { delete impl_; }

#if FLAMEWM_DA_HAS_RANDR && FLAMEWM_DA_HAS_XAPP

namespace {

static int refreshMilliHzRR(const XRRModeInfo* mi) {
    if (mi == 0) return 60000;
    if (mi->dotClock && mi->hTotal && mi->vTotal) {
        long long milli = (1000000LL * static_cast<long long>(mi->dotClock)) /
                          (static_cast<long long>(mi->hTotal) * static_cast<long long>(mi->vTotal));
        if (milli > 0) return static_cast<int>(milli);
    }
    return 60000;
}

static bool displayAvailableDA(Display* dpy) {
    if (dpy == 0) return false;
    if (!::xrandr.supported) return false;
    return true;
}

} // namespace

api::Result<api::DisplaySnapshot> DisplayAdapter::queryFresh() {
    if (xapp == 0 || xapp->display() == 0) {
        return api::Result<api::DisplaySnapshot>::Err(api::Error::Unavailable, "no display");
    }
    Display* dpy = xapp->display();
    Window root = xapp->root();
    if (!displayAvailableDA(dpy)) {
        return api::Result<api::DisplaySnapshot>::Err(api::Error::Unavailable, "xrandr not supported");
    }
    XRRScreenResources* res = XRRGetScreenResources(dpy, root);
    if (res == 0) {
        return api::Result<api::DisplaySnapshot>::Err(api::Error::Unavailable, "XRRGetScreenResources failed");
    }
    api::DisplaySnapshot snap;
    snap.generation = ++impl_->generation;
    if (snap.generation == 0) snap.generation = ++impl_->generation;

    RROutput primaryOut = XRRGetOutputPrimary(dpy, root);

    for (int i = 0; i < res->noutput; ++i) {
        XRROutputInfo* oi = XRRGetOutputInfo(dpy, res, res->outputs[i]);
        if (oi == 0) continue;
        api::OutputSnapshot out;
        std::string conn;
        if (oi->name != 0 && oi->nameLen > 0) conn.assign(oi->name, static_cast<size_t>(oi->nameLen));
        else if (oi->name != 0) conn = std::string(oi->name);
        out.connector = conn;
        out.edidIdentity = std::string();
        out.id = api::OutputId(conn);
        out.connected = (oi->connection == RR_Connected);
        out.primary = (primaryOut != 0 && res->outputs[i] == primaryOut);
        out.shellScalePercent = 100;

        for (int m = 0; m < oi->nmode; ++m) {
            RRMode mid = oi->modes[m];
            for (int k = 0; k < res->nmode; ++k) {
                if (res->modes[k].id == mid) {
                    XRRModeInfo* mi = &res->modes[k];
                    api::DisplayMode dm;
                    dm.id = api::ModeId(static_cast<uint64_t>(mid));
                    dm.resolution = api::Size(static_cast<int>(mi->width), static_cast<int>(mi->height));
                    dm.refreshMilliHz = refreshMilliHzRR(mi);
                    dm.preferred = false;
                    out.modes.push_back(dm);
                    break;
                }
            }
        }

        if (oi->crtc != 0) {
            XRRCrtcInfo* ci = XRRGetCrtcInfo(dpy, res, oi->crtc);
            if (ci != 0) {
                bool enabled = (ci->mode != 0);
                if (enabled) {
                    out.geometry = api::Rect(ci->x, ci->y, static_cast<int>(ci->width), static_cast<int>(ci->height));
                    out.currentMode = api::ModeId(static_cast<uint64_t>(ci->mode));
                } else {
                    out.geometry = api::Rect(0, 0, 0, 0);
                    out.currentMode = api::ModeId();
                }
                if (enabled && ci->mode != 0) {
                    bool found = false;
                    for (size_t mm = 0; mm < out.modes.size(); ++mm) {
                        if (out.modes[mm].id.value == static_cast<uint64_t>(ci->mode)) { found = true; break; }
                    }
                    if (!found) {
                        for (int k = 0; k < res->nmode; ++k) {
                            if (res->modes[k].id == ci->mode) {
                                XRRModeInfo* mi = &res->modes[k];
                                api::DisplayMode dm;
                                dm.id = api::ModeId(static_cast<uint64_t>(ci->mode));
                                dm.resolution = api::Size(static_cast<int>(mi->width), static_cast<int>(mi->height));
                                dm.refreshMilliHz = refreshMilliHzRR(mi);
                                dm.preferred = false;
                                out.modes.push_back(dm);
                                out.currentMode = dm.id;
                                break;
                            }
                        }
                    }
                }
                XRRFreeCrtcInfo(ci);
            }
        } else {
            out.geometry = api::Rect(0, 0, 0, 0);
            out.currentMode = api::ModeId();
        }

        snap.outputs.push_back(out);
        XRRFreeOutputInfo(oi);
    }
    XRRFreeScreenResources(res);
    snap.pending.active = false;
    return api::Result<api::DisplaySnapshot>::Ok(snap);
}

api::Status DisplayAdapter::capture() {
    if (xapp == 0 || xapp->display() == 0) {
        return api::Status::make(api::Error::Unavailable, "no display");
    }
    Display* dpy = xapp->display();
    if (!displayAvailableDA(dpy)) {
        return api::Status::make(api::Error::Unavailable, "xrandr not supported");
    }
    Window root = xapp->root();
    XRRScreenResources* res = XRRGetScreenResources(dpy, root);
    if (res == 0) {
        return api::Status::make(api::Error::Unavailable, "XRRGetScreenResources failed");
    }
    XRRFreeScreenResources(res);
    return api::Status::Ok();
}

api::Status DisplayAdapter::applyMode(api::OutputId output, api::ModeId mode, api::TransactionId* outTx) {
    if (!output.valid()) {
        return api::Status::make(api::Error::InvalidArgument, "invalid output");
    }
    if (!mode.valid()) {
        return api::Status::make(api::Error::InvalidArgument, "invalid mode");
    }
    if (xapp == 0 || xapp->display() == 0) {
        return api::Status::make(api::Error::Unavailable, "no display");
    }
    Display* dpy = xapp->display();
    Window root = xapp->root();
    if (!displayAvailableDA(dpy)) {
        return api::Status::make(api::Error::Unavailable, "xrandr not supported");
    }
    XRRScreenResources* res = XRRGetScreenResources(dpy, root);
    if (res == 0) {
        return api::Status::make(api::Error::Unavailable, "XRRGetScreenResources failed");
    }
    bool foundOutput = false;
    bool modeOk = false;
    RRCrtc targetCrtc = 0;
    int capX = 0, capY = 0;
    unsigned capW = 0, capH = 0;
    RRMode capMode = 0;
    Rotation capRot = 0;
    std::string outputKey = output.key;

    for (int i = 0; i < res->noutput; ++i) {
        XRROutputInfo* oi = XRRGetOutputInfo(dpy, res, res->outputs[i]);
        if (oi == 0) continue;
        std::string conn;
        if (oi->name != 0 && oi->nameLen > 0) conn.assign(oi->name, static_cast<size_t>(oi->nameLen));
        else if (oi->name != 0) conn = std::string(oi->name);
        bool isTarget = (conn == outputKey);
        if (!isTarget) {
            size_t pos = outputKey.rfind(':');
            std::string outConn = (pos == std::string::npos) ? outputKey : outputKey.substr(pos + 1);
            if (conn == outConn) isTarget = true;
        }
        if (isTarget) {
            foundOutput = true;
            if (oi->connection != RR_Connected) {
                XRRFreeOutputInfo(oi);
                XRRFreeScreenResources(res);
                return api::Status::make(api::Error::NotFound, "output not connected");
            }
            if (oi->crtc == 0) {
                XRRFreeOutputInfo(oi);
                XRRFreeScreenResources(res);
                return api::Status::make(api::Error::NotFound, "output has no crtc");
            }
            targetCrtc = oi->crtc;
            for (int m = 0; m < oi->nmode; ++m) {
                if (static_cast<uint64_t>(oi->modes[m]) == mode.value) { modeOk = true; break; }
            }
            if (!modeOk) {
                XRRFreeOutputInfo(oi);
                XRRFreeScreenResources(res);
                return api::Status::make(api::Error::InvalidArgument, "mode not available for output");
            }
            XRRCrtcInfo* ci = XRRGetCrtcInfo(dpy, res, oi->crtc);
            if (ci == 0) {
                XRRFreeOutputInfo(oi);
                XRRFreeScreenResources(res);
                return api::Status::make(api::Error::Unavailable, "XRRGetCrtcInfo failed");
            }
            capX = ci->x;
            capY = ci->y;
            capW = ci->width;
            capH = ci->height;
            capMode = ci->mode;
            capRot = ci->rotation;
            XRRFreeCrtcInfo(ci);
            XRRFreeOutputInfo(oi);
            break;
        }
        XRRFreeOutputInfo(oi);
    }
    if (!foundOutput) {
        XRRFreeScreenResources(res);
        return api::Status::make(api::Error::NotFound, "output not found");
    }
    if (!modeOk) {
        XRRFreeScreenResources(res);
        return api::Status::make(api::Error::InvalidArgument, "mode not available");
    }

    XRRCrtcInfo* ci2 = XRRGetCrtcInfo(dpy, res, targetCrtc);
    if (ci2 == 0) {
        XRRFreeScreenResources(res);
        return api::Status::make(api::Error::Unavailable, "XRRGetCrtcInfo failed before apply");
    }
    RRMode newMode = static_cast<RRMode>(mode.value);
    int st = XRRSetCrtcConfig(dpy, res, targetCrtc, CurrentTime,
                              ci2->x, ci2->y, newMode, ci2->rotation,
                              ci2->outputs, ci2->noutput);
    XRRFreeCrtcInfo(ci2);
    XRRFreeScreenResources(res);
    if (st != Success) {
        return api::Status::make(api::Error::InternalFailure, "XRRSetCrtcConfig failed");
    }

    api::TransactionId tx(static_cast<uint64_t>(impl_->nextTxId++));
    if (impl_->nextTxId == 0) impl_->nextTxId = 1;
    if (tx.value == 0) {
        tx = api::TransactionId(static_cast<uint64_t>(impl_->nextTxId++));
        if (impl_->nextTxId == 0) impl_->nextTxId = 1;
    }
    impl_->captured.valid = true;
    impl_->captured.tx = tx;
    impl_->captured.outputKey = outputKey;
    impl_->captured.x = capX;
    impl_->captured.y = capY;
    impl_->captured.w = capW;
    impl_->captured.h = capH;
    impl_->captured.oldMode = static_cast<uint64_t>(capMode);
    impl_->captured.oldRotation = static_cast<int>(capRot);

    if (outTx) *outTx = tx;
    return api::Status::Ok();
}

api::Status DisplayAdapter::restore(api::TransactionId tx) {
    if (!tx.valid()) {
        return api::Status::make(api::Error::InvalidArgument, "invalid tx");
    }
    if (!impl_->captured.valid) {
        return api::Status::make(api::Error::NotFound, "no captured state");
    }
    if (impl_->captured.tx != tx) {
        return api::Status::make(api::Error::NotFound, "tx mismatch");
    }
    if (xapp == 0 || xapp->display() == 0) {
        impl_->captured.valid = false;
        return api::Status::Ok();
    }
    Display* dpy = xapp->display();
    Window root = xapp->root();
    if (!displayAvailableDA(dpy)) {
        impl_->captured.valid = false;
        return api::Status::Ok();
    }
    XRRScreenResources* res = XRRGetScreenResources(dpy, root);
    if (res == 0) {
        impl_->captured.valid = false;
        return api::Status::make(api::Error::Unavailable, "XRRGetScreenResources failed on revert");
    }
    RRCrtc targetCrtc = 0;
    bool found = false;
    for (int i = 0; i < res->noutput; ++i) {
        XRROutputInfo* oi = XRRGetOutputInfo(dpy, res, res->outputs[i]);
        if (oi == 0) continue;
        std::string conn;
        if (oi->name != 0 && oi->nameLen > 0) conn.assign(oi->name, static_cast<size_t>(oi->nameLen));
        else if (oi->name != 0) conn = std::string(oi->name);
        bool isTarget = (conn == impl_->captured.outputKey);
        if (!isTarget) {
            size_t pos = impl_->captured.outputKey.rfind(':');
            std::string outConn = (pos == std::string::npos) ? impl_->captured.outputKey : impl_->captured.outputKey.substr(pos + 1);
            if (conn == outConn) isTarget = true;
        }
        if (isTarget) {
            if (oi->crtc != 0) { targetCrtc = oi->crtc; found = true; }
            XRRFreeOutputInfo(oi);
            break;
        }
        XRRFreeOutputInfo(oi);
    }
    if (!found) {
        XRRFreeScreenResources(res);
        impl_->captured.valid = false;
        return api::Status::Ok();
    }
    XRRCrtcInfo* ci = XRRGetCrtcInfo(dpy, res, targetCrtc);
    if (ci == 0) {
        XRRFreeScreenResources(res);
        impl_->captured.valid = false;
        return api::Status::make(api::Error::Unavailable, "XRRGetCrtcInfo failed on revert");
    }
    RRMode revertMode = static_cast<RRMode>(impl_->captured.oldMode);
    int st = XRRSetCrtcConfig(dpy, res, targetCrtc, CurrentTime,
                              impl_->captured.x, impl_->captured.y,
                              revertMode, static_cast<Rotation>(impl_->captured.oldRotation),
                              ci->outputs, ci->noutput);
    XRRFreeCrtcInfo(ci);
    XRRFreeScreenResources(res);
    impl_->captured.valid = false;
    if (st != Success) {
        return api::Status::make(api::Error::InternalFailure, "XRRSetCrtcConfig revert failed");
    }
    return api::Status::Ok();
}

#else // !FLAMEWM_DA_HAS_RANDR || !FLAMEWM_DA_HAS_XAPP

api::Result<api::DisplaySnapshot> DisplayAdapter::queryFresh() {
    (void)impl_;
    return api::Result<api::DisplaySnapshot>::Err(api::Error::Unavailable, "xrandr not available");
}

api::Status DisplayAdapter::capture() {
    return api::Status::make(api::Error::Unavailable, "xrandr not available");
}

api::Status DisplayAdapter::applyMode(api::OutputId, api::ModeId, api::TransactionId* outTx) {
    if (outTx) *outTx = api::TransactionId();
    return api::Status::make(api::Error::Unavailable, "xrandr not available");
}

api::Status DisplayAdapter::restore(api::TransactionId) {
    return api::Status::make(api::Error::Unavailable, "xrandr not available");
}

#endif // FLAMEWM_DA_HAS_RANDR

} // namespace icewm
} // namespace engine
} // namespace flamewm
