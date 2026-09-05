#include "flamewm/platform/system/service.h"
#include "flamewm/platform/reactor/service.h"
#include "flamewm/integrations/dbusdispatcher.h"
#include "flamewm/integrations/networkmanager.h"
#include "flamewm/integrations/mpris.h"
#include "flamewm/integrations/pulse.h"

#include <algorithm>
#include <map>
#include <stdexcept>

namespace flamewm {
namespace platform {
namespace system {

// Forwarders defined before Impl so ctor can use complete types.
class NmForwarder : public integrations::NetworkManagerListener {
public:
    explicit NmForwarder(SystemService* s) : svc_(s) {}
    void onNetworkStatus(const integrations::NetworkStatus& st, uint64_t gen) {
        if (!svc_ || !svc_->implForListener()) return;
        if (svc_->networkManager() && svc_->networkManager()->isStale(gen)) return;
        ServiceState ss = ServiceState::Unknown;
        std::string detail;
        switch (st.state) {
            case integrations::NetworkUnavailable: ss = ServiceState::Unavailable; detail = ""; break;
            case integrations::NetworkDisconnected: ss = ServiceState::Unavailable; detail = "disconnected"; break;
            case integrations::NetworkWired: ss = ServiceState::Available; detail = "wired"; break;
            case integrations::NetworkWifiConnecting: ss = ServiceState::Available; detail = st.activeSsid.empty() ? "wifi connecting" : st.activeSsid + " (connecting)"; break;
            case integrations::NetworkWifiConnected: ss = ServiceState::Available; detail = st.activeSsid.empty() ? "connected" : st.activeSsid; break;
            default: ss = ServiceState::Unknown; detail = st.activeSsid; break;
        }
        if (st.state == integrations::NetworkUnavailable) ss = ServiceState::Unavailable;
        svc_->setNetworkManagerState(ss, detail);
    }
private:
    SystemService* svc_;
};

class MprisForwarder : public integrations::MprisListener {
public:
    explicit MprisForwarder(SystemService* s) : svc_(s) {}
    void onPlayersChanged(const std::vector<integrations::PlayerInfo>& players,
                          const std::string& activeBusName, uint64_t gen) {
        if (!svc_ || !svc_->implForListener()) return;
        if (svc_->mpris() && svc_->mpris()->isStale(gen)) return;
        if (players.empty()) { svc_->setMprisState(ServiceState::Unavailable, ""); return; }
        std::string title;
        for (size_t i = 0; i < players.size(); ++i) {
            if (players[i].busName == activeBusName) {
                title = players[i].trackTitle.empty() ? players[i].identity : players[i].trackTitle;
                if (!players[i].artist.empty() && !title.empty()) title = players[i].artist + " - " + title;
                break;
            }
        }
        if (title.empty() && !activeBusName.empty()) title = activeBusName;
        svc_->setMprisState(ServiceState::Available, title);
    }
private:
    SystemService* svc_;
};

class PulseForwarder : public integrations::PulseListener {
public:
    explicit PulseForwarder(SystemService* s) : svc_(s) {}
    void onPulseStatus(const integrations::PulseStatus& st, uint64_t gen) {
        if (!svc_ || !svc_->implForListener()) return;
        if (svc_->pulseAudio() && svc_->pulseAudio()->isStale(gen)) return;
        if (st.state == integrations::PulseReady && st.available) svc_->setPulseState(ServiceState::Available);
        else if (st.state == integrations::PulseDisconnected || st.state == integrations::PulseFailed) svc_->setPulseState(ServiceState::Unavailable);
        else svc_->setPulseState(ServiceState::Unknown);
    }
private:
    SystemService* svc_;
};

struct SystemService::Impl {
    reactor::ReactorService* reactor;
    SystemSnapshot snap;
    int nextListenerId;
    std::map<int, std::function<void(SystemSnapshot)> > listeners;
    uint64_t nmGeneration;
    uint64_t mprisGeneration;
    uint64_t pulseGeneration;
    int pulseReconnectTimer;
    uint64_t pulseBackoffMs;
    bool notifyPending;
    int notifyDeferHandle;
    ::flamewm::integrations::ReactorBridge* dbusBridge;
    ::flamewm::integrations::NetworkManager* nm;
    ::flamewm::integrations::Mpris* mpris;
    ::flamewm::integrations::PulseAudio* pulse;
    ::flamewm::integrations::DBusDispatcher* dispatcher;
    NmForwarder* nmFwd;
    MprisForwarder* mprisFwd;
    PulseForwarder* pulseFwd;
    bool integrationsBound;
    Impl(reactor::ReactorService* r)
        : reactor(r), snap(), nextListenerId(1), nmGeneration(0), mprisGeneration(0), pulseGeneration(0)
        , pulseReconnectTimer(0), pulseBackoffMs(0), notifyPending(false), notifyDeferHandle(0)
        , dbusBridge(0), nm(0), mpris(0), pulse(0), dispatcher(0), nmFwd(0), mprisFwd(0), pulseFwd(0), integrationsBound(false) {}
    void cancelPulseReconnect() { if (pulseReconnectTimer && reactor) reactor->removeTimer(pulseReconnectTimer); pulseReconnectTimer = 0; }
    void notifyNow() { std::map<int, std::function<void(SystemSnapshot)> > copy = listeners; for (auto &kv : copy) if (kv.second) kv.second(snap); }
    void scheduleNotify() {
        if (notifyPending) return;
        notifyPending = true;
        Impl* self = this;
        reactor->defer([self]() { self->notifyPending = false; self->notifyDeferHandle = 0; self->notifyNow(); });
    }
};

SystemService::SystemService(reactor::ReactorService* reactor) : impl_(new Impl(reactor)) {
    if (!reactor) { delete impl_; impl_ = nullptr; throw std::invalid_argument("SystemService: reactor must not be null"); }
    if (!reactor->port()) { delete impl_; impl_ = nullptr; throw std::invalid_argument("SystemService: reactor has no MainLoopPort"); }
    try {
        impl_->dbusBridge = new integrations::ReactorBridge(reactor);
        impl_->dispatcher = new integrations::DBusDispatcher(impl_->dbusBridge);
        impl_->nm = new integrations::NetworkManager(impl_->dispatcher);
        impl_->mpris = new integrations::Mpris(impl_->dispatcher);
        impl_->pulse = new integrations::PulseAudio();
        impl_->pulse->setMainLoopAdapter(impl_->dbusBridge);
        impl_->pulse->connectAsync();
        impl_->nmFwd = new NmForwarder(this);
        impl_->nm->addListener(impl_->nmFwd);
        impl_->mprisFwd = new MprisForwarder(this);
        impl_->mpris->addListener(impl_->mprisFwd);
        impl_->pulseFwd = new PulseForwarder(this);
        impl_->pulse->addListener(impl_->pulseFwd);
        impl_->nmFwd->onNetworkStatus(impl_->nm->status(), impl_->nm->generation());
        impl_->mprisFwd->onPlayersChanged(impl_->mpris->players(), impl_->mpris->activePlayerBusName(), impl_->mpris->generation());
        impl_->pulseFwd->onPulseStatus(impl_->pulse->status(), impl_->pulse->generation());
        impl_->integrationsBound = true;
    } catch (...) {
        if (impl_->pulseFwd) { if (impl_->pulse) impl_->pulse->removeListener(impl_->pulseFwd); delete impl_->pulseFwd; impl_->pulseFwd = 0; }
        if (impl_->mprisFwd) { if (impl_->mpris) impl_->mpris->removeListener(impl_->mprisFwd); delete impl_->mprisFwd; impl_->mprisFwd = 0; }
        if (impl_->nmFwd) { if (impl_->nm) impl_->nm->removeListener(impl_->nmFwd); delete impl_->nmFwd; impl_->nmFwd = 0; }
        if (impl_->pulse) { impl_->pulse->disconnect(); delete impl_->pulse; impl_->pulse = 0; }
        if (impl_->mpris) { delete impl_->mpris; impl_->mpris = 0; }
        if (impl_->nm) { delete impl_->nm; impl_->nm = 0; }
        if (impl_->dispatcher) { impl_->dispatcher->shutdown(); delete impl_->dispatcher; impl_->dispatcher = 0; }
        if (impl_->dbusBridge) { delete impl_->dbusBridge; impl_->dbusBridge = 0; }
        impl_->integrationsBound = false;
    }
}

SystemService::~SystemService() {
    if (impl_) {
        impl_->cancelPulseReconnect();
        if (impl_->nm && impl_->nmFwd) impl_->nm->removeListener(impl_->nmFwd);
        if (impl_->mpris && impl_->mprisFwd) impl_->mpris->removeListener(impl_->mprisFwd);
        if (impl_->pulse && impl_->pulseFwd) impl_->pulse->removeListener(impl_->pulseFwd);
        if (impl_->pulse) impl_->pulse->disconnect();
        if (impl_->dispatcher) impl_->dispatcher->shutdown();
        delete impl_->pulseFwd;
        delete impl_->mprisFwd;
        delete impl_->nmFwd;
        delete impl_->pulse;
        delete impl_->mpris;
        delete impl_->nm;
        delete impl_->dispatcher;
        delete impl_->dbusBridge;
        impl_->listeners.clear();
        delete impl_;
    }
}

reactor::ReactorService* SystemService::reactor() const { return impl_ ? impl_->reactor : nullptr; }
SystemSnapshot SystemService::snapshot() const { return impl_->snap; }

void SystemService::setNetworkManagerState(ServiceState s, const std::string& detail) {
    bool changed = false;
    if (impl_->snap.networkManager != s) { impl_->snap.networkManager = s; changed = true; }
    if (impl_->snap.networkState != detail) { impl_->snap.networkState = detail; changed = true; }
    if (changed) impl_->scheduleNotify();
}
void SystemService::setMprisState(ServiceState s, const std::string& player) {
    bool changed = false;
    if (impl_->snap.mpris != s) { impl_->snap.mpris = s; changed = true; }
    if (impl_->snap.mprisPlayer != player) { impl_->snap.mprisPlayer = player; changed = true; }
    if (changed) impl_->scheduleNotify();
}
void SystemService::setPulseState(ServiceState s) {
    bool changed = false;
    if (impl_->snap.pulse != s) { impl_->snap.pulse = s; changed = true; }
    bool avail = (s == ServiceState::Available);
    if (impl_->snap.pulseAvailable != avail) { impl_->snap.pulseAvailable = avail; changed = true; }
    if (changed) {
        if (s == ServiceState::Available) { impl_->pulseBackoffMs = 0; impl_->cancelPulseReconnect(); }
        impl_->scheduleNotify();
    }
}
void SystemService::onNetworkManagerOwnerLost() {
    ++impl_->nmGeneration;
    if (impl_->nm) impl_->nm->onServiceVanished();
    bool changed = false;
    if (impl_->snap.networkManager != ServiceState::Unavailable) { impl_->snap.networkManager = ServiceState::Unavailable; changed = true; }
    if (!impl_->snap.networkState.empty()) { impl_->snap.networkState.clear(); changed = true; }
    if (changed) impl_->scheduleNotify();
}
void SystemService::onMprisOwnerLost(const std::string& player) {
    ++impl_->mprisGeneration;
    if (impl_->mpris) { if (player.empty()) impl_->mpris->onServiceOwnerChanged(false); else impl_->mpris->onPlayerVanished(player); }
    bool changed = false;
    if (impl_->snap.mprisPlayer == player || player.empty()) {
        if (impl_->snap.mpris != ServiceState::Unavailable) { impl_->snap.mpris = ServiceState::Unavailable; changed = true; }
        if (!impl_->snap.mprisPlayer.empty()) { impl_->snap.mprisPlayer.clear(); changed = true; }
    }
    if (changed) impl_->scheduleNotify();
}
void SystemService::onPulseDisconnected() {
    ++impl_->pulseGeneration;
    if (impl_->pulse) impl_->pulse->onServerRestart();
    setPulseState(ServiceState::Unavailable);
    schedulePulseReconnect();
}
void SystemService::schedulePulseReconnect() {
    if (impl_->pulseReconnectTimer) return;
    uint64_t backoff = impl_->pulseBackoffMs;
    if (backoff == 0) backoff = 250; else if (backoff < 30000) backoff = std::min<uint64_t>(backoff * 2, 30000);
    impl_->pulseBackoffMs = backoff;
    uint64_t capturedGen = impl_->pulseGeneration;
    SystemService* self = this; Impl* ip = impl_;
    int h = impl_->reactor->addTimer(backoff, [self, ip, capturedGen]() {
        if (!self || !ip) return;
        if (capturedGen != ip->pulseGeneration) return;
        ip->pulseReconnectTimer = 0;
        (void)self;
    }, false);
    impl_->pulseReconnectTimer = h;
}
int SystemService::addListener(std::function<void(SystemSnapshot)> cb) { if (!cb) return 0; int id = impl_->nextListenerId++; impl_->listeners[id] = cb; return id; }
void SystemService::removeListener(int id) { auto it = impl_->listeners.find(id); if (it != impl_->listeners.end()) impl_->listeners.erase(it); }
void SystemService::bindNetworkManager(integrations::NetworkManager* nm) { if (!impl_ || impl_->integrationsBound || !nm) return; impl_->nm = nm; impl_->nmFwd = new NmForwarder(this); impl_->nm->addListener(impl_->nmFwd); impl_->nmFwd->onNetworkStatus(impl_->nm->status(), impl_->nm->generation()); }
void SystemService::bindMpris(integrations::Mpris* m) { if (!impl_ || impl_->integrationsBound || !m) return; impl_->mpris = m; impl_->mprisFwd = new MprisForwarder(this); impl_->mpris->addListener(impl_->mprisFwd); impl_->mprisFwd->onPlayersChanged(impl_->mpris->players(), impl_->mpris->activePlayerBusName(), impl_->mpris->generation()); }
void SystemService::bindPulse(integrations::PulseAudio* p) { if (!impl_ || impl_->integrationsBound || !p) return; impl_->pulse = p; impl_->pulseFwd = new PulseForwarder(this); impl_->pulse->addListener(impl_->pulseFwd); impl_->pulseFwd->onPulseStatus(impl_->pulse->status(), impl_->pulse->generation()); }
integrations::ReactorBridge* SystemService::dbusBridge() { return impl_ ? impl_->dbusBridge : 0; }
integrations::NetworkManager* SystemService::networkManager() { return impl_ ? impl_->nm : 0; }
integrations::Mpris* SystemService::mpris() { return impl_ ? impl_->mpris : 0; }
integrations::PulseAudio* SystemService::pulseAudio() { return impl_ ? impl_->pulse : 0; }
uint64_t SystemService::nmGeneration() const { return impl_ ? impl_->nmGeneration : 0; }
uint64_t SystemService::mprisGeneration() const { return impl_ ? impl_->mprisGeneration : 0; }
uint64_t SystemService::pulseGeneration() const { return impl_ ? impl_->pulseGeneration : 0; }
bool SystemService::hasReactorBridge() const { return impl_ && impl_->dbusBridge && impl_->dbusBridge->isAttached(); }

} // namespace system
} // namespace platform
} // namespace flamewm
