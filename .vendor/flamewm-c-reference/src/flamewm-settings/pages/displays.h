#ifndef FLAMEWM_SETTINGS_DISPLAYS_H
#define FLAMEWM_SETTINGS_DISPLAYS_H
#include "../../flamewm/api/display.h"
#include "../../flamewm/api/ids.h"
#include <string>
#include <vector>
#include <cstdint>
namespace flamewm { namespace control { class ControlClient; } }
namespace flamewm { namespace settings {
struct DisplaysPage {
    // Control-only: all state via ControlClient. No Runtime/DisplayManager deps.
    // UI state only: selected output drives which controls are shown.
    std::string selectedOutputId_;
    flamewm::api::DisplaySnapshot cachedSnapshot_;
    bool hasSnapshot_;
    flamewm::control::ControlClient* boundControlClient;
    DisplaysPage(): selectedOutputId_(), cachedSnapshot_(), hasSnapshot_(false), boundControlClient(0) {}
    bool reloadFromControl(flamewm::control::ControlClient* client, std::string* error);
    const flamewm::api::DisplaySnapshot& snapshot() const { return cachedSnapshot_; }
    bool hasSnapshot() const { return hasSnapshot_; }
    uint64_t generation() const { return cachedSnapshot_.generation; }
    std::string selectedOutputId() const { return selectedOutputId_; }
    void selectOutput(const std::string& key){ selectedOutputId_ = key; }
    const flamewm::api::OutputSnapshot* selectedOutput() const {
        if(selectedOutputId_.empty()) return 0;
        for(size_t i=0;i<cachedSnapshot_.outputs.size();++i) if(cachedSnapshot_.outputs[i].id.key==selectedOutputId_) return &cachedSnapshot_.outputs[i];
        return 0;
    }
    std::vector<flamewm::api::DisplayMode> modesForSelected() const {
        const flamewm::api::OutputSnapshot* o = selectedOutput();
        return o ? o->modes : std::vector<flamewm::api::DisplayMode>();
    }
    static std::vector<int> scaleOptions() {
        std::vector<int> v; v.push_back(100); v.push_back(125); v.push_back(150); v.push_back(175); v.push_back(200); return v;
    }
    // Only selected output exposes resolution/scale.
    // BeginModeChange with 15s confirmation (WM-owned revert).
    bool setResolutionForSelected(flamewm::control::ControlClient* client, const flamewm::api::ModeId& mode, std::string* err);
    bool setScaleForSelected(flamewm::control::ControlClient* client, int pct, std::string* err);
    bool keep(flamewm::control::ControlClient* client, std::string* err);
    bool revert(flamewm::control::ControlClient* client, std::string* err);
    bool isPendingActive() const { return hasSnapshot_ && cachedSnapshot_.hasPending(); }
    flamewm::api::TransactionId pendingTx() const { return cachedSnapshot_.pending.tx; }
};
}} // namespace
#endif
