#ifndef FLAMEWM_INTEGRATIONS_NETWORKMANAGER_H
#define FLAMEWM_INTEGRATIONS_NETWORKMANAGER_H

#include <string>
#include <vector>
#include <stdint.h>
#include <map>
#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
#include <dbus/dbus.h>
#endif

namespace flamewm {
namespace integrations {

// NetworkManager integration — system bus, async only, no blocking CLI polling.
//
// Security invariant: secrets never appear in argv, logs, or custom plaintext
// storage. New protected networks use a secure NM secret path/agent or
// delegate to host settings. Known/saved profiles connect directly.
//
// Owner loss clears stale object paths before reconnect (FAILURES.md).

enum NetworkState {
    NetworkUnavailable = 0,
    NetworkDisconnected,
    NetworkWired,
    NetworkWifiConnecting,
    NetworkWifiConnected
};

struct AccessPoint {
    std::string path;      // D-Bus object path
    std::string ssid;
    int strength;          // 0..100
    bool secured;          // requires secret
    bool known;            // saved profile exists
};

struct NetworkStatus {
    NetworkState state;
    std::string activeSsid;
    int activeStrength;    // 0..100
    std::string activePath;
    std::vector<AccessPoint> visibleAps;
    bool wifiEnabled;
    bool networkingEnabled;

    NetworkStatus() : state(NetworkUnavailable), activeStrength(0), wifiEnabled(false), networkingEnabled(false) {}
};

class DBusDispatcher;

class NetworkManagerListener {
public:
    virtual ~NetworkManagerListener() {}
    virtual void onNetworkStatus(const NetworkStatus& s, uint64_t generation) = 0;
};

// Bounded backoff config
struct BackoffConfig {
    int initialMs;  // e.g. 200
    int maxMs;      // e.g. 8000
    int maxAttempts;// e.g. 8 then stay disconnected until owner appears
    BackoffConfig() : initialMs(250), maxMs(8000), maxAttempts(8) {}
};

class NetworkManager {
public:
    NetworkManager();
    explicit NetworkManager(DBusDispatcher* dispatcher);
    ~NetworkManager();

    NetworkManager(const NetworkManager&) = delete;
    NetworkManager& operator=(const NetworkManager&) = delete;

    // Listener lifecycle: caller owns listener; must remove before listener destroyed.
    void addListener(NetworkManagerListener* l);
    void removeListener(NetworkManagerListener* l);

    // Generation for stale callback guards
    uint64_t generation() const { return generation_; }
    bool isStale(uint64_t captured) const { return captured != generation_; }

    // Service lifecycle (driven by dispatcher owner changes)
    void onServiceOwnerChanged(bool hasOwner, uint64_t dispatcherGen);
    void onServiceAppeared();
    void onServiceVanished(); // clears stale paths, bumps generation, schedules backoff

    const NetworkStatus& status() const { return status_; }
    bool isAvailable() const { return available_; }

    // Operations — async intent only, never blocking
    // Returns false if unavailable or validation fails.
    bool setWifiEnabled(bool enabled);
    bool requestConnectKnown(const std::string& apPath, uint64_t callerGen);
    // For new protected networks: do NOT pass secret here. This triggers secure path.
    // Caller must have collected secret via agent/host-settings handoff.
    // The implementation will use NM AddAndActivate with proper secret agent flow,
    // not argv/log. Passing raw secret here is intentionally unsupported.
    bool requestConnectNewSecure(const std::string& apPath, uint64_t callerGen);
    bool requestDisconnect(uint64_t callerGen);
    bool requestScan();

    // Backoff handling — call from timer
    void onReconnectTimer();
    int nextBackoffMs() const;
    void resetBackoff();

    // For testing: inject status directly (simulates D-Bus signal)
    void injectStatus(const NetworkStatus& s, uint64_t gen);

    // AP refresh coalescing
    void requestApRefreshCoalesced();

    // Stats
    int backoffAttempts() const { return backoffAttempts_; }

private:
    struct Impl;

    void bumpGeneration();
    void clearStaleState();
    void notifyListeners();
    void scheduleReconnect();
    void cancelReconnect();

    uint64_t generation_;
    NetworkStatus status_;
    bool available_;
    bool pendingReconnect_;
    int backoffAttempts_;
    BackoffConfig backoff_;
    int currentBackoffMs_;
    bool apRefreshPending_;

    std::vector<NetworkManagerListener*> listeners_;
    std::map<std::string, std::string> knownProfiles_; // apPath -> connection path

    // Reconnect timer owned externally via dispatcher; we track id here
    int reconnectTimerId_;

    DBusDispatcher* dispatcher_;
    bool ownsDispatcher_;
    Impl* impl_;

#if defined(HAVE_DBUS) || defined(HAVE_FLAMEWM_DBUS)
    void trackPending(struct DBusPendingCall* pending);
    void untrackPending(struct DBusPendingCall* pending);
    static dbus_bool_t addWatch(struct DBusWatch* watch, void* data);
    static void removeWatch(struct DBusWatch* watch, void* data);
    static void toggleWatch(struct DBusWatch* watch, void* data);
    static dbus_bool_t addTimeout(struct DBusTimeout* timeout, void* data);
    static void removeTimeout(struct DBusTimeout* timeout, void* data);
    static void toggleTimeout(struct DBusTimeout* timeout, void* data);
    static DBusHandlerResult filterMessage(struct DBusConnection* connection,
                                       struct DBusMessage* message, void* data);
    static void rootReply(struct DBusPendingCall* pending, void* data);
    static void reply(struct DBusPendingCall* pending, void* data);
    void handleSignal(struct DBusMessage* message);
    void handleRootProperties(struct DBusMessage* message, uint64_t capturedGeneration);
#endif
};

} // namespace integrations
} // namespace flamewm

#endif
