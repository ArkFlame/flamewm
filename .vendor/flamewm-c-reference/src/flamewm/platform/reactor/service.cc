#include "flamewm/platform/reactor/service.h"

#include <map>
#include <stdexcept>
#include <string>

namespace flamewm {
namespace platform {
namespace reactor {

struct ReactorService::Impl {
    api::MainLoopPort* loop;
    std::map<int, api::MainLoopPort::FdHandle> fds;
    std::map<int, api::MainLoopPort::TimerHandle> timers;

    explicit Impl(api::MainLoopPort* l) : loop(l) {}
};

ReactorService::ReactorService(api::MainLoopPort* loop)
    : impl_(new Impl(loop)) {
    if (!loop) {
        delete impl_;
        impl_ = nullptr;
        throw std::invalid_argument("ReactorService: MainLoopPort must not be null");
    }
}

ReactorService::~ReactorService() {
    if (impl_) {
        stopAll();
        delete impl_;
    }
}

api::MainLoopPort* ReactorService::port() const {
    return impl_ ? impl_->loop : nullptr;
}

int ReactorService::addFd(int fd, int events, api::MainLoopPort::FdCallback cb) {
    if (!impl_ || !impl_->loop) {
        throw std::logic_error("ReactorService: no MainLoopPort");
    }
    if (fd < 0) {
        throw std::invalid_argument("ReactorService::addFd: fd must be >= 0");
    }
    if (!cb) {
        throw std::invalid_argument("ReactorService::addFd: callback must not be empty");
    }
    api::MainLoopPort::FdHandle h = impl_->loop->addPoll(fd, events, cb);
    impl_->fds[h.id] = h;
    return h.id;
}

void ReactorService::removeFd(int handle) {
    if (!impl_) {
        return;
    }
    std::map<int, api::MainLoopPort::FdHandle>::iterator it = impl_->fds.find(handle);
    if (it == impl_->fds.end()) {
        return;
    }
    if (impl_->loop) {
        impl_->loop->removePoll(it->second);
    }
    impl_->fds.erase(it);
}

int ReactorService::addTimer(uint64_t ms, api::MainLoopPort::TimerCallback cb, bool repeat) {
    if (!impl_ || !impl_->loop) {
        throw std::logic_error("ReactorService: no MainLoopPort");
    }
    if (!cb) {
        throw std::invalid_argument("ReactorService::addTimer: callback must not be empty");
    }
    api::MainLoopPort::TimerHandle h = impl_->loop->addTimer(ms, cb, repeat);
    impl_->timers[h.id] = h;
    return h.id;
}

void ReactorService::removeTimer(int handle) {
    if (!impl_) {
        return;
    }
    std::map<int, api::MainLoopPort::TimerHandle>::iterator it = impl_->timers.find(handle);
    if (it == impl_->timers.end()) {
        return;
    }
    if (impl_->loop) {
        impl_->loop->removeTimer(it->second);
    }
    impl_->timers.erase(it);
}

void ReactorService::defer(api::MainLoopPort::TimerCallback cb) {
    if (!impl_ || !impl_->loop) {
        throw std::logic_error("ReactorService: no MainLoopPort");
    }
    if (!cb) {
        throw std::invalid_argument("ReactorService::defer: callback must not be empty");
    }
    api::MainLoopPort::TimerHandle h = impl_->loop->defer(cb);
    impl_->timers[h.id] = h;
}

void ReactorService::stopAll() {
    if (!impl_) {
        return;
    }
    if (impl_->loop) {
        for (std::map<int, api::MainLoopPort::FdHandle>::iterator it = impl_->fds.begin();
             it != impl_->fds.end(); ++it) {
            impl_->loop->removePoll(it->second);
        }
        for (std::map<int, api::MainLoopPort::TimerHandle>::iterator it = impl_->timers.begin();
             it != impl_->timers.end(); ++it) {
            impl_->loop->removeTimer(it->second);
        }
    }
    impl_->fds.clear();
    impl_->timers.clear();
}

} // namespace reactor
} // namespace platform
} // namespace flamewm
