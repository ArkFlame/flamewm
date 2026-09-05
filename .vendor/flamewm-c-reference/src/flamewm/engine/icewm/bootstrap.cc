#include "flamewm/engine/icewm/bootstrap.h"

#include "flamewm/api/ports.h"
#include "flamewm/engine/icewm/access.h"
#include "flamewm/engine/icewm/bridge.h"
#include "flamewm/platform/host.h"
#include "flamewm/core/runtime.h"
#include "flamewm/control/server.h"
#include "flamewm/shell/shell.h"

// Adapters — each header is X11-free; heavy X/IceWM includes (if any)
// live in the corresponding .cc only, so bootstrap stays vanilla-safe.
#include "flamewm/engine/icewm/application_adapter.h"
#include "flamewm/engine/icewm/background_adapter.h"
#include "flamewm/engine/icewm/display_adapter.h"
#include "flamewm/engine/icewm/input_adapter.h"
#include "flamewm/engine/icewm/mainloop_adapter.h"
#include "flamewm/engine/icewm/session_adapter.h"
#include "flamewm/engine/icewm/shortcut_adapter.h"
#include "flamewm/engine/icewm/tray_adapter.h"
#include "flamewm/engine/icewm/window_adapter.h"
#include "flamewm/engine/icewm/workarea_adapter.h"
#include "flamewm/engine/icewm/workspace_adapter.h"
#include "flamewm/engine/icewm/ui/backend.h"

#include <cstdlib>

namespace flamewm {
namespace engine {
namespace icewm {

namespace {

// Product-only composition root owned by Bootstrap. Raw pointers with
// manual lifetime so vanilla builds (no FLAMEWM_PRODUCT_BUILD) pay no
// static-initializer cost and icewm target never links this TU.
struct BootstrapState {
    bool bridgeAttached;
    bool nativeBackendInitialized;
    flamewm::platform::PlatformHost* host;
    flamewm::control::ControlServer* server;
    flamewm::shell::Shell*           shell;
    WindowAdapter*       window;
    WorkspaceAdapter*    workspace;
    DisplayAdapter*      display;
    ShortcutAdapter*     shortcut;
    WorkAreaAdapter*     workArea;
    MainLoopAdapter*     mainLoop;
    InputAdapter*        input;
    ApplicationAdapter*  application;
    SessionAdapter*      session;
    BackgroundAdapter*   background;
    TrayAdapter*         tray;

