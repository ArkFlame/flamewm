#include "dbusdispatcher.h"
#include "yapp.h"
#include "ytimer.h"
#include "flamewm/platform/reactor/service.h"
#include <algorithm>

#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
#include <dbus/dbus.h>
#endif

// Build must pass without libdbus or IceWM dev headers (graceful degrade).
// Wrappers are real IceWM poll/timer objects; the adapter owns registration.

namespace flamewm {
namespace integrations {

// Wrappers are owned by the dispatcher and always unregistered before delete.
class DBusDispatcher::WatchPoll : public YPoll<DBusDispatcher> {
public:
    WatchPoll(DBusDispatcher* d, int id, int fd, bool read, bool write)
        : YPoll<DBusDispatcher>(d), id_(id), read_(read), write_(write), enabled_(true) { initializePoll(fd); }
    ~WatchPoll() { unregisterPoll(); }
    virtual bool forRead() { return enabled_ && read_; }
    virtual bool forWrite() { return enabled_ && write_; }
    virtual void notifyRead() { owner()->handleWatchReady(id_, true, false); }
    virtual void notifyWrite() { owner()->handleWatchReady(id_, false, true); }
    int id_;
    bool read_;
    bool write_;
    bool enabled_;
    void setEnabled(bool enabled) { enabled_ = enabled; }
};

class DBusDispatcher::TimeoutTimer : public YTimerListener {
public:
    TimeoutTimer(DBusDispatcher* d, int id, int intervalMs)
        : disp_(d), id_(id), timer_(intervalMs, this, false, true) {}
    ~TimeoutTimer() { timer_.stopTimer(); }
    void stopTimer() { timer_.stopTimer(); }
    void setInterval(long ms) { timer_.setInterval(ms); }
    void startTimer() { timer_.startTimer(); }
    bool handleTimer(YTimer*) {
        if (!disp_ || disp_->isShutdown()) return false;
        DBusDispatcher* savedDisp = disp_;
        int savedId = id_;
        savedDisp->handleTimeout(savedId);
        // UAF fix: handleTimeout may have called removeTimeout which deletes this.
        // Do not touch `this` after dispatch; verify liveness via dispatcher maps.
        std::map<int, TimeoutTimer*>::const_iterator tit = savedDisp->timeoutTimers_.find(savedId);
        if (tit == savedDisp->timeoutTimers_.end()) return false;
        std::map<int, DBusTimeoutInfo>::const_iterator it = savedDisp->timeouts_.find(savedId);
        if (it == savedDisp->timeouts_.end() || !it->second.enabled || !mainLoop) return false;
        tit->second->timer_.startTimer(it->second.intervalMs);
        return false;
    }
    DBusDispatcher* disp_;
    int id_;
    YTimer timer_;
};

DBusDispatcher::DBusDispatcher(IMainLoopAdapter* adapter)
    : adapter_(adapter ? adapter : &nullAdapter_)
    , shutdown_(false)
    , connection_(0)
    , nextWatchId_(1)
    , nextTimeoutId_(1)
{
}

DBusDispatcher::~DBusDispatcher() {
    shutdown();
}

uint64_t DBusDispatcher::generationFor(const std::string& service) const {
    auto it = generations_.find(service);
    return it == generations_.end() ? 0 : it->second;
}

uint64_t DBusDispatcher::bumpGeneration(const std::string& service) {
    uint64_t g = generationFor(service) + 1;
    generations_[service] = g;
    pendingReconcile_[service] = false;
    return g;
}

bool DBusDispatcher::isStale(const std::string& service, uint64_t capturedGen) const {
    return capturedGen != generationFor(service);
}

int DBusDispatcher::addWatch(int fd, bool read, bool write, const std::string& service) {
    if (shutdown_) return -1;
    int id = nextWatchId_++;
    DBusWatchInfo info;
    info.id = id;
    info.fd = fd;
    info.watchRead = read;
    info.watchWrite = write;
    info.enabled = true;
    info.generation = generationFor(service);
    info.service = service;
    watches_[id] = info;
    WatchPoll* p = new WatchPoll(this, id, fd, read, write);
    watchPolls_[id] = p;
    adapter_->registerPoll(p);
    return id;
}

void DBusDispatcher::removeWatch(int watchId) {
    auto it = watches_.find(watchId);
    if (it == watches_.end()) return;
    auto pit = watchPolls_.find(watchId);
    if (pit != watchPolls_.end()) {
        adapter_->unregisterPoll(pit->second);
        pit->second->unregisterPoll();
        delete pit->second;
        watchPolls_.erase(pit);
    }
    watches_.erase(it);
}

void DBusDispatcher::setWatchEnabled(int watchId, bool enabled) {
    auto it = watches_.find(watchId);
    if (it == watches_.end()) return;
    it->second.enabled = enabled;
    auto pit = watchPolls_.find(watchId);
    if (pit != watchPolls_.end()) pit->second->setEnabled(enabled);
}

void DBusDispatcher::handleWatchReady(int watchId, bool readable, bool writable) {
    if (shutdown_) return;
    auto it = watches_.find(watchId);
    if (it == watches_.end()) return;
    if (!it->second.enabled) return;
    if (isStale(it->second.service, it->second.generation)) return;
    if (readable || writable) {
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
        for (size_t i = 0; i < connections_.size(); ++i)
            if (connections_[i]) dbus_connection_read_write_dispatch(connections_[i], 0);
#endif
        dispatchPending(it->second.service);
    }
}

void DBusDispatcher::addConnection(DBusConnection* connection) {
    if (!connection) return;
    if (std::find(connections_.begin(), connections_.end(), connection) == connections_.end())
        connections_.push_back(connection);
    if (!connection_) connection_ = connection;
}

void DBusDispatcher::removeConnection(DBusConnection* connection) {
    connections_.erase(std::remove(connections_.begin(), connections_.end(), connection), connections_.end());
    if (connection_ == connection) connection_ = connections_.empty() ? 0 : connections_.front();
}

int DBusDispatcher::addTimeout(int intervalMs, const std::string& service) {
    if (shutdown_) return -1;
    int id = nextTimeoutId_++;
    DBusTimeoutInfo info;
    info.id = id;
    info.intervalMs = intervalMs;
    info.enabled = true;
    info.generation = generationFor(service);
    info.service = service;
    timeouts_[id] = info;
    TimeoutTimer* tt = new TimeoutTimer(this, id, intervalMs);
    timeoutTimers_[id] = tt;
    if (intervalMs > 0) {
        tt->setInterval(intervalMs);
        adapter_->registerTimer(&tt->timer_);
        if (mainLoop) tt->startTimer();
    }
    return id;
}

void DBusDispatcher::removeTimeout(int timeoutId) {
    auto it = timeouts_.find(timeoutId);
    if (it == timeouts_.end()) return;
    auto tit = timeoutTimers_.find(timeoutId);
    if (tit != timeoutTimers_.end()) {
        adapter_->unregisterTimer(&tit->second->timer_);
        tit->second->stopTimer();
        delete tit->second;
        timeoutTimers_.erase(tit);
    }
    timeouts_.erase(it);
}

void DBusDispatcher::setTimeoutEnabled(int timeoutId, bool enabled) {
    auto it = timeouts_.find(timeoutId);
    if (it == timeouts_.end()) return;
    it->second.enabled = enabled;
    auto tit = timeoutTimers_.find(timeoutId);
    if (tit == timeoutTimers_.end()) return;
    if (enabled && it->second.intervalMs > 0) {
        adapter_->registerTimer(&tit->second->timer_);
        if (mainLoop) tit->second->startTimer();
    } else {
        adapter_->unregisterTimer(&tit->second->timer_);
        tit->second->stopTimer();
    }
}

void DBusDispatcher::handleTimeout(int timeoutId) {
    if (shutdown_) return;
    auto it = timeouts_.find(timeoutId);
    if (it == timeouts_.end()) return;
    if (!it->second.enabled) return;
    if (isStale(it->second.service, it->second.generation)) return;
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    for (size_t i = 0; i < connections_.size(); ++i)
        if (connections_[i]) dbus_connection_read_write_dispatch(connections_[i], 0);
#endif
    dispatchPending(it->second.service);
}

void DBusDispatcher::requestReconcile(const std::string& service) {
    if (shutdown_) return;
    auto it = pendingReconcile_.find(service);
    if (it != pendingReconcile_.end() && it->second) return;
    pendingReconcile_[service] = true;
}

bool DBusDispatcher::hasPendingReconcile(const std::string& service) const {
    auto it = pendingReconcile_.find(service);
    return it != pendingReconcile_.end() && it->second;
}

void DBusDispatcher::clearReconcile(const std::string& service) {
    pendingReconcile_[service] = false;
}

int DBusDispatcher::dispatchPending(const std::string&, int maxMessages) {
    if (shutdown_) return 0;
    if (maxMessages <= 0) return 0;
    if (maxMessages > kMaxDispatchPerTurn) maxMessages = kMaxDispatchPerTurn;
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    if (connections_.empty() && connection_) connections_.push_back(connection_);
    if (connections_.empty()) return 0;
    int dispatched = 0;
    bool progress = true;
    while (dispatched < maxMessages && progress) {
        progress = false;
        for (size_t i = 0; i < connections_.size() && dispatched < maxMessages; ++i) {
            DBusConnection* c = connections_[i];
            if (c && dbus_connection_get_dispatch_status(c) == DBUS_DISPATCH_DATA_REMAINS) {
                dbus_connection_dispatch(c);
                ++dispatched;
                progress = true;
            }
        }
    }
    return dispatched;
#else
    return 0;
#endif
}

void DBusDispatcher::shutdown() {
    if (shutdown_) return;
    for (auto &kv : watchPolls_) {
        adapter_->unregisterPoll(kv.second);
        kv.second->unregisterPoll();
        delete kv.second;
    }
    watchPolls_.clear();
    watches_.clear();
    for (auto &kv : timeoutTimers_) {
        adapter_->unregisterTimer(&kv.second->timer_);
        kv.second->stopTimer();
        delete kv.second;
    }
    timeoutTimers_.clear();
    timeouts_.clear();
    pendingReconcile_.clear();
    shutdown_ = true;
}

// ---- ReactorBridge (single event-loop integration, no second reactor) ----

ReactorBridge::ReactorBridge(platform::reactor::ReactorService* reactor)
    : reactor_(reactor) {}

ReactorBridge::~ReactorBridge() {
    removeAll();
}

void ReactorBridge::removeAll() {
    if (!reactor_) {
        pollHandles_.clear();
        timerHandles_.clear();
        return;
    }
    for (std::map<YPollBase*, int>::iterator it = pollHandles_.begin();
         it != pollHandles_.end(); ++it) {
        // Underlying YPollBase owned by DBusDispatcher::WatchPoll — just
        // remove the fd registration from reactor.
        reactor_->removeFd(it->second);
    }
    pollHandles_.clear();
    for (std::map<YTimer*, int>::iterator it = timerHandles_.begin();
         it != timerHandles_.end(); ++it) {
        reactor_->removeTimer(it->second);
    }
    timerHandles_.clear();
}

void ReactorBridge::registerPoll(YPollBase* poll) {
    if (!poll || !reactor_) return;
    if (pollHandles_.find(poll) != pollHandles_.end()) return;
    int fd = poll->fd();
    if (fd < 0) return;
    // Map YPollBase forRead/forWrite to reactor events bitmask: 1=read,4=write,8=except
    int events = 0;
    if (poll->forRead()) events |= 1;
    if (poll->forWrite()) events |= 4;
    if (events == 0) events = 1; // default read if neither (defensive)
    YPollBase* savedPoll = poll;
    int h = reactor_->addFd(fd, events, [savedPoll](int, int ev) {
        if (!savedPoll) return;
        bool r = (ev & 1) != 0 || (ev & 8) != 0;
        bool w = (ev & 4) != 0;
        if (r) savedPoll->notifyRead();
        if (w) savedPoll->notifyWrite();
    });
    pollHandles_[poll] = h;
}

void ReactorBridge::unregisterPoll(YPollBase* poll) {
    if (!poll || !reactor_) return;
    std::map<YPollBase*, int>::iterator it = pollHandles_.find(poll);
    if (it == pollHandles_.end()) return;
    reactor_->removeFd(it->second);
    pollHandles_.erase(it);
}

void ReactorBridge::registerTimer(YTimer* t) {
    if (!t || !reactor_) return;
    if (timerHandles_.find(t) != timerHandles_.end()) return;
    long ms = t->getInterval();
    if (ms < 0) ms = 0;
    YTimer* saved = t;
    int h = reactor_->addTimer((uint64_t)ms, [saved]() {
        if (saved && saved->getTimerListener())
            saved->getTimerListener()->handleTimer(saved);
    }, false);
    timerHandles_[t] = h;
}

void ReactorBridge::unregisterTimer(YTimer* t) {
    if (!t || !reactor_) return;
    std::map<YTimer*, int>::iterator it = timerHandles_.find(t);
    if (it == timerHandles_.end()) return;
    reactor_->removeTimer(it->second);
    timerHandles_.erase(it);
}

} // namespace integrations
} // namespace flamewm
