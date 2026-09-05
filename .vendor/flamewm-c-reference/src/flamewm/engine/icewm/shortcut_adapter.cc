#include "flamewm/engine/icewm/shortcut_adapter.h"

#include <algorithm>
#include <cctype>
#include <vector>

#if defined(HAVE_X11) && __has_include(<X11/Xlib.h>)
#include <X11/Xlib.h>
#include <X11/keysym.h>
#include <X11/X.h>
#ifdef Status
#undef Status
#endif
#define FLAMEWM_HAS_X11 1
#if __has_include("yconfig.h")
#include "yconfig.h"
#define FLAMEWM_HAS_YCONFIG 1
#else
#define FLAMEWM_HAS_YCONFIG 0
#endif
// YXApplication drags Xrender.h; only include if available
#if __has_include(<X11/extensions/Xrender.h>) && __has_include("yxapp.h")
#include "yxapp.h"
#ifdef Status
#undef Status
#endif
#define FLAMEWM_HAS_YXAPP 1
#else
#define FLAMEWM_HAS_YXAPP 0
#if FLAMEWM_HAS_X11
extern class YXApplication* xapp;
#endif
#endif
#if __has_include("keysyms.h")
#include "keysyms.h"
#endif
#else
#define FLAMEWM_HAS_X11 0
#define FLAMEWM_HAS_YCONFIG 0
#define FLAMEWM_HAS_YXAPP 0
#endif

#ifndef IS_POINTER
#define IS_POINTER(k) (0x0000FEE0U <= (k) && (k) <= 0x0000FEFFU)
#endif
#ifndef IS_XF86KEY
#define IS_XF86KEY(k) (0x1008FE01U <= (k) && (k) <= 0x1008FFFFU)
#endif