    BootstrapState()
        : bridgeAttached(false)
        , nativeBackendInitialized(false)
        , host(nullptr)
        , server(nullptr)
        , shell(nullptr)
        , window(nullptr)
        , workspace(nullptr)
        , display(nullptr)
        , shortcut(nullptr)
        , workArea(nullptr)
        , mainLoop(nullptr)
        , input(nullptr)
        , application(nullptr)
        , session(nullptr)
        , background(nullptr)
        , tray(nullptr)
    {}
};

static BootstrapState* g_state = nullptr;

static std::string enginePreferencesPath() {
    const char* xdg = std::getenv("XDG_CONFIG_HOME");
    if (xdg != nullptr && xdg[0] != '\0')
        return std::string(xdg) + "/flamewm/engine-preferences";
    const char* home = std::getenv("HOME");
    if (home != nullptr && home[0] != '\0')
        return std::string(home) + "/.config/flamewm/engine-preferences";
    return std::string("/tmp/flamewm/engine-preferences");
}

static BootstrapState* state() {
    if (g_state == nullptr)
        g_state = new BootstrapState();
    return g_state;
}

static void teardownState() {
    if (g_state == nullptr)
        return;
    // Reverse startup order. Shell and ControlServer must stop while host
    // services and their callbacks are still alive.
    if (g_state->shell) {
        // Bootstrap owns the server lifetime; prevent Shell's optional bus
        // integration from stopping it during Shell::stop().
        g_state->shell->setControlServer(nullptr);
        g_state->shell->stop();
        delete g_state->shell;
        g_state->shell = nullptr;
    }
    if (g_state->server) {
        g_state->server->stop();
        delete g_state->server;
        g_state->server = nullptr;
    }
    if (g_state->bridgeAttached) {
        Bridge::instance().detach();
        g_state->bridgeAttached = false;
    }
    if (g_state->host) {
        g_state->host->stop();
        delete g_state->host;
        g_state->host = nullptr;
    }
    if (g_state->nativeBackendInitialized) {
        flamewm::engine::icewm::ui::IceWMBackend::instance().shutdown();
        g_state->nativeBackendInitialized = false;
    }
    delete g_state->tray;        g_state->tray = nullptr;
    delete g_state->background;  g_state->background = nullptr;
    delete g_state->session;     g_state->session = nullptr;
    delete g_state->application; g_state->application = nullptr;
    delete g_state->input;       g_state->input = nullptr;
    delete g_state->mainLoop;    g_state->mainLoop = nullptr;
    delete g_state->workArea;    g_state->workArea = nullptr;
    delete g_state->shortcut;    g_state->shortcut = nullptr;
    delete g_state->display;     g_state->display = nullptr;
    delete g_state->workspace;   g_state->workspace = nullptr;
    delete g_state->window;      g_state->window = nullptr;
    EngineAccess::detachContext();
}

} // namespace

bool Bootstrap::shouldAttach(YWindowManager* manager) {
#ifdef FLAMEWM_PRODUCT_BUILD
    if (Bridge::instance().isAttached())
        return false;
    if (manager == nullptr)
        return false;
    return true;
#else
    (void)manager;
    return false;
#endif
}

void Bootstrap::attachIfProduct(YWindowManager* manager, YWMApp* app) {
#ifdef FLAMEWM_PRODUCT_BUILD
    if (manager == nullptr || app == nullptr)
        return;
    if (!shouldAttach(manager))
        return;
    EngineAccess::attachContext(manager, app);

    BootstrapState* st = state();

    // Idempotency: if state already has a live host/ports, Bridge will
    // reject the second attach — but we avoid even allocating twice.
    if (st->host != nullptr)
        return;

    try {
        // Construct adapters in dependency-neutral order. No IceWM/X11 calls
        // during construction; adapters defer X queries to port method calls
        // which are themselves gated by manager() validity.
        st->window      = new WindowAdapter();
        st->workspace   = new WorkspaceAdapter();
        st->display     = new DisplayAdapter();
        st->shortcut    = new ShortcutAdapter();
        st->workArea    = new WorkAreaAdapter();
        st->mainLoop    = new MainLoopAdapter();
        st->input       = new InputAdapter();
        st->application = new ApplicationAdapter();
        st->session     = new SessionAdapter();
        st->background  = new BackgroundAdapter(enginePreferencesPath());
        st->tray        = new TrayAdapter();

        flamewm::api::EnginePorts ports;
        ports.window      = st->window;
        ports.workspace   = st->workspace;
        ports.display     = st->display;
        ports.shortcut    = st->shortcut;
        ports.workArea    = st->workArea;
        ports.mainLoop    = st->mainLoop;
        ports.input       = st->input;
        ports.application = st->application;
        ports.session     = st->session;
        ports.background  = st->background;
        ports.tray        = st->tray;

        flamewm::platform::PlatformHost::Config config;
        config.settingsPath = flamewm::Runtime::defaultConfigPath();
        config.enginePreferencesPath = st->background->enginePrefsPath();
        st->host = new flamewm::platform::PlatformHost(ports, config);
        if (!st->host->start()) {
            teardownState();
            return;
        }

        if (!flamewm::engine::icewm::ui::IceWMBackend::instance().init()) {
            teardownState();
            return;
        }
        st->nativeBackendInitialized = true;

        // Exactly-once attach. If another thread raced (not expected in
        // IceWM main thread), Bridge rejects second attach.
        if (!Bridge::instance().attach(st->host, ports)) {
            teardownState();
            return;
        }
        st->bridgeAttached = true;

        st->server = new flamewm::control::ControlServer(st->host);
        if (!st->server->start()) {
            teardownState();
            return;
        }

        st->shell = new flamewm::shell::Shell(st->host, st->server);
        if (!st->shell->start()) {
            teardownState();
            return;
        }
    } catch (...) {
        teardownState();
        return;
    }
#else
    // Vanilla icewm: no-op, never links Platform/Shell.
    (void)manager;
    (void)app;
#endif
}

void Bootstrap::detach() {
    // Safe to call from vanilla builds, when detached, or multiple times.
    teardownState();
}

bool Bootstrap::toggleStart(int keyCode, unsigned state) {
#ifdef FLAMEWM_PRODUCT_BUILD
    if (g_state == nullptr || g_state->shell == nullptr)
        return false;
    return g_state->shell->toggleStart(keyCode, state);
#else
    (void)keyCode;
    (void)state;
    return false;
#endif
}

} // namespace icewm
} // namespace engine
} // namespace flamewm
