#ifndef FLAMEWM_DESKTOP_WATCHER_H
#define FLAMEWM_DESKTOP_WATCHER_H

#include <string>
#include <vector>
#include <map>
#include <stdint.h>

namespace flamewm {
namespace desktop {

// Inotify watcher with coalescing, move-cookie pairing, overflow recovery.
// No periodic scan. Consumer must call handleEvents() from select/poll loop.
class DesktopWatcher {
public:
    enum EventType { EvCreate=0, EvDelete=1, EvModify=2, EvMoveFrom=3, EvMoveTo=4, EvOverflow=5, EvInvalidated=6 };
    struct Event { EventType type; std::string name; uint32_t cookie; };
    class Listener {
    public:
        virtual ~Listener(){}
        virtual void onWatcherEvents(const std::vector<Event>& ev) = 0;
        virtual void onOverflowNeedsRescan() = 0;
        virtual void onWatchInvalidated() = 0;
    };

    explicit DesktopWatcher(const std::string& dir);
    ~DesktopWatcher();

    bool start(std::string* error);
    void stop();
    bool isStarted() const { return fd_>=0; }
    int fd() const { return fd_; }

    // Non-blocking poll: read inotify fd, coalesce bursts, pair cookies, dispatch.
    // Returns true if events processed; false on fatal error (caller should rescan+re-add).
    bool handleEvents();

    void setListener(Listener* l){ listener_=l; }

    // For testing: inject raw coalescing
    static std::vector<Event> coalesceAndPair(const std::vector<Event>& raw);

private:
    std::string dir_;
    int fd_;
    int wd_;
    Listener* listener_;

    bool reAdd(std::string* error);
    void dispatch(const std::vector<Event>& ev);

    // Burst coalescing: call after small delay? For now handleEvents does inline coalesce of single read batch.
};

} // namespace desktop
} // namespace flamewm

#endif
