#ifndef FLAMEWM_PANEL_NETWORK_H
#define FLAMEWM_PANEL_NETWORK_H

#include "../integrations/networkmanager.h"
#include "../core/types.h"
#include "anchor.h"
#include <string>
#include <stdint.h>

namespace flamewm {
namespace panel {

// Network status view model — graceful degrade when NM absent.
// No subprocess polling; view driven by NetworkManager signals only.
class NetworkView : public integrations::NetworkManagerListener {
public:
    explicit NetworkView(integrations::NetworkManager* nm);
    ~NetworkView();

    // View state for rendering
    struct State {
        bool visible;          // false when NM unavailable -> hide slot
        bool enabled;          // click/controls enabled
        integrations::NetworkState netState;
        std::string label;     // SSID or "Wired" / "Disconnected" etc.
        std::string iconRole;  // semantic icon role: "network-wireless", "network-wired", etc.
        int strength;          // 0..100
        bool showStrength;     // only for Wi-Fi connected/connecting
        State() : visible(false), enabled(false), netState(integrations::NetworkUnavailable), strength(0), showStrength(false) {}
    };

    const State& state() const { return state_; }
    bool isVisible() const { return state_.visible; }

    // Generation for panel callbacks (mirrors NM generation)
    uint64_t generation() const { return generation_; }

    // Popover placement for NM applet (uses shared anchor)
    PopoverPlacement popoverPlacement(const PanelRect& iconRect, int popoverW, int popoverH,
                                      PanelEdge edge, const WorkArea& wa, int gap) const;

    // Panel interaction intents — validate generation before forwarding
    bool connectKnown(const std::string& apPath);
    bool connectNewSecure(const std::string& apPath);
    bool disconnect();
    bool setWifiEnabled(bool en);
    void requestScan();

private:
    void onNetworkStatus(const integrations::NetworkStatus& s, uint64_t gen);
    void rebuild(const integrations::NetworkStatus& s);
    static std::string iconRoleFor(const integrations::NetworkStatus& s);

    integrations::NetworkManager* nm_;
    State state_;
    uint64_t generation_;
};

} // namespace panel
} // namespace flamewm

#endif
