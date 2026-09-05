#include "runtime.h"
#include "scalemanager.h"
#include "flamewm/core/config.h"
#include "display.h"
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <sstream>
#include <sys/time.h>
#include <cstring>
#ifdef CONFIG_XRANDR
#if defined(__has_include)
#if __has_include(<X11/extensions/Xrandr.h>)
#include <X11/extensions/Xrandr.h>
#endif
#else
#include <X11/extensions/Xrandr.h>
#endif
#endif
#include "ytimer.h"
#include "yxapp.h"
#include "yapp.h"
#include "flamewm/ui/backend.h"
#include "../integrations/dbusdispatcher.h"
#include "../integrations/networkmanager.h"
#include "../integrations/mpris.h"
#include "../integrations/pulse.h"
#include "../panel/network.h"
#include "../panel/media.h"
#include "../panel/audio.h"

namespace flamewm {

class RuntimeMainLoopAdapter : public integrations::IMainLoopAdapter {
public:
    void registerPoll(YPollBase* poll) { if (mainLoop) mainLoop->registerPoll(poll); }
    void unregisterPoll(YPollBase* poll) { if (mainLoop) mainLoop->unregisterPoll(poll); }
    void registerTimer(YTimer* timer) { if (mainLoop) mainLoop->registerTimer(timer); }
    void unregisterTimer(YTimer* timer) { if (mainLoop) mainLoop->unregisterTimer(timer); }
};

Runtime* Runtime::s_instance = 0;

Runtime* Runtime::instance() { return s_instance; }

static uint64_t nowMsMonotonic() {
    struct timeval tv;
    gettimeofday(&tv, 0);
    return (uint64_t)tv.tv_sec * 1000ULL + (uint64_t)tv.tv_usec / 1000ULL;
}

Runtime::Runtime() : displayManager_(), displayRevertTimer_(0), pendingDisplayTxId_(0),
    manager_(0), stopping_(false), initialized_(false), generation_(1), scaleManager_(0), configStore_(0),
    dbusDispatcher_(0), networkManager_(0), mpris_(0), pulseAudio_(0),
    networkView_(0), mediaView_(0), audioView_(0), effectStager_(0), effectStagerCtx_(0), notifying_(false) {
    scaleManager_ = new ScaleManager();
    configStore_ = new ConfigStore();
    effective_ = configStore_->current();
    configPath_ = defaultConfigPath();
    capturedCrtc_.valid = false;
}

Runtime::~Runtime() {
    if (displayRevertTimer_) { displayRevertTimer_->stopTimer(); delete displayRevertTimer_; displayRevertTimer_ = 0; }
    shutdown();
    delete audioView_; audioView_ = 0;
    delete mediaView_; mediaView_ = 0;
    delete networkView_; networkView_ = 0;
    delete pulseAudio_; pulseAudio_ = 0;
    delete mpris_; mpris_ = 0;
    delete networkManager_; networkManager_ = 0;
    delete dbusDispatcher_; dbusDispatcher_ = 0;
    delete scaleManager_; scaleManager_ = 0;
    delete configStore_; configStore_ = 0;
}

bool Runtime::handleTimer(YTimer* timer) {
    if (timer == displayRevertTimer_) {
        handleDisplayTimeout();
    }
    return false;
}

bool Runtime::initialize(YWindowManager* mgr) {
    if (initialized_) return false;
    if (!mgr) return false;
    if (s_instance && s_instance != this) return false;
    manager_ = mgr;
    stopping_ = false;
    initialized_ = true;
    s_instance = this;
    delete audioView_; audioView_ = 0;
    delete mediaView_; mediaView_ = 0;
    delete networkView_; networkView_ = 0;
    delete pulseAudio_; pulseAudio_ = 0;
    delete mpris_; mpris_ = 0;
    delete networkManager_; networkManager_ = 0;
    delete dbusDispatcher_; dbusDispatcher_ = 0;
    static RuntimeMainLoopAdapter loopAdapter;
    dbusDispatcher_ = new integrations::DBusDispatcher(&loopAdapter);
    networkManager_ = new integrations::NetworkManager(dbusDispatcher_);
    mpris_ = new integrations::Mpris(dbusDispatcher_);
    pulseAudio_ = new integrations::PulseAudio();
    pulseAudio_->setMainLoopAdapter(&loopAdapter);
    pulseAudio_->connectAsync();
    networkView_ = new panel::NetworkView(networkManager_);
    mediaView_ = new panel::MediaView(mpris_);
    audioView_ = new panel::AudioView(pulseAudio_);
    if (!configPath_.empty()) {
        std::string err;
        loadEffectiveFromFile(configPath_, &err);
    }
    // Wire IceWM backend availability after X/product init.
    flamewm::ui::UiBackend::init();
    // Initial display snapshot if X available (best-effort, no error propagation)
    if (xapp && xapp->display()) {
        std::string e;
        refreshDisplaySnapshot(&e);
    }
    return true;
}

void Runtime::shutdown() {
    if (stopping_) return;
    stopping_ = true;
    delete audioView_; audioView_ = 0;
    delete mediaView_; mediaView_ = 0;
    delete networkView_; networkView_ = 0;
    if (pulseAudio_) pulseAudio_->disconnect();
    delete pulseAudio_; pulseAudio_ = 0;
    delete mpris_; mpris_ = 0;
    delete networkManager_; networkManager_ = 0;
    if (dbusDispatcher_) dbusDispatcher_->shutdown();
    if (displayRevertTimer_) { displayRevertTimer_->stopTimer(); }
    flamewm::ui::UiBackend::shutdown();
    if (pendingDisplayTxId_ != 0 && displayManager_.pendingTx()) {
        std::string e;
        // Model revert without X (X may be gone)
        displayManager_.revertTx(pendingDisplayTxId_, &e);
        pendingDisplayTxId_ = 0;
        capturedCrtc_.valid = false;
    }
    if (s_instance == this) s_instance = 0;
    manager_ = 0;
    initialized_ = false;
    ++generation_;
}

bool Runtime::isCallbackCurrent(uint64_t capturedGeneration) const {
    return initialized_ && !stopping_ && capturedGeneration == generation_;
}

ScaleManager* Runtime::scaleManager() { return scaleManager_; }
ConfigStore* Runtime::configStore() { return configStore_; }
integrations::DBusDispatcher* Runtime::dbusDispatcher() { return dbusDispatcher_; }
integrations::NetworkManager* Runtime::networkManager() { return networkManager_; }
integrations::Mpris* Runtime::mpris() { return mpris_; }
integrations::PulseAudio* Runtime::pulseAudio() { return pulseAudio_; }
panel::NetworkView* Runtime::networkView() { return networkView_; }
panel::MediaView* Runtime::mediaView() { return mediaView_; }
panel::AudioView* Runtime::audioView() { return audioView_; }

std::string Runtime::defaultConfigPath() {
    const char* xdg = getenv("XDG_CONFIG_HOME");
    std::string base;
    if (xdg && *xdg) base = std::string(xdg) + "/flamewm/flame.conf";
    else {
        const char* home = getenv("HOME");
        if (home && *home) base = std::string(home) + "/.config/flamewm/flame.conf";
        else base = "/tmp/flame.conf";
    }
    return base;
}

bool Runtime::publishEffective(const SettingsSnapshot& snap, std::string* error) {
    std::string err;
    if (!configStore_->applySnapshot(snap, &err)) {
        if (error) *error = err.empty() ? std::string("effective snapshot apply failed") : err;
        return false;
    }
    effective_ = snap;
    return true;
}

bool Runtime::rollbackEffects(std::string* error) {
    if (!effectStager_) return true;
    std::string err;
    if (!effectStager_(effective_, &err, effectStagerCtx_)) {
        if (error) *error = err.empty() ? std::string("effect rollback failed") : err;
        return false;
    }
    return true;
}

void Runtime::notifyListeners() {
    if (notifying_) return;
    notifying_ = true;
    for (size_t i = 0; i < listeners_.size(); ++i) {
        if (listeners_[i] && !stopping_)
            listeners_[i]->onSettingsApplied(effective_);
    }
    notifying_ = false;
}

void Runtime::addSettingsListener(RuntimeSettingsListener* l) {
    if (!l) return;
    for (size_t i = 0; i < listeners_.size(); ++i) if (listeners_[i]==l) return;
    listeners_.push_back(l);
}
void Runtime::removeSettingsListener(RuntimeSettingsListener* l) {
    for (size_t i = 0; i < listeners_.size(); ++i) if (listeners_[i]==l) { listeners_.erase(listeners_.begin()+i); break; }
}

bool Runtime::loadEffectiveFromFile(const std::string& path, std::string* error) {
    std::ifstream in(path.c_str());
    if (!in) {
        return true;
    }
    std::ostringstream ss; ss << in.rdbuf();
    std::string text = ss.str();
    if (configStore_->revision() != effective_.revision) {
        std::string e;
        if (!configStore_->isStale(effective_))
            configStore_->applySnapshot(effective_, &e);
    }
    SettingsSnapshot parsed;
    if (!configStore_->parseSnapshot(text, parsed, error)) return false;
    if (text.find("revision=") == std::string::npos) {
        parsed.revision = effective_.revision + 1;
    } else if (parsed.revision != 0 && parsed.revision <= effective_.revision) {
        if (error) *error = "stale revision";
        return false;
    }
    if (!parsed.isValid(error)) return false;
    if (effectStager_) {
        std::string e;
        if (!effectStager_(parsed, &e, effectStagerCtx_)) {
            std::string rollbackError;
            rollbackEffects(&rollbackError);
            if (error) *error = e.empty() ? std::string("effect stage failed") : e;
            return false;
        }
    }
    if (!publishEffective(parsed, error)) {
        std::string rollbackError;
        rollbackEffects(&rollbackError);
        return false;
    }
    notifyListeners();
    return true;
}

RuntimeApplyResult Runtime::applyCandidateText(const std::string& candidateText) {
    RuntimeApplyResult r;
    if (configStore_->revision() != effective_.revision) {
        std::string e;
        if (!configStore_->isStale(effective_))
            configStore_->applySnapshot(effective_, &e);
    }
    SettingsSnapshot snap;
    std::string err;
    if (!configStore_->parseSnapshot(candidateText, snap, &err)) {
        r.ok = false; r.error = err; r.revision = effective_.revision; return r;
    }
    return applyCandidateSnapshot(snap);
}

RuntimeApplyResult Runtime::applyCandidateSnapshot(const SettingsSnapshot& candidate) {
    RuntimeApplyResult r;
    r.revision = effective_.revision;
    std::string verr;
    if (!candidate.isValid(&verr)) { r.ok=false; r.error=verr; return r; }
    if (candidate.revision == 0 || candidate.revision <= effective_.revision) {
        r.ok=false; r.error="stale revision"; return r;
    }
    if (effectStager_) {
        std::string e;
        if (!effectStager_(candidate, &e, effectStagerCtx_)) {
            std::string rollbackError;
            rollbackEffects(&rollbackError);
            r.ok=false; r.error=e.empty()?std::string("effect stage failed"):e; return r;
        }
    }
    if (!configPath_.empty()) {
        std::string perr;
        if (!configStore_->persistToFile(configPath_, candidate, &perr)) {
            std::string rollbackError;
            rollbackEffects(&rollbackError);
            r.ok=false; r.error=perr; return r;
        }
    }
    std::string publishError;
    if (!publishEffective(candidate, &publishError)) {
        std::string rollbackError;
        rollbackEffects(&rollbackError);
        r.ok=false; r.error=publishError; return r;
    }
    r.ok=true; r.error=""; r.revision = effective_.revision;
    notifyListeners();
    return r;
}

// ---------------------------------------------------------------------------
// FW-DISPLAY-01: XRandR reversible transaction owned by Runtime
// ---------------------------------------------------------------------------

bool Runtime::refreshDisplaySnapshot(std::string* error) {
#ifdef CONFIG_XRANDR
    if (!xapp || !xapp->display()) {
        if (error) *error = "no display";
        return false;
    }
    Display* dpy = xapp->display();
    Window root = xapp->root();
    if (!xrandr.supported) {
        if (error) *error = "xrandr not supported";
        return false;
    }
    XRRScreenResources* res = XRRGetScreenResources(dpy, root);
    if (!res) {
        if (error) *error = "XRRGetScreenResources failed";
        return false;
    }
    DisplaySnapshot snap;
    snap.generation = displayManager_.generation() + 1;
    // Enumerate outputs fresh — no stale RROutput reuse beyond this call
    for (int i = 0; i < res->noutput; ++i) {
        XRROutputInfo* oi = XRRGetOutputInfo(dpy, res, res->outputs[i]);
        if (!oi) continue;
        OutputInfo out;
        out.connector = oi->name ? std::string(oi->name) : std::string();
        out.connected = (oi->connection == RR_Connected);
        out.rrOutput = res->outputs[i];
        out.rrCrtc = oi->crtc;
        out.rotationRaw = 0;
        // EDID not available via generic XRR; durableKey falls back to connector
        out.edidId = "";
        // Available modes for this output
        for (int m = 0; m < oi->nmode; ++m) {
            RRMode mid = oi->modes[m];
            // Find mode info in resources
            for (int k = 0; k < res->nmode; ++k) {
                if (res->modes[k].id == mid) {
                    XRRModeInfo* mi = &res->modes[k];
                    int refresh = 0;
                    if (mi->dotClock && mi->hTotal && mi->vTotal)
                        refresh = (int)((1000LL * mi->dotClock) / (mi->hTotal * mi->vTotal));
                    else
                        refresh = 60000;
                    out.availableModes.push_back(DisplayMode(mid, (int)mi->width, (int)mi->height, refresh));
                    break;
                }
            }
        }
        if (oi->crtc != 0) {
            XRRCrtcInfo* ci = XRRGetCrtcInfo(dpy, res, oi->crtc);
            if (ci) {
                out.enabled = (ci->mode != 0);
                out.posX = ci->x;
                out.posY = ci->y;
                out.rotation = ci->rotation;
                out.rotationRaw = ci->rotation;
                // Find current mode by ci->mode id
                bool found = false;
                for (size_t mm = 0; mm < out.availableModes.size(); ++mm) {
                    if (out.availableModes[mm].id == ci->mode) { out.currentMode = out.availableModes[mm]; found = true; break; }
                }
                if (!found && ci->mode != 0) {
                    for (int k = 0; k < res->nmode; ++k) {
                        if (res->modes[k].id == ci->mode) {
                            XRRModeInfo* mi = &res->modes[k];
                            int refresh = 0;
                            if (mi->dotClock && mi->hTotal && mi->vTotal)
                                refresh = (int)((1000LL * mi->dotClock) / (mi->hTotal * mi->vTotal));
                            else refresh = 60000;
                            out.currentMode = DisplayMode(ci->mode, (int)mi->width, (int)mi->height, refresh);
                            break;
                        }
                    }
                }
                // Primary detection: compare to XRRGetOutputPrimary if available not needed; mark false here, YDesktop logic owns primary
                XRRFreeCrtcInfo(ci);
            }
        } else {
            out.enabled = false;
        }
        // Preserve shellScale from previous snapshot by durableKey
        const OutputInfo* prev = displayManager_.snapshot().findByDurable(out.durableKey());
        if (prev) out.shellScale = prev->shellScale;
        // Preserve primary flag if previously known
        if (prev) out.primary = prev->primary;
        snap.outputs.push_back(out);
        XRRFreeOutputInfo(oi);
    }
    XRRFreeScreenResources(res);
    displayManager_.injectSnapshot(snap);
    return true;
#else
    (void)error;
    return false;
#endif
}

bool Runtime::applyCrtcModeFresh(const std::string& outputKey, const DisplayMode& mode, std::string* error) {
#ifdef CONFIG_XRANDR
    if (!xapp || !xapp->display()) {
        if (error) *error = "no display";
        return false;
    }
    Display* dpy = xapp->display();
    Window root = xapp->root();
    XRRScreenResources* res = XRRGetScreenResources(dpy, root);
    if (!res) {
        if (error) *error = "XRRGetScreenResources failed";
        return false;
    }
    bool foundOutput = false;
    bool modeOk = false;
    RRCrtc targetCrtc = 0;
    XRRCrtcInfo* captured = 0;
    int capX = 0, capY = 0;
    unsigned capW = 0, capH = 0;
    RRMode capMode = 0;
    Rotation capRot = 0;
    // Fresh scan: validate connected output and mode membership before any XRRSetCrtcConfig
    for (int i = 0; i < res->noutput; ++i) {
        XRROutputInfo* oi = XRRGetOutputInfo(dpy, res, res->outputs[i]);
        if (!oi) continue;
        std::string conn = oi->name ? std::string(oi->name) : std::string();
        std::string key = conn; // EDID fallback same as display snapshot
        // Also try edid:connector key if present in snapshot mapping
        // For validation we accept either connector match or durableKey match
        bool isTarget = (key == outputKey);
        if (!isTarget) {
            // Check via snapshot durable keys for this output
            // If outputKey contains ':', connector is after colon
            size_t pos = outputKey.rfind(':');
            std::string outConn = (pos == std::string::npos) ? outputKey : outputKey.substr(pos+1);
            if (conn == outConn) isTarget = true;
        }
        if (isTarget) {
            foundOutput = true;
            if (oi->connection != RR_Connected) {
                if (error) *error = "output not connected";
                XRRFreeOutputInfo(oi);
                XRRFreeScreenResources(res);
                return false;
            }
            if (oi->crtc == 0) {
                if (error) *error = "output has no crtc";
                XRRFreeOutputInfo(oi);
                XRRFreeScreenResources(res);
                return false;
            }
            targetCrtc = oi->crtc;
            // Validate mode membership: mode.id must be in oi->modes
            for (int m = 0; m < oi->nmode; ++m) {
                if (oi->modes[m] == mode.id) { modeOk = true; break; }
            }
            if (!modeOk) {
                if (error) *error = "mode not available for output";
                XRRFreeOutputInfo(oi);
                XRRFreeScreenResources(res);
                return false;
            }
            // Capture old CRTC before mutating
            XRRCrtcInfo* ci = XRRGetCrtcInfo(dpy, res, oi->crtc);
            if (!ci) {
                if (error) *error = "XRRGetCrtcInfo failed";
                XRRFreeOutputInfo(oi);
                XRRFreeScreenResources(res);
                return false;
            }
            captured = ci; // will free after apply
            capX = ci->x; capY = ci->y; capW = ci->width; capH = ci->height; capMode = ci->mode; capRot = ci->rotation;
            XRRFreeOutputInfo(oi);
            break;
        }
        XRRFreeOutputInfo(oi);
    }
    if (!foundOutput) {
        if (error) *error = "output not found";
        XRRFreeScreenResources(res);
        return false;
    }
    if (!modeOk) {
        if (error) *error = "mode not available";
        if (captured) XRRFreeCrtcInfo(captured);
        XRRFreeScreenResources(res);
        return false;
    }
    // Save captured for revert (old mode details)
    capturedCrtc_.outputKey = outputKey;
    capturedCrtc_.x = capX;
    capturedCrtc_.y = capY;
    capturedCrtc_.width = capW;
    capturedCrtc_.height = capH;
    // Find old DisplayMode by capMode for transaction bookkeeping
    capturedCrtc_.oldMode = DisplayMode();
    for (int k = 0; k < res->nmode; ++k) {
        if (res->modes[k].id == capMode) {
            XRRModeInfo* mi = &res->modes[k];
            int refresh = 0;
            if (mi->dotClock && mi->hTotal && mi->vTotal)
                refresh = (int)((1000LL * mi->dotClock) / (mi->hTotal * mi->vTotal));
            else refresh = 60000;
            capturedCrtc_.oldMode = DisplayMode(capMode, (int)mi->width, (int)mi->height, refresh);
            break;
        }
    }
    capturedCrtc_.oldRotation = (int)capRot;
    capturedCrtc_.valid = true;

    // Re-fetch CRTC info for apply (ensure fresh after capture)
    XRRCrtcInfo* ci2 = XRRGetCrtcInfo(dpy, res, targetCrtc);
    if (!ci2) {
        if (error) *error = "XRRGetCrtcInfo failed before apply";
        if (captured) XRRFreeCrtcInfo(captured);
        XRRFreeScreenResources(res);
        capturedCrtc_.valid = false;
        return false;
    }
    // Apply via XRRSetCrtcConfig — no stale RROutput/RRCrtc/RRMode reuse beyond this fresh enumeration
    Status st = XRRSetCrtcConfig(dpy, res, targetCrtc, CurrentTime,
                                 ci2->x, ci2->y, mode.id, ci2->rotation,
                                 ci2->outputs, ci2->noutput);
    XRRFreeCrtcInfo(ci2);
    if (captured) XRRFreeCrtcInfo(captured);
    XRRFreeScreenResources(res);
    if (st != Success) {
        if (error) *error = "XRRSetCrtcConfig failed";
        capturedCrtc_.valid = false;
        return false;
    }
    return true;
#else
    (void)outputKey; (void)mode; (void)error;
    return false;
#endif
}

bool Runtime::revertCapturedCrtc(std::string* error) {
    if (!capturedCrtc_.valid) return true;
#ifdef CONFIG_XRANDR
    if (!xapp || !xapp->display()) {
        capturedCrtc_.valid = false;
        return true;
    }
    Display* dpy = xapp->display();
    Window root = xapp->root();
    XRRScreenResources* res = XRRGetScreenResources(dpy, root);
    if (!res) {
        if (error) *error = "XRRGetScreenResources failed on revert";
        capturedCrtc_.valid = false;
        return false;
    }
    // Fresh lookup again — do not reuse stale XIDs
    RRCrtc targetCrtc = 0;
    bool found = false;
    for (int i = 0; i < res->noutput; ++i) {
        XRROutputInfo* oi = XRRGetOutputInfo(dpy, res, res->outputs[i]);
        if (!oi) continue;
        std::string conn = oi->name ? std::string(oi->name) : std::string();
        std::string key = conn;
        bool isTarget = (key == capturedCrtc_.outputKey);
        if (!isTarget) {
            size_t pos = capturedCrtc_.outputKey.rfind(':');
            std::string outConn = (pos == std::string::npos) ? capturedCrtc_.outputKey : capturedCrtc_.outputKey.substr(pos+1);
            if (conn == outConn) isTarget = true;
        }
        if (isTarget) {
            if (oi->crtc != 0) { targetCrtc = oi->crtc; found = true; }
            XRRFreeOutputInfo(oi);
            break;
        }
        XRRFreeOutputInfo(oi);
    }
    if (!found) {
        // Output gone — nothing to revert on X side, just clear model
        XRRFreeScreenResources(res);
        capturedCrtc_.valid = false;
        return true;
    }
    XRRCrtcInfo* ci = XRRGetCrtcInfo(dpy, res, targetCrtc);
    if (!ci) {
        XRRFreeScreenResources(res);
        capturedCrtc_.valid = false;
        return false;
    }
    RRMode revertMode = capturedCrtc_.oldMode.id;
    // If old mode was 0 (disabled), keep 0
    Status st = XRRSetCrtcConfig(dpy, res, targetCrtc, CurrentTime,
                                 capturedCrtc_.x, capturedCrtc_.y,
                                 revertMode, (Rotation)capturedCrtc_.oldRotation,
                                 ci->outputs, ci->noutput);
    XRRFreeCrtcInfo(ci);
    XRRFreeScreenResources(res);
    capturedCrtc_.valid = false;
    if (st != Success) {
        if (error) *error = "XRRSetCrtcConfig revert failed";
        return false;
    }
    return true;
#else
    capturedCrtc_.valid = false;
    (void)error;
    return true;
#endif
}

bool Runtime::requestDisplayMode(const std::string& outputKey, const DisplayMode& mode, uint64_t timeoutMs, std::string* error) {
    if (stopping_) { if (error) *error = "shutting down"; return false; }
    if (displayManager_.pendingTx()) { if (error) *error = "tx already pending"; return false; }
    if (outputKey.empty()) { if (error) *error = "empty outputKey"; return false; }
    if (timeoutMs == 0) timeoutMs = 15000;
    // Pure-model validation + deadline setup first (captures old mode/generation)
    uint64_t now = nowMsMonotonic();
    if (!displayManager_.beginResolutionTx(outputKey, mode, now, timeoutMs, error)) {
        return false;
    }
    const ResolutionTransaction* tx = displayManager_.pendingTx();
    if (!tx) { if (error) *error = "tx not pending after begin"; return false; }
    pendingDisplayTxId_ = tx->txId;

    // Now X apply with fresh enumeration (no stale XIDs). If X not available (test/no-X), skip X apply.
    bool needX = (xapp && xapp->display() && xrandr.supported);
    if (needX) {
        std::string xerr;
        if (!applyCrtcModeFresh(outputKey, mode, &xerr)) {
            // Roll back pure model tx on X failure
            std::string e;
            displayManager_.revertTx(pendingDisplayTxId_, &e);
            pendingDisplayTxId_ = 0;
            if (error) *error = xerr;
            return false;
        }
        // Refresh snapshot to observe new topology generation
        std::string re;
        refreshDisplaySnapshot(&re);
    } else {
        // Headless/test: mark pending as if applied; capture old for revert path by using DisplayManager oldMode
        capturedCrtc_.outputKey = outputKey;
        capturedCrtc_.oldMode = tx->oldMode;
        capturedCrtc_.x = tx->oldPosX;
        capturedCrtc_.y = tx->oldPosY;
        capturedCrtc_.oldRotation = tx->oldRotation;
        capturedCrtc_.valid = true;
    }

    // Start 15s bounded revert timer
    if (!displayRevertTimer_) displayRevertTimer_ = new YTimer(15000, this, false);
    displayRevertTimer_->setInterval((long)timeoutMs);
    displayRevertTimer_->startTimer();
    return true;
}

bool Runtime::keepDisplayTx(std::string* error) {
    if (!displayManager_.pendingTx() || pendingDisplayTxId_ == 0) { if (error) *error = "no pending tx"; return false; }
    uint64_t id = pendingDisplayTxId_;
    if (!displayManager_.confirmTx(id, error)) return false;
    pendingDisplayTxId_ = 0;
    capturedCrtc_.valid = false;
    if (displayRevertTimer_) displayRevertTimer_->stopTimer();
    // Refresh snapshot to pick up confirmed topology
    if (xapp && xapp->display()) { std::string e; refreshDisplaySnapshot(&e); }
    return true;
}

bool Runtime::revertDisplayTx(std::string* error) {
    if (!displayManager_.pendingTx() || pendingDisplayTxId_ == 0) { if (error) *error = "no pending tx"; return false; }
    std::string xerr;
    bool xok = revertCapturedCrtc(&xerr);
    uint64_t id = pendingDisplayTxId_;
    std::string e;
    bool mok = displayManager_.revertTx(id, &e);
    pendingDisplayTxId_ = 0;
    if (displayRevertTimer_) displayRevertTimer_->stopTimer();
    if (xapp && xapp->display()) { std::string re; refreshDisplaySnapshot(&re); }
    if (!xok && error) *error = xerr;
    else if (!mok && error) *error = e;
    return xok && mok;
}

bool Runtime::handleDisplayTimeout() {
    if (!displayManager_.pendingTx() || pendingDisplayTxId_ == 0) return false;
    uint64_t now = nowMsMonotonic();
    if (!displayManager_.pendingTx()->isExpired(now)) {
        // Fallback: also check via onTimeoutCheck which uses same deadline
        // If not yet expired, keep pending
        return false;
    }
    std::string e;
    revertCapturedCrtc(&e);
    uint64_t id = pendingDisplayTxId_;
    displayManager_.revertTx(id, &e);
    pendingDisplayTxId_ = 0;
    if (displayRevertTimer_) displayRevertTimer_->stopTimer();
    if (xapp && xapp->display()) { std::string re; refreshDisplaySnapshot(&re); }
    return true;
}

void Runtime::onDisplayHotplug(bool revertPendingIfGenerationChanged) {
    if (!xapp || !xapp->display()) return;
    // Fresh snapshot with new generation
    DisplaySnapshot before = displayManager_.snapshot();
    std::string e;
    refreshDisplaySnapshot(&e);
    DisplaySnapshot fresh = displayManager_.snapshot();
    if (pendingDisplayTxId_ != 0 && displayManager_.pendingTx()) {
        // Delegate to DisplayManager generation guard: if generation jumped >1 or output gone, revert
        bool hadHotplugRevert = displayManager_.onHotplug(fresh);
        if (hadHotplugRevert) {
            std::string re;
            revertCapturedCrtc(&re);
            pendingDisplayTxId_ = 0;
            if (displayRevertTimer_) displayRevertTimer_->stopTimer();
        } else if (revertPendingIfGenerationChanged && fresh.generation != before.generation) {
            // Topology changed while tx pending — conservative revert unless generation guard already handled
            // DisplayManager keeps pending if generation +1; we still revert if output disappeared
            const OutputInfo* out = fresh.findByDurable(displayManager_.pendingTx()->outputKey);
            if (!out) {
                std::string re;
                revertCapturedCrtc(&re);
                displayManager_.onSettingsCrash();
                pendingDisplayTxId_ = 0;
                if (displayRevertTimer_) displayRevertTimer_->stopTimer();
            }
        }
    }
}

void Runtime::onSettingsProcessDeath() {
    if (pendingDisplayTxId_ != 0 && displayManager_.pendingTx()) {
        std::string e;
        revertCapturedCrtc(&e);
        displayManager_.onSettingsCrash();
        pendingDisplayTxId_ = 0;
        if (displayRevertTimer_) displayRevertTimer_->stopTimer();
        if (xapp && xapp->display()) { std::string re; refreshDisplaySnapshot(&re); }
    }
}

void Runtime::injectDisplaySnapshot(const DisplaySnapshot& snap) {
    displayManager_.injectSnapshot(snap);
}

} // namespace flamewm
