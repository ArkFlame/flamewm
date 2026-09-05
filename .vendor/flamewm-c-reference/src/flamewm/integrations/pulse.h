#ifndef FLAMEWM_INTEGRATIONS_PULSE_H
#define FLAMEWM_INTEGRATIONS_PULSE_H

#include <string>
#include <vector>
#include <stdint.h>
#include "dbusdispatcher.h"

// Forward decl to avoid hard libpulse dependency; graceful degrade without dev headers.
struct pa_context;
struct pa_mainloop_api;

namespace flamewm {
namespace integrations {

enum PulseState {
    PulseDisconnected = 0,
    PulseConnecting,
    PulseReady,
    PulseFailed
};

struct PulseStatus {
    PulseState state;
    bool hasSink;
    std::string sinkName;
    int volumePercent; // 0..100 (averaged or mono)
    bool muted;
    bool available; // false when server absent

    PulseStatus() : state(PulseDisconnected), hasSink(false), volumePercent(0), muted(false), available(false) {}
};

class PulseListener {
public:
    virtual ~PulseListener() {}
    virtual void onPulseStatus(const PulseStatus& s, uint64_t generation) = 0;
};

// Preferred async pa_mainloop_api adapter onto IceWM poll/timer.
// If libpulse headers absent, this still compiles (graceful degrade).
// If threaded fallback ever used, marshal to X owner thread (document PSS cost).

struct BackoffPulse {
    int initialMs;
    int maxMs;
    int maxAttempts;
    BackoffPulse() : initialMs(300), maxMs(8000), maxAttempts(8) {}
};

class PulseAudio {
public:
    PulseAudio();
    ~PulseAudio();

    PulseAudio(const PulseAudio&) = delete;
    PulseAudio& operator=(const PulseAudio&) = delete;

    void addListener(PulseListener* l);
    void removeListener(PulseListener* l);
    void setMainLoopAdapter(IMainLoopAdapter* adapter);

    uint64_t generation() const { return generation_; }
    uint64_t serverGeneration() const { return serverGeneration_; }
    bool isStale(uint64_t captured) const { return captured != generation_; }
    bool isServerStale(uint64_t capturedServerGen) const { return capturedServerGen != serverGeneration_; }

    const PulseStatus& status() const { return status_; }
    PulseState state() const { return status_.state; }

    // Lifecycle — call from dispatcher/owner thread only
    void connectAsync();      // starts async connect, sets Connecting
    void disconnect();        // clears watches, bumps generation
    void onServerRestart();   // disconnected -> bounded reconnect (like NetworkManager vanish)
    void onContextStateChanged(int pa_state, uint64_t callerGen);
    void onSinkInfo(const std::string& name, int volPct, bool mute, uint64_t callerGen);
    void onSubscriptionEvent(int eventType, uint64_t callerGen);

    // Controls — async, generation-guarded, no blocking
    bool setVolume(int percent, uint64_t callerGen);
    bool setMute(bool mute, uint64_t callerGen);

    // Adapter callback uses this to perform one deferred sink reconciliation.
    void reconcileSink(uint64_t callerGen);

    // Backoff
    void onReconnectTimer();
    int nextBackoffMs() const;
    void resetBackoff();
    int backoffAttempts() const { return backoffAttempts_; }

    // For testing
    void injectStatus(const PulseStatus& s, uint64_t gen);

    // Adapter hooks — real libpulse wiring calls these
    // Preferred: adapter installs pa_mainloop_api that forwards to IceWM poll/timer
    pa_mainloop_api* createPollAdapter(); // returns owned api or null if unavailable
    void destroyPollAdapter(pa_mainloop_api* api);

    // Document threaded fallback cost: caller must note PSS/thread overhead
    static const char* threadedFallbackPssNote() {
        return "Threaded libpulse fallback adds ~1 helper thread + PSS; marshal callbacks to X owner thread";
    }

private:
    void bumpGeneration();
    void bumpServerGeneration();
    void clearStaleState();
    void notifyListeners();
    void scheduleReconnect();
    void cancelReconnect();

    uint64_t generation_;
    uint64_t serverGeneration_;
    PulseStatus status_;
    bool pendingReconnect_;
    int backoffAttempts_;
    BackoffPulse backoff_;
    int currentBackoffMs_;

    std::vector<PulseListener*> listeners_;

    // Opaque handles — only valid when Pulse available
    pa_context* context_;
    pa_mainloop_api* api_;
    bool usingThreadedFallback_;
    IMainLoopAdapter* loopAdapter_;
    NullMainLoopAdapter nullAdapter_;
    bool sinkSubscriptionActive_;
    bool sinkReconcilePending_;
    int lastIntentVolume_;
    bool lastIntentMute_;
    void* callbackToken_;
};

} // namespace integrations
} // namespace flamewm

#endif
