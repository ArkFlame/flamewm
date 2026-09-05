#ifndef FLAMEWM_PANEL_OUTPUTPANEL_H
#define FLAMEWM_PANEL_OUTPUTPANEL_H

#include "../core/types.h"
#include "types.h"
#include <cstdint>

namespace flamewm {
namespace panel {

// Per-output geometry/strut computation.
// Pure model — no X calls. Thickness is logical panelSize scaled by output scale.
class OutputPanel {
public:
    OutputPanel(const OutputId& id, const Rect& outputRect, PanelEdge edge, int scalePct);
    ~OutputPanel();

    const OutputId& id() const { return id_; }
    const Rect& outputRect() const { return outputRect_; }
    PanelEdge edge() const { return edge_; }
    int scalePct() const { return scalePct_; }

    void setOutputRect(const Rect& r);
    void setEdge(PanelEdge e);
    void setScalePct(int pct); // 100/125/150/175/200 validated; no-op if invalid

    // Pure geometry results. Thickness = scaled panel size.
    Rect panelRect() const;
    Strut strut() const;
    // Remaining work area after reserving strut (output minus panel strip).
    Rect workArea() const;
    // Child layout axis: true if horizontal (Bottom/Top), false if vertical.
    bool isHorizontalAxis() const;

    // Thickness helper exposed for verification.
    int thickness() const;

    // Placeholder handle fields (no real Window until X hook lands).
    // Keep as uintptr_t placeholder to avoid Xlib include.
    uintptr_t windowHandle() const { return windowHandle_; }
    void setWindowHandle(uintptr_t h) { windowHandle_ = h; }

    bool hasStartButton() const { return hasStartButton_; }

private:
    OutputId id_;
    Rect outputRect_;
    PanelEdge edge_;
    int scalePct_;
    uintptr_t windowHandle_;
    bool hasStartButton_;
};

} // namespace panel
} // namespace flamewm

#endif // FLAMEWM_PANEL_OUTPUTPANEL_H
