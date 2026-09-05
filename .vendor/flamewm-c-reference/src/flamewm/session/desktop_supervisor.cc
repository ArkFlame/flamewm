#include "flamewm/session/desktop_supervisor.h"

#include <unistd.h>
#include <signal.h>
#include <sys/wait.h>
#include <sys/select.h>
#include <errno.h>
#include <string.h>
#include <time.h>
#include <cstdio>
#include <cstdlib>

#ifdef __linux__
#include <sys/prctl.h>
#ifndef PR_SET_PDEATHSIG
#define PR_SET_PDEATHSIG 1
#endif
#endif

namespace flamewm {
namespace session {

namespace {

volatile sig_atomic_t gStop = 0;
volatile sig_atomic_t gChildSig = 0;

void onTerm(int) { gStop = 1; }
void onChld(int) { gChildSig = 1; }

long backoffMs(int attempt) {
    // 500, 1000, 2000, 4000, 8000
    long v = 500L << attempt;
    if (v > 8000L) v = 8000L;
    if (v < 500L) v = 500L;
    return v;
}

void sleepMs(long ms) {
    struct timespec ts;
    ts.tv_sec = ms / 1000;
    ts.tv_nsec = (ms % 1000) * 1000000L;
    while (nanosleep(&ts, &ts) == -1 && errno == EINTR) {}
}

} // namespace

DesktopSupervisor::DesktopSupervisor(const std::string& desktopExe,
                                   int parentReadFd)
    : desktopExe_(desktopExe)
    , parentReadFd_(parentReadFd)
    , childPid_(-1)
    , restarts_(0)
    , stopping_(false)
    , gaveUp_(false) {}

DesktopSupervisor::~DesktopSupervisor() {}

void DesktopSupervisor::requestStop() {
    stopping_ = true;
    gStop = 1;
}

bool DesktopSupervisor::isParentAlive() const {
    if (parentReadFd_ >= 0) {
        char buf;
        ssize_t r = read(parentReadFd_, &buf, 1);
        if (r == 0) return false; // EOF — parent closed write end (died)
        if (r < 0 && errno != EAGAIN && errno != EINTR) {
            // EBADF etc — treat as dead
            if (errno == EBADF) return false;
        }
        // EAGAIN/EINTR or data — parent alive; if data was read it was stray; ignore
        (void)buf;
        return true;
    }
    // pipe-less fallback: parent death signal will arrive via SIGTERM/prctl
    // Also check ppid vs 1 as secondary heuristic on Linux
    pid_t ppid = getppid();
    if (ppid == 1) return false;
    return true;
}

int DesktopSupervisor::spawnOnce() {
    if (desktopExe_.empty()) return -1;
    pid_t pid = fork();
    if (pid < 0) {
        return -1;
    }
    if (pid == 0) {
#ifdef __linux__
        // Ensure child dies if supervisor (session) dies; prefer SIGTERM so desktop shuts down gracefully
        prctl(PR_SET_PDEATHSIG, SIGTERM);
        // Race: parent may have died between fork and prctl — check ppid
        if (getppid() == 1) _exit(0);
#endif
        // Close parent pipe fds in child — only supervisor holds them
        // Child does not need them. Avoid leaking.
        // parentReadFd is intentionally not inherited as useful; close if accidentally inherited
        // (it was created with pipe, not CLOEXEC explicitly; close manually)
        // Note: parentReadFd is owned by supervisor, not child.
        // No shell: exec vector
        execlp(desktopExe_.c_str(), desktopExe_.c_str(), static_cast<char*>(0));
        _exit(127);
    }
    childPid_ = (int)pid;
    return childPid_;
}

void DesktopSupervisor::killChild(int sig) {
    if (childPid_ > 0) {
        kill((pid_t)childPid_, sig);
    }
}

int DesktopSupervisor::run() {
    // Install handlers
    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = onTerm;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = 0;
    sigaction(SIGTERM, &sa, 0);
    sigaction(SIGINT, &sa, 0);

    struct sigaction saChld;
    memset(&saChld, 0, sizeof(saChld));
    saChld.sa_handler = onChld;
    sigemptyset(&saChld.sa_mask);
    saChld.sa_flags = SA_NOCLDSTOP;
    sigaction(SIGCHLD, &saChld, 0);

    // Ignore SIGPIPE — write end closed is reported via read EOF, not signal
    signal(SIGPIPE, SIG_IGN);

    // Initial spawn
    gStop = stopping_ ? 1 : 0;
    gChildSig = 0;

    if (!stopping_ && !gaveUp_) {
        if (spawnOnce() < 0) {
            // Fork failed — count as one restart attempt then backoff
            restarts_++;
            if (restarts_ >= kMaxRestarts) gaveUp_ = true;
        }
    }

    while (!gStop && !gaveUp_) {
        // Quick parent liveness before blocking
        if (!isParentAlive()) {
            stopping_ = true;
            break;
        }

        // Wait for child OR parent pipe EOF OR SIGTERM
        // Use select on parentReadFd with 1s timeout to poll parent liveness + reap child
        // No busy polling: 1s select is idle-lightweight; child exit is caught via SIGCHLD/waitpid
        if (parentReadFd_ >= 0) {
            fd_set rfds;
            FD_ZERO(&rfds);
            FD_SET(parentReadFd_, &rfds);
            struct timeval tv;
            tv.tv_sec = 1;
            tv.tv_usec = 0;
            int r = select(parentReadFd_ + 1, &rfds, 0, 0, &tv);
            if (r > 0 && FD_ISSET(parentReadFd_, &rfds)) {
                // Parent closed pipe — die
                stopping_ = true;
                break;
            }
            if (r < 0 && errno != EINTR) {
                // select error — conservative: consider parent gone if EBADF
                if (errno == EBADF) {
                    stopping_ = true;
                    break;
                }
            }
        } else {
            // No pipe: sleep 1s interruptibly (SIGCHLD/SIGTERM wake via EINTR)
            // Use ppoll-like behavior via nanosleep loop + signal checks
            struct timespec ts;
            ts.tv_sec = 1;
            ts.tv_nsec = 0;
            while (nanosleep(&ts, &ts) == -1 && errno == EINTR) {
                if (gStop || gChildSig) break;
            }
            if (gStop) break;
            if (!isParentAlive()) { stopping_ = true; break; }
        }

        if (gStop) break;

        if (gChildSig) {
            gChildSig = 0;
            int status = 0;
            pid_t w = waitpid((pid_t)childPid_, &status, WNOHANG);
            if (w == (pid_t)childPid_) {
                childPid_ = -1;
                if (stopping_) break;
                if (restarts_ >= kMaxRestarts) {
                    gaveUp_ = true;
                    fprintf(stderr, "flamewm-desktop: giving up after %d restarts; session remains usable\n",
                            kMaxRestarts);
                    break;
                }
                // Log exit cause (no translation deps — plain fprintf)
                if (WIFEXITED(status)) {
                    if (WEXITSTATUS(status) != 0) {
                        fprintf(stderr, "flamewm-desktop exited status %d\n", WEXITSTATUS(status));
                    }
                } else if (WIFSIGNALED(status)) {
                    fprintf(stderr, "flamewm-desktop killed by signal %d\n", WTERMSIG(status));
                }
                long ms = backoffMs(restarts_);
                restarts_++;
                if (restarts_ >= kMaxRestarts && !gaveUp_) {
                    // Will be checked next loop; still sleep once
                }
                sleepMs(ms);
                if (gStop || !isParentAlive()) { stopping_ = true; break; }
                if ( gaveUp_) break;
                if (spawnOnce() < 0) {
                    // fork/exec failed — restarts already incremented; sleep again
                    sleepMs(backoffMs(restarts_ - 1));
                }
                // childPid set; loop to wait again
            } else if (w == 0) {
                // No child exited yet — spurious SIGCHLD or other child
                // Reap any other zombies non-blockingly to avoid accumulation
                int st2;
                while (waitpid(-1, &st2, WNOHANG) > 0) {}
            } else {
                // waitpid error — if ECHILD, spawn anew
                if (errno == ECHILD && childPid_ == -1 && !stopping_ && !gaveUp_) {
                    // child already reaped elsewhere; treat as exit requiring restart
                    if (restarts_ >= kMaxRestarts) { gaveUp_ = true; break; }
                    long ms = backoffMs(restarts_);
                    restarts_++;
                    sleepMs(ms);
                    if (gStop || !isParentAlive()) { stopping_ = true; break; }
                    spawnOnce();
                }
            }
        } else {
            // No SIGCHLD — check if child died without signal (rare). Non-blocking waitpid.
            if (childPid_ > 0) {
                int status = 0;
                pid_t w = waitpid((pid_t)childPid_, &status, WNOHANG);
                if (w == (pid_t)childPid_) {
                    gChildSig = 1; // will be handled next iteration
                } else if (w < 0 && errno == ECHILD) {
                    childPid_ = -1;
                    gChildSig = 1;
                }
            } else if (childPid_ == -1 && !stopping_ && !gaveUp_) {
                // No child running but we expected one — spawn
                if (restarts_ >= kMaxRestarts) { gaveUp_ = true; break; }
                long ms = backoffMs(restarts_);
                restarts_++;
                sleepMs(ms);
                if (gStop || !isParentAlive()) { stopping_ = true; break; }
                spawnOnce();
            }
        }
    }

    // Clean shutdown: forward SIGTERM to child, wait boundedly, SIGKILL if needed
    if (childPid_ > 0) {
        killChild(SIGTERM);
        int status = 0;
        // Wait up to 1.5s boundedly: 3 * 500ms slices checking waitpid
        for (int i = 0; i < 3; ++i) {
            pid_t w = waitpid((pid_t)childPid_, &status, WNOHANG);
            if (w == (pid_t)childPid_) { childPid_ = -1; break; }
            if (w < 0 && errno == ECHILD) { childPid_ = -1; break; }
            sleepMs(500);
        }
        if (childPid_ > 0) {
            killChild(SIGKILL);
            int st2 = 0;
            waitpid((pid_t)childPid_, &st2, 0);
            childPid_ = -1;
        }
    } else {
        // Reap any leftover zombies
        int st2 = 0;
        while (waitpid(-1, &st2, WNOHANG) > 0) {}
    }

    // Restore default handlers
    signal(SIGTERM, SIG_DFL);
    signal(SIGINT, SIG_DFL);
    signal(SIGCHLD, SIG_DFL);
    signal(SIGPIPE, SIG_DFL);

    // Close parent pipe read end if we own it
    if (parentReadFd_ >= 0) {
        close(parentReadFd_);
        parentReadFd_ = -1;
    }

    return gaveUp_ ? 1 : 0;
}

} // namespace session
} // namespace flamewm
