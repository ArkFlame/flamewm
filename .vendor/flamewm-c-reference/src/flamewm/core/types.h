#ifndef FLAMEWM_CORE_TYPES_H
#define FLAMEWM_CORE_TYPES_H

#include <string>

namespace flamewm {

// Durable output identity: EDID hash + connector tie-breaker,
// connector fallback when EDID absent.
struct OutputId {
    std::string durableId; // stable key for persistence
    std::string connector; // e.g. "HDMI-1", "eDP-1"
    std::string edidHash;  // hex hash, empty if unavailable

    OutputId() {}
    OutputId(const std::string& did, const std::string& conn, const std::string& edid)
        : durableId(did), connector(conn), edidHash(edid) {}

    bool empty() const { return durableId.empty() && connector.empty(); }

    bool operator==(const OutputId& o) const {
        return durableId == o.durableId && connector == o.connector && edidHash == o.edidHash;
    }
    bool operator!=(const OutputId& o) const { return !(*this == o); }
    bool operator<(const OutputId& o) const {
        if (durableId != o.durableId) return durableId < o.durableId;
        if (connector != o.connector) return connector < o.connector;
        return edidHash < o.edidHash;
    }
};

enum PanelEdge {
    PanelEdgeBottom = 0,
    PanelEdgeTop    = 1,
    PanelEdgeLeft   = 2,
    PanelEdgeRight  = 3
};

inline const char* panelEdgeName(PanelEdge e) {
    switch (e) {
        case PanelEdgeBottom: return "Bottom";
        case PanelEdgeTop:    return "Top";
        case PanelEdgeLeft:   return "Left";
        case PanelEdgeRight:  return "Right";
        default: return "Unknown";
    }
}

inline bool isHorizontalEdge(PanelEdge e) {
    return e == PanelEdgeTop || e == PanelEdgeBottom;
}
inline bool isVerticalEdge(PanelEdge e) {
    return e == PanelEdgeLeft || e == PanelEdgeRight;
}

} // namespace flamewm
#endif // FLAMEWM_CORE_TYPES_H
