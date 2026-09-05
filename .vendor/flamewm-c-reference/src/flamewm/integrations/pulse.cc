#include "pulse.h"
#include <algorithm>
#include <sys/time.h>

// Conditional libpulse includes — build passes without dev headers.
// When available, define HAVE_FLAMEWM_PULSE and include pulse headers in the adapter only.
#ifdef HAVE_CONFIG_H
#include "config.h"
#endif

#if defined(HAVE_PULSE) || defined(HAVE_FLAMEWM_PULSE)
#include <pulse/pulseaudio.h>
#define FLAMEWM_HAS_PULSE 1
#endif

#ifndef FLAMEWM_HAS_PULSE
// Minimal forward stubs so code compiles without libpulse
struct pa_context {};
struct pa_mainloop_api {};
#ifndef PA_CONTEXT_UNCONNECTED
#define PA_CONTEXT_UNCONNECTED 0
#define PA_CONTEXT_CONNECTING 1
#define PA_CONTEXT_READY 2
#define PA_CONTEXT_FAILED 3
#define PA_CONTEXT_TERMINATED 4
#endif
#endif

namespace flamewm {
namespace integrations {

#if FLAMEWM_HAS_PULSE
struct PulseCallbackToken {
    PulseAudio* owner;
    uint64_t generation;
    PulseCallbackToken(PulseAudio* p, uint64_t g) : owner(p), generation(g) {}
};

static void pulseContextState(pa_context* context, void* userdata) {
    PulseCallbackToken* token = static_cast<PulseCallbackToken*>(userdata);
    if (token && token->owner && context)
        token->owner->onContextStateChanged((int)pa_context_get_state(context), token->generation);
}

static void pulseSubscription(pa_context*, pa_subscription_event_type_t, uint32_t, void* userdata) {
    PulseCallbackToken* token = static_cast<PulseCallbackToken*>(userdata);
    if (token && token->owner) token->owner->onSubscriptionEvent(0, token->generation);
}

static void pulseSinkInfo(pa_context*, const pa_sink_info* info, int eol, void* userdata) {
    PulseCallbackToken* token = static_cast<PulseCallbackToken*>(userdata);
    if (!token || !token->owner || eol != 0) return;
    const int percent = (int)((pa_cvolume_avg(&info->volume) * 100U + PA_VOLUME_NORM / 2U) / PA_VOLUME_NORM);
    token->owner->onSinkInfo(info->name ? info->name : "", percent, info->mute != 0, token->generation);
}
#endif

PulseAudio::PulseAudio()
    : generation_(1)
    , serverGeneration_(1)
    , pendingReconnect_(false)
    , backoffAttempts_(0)
    , currentBackoffMs_(0)
    , context_(0)
    , api_(0)
    , usingThreadedFallback_(false)
    , loopAdapter_(&nullAdapter_)
    , sinkSubscriptionActive_(false)
    , sinkReconcilePending_(false)
    , lastIntentVolume_(-1)
    , lastIntentMute_(false)
    , callbackToken_(0)
{
    status_.state = PulseDisconnected;
    status_.available = false;
}

PulseAudio::~PulseAudio() {
    disconnect();
    listeners_.clear();
}

void PulseAudio::addListener(PulseListener* l) {
    if (!l) return;
    if (std::find(listeners_.begin(), listeners_.end(), l) != listeners_.end()) return;
    listeners_.push_back(l);
}
void PulseAudio::removeListener(PulseListener* l) {
    listeners_.erase(std::remove(listeners_.begin(), listeners_.end(), l), listeners_.end());
}

void PulseAudio::setMainLoopAdapter(IMainLoopAdapter* adapter) {
    if (api_ || context_) return;
    loopAdapter_ = adapter ? adapter : &nullAdapter_;
}

void PulseAudio::connectAsync() {
    if (status_.state == PulseReady || status_.state == PulseConnecting) return;
    status_.state = PulseConnecting;
    // Native pa_context creation is compiled only when libpulse is available.
    // Callbacks carry current generation; no synchronous server operation here.
#if FLAMEWM_HAS_PULSE
    api_ = createPollAdapter();
    if (api_) {
        context_ = pa_context_new(api_, "FlameWM");
        if (context_) {
            callbackToken_ = new PulseCallbackToken(this, generation_);
            pa_context_set_state_callback(context_, pulseContextState, callbackToken_);
        }
        if (!context_ || pa_context_connect(context_, 0, PA_CONTEXT_NOAUTOSPAWN, 0) < 0) {
            if (context_) {
                pa_context_set_state_callback(context_, 0, 0);
                pa_context_unref(context_);
            }
            context_ = 0;
            delete static_cast<PulseCallbackToken*>(callbackToken_);
            callbackToken_ = 0;
            status_.state = PulseFailed;
            status_.available = false;
            scheduleReconnect();
        }
    }
#endif
    notifyListeners();
}

void PulseAudio::disconnect() {
    if (status_.state == PulseDisconnected && !context_) return;
    bumpGeneration();
    // Unregister-before-free: detach every callback before token/context delete
    // Real: pa_context_disconnect, pa_context_unref, destroyPollAdapter
    if (context_) {
#if FLAMEWM_HAS_PULSE
        pa_context_set_state_callback(context_, 0, 0);
        pa_context_set_subscribe_callback(context_, 0, 0);
        pa_context_disconnect(context_);
        pa_context_unref(context_);
        delete static_cast<PulseCallbackToken*>(callbackToken_);
        callbackToken_ = 0;
#endif
        context_ = 0;
    }
    sinkSubscriptionActive_ = false;
    sinkReconcilePending_ = false;
    if (api_) {
        destroyPollAdapter(api_);
        api_ = 0;
    }
    clearStaleState();
    status_.state = PulseDisconnected;
    status_.available = false;
    notifyListeners();
}

void PulseAudio::onServerRestart() {
    // Server restart -> disconnected -> bounded reconnect (FAILURES.md)
    bumpServerGeneration();
    disconnect();
    scheduleReconnect();
}

void PulseAudio::onContextStateChanged(int pa_state, uint64_t callerGen) {
    if (isStale(callerGen)) return;
    switch (pa_state) {
        case PA_CONTEXT_READY:
            status_.state = PulseReady;
            status_.available = true;
            resetBackoff();
            cancelReconnect();
            sinkSubscriptionActive_ = true;
            sinkReconcilePending_ = true;
#if FLAMEWM_HAS_PULSE
            pa_context_set_subscribe_callback(context_, pulseSubscription, callbackToken_);
            pa_context_subscribe(context_, PA_SUBSCRIPTION_MASK_SINK, 0, callbackToken_);
            if (status_.hasSink)
                pa_context_get_sink_info_by_name(context_, status_.sinkName.c_str(), pulseSinkInfo, callbackToken_);
#endif
            break;
        case PA_CONTEXT_FAILED:
        case PA_CONTEXT_TERMINATED:
            status_.state = PulseFailed;
            status_.available = false;
            scheduleReconnect();
            break;
        case PA_CONTEXT_CONNECTING:
        case PA_CONTEXT_UNCONNECTED:
        default:
            status_.state = PulseConnecting;
            break;
    }
    notifyListeners();
}

void PulseAudio::onSinkInfo(const std::string& name, int volPct, bool mute, uint64_t callerGen) {
    if (isStale(callerGen)) return;
    bool changed = status_.sinkName != name || status_.hasSink != !name.empty();
    status_.sinkName = name;
    status_.hasSink = !name.empty();
    if (volPct < 0) volPct = 0;
    if (volPct > 100) volPct = 100;
    changed = changed || status_.volumePercent != volPct || status_.muted != mute;
    status_.volumePercent = volPct;
    status_.muted = mute;
    if (changed) notifyListeners();
}

void PulseAudio::onSubscriptionEvent(int /*eventType*/, uint64_t callerGen) {
    if (isStale(callerGen)) return;
    if (!sinkSubscriptionActive_) return;
    // Coalesce signals; reconciliation performs one asynchronous sink query.
    sinkReconcilePending_ = true;
}

void PulseAudio::reconcileSink(uint64_t callerGen) {
    if (isStale(callerGen) || !sinkReconcilePending_) return;
    sinkReconcilePending_ = false;
#if FLAMEWM_HAS_PULSE
    if (context_ && status_.hasSink)
        pa_context_get_sink_info_by_name(context_, status_.sinkName.c_str(), pulseSinkInfo, callbackToken_);
#endif
}

bool PulseAudio::setVolume(int percent, uint64_t callerGen) {
    if (isStale(callerGen)) return false;
    if (status_.state != PulseReady) return false;
    if (percent < 0) percent = 0;
    if (percent > 100) percent = 100;
    if (status_.hasSink && status_.volumePercent == percent) return false;
    lastIntentVolume_ = percent;
#if FLAMEWM_HAS_PULSE
    if (!context_ || !status_.hasSink) return false;
    pa_cvolume volume;
    pa_cvolume_set(&volume, 2, (pa_volume_t)((percent * PA_VOLUME_NORM) / 100));
    pa_context_set_sink_volume_by_name(context_, status_.sinkName.c_str(), &volume, 0, this);
#endif
    return true;
}

bool PulseAudio::setMute(bool mute, uint64_t callerGen) {
    if (isStale(callerGen)) return false;
    if (status_.state != PulseReady) return false;
    if (status_.muted == mute) return false;
    lastIntentMute_ = mute;
#if FLAMEWM_HAS_PULSE
    if (!context_ || !status_.hasSink) return false;
    pa_context_set_sink_mute_by_name(context_, status_.sinkName.c_str(), mute ? 1 : 0, 0, this);
#endif
    return true;
}

void PulseAudio::onReconnectTimer() {
    pendingReconnect_ = false;
    if (status_.state == PulseReady) return;
    if (backoffAttempts_ >= backoff_.maxAttempts) return; // bounded, no storm
    ++backoffAttempts_;
    currentBackoffMs_ = nextBackoffMs();
    connectAsync();
    if (backoffAttempts_ < backoff_.maxAttempts) scheduleReconnect();
}

int PulseAudio::nextBackoffMs() const {
    if (backoffAttempts_ <= 0) return backoff_.initialMs;
    int ms = backoff_.initialMs;
    for (int i = 1; i < backoffAttempts_; ++i) {
        ms *= 2;
        if (ms > backoff_.maxMs) { ms = backoff_.maxMs; break; }
    }
    if (ms > backoff_.maxMs) ms = backoff_.maxMs;
    return ms;
}

void PulseAudio::resetBackoff() {
    backoffAttempts_ = 0;
    currentBackoffMs_ = 0;
}

void PulseAudio::injectStatus(const PulseStatus& s, uint64_t gen) {
    if (isStale(gen)) return;
    status_ = s;
    if (s.state == PulseReady) resetBackoff();
    notifyListeners();
}

pa_mainloop_api* PulseAudio::createPollAdapter() {
    if (api_) return api_;
    // Preferred: build pa_mainloop_api whose io/time callbacks map to
    // DBusDispatcher-style IceWM poll/timer (same IMainLoopAdapter).
    // When built without libpulse, return null and graceful degrade.
#if FLAMEWM_HAS_PULSE
    // Shared adapter is required; threaded mainloop would add an unowned thread.
    // Full callback wrapper is supplied by host adapter integration.
    if (loopAdapter_ == &nullAdapter_) return 0;
    usingThreadedFallback_ = false;
#endif
    return api_;
}

void PulseAudio::destroyPollAdapter(pa_mainloop_api* api) {
    if (!api) return;
#if FLAMEWM_HAS_PULSE
    (void)api;
#endif
    if (api == api_) api_ = 0;
}

void PulseAudio::bumpGeneration() { ++generation_; }
void PulseAudio::bumpServerGeneration() { ++serverGeneration_; ++generation_; }

void PulseAudio::clearStaleState() {
    status_.hasSink = false;
    status_.sinkName.clear();
    // keep last vol/mute for UI but mark unavailable
}

void PulseAudio::notifyListeners() {
    uint64_t gen = generation_;
    PulseStatus copy = status_;
    std::vector<PulseListener*> ls = listeners_;
    for (size_t i = 0; i < ls.size(); ++i) if (ls[i]) ls[i]->onPulseStatus(copy, gen);
}

void PulseAudio::scheduleReconnect() {
    if (pendingReconnect_) return;
    if (backoffAttempts_ >= backoff_.maxAttempts) return;
    pendingReconnect_ = true;
    // Host dispatcher owns timer; callback must call onReconnectTimer().
}

void PulseAudio::cancelReconnect() {
    pendingReconnect_ = false;
}

} // namespace integrations
} // namespace flamewm
