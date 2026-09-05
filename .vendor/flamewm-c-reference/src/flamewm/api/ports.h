#ifndef FLAMEWM_API_PORTS_H
#define FLAMEWM_API_PORTS_H

#ifdef None
#pragma push_macro("None")
#undef None
#define FLAMEWM_PORTS_RN
#endif
#ifdef Status
#pragma push_macro("Status")
#undef Status
#define FLAMEWM_PORTS_RS
#endif

#include "errors.h"
#include "ids.h"
#include "geometry.h"
#include "window.h"
#include "workspace.h"
#include "display.h"
#include "shortcuts.h"
#include "input.h"
#include "background.h"
#include "session.h"
#include "applications.h"

#include <cstdint>
#include <functional>
#include <map>
#include <string>
#include <utility>
#include <vector>

namespace flamewm {
namespace api {

class WindowPort {
public:
    virtual Result<WindowSnapshot> get(WindowRef) = 0;
    virtual std::vector<WindowSnapshot> snapshot() = 0;
    virtual Status activate(WindowRef) = 0;
    virtual Status minimize(WindowRef) = 0;
    virtual Status maximize(WindowRef) = 0;
    virtual Status restore(WindowRef) = 0;
    virtual Status close(WindowRef) = 0;
    virtual Status setOuterGeometry(WindowRef, Rect) = 0;
    virtual Result<Rect> workArea(WindowRef) = 0;
    virtual Result<OutputId> output(WindowRef) = 0;
    virtual ~WindowPort() {}
};

class WorkspacePort {
public:
    virtual Result<WorkspaceSnapshot> snapshot() = 0;
    virtual Status activate(int index, uint64_t expectedRevision) = 0;
    virtual Status moveWindow(WindowRef, int targetWorkspace) = 0;
    virtual Status applyTransform(const WorkspaceTransform&) = 0;
    virtual ~WorkspacePort() {}
};

class DisplayPort {
public:
    virtual Result<DisplaySnapshot> queryFresh() = 0;
    virtual Status capture() = 0;
    virtual Status applyMode(OutputId, ModeId, TransactionId* outTx) = 0;
    virtual Status restore(TransactionId) = 0;
    virtual ~DisplayPort() {}
};

class ShortcutPort {
public:
    virtual Status prepare(const std::map<std::string, KeyBinding>& desired) = 0;
    virtual Status commit() = 0;
    virtual void rollback() = 0;
    virtual ~ShortcutPort() {}
};

class WorkAreaPort {
public:
    virtual std::vector<Rect> baseWorkAreas() = 0;
    virtual void applyFlameReservations(const std::vector<std::pair<OutputId, Rect> >& reservations) = 0;
    virtual void requestRecompute() = 0;
    virtual ~WorkAreaPort() {}
};

class MainLoopPort {
public:
    using FdCallback = std::function<void(int fd, int events)>;
    using TimerCallback = std::function<void()>;
    struct FdHandle { int id; };
    struct TimerHandle { int id; };

    virtual FdHandle addPoll(int fd, int events, FdCallback cb) = 0;
    virtual void removePoll(FdHandle) = 0;
    virtual TimerHandle addTimer(uint64_t ms, TimerCallback cb, bool repeat) = 0;
    virtual void removeTimer(TimerHandle) = 0;
    virtual TimerHandle defer(TimerCallback cb) = 0;
    virtual ~MainLoopPort() {}
};

class InputPort {
public:
    virtual Result<PointerPosition> rootPointer() = 0;
    virtual ~InputPort() {}
};

class ApplicationPort {
public:
    virtual Status launch(const DesktopAppId&, const std::vector<std::string>& args) = 0;
    virtual Status launchUri(const std::string& uri) = 0;
    virtual ~ApplicationPort() {}
};

class SessionPort {
public:
    virtual Status lock() = 0;
    virtual Status logout() = 0;
    virtual Status suspend() = 0;
    virtual Status reboot() = 0;
    virtual Status shutdown() = 0;
    virtual SessionCapabilities capabilities() = 0;
    virtual ~SessionPort() {}
};

class BackgroundPort {
public:
    virtual Status project(const BackgroundState&) = 0;
    virtual Status reload() = 0;
    virtual ~BackgroundPort() {}
};

class TrayPort {
public:
    virtual Status setOwner(OutputId) = 0;
    virtual Result<OutputId> owner() = 0;
    virtual ~TrayPort() {}
};

struct EnginePorts {
    WindowPort* window;
    WorkspacePort* workspace;
    DisplayPort* display;
    ShortcutPort* shortcut;
    WorkAreaPort* workArea;
    MainLoopPort* mainLoop;
    InputPort* input;
    ApplicationPort* application;
    SessionPort* session;
    BackgroundPort* background;
    TrayPort* tray;
};

} // namespace api
} // namespace flamewm

#ifdef FLAMEWM_PORTS_RS
#pragma pop_macro("Status")
#undef FLAMEWM_PORTS_RS
#endif
#ifdef FLAMEWM_PORTS_RN
#pragma pop_macro("None")
#undef FLAMEWM_PORTS_RN
#endif

#endif // FLAMEWM_API_PORTS_H
