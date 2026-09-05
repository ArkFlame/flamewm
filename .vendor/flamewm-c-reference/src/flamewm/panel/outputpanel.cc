#include "outputpanel.h"
#include "../ui/metrics.h"
#include "../core/scalemanager.h"

namespace flamewm {
namespace panel {

OutputPanel::OutputPanel(const OutputId& id, const Rect& outputRect, PanelEdge edge, int scalePct)
    : id_(id), outputRect_(outputRect), edge_(edge), scalePct_(scalePct), windowHandle_(0), hasStartButton_(true)
{
    if (!isSupportedScale(scalePct_)) scalePct_ = 100;
}

OutputPanel::~OutputPanel() {}

void OutputPanel::setOutputRect(const Rect& r) { outputRect_ = r; }
void OutputPanel::setEdge(PanelEdge e) { edge_ = e; }
void OutputPanel::setScalePct(int pct) {
    if (isSupportedScale(pct)) scalePct_ = pct;
}

int OutputPanel::thickness() const {
    // Logical default panel 48 scaled.
    return FlameMetrics::logicalToPhysical(FlameMetrics::defaults().panelSize, scalePct_);
}

Rect OutputPanel::panelRect() const {
    int t = thickness();
    switch (edge_) {
        case PanelEdgeBottom:
            return Rect(outputRect_.x, outputRect_.y + outputRect_.h - t, outputRect_.w, t);
        case PanelEdgeTop:
            return Rect(outputRect_.x, outputRect_.y, outputRect_.w, t);
        case PanelEdgeLeft:
            return Rect(outputRect_.x, outputRect_.y, t, outputRect_.h);
        case PanelEdgeRight:
            return Rect(outputRect_.x + outputRect_.w - t, outputRect_.y, t, outputRect_.h);
        default:
            return Rect(outputRect_.x, outputRect_.y + outputRect_.h - t, outputRect_.w, t);
    }
}

Strut OutputPanel::strut() const {
    int t = thickness();
    Strut s;
    switch (edge_) {
        case PanelEdgeBottom: s.bottom = t; break;
        case PanelEdgeTop:    s.top    = t; break;
        case PanelEdgeLeft:   s.left   = t; break;
        case PanelEdgeRight:  s.right  = t; break;
        default: s.bottom = t; break;
    }
    return s;
}

Rect OutputPanel::workArea() const {
    int t = thickness();
    switch (edge_) {
        case PanelEdgeBottom:
            return Rect(outputRect_.x, outputRect_.y, outputRect_.w, outputRect_.h - t);
        case PanelEdgeTop:
            return Rect(outputRect_.x, outputRect_.y + t, outputRect_.w, outputRect_.h - t);
        case PanelEdgeLeft:
            return Rect(outputRect_.x + t, outputRect_.y, outputRect_.w - t, outputRect_.h);
        case PanelEdgeRight:
            return Rect(outputRect_.x, outputRect_.y, outputRect_.w - t, outputRect_.h);
        default:
            return Rect(outputRect_.x, outputRect_.y, outputRect_.w, outputRect_.h - t);
    }
}

bool OutputPanel::isHorizontalAxis() const {
    return isHorizontalEdge(edge_);
}

} // namespace panel
} // namespace flamewm
