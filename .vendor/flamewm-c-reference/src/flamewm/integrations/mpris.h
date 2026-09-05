#ifndef FLAMEWM_INTEGRATIONS_MPRIS_H
#define FLAMEWM_INTEGRATIONS_MPRIS_H

#include <string>
#include <vector>
#include <map>
#include <stdint.h>

struct DBusPendingCall;
struct DBusConnection;

namespace flamewm { namespace integrations { class DBusDispatcher; }
}

namespace flamewm {
namespace integrations {

// MPRIS integration — session bus, signal-driven, no polling.
// Deterministic active-player rule, stale removal, property signals only.

// V4 glyph rule (explicit, state glyph not action glyph):
//   Playing -> Play glyph
//   Paused  -> Pause glyph

enum PlaybackStatus {
    PlaybackStopped = 0,
    PlaybackPlaying,
    PlaybackPaused
};

struct PlayerInfo {
    std::string busName;     // e.g. org.mpris.MediaPlayer2.firefox.instance123
    std::string identity;    // e.g. "Firefox", "VLC"
    std::string trackTitle;
    std::string artist;
    PlaybackStatus status;
    bool canPlay;
    bool canPause;
    bool canNext;
    bool canPrev;

    PlayerInfo() : status(PlaybackStopped), canPlay(false), canPause(false), canNext(false), canPrev(false) {}
};

class MprisListener {
public:
    virtual ~MprisListener() {}
    virtual void onPlayersChanged(const std::vector<PlayerInfo>& players,
                                  const std::string& activeBusName,
                                  uint64_t generation) = 0;
};

class Mpris {
public:
    Mpris();
    explicit Mpris(DBusDispatcher* dispatcher);
    ~Mpris();

    Mpris(const Mpris&) = delete;
    Mpris& operator=(const Mpris&) = delete;

    void addListener(MprisListener* l);
    void removeListener(MprisListener* l);

    uint64_t generation() const { return generation_; }
    bool isStale(uint64_t captured) const { return captured != generation_; }

    // Service owner changes (driven by DBus NameOwnerChanged on org.mpris.*)
    void onPlayerAppeared(const std::string& busName, const std::string& identity);
    void onPlayerVanished(const std::string& busName);
    // Property signal (no polling): PlaybackStatus/Metadata/Can* changed
    void onPlayerPropertiesChanged(const std::string& busName, const PlayerInfo& updated);

    // Reconcile after burst (coalesced)
    void reconcile();

    const std::vector<PlayerInfo>& players() const { return players_; }
    std::string activePlayerBusName() const { return activeBusName_; }
    const PlayerInfo* activePlayer() const;

    // Deterministic active-player rule:
    // 1) Playing player preferred (lexicographically smallest busName among Playing)
    // 2) else Paused player (smallest busName)
    // 3) else first remaining (smallest busName)
    // Single rule drives view visibility.
    static std::string pickActive(const std::vector<PlayerInfo>& players);

    // Bookmark/visibility: slot collapses when no players
    bool hasPlayers() const { return !players_.empty(); }

    // Async control intents (no blocking)
    bool requestPlay(const std::string& busName, uint64_t callerGen);
    bool requestPause(const std::string& busName, uint64_t callerGen);
    bool requestPlayPause(const std::string& busName, uint64_t callerGen);
    bool requestNext(const std::string& busName, uint64_t callerGen);
    bool requestPrev(const std::string& busName, uint64_t callerGen);

    // For testing: inject
    void injectPlayer(const PlayerInfo& p);

    void onServiceOwnerChanged(bool hasOwner);
    DBusDispatcher* dispatcher() const { return dispatcher_; }
    DBusConnection* connection() const { return connection_; }
    bool hasPlayer(const std::string& busName) const { return findPlayerIndex(busName) >= 0; }
    void trackPending(DBusPendingCall* pending);
    void untrackPending(DBusPendingCall* pending);
    void mapOwner(const std::string& unique, const std::string& wellKnown) { ownerByUnique_[unique] = wellKnown; }
    void unmapOwner(const std::string& unique) { ownerByUnique_.erase(unique); }
    std::string mappedOwner(const std::string& unique) const { std::map<std::string, std::string>::const_iterator i = ownerByUnique_.find(unique); return i == ownerByUnique_.end() ? std::string() : i->second; }

private:
    void bumpGeneration();
    void notifyListeners();
    void removeStalePlayer(const std::string& busName);
    int findPlayerIndex(const std::string& busName) const;
    void updateActive();
    void startSessionBus();
    void stopSessionBus();
    void requestPlayerProperties(const std::string& busName, uint64_t gen);

    uint64_t generation_;
    std::vector<PlayerInfo> players_;
    std::string activeBusName_;
    std::vector<MprisListener*> listeners_;
    bool reconcilePending_;
    DBusDispatcher* dispatcher_;
    DBusConnection* connection_;
    void* watchData_;
    void* timeoutData_;
    std::map<std::string, std::string> ownerByUnique_;
    std::vector<DBusPendingCall*> pendingCalls_;
};

} // namespace integrations
} // namespace flamewm

#endif
