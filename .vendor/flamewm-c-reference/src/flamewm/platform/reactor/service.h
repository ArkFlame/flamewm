#ifndef FLAMEWM_PLATFORM_REACTOR_SERVICE_H
#define FLAMEWM_PLATFORM_REACTOR_SERVICE_H

#include "flamewm/api/ports.h"

#include <cstdint>

namespace flamewm {
namespace platform {
namespace reactor {

class ReactorService {
public:
    explicit ReactorService(api::MainLoopPort* loop);
    ~ReactorService();

    ReactorService(const ReactorService&) = delete;
    ReactorService& operator=(const ReactorService&) = delete;

    int addFd(int fd, int events, api::MainLoopPort::FdCallback cb);
    void removeFd(int handle);
    int addTimer(uint64_t ms, api::MainLoopPort::TimerCallback cb, bool repeat);
    void removeTimer(int handle);
    void defer(api::MainLoopPort::TimerCallback cb);
    void stopAll();

    api::MainLoopPort* port() const;

private:
    struct Impl;
    Impl* impl_;
};

} // namespace reactor
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_REACTOR_SERVICE_H
