#ifndef FLAMEWM_SESSION_DESKTOP_SUPERVISOR_H
#define FLAMEWM_SESSION_DESKTOP_SUPERVISOR_H

#include <string>

namespace flamewm {
namespace session {

// DesktopSupervisor — crash-isolated, parent-bound supervisor for flamewm-desktop.
// - Parent-bound via prctl(PR_SET_PDEATHSIG) where available, or pipe EOF.
// - Starts flamewm-desktop via fork/exec (no shell).
// - Bounded restart/backoff: max 5 restarts, exponential 500..8000ms.
// - Clean SIGTERM: forwards SIGTERM to child, reaps, exits without respawn.
// - No polling: uses blocking waitpid and signal handling; backoff via nanosleep.

class DesktopSupervisor {
public:
    explicit DesktopSupervisor(const std::string& desktopExe,
                               int parentReadFd = -1);
    ~DesktopSupervisor();

    DesktopSupervisor(const DesktopSupervisor&) = delete;
    DesktopSupervisor& operator=(const DesktopSupervisor&) = delete;

    // Blocks until parent dies or requestStop() / SIGTERM. Returns exit code.
    int run();

    void requestStop();

private:
    int spawnOnce();
    void killChild(int sig);
    bool isParentAlive() const;

    std::string desktopExe_;
    int parentReadFd_;
    int childPid_;
    int restarts_;
    bool stopping_;
    bool gaveUp_;

    static const int kMaxRestarts = 5;
};

} // namespace session
} // namespace flamewm

#endif // FLAMEWM_SESSION_DESKTOP_SUPERVISOR_H
