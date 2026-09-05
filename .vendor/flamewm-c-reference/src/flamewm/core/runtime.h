#ifndef FLAMEWM_CORE_RUNTIME_H
#define FLAMEWM_CORE_RUNTIME_H

#include <stdint.h>
#include <string>
#include <vector>

#include "flamewm/core/config.h"
#include "display.h"
#include "types.h"
#include "ytimer.h"

// Forward decl to avoid X dependency in header
class YWindowManager;

namespace flamewm {

// Lifecycle contract (from EXECUTION.md Phase1):
// IceWM app/X initialized
//  -> prefs/theme loaded
//  -> YWindowManager exists
//  -> icon/pixmap authorities initialized
//  -> Runtime constructed (this class) -> panel/taskbar may consume
//
// Shutdown reverse:
// mark stopping -> reject new callbacks -> stop timers/polls -> destroy
// popovers/controllers -> clear frame refs -> destroy Runtime -> manager/X teardown
//
// Insertion point (source-traced in wmapp.cc):
//   YWMApp::YWMApp ctor after:
//     manager = new YWindowManager(...);
//     registerProtocols2(managerWindow);
//     initIcons(); initIconSize(); WPixRes::initPixmaps(this);
//   and before lazy taskbar creation / manager->initWorkspaces().
//   Exact line must be re-traced in current wmapp.cc before patch.
//
// This skeleton is safe to construct/destruct without X for unit tests.

class ScaleManager;
class ConfigStore;
namespace integrations {
class DBusDispatcher;
class NetworkManager;
class Mpris;
class PulseAudio;
}
namespace panel {
class NetworkView;
class MediaView;
class AudioView;
}

struct RuntimeApplyResult {
    bool ok;
    std::string error;
    uint64_t revision;
    RuntimeApplyResult() : ok(false), revision(0) {}
};

class RuntimeSettingsListener {
public:
    virtual ~RuntimeSettingsListener() {}
    virtual void onSettingsApplied(const SettingsSnapshot& effective) = 0;
};

class Runtime : public YTimerListener {
public:
    static Runtime* instance();

    Runtime();
    ~Runtime();

    // Initialize with live manager; returns false if already initialized or null.
    bool initialize(YWindowManager* mgr);
    void shutdown();

    bool isStopping() const { return stopping_; }
    bool isInitialized() const { return initialized_; }
    // Spec aliases (FW-CORE-01 required names)
    bool stopping() const { return stopping_; }
    bool initialized() const { return initialized_; }
    uint64_t generation() const { return generation_; }
    YWindowManager* manager() const { return manager_; }
    bool isCallbackCurrent(uint64_t capturedGeneration) const;

    ScaleManager* scaleManager();
    ConfigStore* configStore();
    integrations::DBusDispatcher* dbusDispatcher();
    integrations::NetworkManager* networkManager();
    integrations::Mpris* mpris();
    integrations::PulseAudio* pulseAudio();
    panel::NetworkView* networkView();
    panel::MediaView* mediaView();
    panel::AudioView* audioView();

    // --- FW-CONFIG-01 effective snapshot authority (C2) ---
    const SettingsSnapshot& effectiveSettings() const { return effective_; }
    uint64_t effectiveRevision() const { return effective_.revision; }
    const std::string& configPath() const { return configPath_; }
    void setConfigPath(const std::string& path) { configPath_ = path; }
    static std::string defaultConfigPath();

    // Load at startup from file; publishes effective if newer. No persistence.
    bool loadEffectiveFromFile(const std::string& path, std::string* error);

    // C2 flow: candidate -> parse -> validate entire snapshot -> stale check
    // -> stage required runtime effects -> on mandatory failure => no publish/no persist
    // -> atomic persist -> publish once -> notify once
    RuntimeApplyResult applyCandidateText(const std::string& candidateText);
    RuntimeApplyResult applyCandidateSnapshot(const SettingsSnapshot& candidate);

    // Subscription: notified exactly once per successful publish.
    void addSettingsListener(RuntimeSettingsListener* l);
    void removeSettingsListener(RuntimeSettingsListener* l);

    typedef bool (*EffectStager)(const SettingsSnapshot& candidate, std::string* error, void* ctx);
    void setEffectStager(EffectStager fn, void* ctx) { effectStager_ = fn; effectStagerCtx_ = ctx; }

    // Guard: increment generation on each initialize/shutdown cycle.
    // Callbacks must capture generation and check before mutating.

    // --- FW-DISPLAY-01: WM-owned reversible display transaction ---
    DisplayManager* displayManager() { return &displayManager_; }
    const DisplayManager* displayManager() const { return &displayManager_; }
    bool requestDisplayMode(const std::string& outputKey, const DisplayMode& mode, uint64_t timeoutMs, std::string* error);
    bool keepDisplayTx(std::string* error);
    bool revertDisplayTx(std::string* error);
    bool handleDisplayTimeout();
    void onDisplayHotplug(bool revertPendingIfGenerationChanged);
    void onSettingsProcessDeath();
    void injectDisplaySnapshot(const DisplaySnapshot& snap);
    bool handleTimer(YTimer* timer) override;

private:
    bool publishEffective(const SettingsSnapshot& snap, std::string* error);
    bool rollbackEffects(std::string* error);
    void notifyListeners();
    bool refreshDisplaySnapshot(std::string* error);
    bool applyCrtcModeFresh(const std::string& outputKey, const DisplayMode& mode, std::string* error);
    bool revertCapturedCrtc(std::string* error);

    struct CapturedCrtc {
        std::string outputKey;
        int x;
        int y;
        unsigned width;
        unsigned height;
        DisplayMode oldMode;
        int oldRotation;
        bool valid;
        CapturedCrtc():x(0),y(0),width(0),height(0),oldRotation(0),valid(false) {}
    } capturedCrtc_;

    DisplayManager displayManager_;
    YTimer* displayRevertTimer_;
    uint64_t pendingDisplayTxId_;

    static Runtime* s_instance;
    YWindowManager* manager_;
    bool stopping_;
    bool initialized_;
    uint64_t generation_;
    ScaleManager* scaleManager_;
    ConfigStore* configStore_;
    integrations::DBusDispatcher* dbusDispatcher_;
    integrations::NetworkManager* networkManager_;
    integrations::Mpris* mpris_;
    integrations::PulseAudio* pulseAudio_;
    panel::NetworkView* networkView_;
    panel::MediaView* mediaView_;
    panel::AudioView* audioView_;

    SettingsSnapshot effective_;
    std::string configPath_;
    std::vector<RuntimeSettingsListener*> listeners_;
    EffectStager effectStager_;
    void* effectStagerCtx_;
    bool notifying_;
};

} // namespace flamewm
#endif
