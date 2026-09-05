#ifndef FLAMEWM_ENGINE_ICEWM_BOOTSTRAP_H
#define FLAMEWM_ENGINE_ICEWM_BOOTSTRAP_H

class YWindowManager;
class YWMApp;

namespace flamewm {
namespace engine {
namespace icewm {

// Bootstrap: product-only composition root for Bridge + PlatformHost + shell.
//
// Contract (ULT-BOOTSTRAP / C5/F5 frozen):
// - Frozen signature: attachIfProduct(YWindowManager*, YWMApp*). Caller
//   passes already-constructed manager/app directly — no Bridge discovery.
// - EngineAccess stores passed manager/app + generation; manager() does NOT
//   depend on Bridge::isAttached() (no circular gate).
// - Called from Flame-only compile path immediately after real manager is
//   constructed and valid (wmapp.cc flame hook). Detach before teardown.
// - Normal icewm compile path: zero attach, zero PlatformHost, zero Shell
//   (guarded by #ifdef FLAMEWM_PRODUCT_BUILD).
// - Attach is idempotent and guarded by Bridge::isAttached() and manager
//   validity. Detach is safe to call multiple times.
// - No X11/IceWM headers leak through this header; all heavy includes live
//   in bootstrap.cc.

class Bootstrap {
public:
    // True only for product builds when Bridge is not yet attached and
    // the passed manager is non-null. Vanilla builds always false.
    static bool shouldAttach(YWindowManager* manager);

    // Frozen signature: idempotent, constructs adapters + EnginePorts +
    // PlatformHost and calls Bridge::instance().attach(...) when
    // FLAMEWM_PRODUCT_BUILD is defined and manager/app are valid. Stores
    // context into EngineAccess (generation bump) before attach. No-op
    // otherwise or if already attached. Exactly-once per process.
    static void attachIfProduct(YWindowManager* manager, YWMApp* app);

    // Idempotent teardown: Shell + ControlServer, Bridge, Host, native UI,
    // adapters, then EngineAccess (generation invalidation).
    // Safe to call when detached or from vanilla builds. Must be called
    // before manager/app are destroyed.
    static void detach();

    // Routes an already-authorized root shortcut to product Start state.
    static bool toggleStart(int keyCode, unsigned state);

private:
    Bootstrap() = delete;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm

#endif // FLAMEWM_ENGINE_ICEWM_BOOTSTRAP_H