namespace flamewm {
namespace engine {
namespace icewm {

namespace {

static std::string trimCopy(const std::string& s) {
    size_t a = 0;
    while (a < s.size() && (s[a] == ' ' || s[a] == '\t' || s[a] == '\n' || s[a] == '\r')) ++a;
    size_t b = s.size();
    while (b > a && (s[b - 1] == ' ' || s[b - 1] == '\t' || s[b - 1] == '\n' || s[b - 1] == '\r')) --b;
    return s.substr(a, b - a);
}
static std::string toLowerCopy(const std::string& s) {
    std::string o = s;
    for (size_t i = 0; i < o.size(); ++i) o[i] = static_cast<char>(::tolower(static_cast<unsigned char>(o[i])));
    return o;
}
static bool isEscapeNorm(const std::string& n) { return n == "Escape"; }

#if FLAMEWM_HAS_X11
struct NativeBinding {
    std::string action;
    std::string keysymStr;
    unsigned key;
    unsigned short kfMod;
    unsigned xm[2];
    unsigned char kc[2];
    bool supered;
    bool isPointer;
    int button;
    NativeBinding() : key(0), kfMod(0), supered(false), isPointer(false), button(0) {
        xm[0] = xm[1] = 0;
        kc[0] = kc[1] = 0;
    }
};

static bool getDisplayRoot(Display** outDpy, Window* outRoot) {
#if FLAMEWM_HAS_YXAPP
    if (::xapp && ::xapp->display()) {
        *outDpy = ::xapp->display();
        Window r = ::xapp->root();
        if (r == None) r = DefaultRootWindow(*outDpy);
        *outRoot = r;
        return true;
    }
#endif
    *outDpy = nullptr;
    *outRoot = None;
    return false;
}

static unsigned kfToXMask(unsigned short kf) {
    unsigned m = 0;
#if FLAMEWM_HAS_YXAPP
    if (::xapp) {
        if (kf & 1) m |= ShiftMask;
        if (kf & 2) m |= ControlMask;
        if (kf & 4) { if (::xapp->AltMask) m |= ::xapp->AltMask; }
        if (kf & 8) { if (::xapp->MetaMask) m |= ::xapp->MetaMask; }
        if (kf & 16) { if (::xapp->SuperMask) m |= ::xapp->SuperMask; }
        if (kf & 32) { if (::xapp->HyperMask) m |= ::xapp->HyperMask; }
        if (kf & 64) { if (::xapp->ModeSwitchMask) m |= ::xapp->ModeSwitchMask; }
        return m;
    }
#endif
    if (kf & 1) m |= ShiftMask;
    if (kf & 2) m |= ControlMask;
    if (kf & 4) m |= Mod1Mask;
    if (kf & 16) m |= Mod4Mask;
    if (kf & 8) m |= Mod1Mask;
    if (kf & 32) m |= Mod4Mask;
    return m;
}

static bool computeNative(const std::string& normalized, NativeBinding* out, std::string* err) {
    out->keysymStr = normalized;
    out->key = 0;
    out->kfMod = 0;
    out->xm[0] = out->xm[1] = 0;
    out->kc[0] = out->kc[1] = 0;
    out->supered = false;
    out->isPointer = false;
    out->button = 0;

#if FLAMEWM_HAS_YCONFIG
    unsigned k = 0;
    unsigned short mo = 0;
    if (!YConfig::parseKey(normalized.c_str(), &k, &mo)) {
        if (err) *err = std::string("invalid keysym: ") + normalized;
        return false;
    }
    out->key = k;
    out->kfMod = mo;
#else
    if (err) *err = "no YConfig::parseKey";
    return false;
#endif

    if (IS_POINTER(out->key)) {
        out->isPointer = true;
        out->button = static_cast<int>(out->key - XK_Pointer_Button1 + Button1);
        if (out->button < Button1 || out->button > Button3) {
            if (err) *err = std::string("unsupported pointer button: ") + normalized;
            return false;
        }
    }

    if (out->kfMod) {
        unsigned km = kfToXMask(out->kfMod);
        bool ok = true;
#if FLAMEWM_HAS_YXAPP
        if (::xapp) {
            if ((out->kfMod & 4) && ::xapp->AltMask == 0) ok = false;
            if ((out->kfMod & 8) && ::xapp->MetaMask == 0) ok = false;
            if ((out->kfMod & 16) && ::xapp->SuperMask == 0) ok = false;
            if ((out->kfMod & 32) && ::xapp->HyperMask == 0) ok = false;
            if ((out->kfMod & 64) && ::xapp->ModeSwitchMask == 0) ok = false;
        }
#endif
        if (!ok || km == 0) {
            if (err) *err = std::string("unsupported modifier for: ") + normalized;
            return false;
        }
        out->xm[0] = out->xm[1] = static_cast<unsigned>(km);
#if FLAMEWM_HAS_YXAPP
        if (::xapp && ::xapp->WinMask && (out->kfMod & (2 | 4)) == (2 | 4)) {
            extern bool modSuperIsCtrlAlt;
            if (modSuperIsCtrlAlt) out->supered = true;
        }
#endif
        if (out->isPointer && out->supered) {
#if FLAMEWM_HAS_YXAPP
            if (::xapp && ::xapp->WinMask) {
                out->xm[1] = ::xapp->WinMask | (out->xm[0] & ~(ControlMask | ::xapp->AltMask));
            }
#endif
        }
    }

    if (!out->isPointer && out->key) {
        Display* dpy = nullptr;
        Window root = None;
        bool haveDpy = getDisplayRoot(&dpy, &root);
#if FLAMEWM_HAS_YXAPP
        if (haveDpy && ::xapp) {
            YKeycodeMap map = ::xapp->getKeycodeMap();
            if (map) {
                const bool sh = ((' ' < out->key && out->key < 'a') || ('z' < out->key && out->key <= 0xFF) || IS_XF86KEY(out->key));
                int n = 0;
                for (int i = map.min; i <= map.max; ++i) {
                    if (map.map[(i - map.min) * map.per] == out->key) {
                        out->kc[n] = static_cast<unsigned char>(i & 0xFF);
                        if (++n >= 2) break;
                    }
                    if (sh && map.map[(i - map.min) * map.per + 1] == out->key) {
                        out->xm[n] |= ShiftMask;
                        out->kc[n] = static_cast<unsigned char>(i & 0xFF);
                        if (++n >= 2) break;
                    }
                }
            } else if (dpy) {
                unsigned k2 = out->key;
                unsigned short m2 = static_cast<unsigned short>(out->xm[0]);
                ::xapp->unshift(&k2, &m2);
                out->xm[0] = m2;
                out->kc[0] = XKeysymToKeycode(dpy, KeySym(k2));
            }
        } else if (haveDpy && dpy) {
            out->kc[0] = XKeysymToKeycode(dpy, KeySym(out->key));
        }
#else
        if (haveDpy && dpy) {
            out->kc[0] = XKeysymToKeycode(dpy, KeySym(out->key));
        }
#endif
        if (out->kc[0] == 0) {
            if (err) *err = std::string("no keycode for: ") + normalized;
            return false;
        }
    }
    return true;
}

struct GrabRec {
    bool isButton;
    int button;
    unsigned char kc;
    unsigned mod;
};

static std::vector<GrabRec> expandGrabs(const NativeBinding& nb) {
    std::vector<GrabRec> out;
#if FLAMEWM_HAS_YXAPP
    unsigned lockMask = LockMask;
    unsigned numLock = ::xapp ? ::xapp->NumLockMask : 0;
#else
    unsigned lockMask = LockMask;
    unsigned numLock = 0;
#endif
    if (nb.isPointer) {
        unsigned mods[8];
        int cnt = 0;
        for (int i = 0; i < 2; ++i) {
            if (i == 0 || (nb.xm[i] != nb.xm[0] && nb.xm[i])) {
                mods[cnt++] = nb.xm[i];
                mods[cnt++] = nb.xm[i] | lockMask;
                if (numLock) {
                    mods[cnt++] = nb.xm[i] | numLock;
                    mods[cnt++] = nb.xm[i] | numLock | lockMask;
                }
            }
        }
        if (cnt == 0) {
            mods[cnt++] = 0;
            mods[cnt++] = lockMask;
            if (numLock) {
                mods[cnt++] = numLock;
                mods[cnt++] = numLock | lockMask;
            }
        }
        for (int i = 0; i < cnt; ++i) {
            GrabRec g; g.isButton = true; g.button = nb.button; g.kc = 0; g.mod = mods[i];
            out.push_back(g);
        }
        return out;
    }
    for (int k = 0; k < 2 && nb.kc[k]; ++k) {
        unsigned base = nb.xm[k];
        unsigned mods[8];
        int cnt = 0;
        mods[cnt++] = base;
        mods[cnt++] = base | lockMask;
        if (numLock) {
            mods[cnt++] = base | numLock;
            mods[cnt++] = base | lockMask | numLock;
        }
        if (nb.supered) {
#if FLAMEWM_HAS_YXAPP
            if (::xapp && ::xapp->WinMask) {
                unsigned wmod = base & ~(ControlMask | ::xapp->AltMask);
                wmod |= ::xapp->WinMask;
                mods[cnt++] = wmod;
                mods[cnt++] = wmod | lockMask;
                if (numLock) {
                    mods[cnt++] = wmod | numLock;
                    mods[cnt++] = wmod | lockMask | numLock;
                }
            }
#endif
        }
        for (int i = 0; i < cnt; ++i) {
            GrabRec g; g.isButton = false; g.button = 0; g.kc = nb.kc[k]; g.mod = mods[i];
            out.push_back(g);
        }
    }
    return out;
}

static void doGrab(Display* dpy, Window root, const GrabRec& g) {
    if (g.isButton) {
        XGrabButton(dpy, static_cast<unsigned>(g.button), g.mod, root, True, ButtonPressMask, GrabModeAsync, GrabModeAsync, None, None);
    } else {
        XGrabKey(dpy, static_cast<int>(g.kc), g.mod, root, False, GrabModeAsync, GrabModeAsync);
    }
}
static void doUngrab(Display* dpy, Window root, const GrabRec& g) {
    if (g.isButton) {
        XUngrabButton(dpy, static_cast<unsigned>(g.button), g.mod, root);
    } else {
        XUngrabKey(dpy, static_cast<int>(g.kc), g.mod, root);
    }
}

static bool s_grabFailed = false;
static XErrorHandler s_prevHandler = nullptr;
static int grabErrorHandler(Display* dpy, XErrorEvent* ev) {
    if (ev->error_code == BadAccess || ev->error_code == BadValue || ev->error_code == BadWindow) {
        s_grabFailed = true;
        return 0;
    }
    if (s_prevHandler) return s_prevHandler(dpy, ev);
    return 0;
}

static bool dispatchAction(const std::string& action) {
    // No D-Bus in key-event path. Consume matched native binding locally.
    (void)action;
    return true;
}

#endif // FLAMEWM_HAS_X11

} // namespace

struct ShortcutAdapter::Impl {
    std::map<std::string, api::KeyBinding> pending;
    std::map<std::string, api::KeyBinding> committed;
    bool hasPending;
#if FLAMEWM_HAS_X11
    std::map<std::string, NativeBinding> committedNative;
    std::map<std::string, NativeBinding> pendingNative;
    std::map<std::string, std::vector<GrabRec> > committedGrabs;
    std::map<std::string, std::vector<GrabRec> > pendingGrabs;
#endif
    Impl() : hasPending(false) {}
};

ShortcutAdapter::ShortcutAdapter() : impl_(new Impl()) {}
ShortcutAdapter::~ShortcutAdapter() { delete impl_; }

api::Status ShortcutAdapter::prepare(const std::map<std::string, api::KeyBinding>& desired) {
    std::map<std::string, api::KeyBinding> normDesired;
    std::map<std::string, std::string> lowerByAction;

    for (std::map<std::string, api::KeyBinding>::const_iterator it = desired.begin(); it != desired.end(); ++it) {
        const std::string& action = it->first;
        std::string raw = trimCopy(it->second.key);
        if (isEscapeNorm(raw)) raw.clear();
        if (raw.empty()) {
            normDesired[action] = api::KeyBinding::Unassigned();
            continue;
        }
        std::string n = trimCopy(raw);
        if (n.size() > 64) {
            return api::Status::make(api::Error::InvalidArgument, std::string("binding too long for ") + action);
        }
#if FLAMEWM_HAS_YCONFIG && FLAMEWM_HAS_X11
        unsigned k = 0; unsigned short m = 0;
        if (!YConfig::parseKey(n.c_str(), &k, &m)) {
            return api::Status::make(api::Error::InvalidArgument, std::string("invalid keysym: ") + n + " for " + action);
        }
#else
        if (n.empty()) {
            return api::Status::make(api::Error::InvalidArgument, std::string("invalid binding for ") + action);
        }
#endif
        normDesired[action] = api::KeyBinding(n);
        lowerByAction[action] = toLowerCopy(n);
    }

    for (std::map<std::string, std::string>::const_iterator a = lowerByAction.begin(); a != lowerByAction.end(); ++a) {
        std::map<std::string, std::string>::const_iterator b = a; ++b;
        for (; b != lowerByAction.end(); ++b) {
            if (a->second == b->second) {
                return api::Status::make(api::Error::Conflict, std::string("conflict: ") + a->first + " vs " + b->first + " (" + a->second + ")");
            }
        }
    }

#if FLAMEWM_HAS_X11
    std::map<std::string, NativeBinding> pNative;
    std::map<std::string, std::vector<GrabRec> > pGrabs;
    for (std::map<std::string, api::KeyBinding>::const_iterator it = normDesired.begin(); it != normDesired.end(); ++it) {
        if (!it->second.assigned()) continue;
        NativeBinding nb;
        nb.action = it->first;
        std::string err;
        if (!computeNative(it->second.key, &nb, &err)) {
            return api::Status::make(api::Error::InvalidArgument, err);
        }
        pNative[it->first] = nb;
        pGrabs[it->first] = expandGrabs(nb);
    }
    impl_->pendingNative = pNative;
    impl_->pendingGrabs = pGrabs;
#endif

    impl_->pending = normDesired;
    impl_->hasPending = true;
    return api::Status::Ok();
}

api::Status ShortcutAdapter::commit() {
    if (!impl_->hasPending) {
        return api::Status::make(api::Error::Unavailable, "no pending transaction");
    }

#if !FLAMEWM_HAS_X11
    impl_->committed = impl_->pending;
    impl_->pending.clear();
    impl_->hasPending = false;
    return api::Status::Ok();
#else
    Display* dpy = nullptr;
    Window root = None;
    bool haveX = getDisplayRoot(&dpy, &root);
    if (!haveX || dpy == nullptr || root == None) {
        impl_->committed = impl_->pending;
        impl_->committedNative = impl_->pendingNative;
        impl_->committedGrabs = impl_->pendingGrabs;
        impl_->pending.clear();
        impl_->pendingNative.clear();
        impl_->pendingGrabs.clear();
        impl_->hasPending = false;
        return api::Status::Ok();
    }

    std::map<std::string, std::vector<GrabRec> > oldGrabs = impl_->committedGrabs;

    XSync(dpy, False);
    s_grabFailed = false;
    XErrorHandler prev = XSetErrorHandler(grabErrorHandler);
    s_prevHandler = prev;

    for (std::map<std::string, std::vector<GrabRec> >::const_iterator it = impl_->pendingGrabs.begin(); it != impl_->pendingGrabs.end(); ++it) {
        for (size_t i = 0; i < it->second.size(); ++i) doGrab(dpy, root, it->second[i]);
    }
    XSync(dpy, False);
    XSetErrorHandler(prev);
    s_prevHandler = nullptr;
    XSync(dpy, False);

    if (s_grabFailed) {
        for (std::map<std::string, std::vector<GrabRec> >::const_iterator it = impl_->pendingGrabs.begin(); it != impl_->pendingGrabs.end(); ++it) {
            for (size_t i = 0; i < it->second.size(); ++i) doUngrab(dpy, root, it->second[i]);
        }
        XSync(dpy, False);
        impl_->pending.clear();
        impl_->pendingNative.clear();
        impl_->pendingGrabs.clear();
        impl_->hasPending = false;
        s_grabFailed = false;
        return api::Status::make(api::Error::EngineRejected, "X grab failed (BadAccess/BadValue)");
    }

    for (std::map<std::string, std::vector<GrabRec> >::const_iterator it = oldGrabs.begin(); it != oldGrabs.end(); ++it) {
        std::map<std::string, api::KeyBinding>::const_iterator pit = impl_->pending.find(it->first);
        std::string oldKey = impl_->committed.count(it->first) ? impl_->committed[it->first].key : "";
        std::string newKey = (pit != impl_->pending.end() && pit->second.assigned()) ? pit->second.key : "";
        std::string oldLow = toLowerCopy(trimCopy(oldKey));
        std::string newLow = toLowerCopy(trimCopy(newKey));
        bool changed = (oldLow != newLow);
        bool removed = (pit == impl_->pending.end() || !pit->second.assigned());
        if (changed || removed) {
            for (size_t i = 0; i < it->second.size(); ++i) doUngrab(dpy, root, it->second[i]);
        }
    }
    XSync(dpy, False);

    impl_->committed = impl_->pending;
    impl_->committedNative = impl_->pendingNative;
    impl_->committedGrabs = impl_->pendingGrabs;
    impl_->pending.clear();
    impl_->pendingNative.clear();
    impl_->pendingGrabs.clear();
    impl_->hasPending = false;
    s_grabFailed = false;
    return api::Status::Ok();
#endif
}

void ShortcutAdapter::rollback() {
#if FLAMEWM_HAS_X11
    if (impl_->hasPending && !impl_->pendingGrabs.empty()) {
        Display* dpy = nullptr;
        Window root = None;
        if (getDisplayRoot(&dpy, &root) && dpy && root != None) {
            for (std::map<std::string, std::vector<GrabRec> >::const_iterator it = impl_->pendingGrabs.begin(); it != impl_->pendingGrabs.end(); ++it) {
                for (size_t i = 0; i < it->second.size(); ++i) doUngrab(dpy, root, it->second[i]);
            }
            XSync(dpy, False);
        }
    }
    impl_->pendingNative.clear();
    impl_->pendingGrabs.clear();
#endif
    impl_->pending.clear();
    impl_->hasPending = false;
}

bool ShortcutAdapter::handleKey(unsigned keycode, unsigned state) const {
#if FLAMEWM_HAS_X11
    if (impl_->committedGrabs.empty()) return false;
#if FLAMEWM_HAS_YXAPP
    if (::xapp == nullptr) return false;
    unsigned km = state & ::xapp->KeyMask;
    for (std::map<std::string, NativeBinding>::const_iterator it = impl_->committedNative.begin(); it != impl_->committedNative.end(); ++it) {
        const NativeBinding& nb = it->second;
        if (nb.isPointer) continue;
        for (int k = 0; k < 2 && nb.kc[k]; ++k) {
            if (keycode == nb.kc[k] && km == nb.xm[k]) {
                dispatchAction(it->first);
                return true;
            }
            if (nb.supered && (km & ::xapp->WinMask)) {
                unsigned altCtrl = ControlMask | ::xapp->AltMask;
                if (((km & ~::xapp->WinMask) | altCtrl) == nb.xm[k] && keycode == nb.kc[k]) {
                    dispatchAction(it->first);
                    return true;
                }
            }
        }
    }
#else
    (void)keycode; (void)state;
    return false;
#endif
    return false;
#else
    (void)keycode; (void)state;
    return false;
#endif
}

bool ShortcutAdapter::handleButton(unsigned button, unsigned state) const {
#if FLAMEWM_HAS_X11
    if (impl_->committedGrabs.empty()) return false;
#if FLAMEWM_HAS_YXAPP
    if (::xapp == nullptr) return false;
    unsigned km = state & ::xapp->KeyMask;
    for (std::map<std::string, NativeBinding>::const_iterator it = impl_->committedNative.begin(); it != impl_->committedNative.end(); ++it) {
        const NativeBinding& nb = it->second;
        if (!nb.isPointer) continue;
        if (static_cast<int>(button) != nb.button) continue;
        if (km == nb.xm[0] || km == nb.xm[1]) {
            dispatchAction(it->first);
            return true;
        }
    }
#else
    (void)button; (void)state;
    return false;
#endif
    return false;
#else
    (void)button; (void)state;
    return false;
#endif
}

} // namespace icewm
} // namespace engine
} // namespace flamewm
