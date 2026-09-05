#ifndef FLAMEWM_INTEGRATIONS_DBUSDISPATCHER_H
#define FLAMEWM_INTEGRATIONS_DBUSDISPATCHER_H

#include <stdint.h>
#include <string>
#include <vector>
#include <map>

// Forward declare to avoid hard X/IMainLoop dependency in header-only builds.
// Actual registration uses IMainLoop if available (yapp.h); otherwise no-op graceful degrade.
class YPollBase;
class YTimer;
class YTimerListener;
struct DBusConnection;

namespace flamewm { namespace platform { namespace reactor { class ReactorService; } } }
namespace flamewm { namespace integrations { class ReactorBridge; } }

namespace flamewm {
namespace integrations {

// Adapter interface proposed for IceWM main-loop integration.
// Do NOT edit yapp.cc poll/timer dispatch directly; implement this interface
// and inject it into DBusDispatcher. Future wiring will forward to ::mainLoop.
class IMainLoopAdapter {
public:
    virtual ~IMainLoopAdapter() {}
    virtual void registerPoll(YPollBase* poll) = 0;
    virtual void unregisterPoll(YPollBase* poll) = 0;
    virtual void registerTimer(YTimer* t) = 0;
    virtual void unregisterTimer(YTimer* t) = 0;
};

// Null adapter — graceful degrade when no event loop available (tests / missing X)
class NullMainLoopAdapter : public IMainLoopAdapter {
public:
    void registerPoll(YPollBase*) {}
    void unregisterPoll(YPollBase*) {}
    void registerTimer(YTimer*) {}
    void unregisterTimer(YTimer*) {}
};

// Reactor-backed adapter — single event-loop integration via platform/reactor.
// No second D-Bus reactor; all watches/timeouts multiplex through ReactorService.
// Unregister-before-free preserved via handle maps; generation tokens remain in
// DBusDispatcher. X11-free: only YPollBase/YTimer forward decls needed.
class ReactorBridge : public IMainLoopAdapter {
public:
    explicit ReactorBridge(platform::reactor::ReactorService* reactor);
    ~ReactorBridge();
    void registerPoll(YPollBase* poll);
    void unregisterPoll(YPollBase* poll);
    void registerTimer(YTimer* t);
    void unregisterTimer(YTimer* t);
    bool isAttached() const { return reactor_ != 0; }
    platform::reactor::ReactorService* reactor() const { return reactor_; }
private:
    platform::reactor::ReactorService* reactor_;
    std::map<YPollBase*, int> pollHandles_;
    std::map<YTimer*, int> timerHandles_;
    // helpers
    void removeAll();
};

// Forward decl for conditional dbus availability.
// When libdbus is present at build time, define HAVE_DBUS or HAVE_FLAMEWM_DBUS.
struct DBusWatchInfo {
    int id;
    int fd;
    bool watchRead;
    bool watchWrite;
    bool enabled;
    uint64_t generation;
    std::string service;
};

struct DBusTimeoutInfo {
    int id;
    int intervalMs;
    bool enabled;
    uint64_t generation;
    std::string service;
};

// Bounded dispatch budget per event-loop turn (avoid starvation)
static const int kMaxDispatchPerTurn = 32;

// DBusDispatcher maps D-Bus watches/timeouts onto IceWM poll/timer ownership.
// Lifetime rules:
//  - unregister-before-free for every poll/timer
//  - service generation tokens: stale callbacks after owner/reconnect ignored
//  - callback wrappers owned by dispatcher
//  - no synchronous blocking D-Bus calls on X loop (caller must use async APIs)
class DBusDispatcher {
public:
    explicit DBusDispatcher(IMainLoopAdapter* adapter);
    ~DBusDispatcher();

    // Non-copyable
    DBusDispatcher(const DBusDispatcher&) = delete;
    DBusDispatcher& operator=(const DBusDispatcher&) = delete;

    // Generation management per logical service (NM, MPRIS, etc.)
    // Call bumpGeneration(service) on owner loss/reconnect; stale callbacks carry old gen.
    uint64_t generationFor(const std::string& service) const;
    uint64_t bumpGeneration(const std::string& service);
    bool isStale(const std::string& service, uint64_t capturedGen) const;

    // Watch management (caller typically from DBusAddWatchFunction)
    // Returns watch id. Caller must keep fd valid until remove.
    int addWatch(int fd, bool read, bool write, const std::string& service);
    void removeWatch(int watchId);
    void setWatchEnabled(int watchId, bool enabled);
    void handleWatchReady(int watchId, bool readable, bool writable);

    // Timeout management (DBusAddTimeoutFunction)
    int addTimeout(int intervalMs, const std::string& service);
    void removeTimeout(int timeoutId);
    void setTimeoutEnabled(int timeoutId, bool enabled);
    void handleTimeout(int timeoutId);

    // Coalesced reconciliation: request a deferred notify (coalesced to one timer tick)
    // Listener will be called at most once per turn even if requested many times.
    void requestReconcile(const std::string& service);
    bool hasPendingReconcile(const std::string& service) const;
    void clearReconcile(const std::string& service);

    // Bounded dispatch entry point — call from poll/timer callbacks.
    // Returns number of messages dispatched this turn.
    int dispatchPending(const std::string& service, int maxMessages = kMaxDispatchPerTurn);

    // Unregister all watches/timeouts before destruction (FAILURES.md callback lifetime)
    void shutdown();

    bool isShutdown() const { return shutdown_; }

    // Stats for diagnostics
    size_t watchCount() const { return watches_.size(); }
    size_t timeoutCount() const { return timeouts_.size(); }

    // Adapter access (for late binding to real mainLoop)
    void setAdapter(IMainLoopAdapter* a) { adapter_ = a ? a : &nullAdapter_; }
    void setConnection(DBusConnection* connection) { connection_ = connection; }
    void addConnection(DBusConnection* connection);
    void removeConnection(DBusConnection* connection);

private:
    // Internal poll wrapper owned by dispatcher
    class WatchPoll;
    class TimeoutTimer;

    IMainLoopAdapter* adapter_;
    NullMainLoopAdapter nullAdapter_;
    bool shutdown_;
    DBusConnection* connection_;
    std::vector<DBusConnection*> connections_;

    std::map<std::string, uint64_t> generations_;
    std::map<std::string, bool> pendingReconcile_;

    std::map<int, DBusWatchInfo> watches_;
    std::map<int, WatchPoll*> watchPolls_;

    std::map<int, DBusTimeoutInfo> timeouts_;
    std::map<int, TimeoutTimer*> timeoutTimers_;

    int nextWatchId_;
    int nextTimeoutId_;
};

} // namespace integrations
} // namespace flamewm

#endif
