#pragma once

#include "flamewm/api/ports.h"

namespace flamewm {
namespace engine {
namespace icewm {

class MainLoopAdapter : public api::MainLoopPort {
public:
    MainLoopAdapter();
    ~MainLoopAdapter() override;

    FdHandle addPoll(int fd, int events, FdCallback cb) override;
    void removePoll(FdHandle handle) override;
    TimerHandle addTimer(uint64_t ms, TimerCallback cb, bool repeat) override;
    void removeTimer(TimerHandle handle) override;
    TimerHandle defer(TimerCallback cb) override;

private:
    struct Impl;
    Impl* impl_;
    friend class FdWrapper;
    friend class TimerListener;
    friend class TimerCleanupListener;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm
