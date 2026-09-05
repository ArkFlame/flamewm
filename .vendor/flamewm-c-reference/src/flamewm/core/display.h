#ifndef FLAMEWM_CORE_DISPLAY_H
#define FLAMEWM_CORE_DISPLAY_H

#include "types.h"
#include <string>
#include <vector>
#include <map>
#include <stdint.h>
#ifdef CONFIG_XRANDR
#include <X11/extensions/Xrandr.h>
#endif

namespace flamewm {

// Output model — matches V5 contract, no numeric index persistence.
struct DisplayMode {
#ifdef CONFIG_XRANDR
    RRMode id;
#else
    uint32_t id;
#endif
    int width;
    int height;
    int refreshMilliHz; // e.g. 60000
    DisplayMode() : id(0), width(0), height(0), refreshMilliHz(0) {}
#ifdef CONFIG_XRANDR
    DisplayMode(RRMode i,int w,int h,int r):id(i),width(w),height(h),refreshMilliHz(r){}
#else
    DisplayMode(uint32_t i,int w,int h,int r):id(i),width(w),height(h),refreshMilliHz(r){}
#endif
    bool operator==(const DisplayMode& o) const { return id==o.id && width==o.width && height==o.height; }
};

struct OutputInfo {
    std::string connector; // e.g. HDMI-1
    std::string edidId;    // hex EDID hash, may be empty
    bool connected;
    bool enabled;
    bool primary;
#ifdef CONFIG_XRANDR
    RROutput rrOutput;
    RRCrtc rrCrtc;
    Rotation rotationRaw;
#else
    uint32_t rrOutput;
    uint32_t rrCrtc;
#endif
    std::vector<DisplayMode> availableModes;
    DisplayMode currentMode;
    int posX;
    int posY;
    int rotation; // 0,90,180,270
    int shellScale; // 100/125/150/175/200 percent, Flame chrome only

#ifdef CONFIG_XRANDR
    OutputInfo():connected(false),enabled(false),primary(false),rrOutput(0),rrCrtc(0),rotationRaw(0),posX(0),posY(0),rotation(0),shellScale(100){}
#else
    OutputInfo():connected(false),enabled(false),primary(false),rrOutput(0),rrCrtc(0),posX(0),posY(0),rotation(0),shellScale(100){}
#endif

    OutputId durableId() const;
    std::string durableKey() const;
};

// Durable identity: EDID+connector else connector fallback, never numeric index.
inline OutputId OutputInfo::durableId() const {
    OutputId id;
    id.connector = connector;
    id.edidHash = edidId;
    if (!edidId.empty()) id.durableId = edidId + ":" + connector;
    else id.durableId = connector;
    return id;
}
inline std::string OutputInfo::durableKey() const { return durableId().durableId; }

// Topology snapshot with generation for stale detection.
struct DisplaySnapshot {
    uint64_t generation; // monotonic topology generation
    std::vector<OutputInfo> outputs;
    DisplaySnapshot():generation(0){}
    const OutputInfo* findByDurable(const std::string& key) const;
    const OutputInfo* findByConnector(const std::string& conn) const;
};

// Resolution transaction owned by WM/display authority.
// Flow: Settings request -> validate -> capture old -> apply -> observe RandR -> bounded revert timer -> Keep/Revert/timeout/crash revert.
// Do not persist unconfirmed mode. Settings crash reverts via deadline.
enum ResolutionTxState { TxIdle, TxPending, TxConfirmed, TxReverted, TxTimedOut };

struct ResolutionTransaction {
    uint64_t txId;
    uint64_t generation; // snapshot generation this tx was based on
    std::string outputKey; // durableKey of target output
    DisplayMode candidateMode;
    DisplayMode oldMode;
    int oldPosX;
    int oldPosY;
    int oldRotation;
    uint64_t deadlineMs; // monotonic ms deadline for revert
    ResolutionTxState state;

    ResolutionTransaction():txId(0),generation(0),oldPosX(0),oldPosY(0),oldRotation(0),deadlineMs(0),state(TxIdle){}
    bool isPending() const { return state==TxPending; }
    bool isExpired(uint64_t nowMs) const { return isPending() && nowMs >= deadlineMs; }
};

class DisplayManager {
public:
    DisplayManager();
    // Snapshot access - Settings reads fresh snapshot each time.
    DisplaySnapshot snapshot() const { return snap_; }
    uint64_t generation() const { return snap_.generation; }

    // UI state only — never persisted as authority.
    void setSelectedOutputId(const std::string& key) { selectedOutputId_ = key; }
    const std::string& selectedOutputId() const { return selectedOutputId_; }
    const OutputInfo* selectedOutput() const;

    // Scale per-output Flame chrome only, 100-200 buckets.
    bool setShellScale(const std::string& durableKey, int pct, std::string* error);

    // Transaction API — owned by WM/display authority.
    // validate + capture old + create pending tx. Returns false if invalid.
    bool beginResolutionTx(const std::string& outputKey, const DisplayMode& mode, uint64_t nowMs, uint64_t timeoutMs, std::string* error);
    bool confirmTx(uint64_t txId, std::string* error); // user pressed Keep
    bool revertTx(uint64_t txId, std::string* error);  // user pressed Revert or timeout/crash
    bool onTimeoutCheck(uint64_t nowMs); // returns true if reverted due to timeout
    bool onHotplug(const DisplaySnapshot& fresh); // stale tx rejected if generation mismatch

    // Called on Settings crash — revert any pending tx.
    void onSettingsCrash();

    // Access pending tx if any.
    const ResolutionTransaction* pendingTx() const { return pending_.isPending() ? &pending_ : 0; }

    // For tests / WM authority to inject fresh snapshot (simulates RandR observe).
    void injectSnapshot(const DisplaySnapshot& s);

private:
    DisplaySnapshot snap_;
    std::string selectedOutputId_; // UI state only
    ResolutionTransaction pending_;
    uint64_t nextTxId_;
    std::map<std::string,int> persistedScales_; // durableKey->pct (confirmation-gated for mode, immediate for scale? scale immediate)
};

} // namespace flamewm
#endif
