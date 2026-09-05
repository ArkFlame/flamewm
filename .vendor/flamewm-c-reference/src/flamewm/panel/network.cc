#include "network.h"

namespace flamewm {
namespace panel {

NetworkView::NetworkView(integrations::NetworkManager* nm)
    : nm_(nm), generation_(nm ? nm->generation() : 1)
{
    if (nm_) nm_->addListener(this);
    if (nm_) rebuild(nm_->status());
    else { state_.visible = false; state_.enabled = false; }
}

NetworkView::~NetworkView() {
    if (nm_) nm_->removeListener(this);
}

PopoverPlacement NetworkView::popoverPlacement(const PanelRect& iconRect, int popoverW, int popoverH,
                                               PanelEdge edge, const WorkArea& wa, int gap) const {
    return PopoverAnchor::anchor(iconRect, popoverW, popoverH, edge, wa, gap);
}

bool NetworkView::connectKnown(const std::string& apPath) {
    if (!nm_) return false;
    if (nm_->isStale(generation_)) return false;
    return nm_->requestConnectKnown(apPath, generation_);
}
bool NetworkView::connectNewSecure(const std::string& apPath) {
    if (!nm_) return false;
    if (nm_->isStale(generation_)) return false;
    return nm_->requestConnectNewSecure(apPath, generation_);
}
bool NetworkView::disconnect() {
    if (!nm_) return false;
    if (nm_->isStale(generation_)) return false;
    return nm_->requestDisconnect(generation_);
}
bool NetworkView::setWifiEnabled(bool en) {
    if (!nm_) return false;
    return nm_->setWifiEnabled(en);
}
void NetworkView::requestScan() {
    if (!nm_) return;
    nm_->requestScan();
}

void NetworkView::onNetworkStatus(const integrations::NetworkStatus& s, uint64_t gen) {
    generation_ = gen;
    rebuild(s);
}

void NetworkView::rebuild(const integrations::NetworkStatus& s) {
    // Graceful degrade: unavailable -> hidden/disabled slot (no dead icon)
    if (s.state == integrations::NetworkUnavailable) {
        state_.visible = false;
        state_.enabled = false;
        state_.netState = s.state;
        state_.label.clear();
        state_.iconRole = "network-offline";
        state_.strength = 0;
        state_.showStrength = false;
        return;
    }
    state_.visible = true;
    state_.enabled = true;
    state_.netState = s.state;
    state_.strength = s.activeStrength;
    state_.iconRole = iconRoleFor(s);
    switch (s.state) {
        case integrations::NetworkWired: state_.label = "Wired"; state_.showStrength = false; break;
        case integrations::NetworkWifiConnected: state_.label = s.activeSsid.empty() ? "Wi-Fi" : s.activeSsid; state_.showStrength = true; break;
        case integrations::NetworkWifiConnecting: state_.label = s.activeSsid.empty() ? "Connecting…" : s.activeSsid; state_.showStrength = true; break;
        case integrations::NetworkDisconnected: state_.label = "Disconnected"; state_.showStrength = false; break;
        default: state_.label = "Network"; state_.showStrength = false; break;
    }
}

std::string NetworkView::iconRoleFor(const integrations::NetworkStatus& s) {
    // Semantic icon role with fallback behavior (light/dark tested)
    switch (s.state) {
        case integrations::NetworkWired: return "network-wired";
        case integrations::NetworkWifiConnected:
        case integrations::NetworkWifiConnecting: return "network-wireless";
        case integrations::NetworkDisconnected: return "network-offline";
        case integrations::NetworkUnavailable: return "network-offline";
        default: return "network-idle";
    }
}

} // namespace panel
} // namespace flamewm
