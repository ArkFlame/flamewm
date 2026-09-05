#include "display.h"
#include "../ui/metrics.h"

namespace flamewm {

DisplayManager::DisplayManager(): nextTxId_(1) {}

const OutputInfo* DisplaySnapshot::findByDurable(const std::string& key) const {
    for (size_t i=0;i<outputs.size();++i) if (outputs[i].durableKey()==key) return &outputs[i];
    return 0;
}
const OutputInfo* DisplaySnapshot::findByConnector(const std::string& conn) const {
    for (size_t i=0;i<outputs.size();++i) if (outputs[i].connector==conn) return &outputs[i];
    return 0;
}

const OutputInfo* DisplayManager::selectedOutput() const {
    if (selectedOutputId_.empty()) return 0;
    return snap_.findByDurable(selectedOutputId_);
}

bool DisplayManager::setShellScale(const std::string& durableKey, int pct, std::string* error) {
    if (!isSupportedScale(pct)) { if(error)*error="scale must be 100/125/150/175/200"; return false; }
    if (durableKey.empty()) { if(error)*error="empty output key"; return false; }
    // Flame chrome only — no global metric mutation.
    for (size_t i=0;i<snap_.outputs.size();++i) {
        if (snap_.outputs[i].durableKey()==durableKey) {
            snap_.outputs[i].shellScale = pct;
            break;
        }
    }
    persistedScales_[durableKey]=pct;
    return true;
}

bool DisplayManager::beginResolutionTx(const std::string& outputKey, const DisplayMode& mode, uint64_t nowMs, uint64_t timeoutMs, std::string* error) {
    if (outputKey.empty()) { if(error)*error="empty outputKey"; return false; }
    const OutputInfo* out = snap_.findByDurable(outputKey);
    if (!out) { if(error)*error="output not found"; return false; }
    bool modeFound=false;
    for (size_t i=0;i<out->availableModes.size();++i) if (out->availableModes[i].id==mode.id) { modeFound=true; break; }
    if (!modeFound) { if(error)*error="mode not available"; return false; }
    if (pending_.isPending()) { if(error)*error="tx already pending"; return false; }
    pending_.txId = nextTxId_++;
    pending_.generation = snap_.generation;
    pending_.outputKey = outputKey;
    pending_.candidateMode = mode;
    pending_.oldMode = out->currentMode;
    pending_.oldPosX = out->posX;
    pending_.oldPosY = out->posY;
    pending_.oldRotation = out->rotation;
    pending_.deadlineMs = nowMs + timeoutMs;
    pending_.state = TxPending;
    // Do not persist unconfirmed — apply is provisional, observed via RandR.
    return true;
}

bool DisplayManager::confirmTx(uint64_t txId, std::string* error) {
    if (!pending_.isPending() || pending_.txId != txId) { if(error)*error="no such pending tx"; return false; }
    pending_.state = TxConfirmed;
    // Now mode would be persisted by caller (WM) — we just mark confirmed.
    pending_ = ResolutionTransaction();
    return true;
}

bool DisplayManager::revertTx(uint64_t txId, std::string* error) {
    if (!pending_.isPending() || pending_.txId != txId) { if(error)*error="no such pending tx"; return false; }
    // Restore old mode in snapshot (simulates WM reverting CRTC).
    for (size_t i=0;i<snap_.outputs.size();++i) {
        if (snap_.outputs[i].durableKey()==pending_.outputKey) {
            snap_.outputs[i].currentMode = pending_.oldMode;
            snap_.outputs[i].posX = pending_.oldPosX;
            snap_.outputs[i].posY = pending_.oldPosY;
            snap_.outputs[i].rotation = pending_.oldRotation;
            break;
        }
    }
    pending_.state = TxReverted;
    pending_ = ResolutionTransaction();
    return true;
}

bool DisplayManager::onTimeoutCheck(uint64_t nowMs) {
    if (!pending_.isPending()) return false;
    if (nowMs >= pending_.deadlineMs) {
        // timeout -> revert
        uint64_t id = pending_.txId;
        std::string e;
        revertTx(id, &e);
        // mark as timed out for caller visibility — pending cleared, so caller checks return true means reverted.
        return true;
    }
    return false;
}

bool DisplayManager::onHotplug(const DisplaySnapshot& fresh) {
    snap_ = fresh;
    if (pending_.isPending()) {
        const OutputInfo* out = snap_.findByDurable(pending_.outputKey);
        if (!out) {
            uint64_t id = pending_.txId;
            std::string e;
            // clear pending even though output gone — restore is no-op but pending cleared
            pending_.state = TxReverted;
            pending_ = ResolutionTransaction();
            (void)id; (void)e;
            return true;
        }
        // stale generation: fresh jumped more than 1 beyond pending base -> reject
        if (fresh.generation > pending_.generation + 1) {
            uint64_t id = pending_.txId;
            std::string e;
            revertTx(id, &e);
            return true;
        }
    }
    return false;
}

void DisplayManager::onSettingsCrash() {
    if (pending_.isPending()) {
        uint64_t id = pending_.txId;
        std::string e;
        revertTx(id, &e);
    }
}

void DisplayManager::injectSnapshot(const DisplaySnapshot& s) { snap_ = s; }

} // namespace flamewm
