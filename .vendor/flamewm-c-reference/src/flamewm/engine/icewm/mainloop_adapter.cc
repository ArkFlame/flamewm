#include "flamewm/engine/icewm/mainloop_adapter.h"
#include "flamewm/engine/icewm/access.h"

#include "yapp.h"
#include "ypoll.h"
#include "ytimer.h"

#include <map>
#include <vector>

namespace flamewm {
namespace engine {
namespace icewm {

// Forward for listener -> impl backref
class FdWrapper;
class TimerListener;
class TimerCleanupListener;

struct MainLoopAdapter::Impl {
    int nextFdId;
    int nextTimerId;
    bool shuttingDown;
    bool destroyPending;
    int callbackDepth;
    std::map<int, FdWrapper*> polls;
    std::vector<FdWrapper*> retiredPolls;
    struct TimerEntry {
        YTimer* timer;
        TimerListener* listener;
        uint64_t gen;
        bool repeat;
        TimerEntry() : timer(nullptr), listener(nullptr), gen(0), repeat(false) {}
    };
    std::map<int, TimerEntry> timers;
    std::vector<TimerEntry> retiredTimers;
    YTimer* timerCleanup;
    TimerCleanupListener* timerCleanupListener;
    bool timerCleanupActive;
    Impl() : nextFdId(1), nextTimerId(1), shuttingDown(false), destroyPending(false),
        callbackDepth(0), timerCleanup(nullptr), timerCleanupListener(nullptr),
        timerCleanupActive(false) {}
    void drainRetiredTimers();
    void drainRetiredPolls();
    void retireTimer(const TimerEntry& entry);
    void retirePoll(FdWrapper* wrapper);
    void finishCallback();
    void scheduleTimerCleanup();
};

class FdWrapper : public YPollBase {
public:
    FdWrapper(int id, int fd, int events,
              api::MainLoopPort::FdCallback cb,
              uint64_t gen)
        : id_(id), fd_(fd), events_(events), cb_(cb), gen_(gen), callbackActive_(false), impl_(nullptr) {
        initializePoll(fd_);
    }
    ~FdWrapper() {
        unregisterPoll();
    }
    virtual bool forRead() {
        if ((events_ & 1) != 0) return true;
        if ((events_ & 8) != 0) return true;
        return false;
    }
    virtual bool forWrite() {
        return (events_ & 4) != 0;
    }
    virtual void notifyRead() {
        if (!cb_) return;
        if (gen_ != EngineAccess::generation()) return;
        int out = 1;
        if ((events_ & 8) != 0) out |= 8;
        MainLoopAdapter::Impl* activeImpl = impl_;
        callbackActive_ = true;
        ++activeImpl->callbackDepth;
        cb_(fd_, out);
        callbackActive_ = false;
        --activeImpl->callbackDepth;
        activeImpl->finishCallback();
    }
    virtual void notifyWrite() {
        if (!cb_) return;
        if (gen_ != EngineAccess::generation()) return;
        MainLoopAdapter::Impl* activeImpl = impl_;
        callbackActive_ = true;
        ++activeImpl->callbackDepth;
        cb_(fd_, 4);
        callbackActive_ = false;
        --activeImpl->callbackDepth;
        activeImpl->finishCallback();
    }
    int id() const { return id_; }
    void setImpl(MainLoopAdapter::Impl* impl) { impl_ = impl; }
private:
    friend class MainLoopAdapter;
    int id_;
    int fd_;
    int events_;
    api::MainLoopPort::FdCallback cb_;
    uint64_t gen_;
    bool callbackActive_;
    MainLoopAdapter::Impl* impl_;
};

class TimerListener : public YTimerListener {
public:
    TimerListener(int id, uint64_t gen,
                  api::MainLoopPort::TimerCallback cb,
                  bool repeat)
        : id_(id), gen_(gen), cb_(cb), repeat_(repeat), impl_(nullptr), callbackActive_(false) {}
    void setImpl(MainLoopAdapter::Impl* impl) { impl_ = impl; }
    virtual bool handleTimer(YTimer* timer);
    int id_;
    uint64_t gen_;
    api::MainLoopPort::TimerCallback cb_;
    bool repeat_;
    MainLoopAdapter::Impl* impl_;
    bool callbackActive_;
};

class TimerCleanupListener : public YTimerListener {
public:
    explicit TimerCleanupListener(MainLoopAdapter::Impl* impl) : impl_(impl) {}
    virtual bool handleTimer(YTimer* timer);
private:
    MainLoopAdapter::Impl* impl_;
};

bool TimerCleanupListener::handleTimer(YTimer* timer) {
    (void)timer;
    if (impl_ == nullptr) return false;
    impl_->timerCleanupActive = true;
    ++impl_->callbackDepth;
    --impl_->callbackDepth;
    impl_->timerCleanupActive = false;
    // This timer has a positive interval, so it cannot be selected by the
    // handleTimeouts() pass which retired the timers being drained here.
    impl_->drainRetiredTimers();
    impl_->finishCallback();
    return false;
}

bool TimerListener::handleTimer(YTimer* timer) {
    (void)timer;
    MainLoopAdapter::Impl* activeImpl = impl_;
    if (activeImpl == nullptr) return false;
    std::map<int, MainLoopAdapter::Impl::TimerEntry>::iterator it =
        activeImpl->timers.find(id_);
    if (it == activeImpl->timers.end()) return false;
    if (it->second.listener != this) return false;
    if (it->second.gen != gen_) return false;
    if (gen_ != EngineAccess::generation()) return false;
    if (activeImpl->shuttingDown) return false;
    api::MainLoopPort::TimerCallback local = cb_;
    callbackActive_ = true;
    ++activeImpl->callbackDepth;
    if (local) local();
    callbackActive_ = false;
    --activeImpl->callbackDepth;
    std::map<int, MainLoopAdapter::Impl::TimerEntry>::const_iterator it2 =
        activeImpl->timers.find(id_);
    if (it2 == activeImpl->timers.end()) {
        activeImpl->finishCallback();
        return false;
    }
    if (it2->second.listener != this) {
        activeImpl->finishCallback();
        return false;
    }
    if (it2->second.gen != gen_) {
        activeImpl->finishCallback();
        return false;
    }
    if (gen_ != EngineAccess::generation()) {
        activeImpl->finishCallback();
        return false;
    }
    if (activeImpl->shuttingDown) {
        activeImpl->finishCallback();
        return false;
    }
    if (!repeat_) {
        activeImpl->retireTimer(it2->second);
        activeImpl->timers.erase(it2);
        activeImpl->finishCallback();
        return false;
    }
    bool keep = repeat_;
    activeImpl->finishCallback();
    return keep;
}

void MainLoopAdapter::Impl::finishCallback() {
    if (callbackDepth != 0)
        return;
    drainRetiredPolls();
    if (destroyPending)
        delete this;
}

void MainLoopAdapter::Impl::drainRetiredPolls() {
    if (callbackDepth != 0 || retiredPolls.empty()) return;
    for (std::vector<FdWrapper*>::iterator it = retiredPolls.begin();
         it != retiredPolls.end(); ++it) {
        delete *it;
    }
    retiredPolls.clear();
}

void MainLoopAdapter::Impl::drainRetiredTimers() {
    if (callbackDepth != 0 || retiredTimers.empty()) return;
    for (std::vector<TimerEntry>::iterator it = retiredTimers.begin();
         it != retiredTimers.end(); ++it) {
        delete it->timer;
        delete it->listener;
    }
    retiredTimers.clear();
}

void MainLoopAdapter::Impl::retireTimer(const TimerEntry& entry) {
    retiredTimers.push_back(entry);
    scheduleTimerCleanup();
}

void MainLoopAdapter::Impl::scheduleTimerCleanup() {
    if (shuttingDown || ::mainLoop == nullptr)
        return;
    if (timerCleanup != nullptr) {
        if (!timerCleanupActive)
            timerCleanup->startTimer();
        return;
    }
    timerCleanupListener = new TimerCleanupListener(this);
    timerCleanup = new YTimer(1L, timerCleanupListener, false, true);
    timerCleanup->startTimer();
}

void MainLoopAdapter::Impl::retirePoll(FdWrapper* wrapper) {
    retiredPolls.push_back(wrapper);
}

MainLoopAdapter::MainLoopAdapter() : impl_(new Impl()) {}

MainLoopAdapter::~MainLoopAdapter() {
    if (impl_) {
        Impl* doomed = impl_;
        doomed->shuttingDown = true;
        for (std::map<int, FdWrapper*>::iterator it = doomed->polls.begin();
             it != doomed->polls.end(); ++it) {
            FdWrapper* w = it->second;
            if (w) {
                w->unregisterPoll();
                if (w->callbackActive_)
                    doomed->retirePoll(w);
                else
                    delete w;
            }
        }
        doomed->polls.clear();
        for (std::map<int, Impl::TimerEntry>::iterator it = doomed->timers.begin();
             it != doomed->timers.end(); ++it) {
            YTimer* t = it->second.timer;
            TimerListener* l = it->second.listener;
            if (t) {
                t->stopTimer();
            }
            if (l && l->callbackActive_) {
                doomed->retireTimer(it->second);
            } else {
                delete t;
                delete l;
            }
        }
        doomed->timers.clear();
        doomed->drainRetiredPolls();
        doomed->drainRetiredTimers();
        if (doomed->timerCleanup && !doomed->timerCleanupActive) {
            doomed->timerCleanup->stopTimer();
            delete doomed->timerCleanup;
            delete doomed->timerCleanupListener;
            doomed->timerCleanup = nullptr;
            doomed->timerCleanupListener = nullptr;
        }
        if (doomed->callbackDepth == 0)
            delete doomed;
        else
            doomed->destroyPending = true;
        impl_ = nullptr;
    }
}

MainLoopAdapter::FdHandle MainLoopAdapter::addPoll(int fd, int events, FdCallback cb) {
    impl_->drainRetiredTimers();
    FdHandle h;
    h.id = impl_->nextFdId++;
    if (fd < 0 || !cb) {
        return h;
    }
    uint64_t gen = EngineAccess::generation();
    FdWrapper* w = new FdWrapper(h.id, fd, events, cb, gen);
    w->setImpl(impl_);
    impl_->polls[h.id] = w;
    if (::mainLoop) {
        w->registerPoll(fd);
    }
    return h;
}

void MainLoopAdapter::removePoll(FdHandle handle) {
    impl_->drainRetiredTimers();
    std::map<int, FdWrapper*>::iterator it = impl_->polls.find(handle.id);
    if (it == impl_->polls.end()) return;
    FdWrapper* w = it->second;
    if (w) {
        w->unregisterPoll();
        if (w->callbackActive_) {
            impl_->retirePoll(w);
        } else {
            delete w;
        }
    }
    impl_->polls.erase(it);
}

MainLoopAdapter::TimerHandle MainLoopAdapter::addTimer(uint64_t ms, TimerCallback cb, bool repeat) {
    impl_->drainRetiredTimers();
    TimerHandle h;
    h.id = impl_->nextTimerId++;
    if (!cb) {
        return h;
    }
    uint64_t gen = EngineAccess::generation();
    TimerListener* l = new TimerListener(h.id, gen, cb, repeat);
    l->setImpl(impl_);
    YTimer* t = new YTimer(static_cast<long>(ms), l, false, true);
    Impl::TimerEntry e;
    e.timer = t;
    e.listener = l;
    e.gen = gen;
    e.repeat = repeat;
    impl_->timers[h.id] = e;
    if (::mainLoop) {
        t->startTimer();
    }
    return h;
}

void MainLoopAdapter::removeTimer(TimerHandle handle) {
    impl_->drainRetiredTimers();
    std::map<int, Impl::TimerEntry>::iterator it = impl_->timers.find(handle.id);
    if (it == impl_->timers.end()) return;
    YTimer* t = it->second.timer;
    TimerListener* l = it->second.listener;
    if (t) {
        t->stopTimer();
    }
    if (l && l->callbackActive_) {
        impl_->retireTimer(it->second);
    } else {
        delete t;
        delete l;
    }
    impl_->timers.erase(it);
}

MainLoopAdapter::TimerHandle MainLoopAdapter::defer(TimerCallback cb) {
    impl_->drainRetiredTimers();
    return addTimer(0, cb, false);
}

} // namespace icewm
} // namespace engine
} // namespace flamewm
