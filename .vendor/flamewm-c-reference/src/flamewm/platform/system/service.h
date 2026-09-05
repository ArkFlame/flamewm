#ifndef FLAMEWM_PLATFORM_SYSTEM_SERVICE_H
#define FLAMEWM_PLATFORM_SYSTEM_SERVICE_H

#include <cstdint>
#include <functional>
#include <string>

namespace flamewm { namespace integrations { class ReactorBridge; class NetworkManager; class Mpris; class PulseAudio; class DBusDispatcher; } }
namespace flamewm {
namespace platform {
namespace reactor { class ReactorService; }
namespace system {

enum class ServiceState {
    Unknown = 0,
    Available = 1,
    Unavailable = 2
};

struct SystemSnapshot {
    ServiceState networkManager;
    ServiceState mpris;
    ServiceState pulse;
    std::string networkState;
    std::string mprisPlayer;
    bool pulseAvailable;

    SystemSnapshot()
        : networkManager(ServiceState::Unknown)
        , mpris(ServiceState::Unknown)
        , pulse(ServiceState::Unknown)
        , networkState()
        , mprisPlayer()
        , pulseAvailable(false) {}
};

class SystemService {
public:
    explicit SystemService(reactor::ReactorService* reactor);
    ~SystemService();

    SystemService(const SystemService&) = delete;
    SystemService& operator=(const SystemService&) = delete;

    reactor::ReactorService* reactor() const;

    SystemSnapshot snapshot() const;

    void setNetworkManagerState(ServiceState s, const std::string& detail);
    void setMprisState(ServiceState s, const std::string& player);
    void setPulseState(ServiceState s);

    // Event-driven service lifecycle (no polling). Owner loss clears stale
    // state immediately; reconnect is bounded backoff via reactor timer.
    void onNetworkManagerOwnerLost();
    void onMprisOwnerLost(const std::string& player);
    void onPulseDisconnected();
    void schedulePulseReconnect();

    // C19 wiring: bind real integrations behind single reactor event-loop.
    // No second D-Bus reactor is created; all fd/timer work multiplexes
    // through ReactorService. Ownership: SystemService owns the integrations
    // it creates via bind*(). Unregister-before-free preserved on destruction.
    void bindNetworkManager(integrations::NetworkManager* nm);
    void bindMpris(integrations::Mpris* mpris);
    void bindPulse(integrations::PulseAudio* pulse);
    // DBus/Pulse bridge accessors — null if not bound.
    integrations::ReactorBridge* dbusBridge();
    integrations::NetworkManager* networkManager();
    integrations::Mpris* mpris();
    integrations::PulseAudio* pulseAudio();
    uint64_t nmGeneration() const;
    uint64_t mprisGeneration() const;
    uint64_t pulseGeneration() const;
    bool hasReactorBridge() const;

    int addListener(std::function<void(SystemSnapshot)> cb);
    void removeListener(int id);

    bool implForListener() const { return impl_ != nullptr; }

private:
    struct Impl;
    Impl* impl_;
};

} // namespace system
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_SYSTEM_SERVICE_H
