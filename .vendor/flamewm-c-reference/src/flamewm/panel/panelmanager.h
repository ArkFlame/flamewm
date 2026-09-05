#ifndef FLAMEWM_PANEL_PANELMANAGER_H
#define FLAMEWM_PANEL_PANELMANAGER_H

#include "../core/types.h"
#include "types.h"
#include "outputpanel.h"
#include <map>
#include <vector>

namespace flamewm {
namespace panel {

// Pure model of per-output panels. No X configure yet.
// Transaction: validateOutputEdge -> compute panel rect/strut -> stage child
// layout axis -> coalesce workArea. Single transaction owner, guard no-op.
// Hook contract: XEmbed tray has a single selection owner across all panels;
// presented on appropriate panel (active/primary) without duplicating owner.
class PanelManager {
public:
    PanelManager();
    ~PanelManager();

    // Output lifecycle.
    bool addOutput(const OutputId& id, const Rect& rect, PanelEdge edge = PanelEdgeBottom, int scalePct = 100);
    bool removeOutput(const OutputId& id);
    bool hasOutput(const OutputId& id) const;
    size_t count() const;
    unsigned long generation() const { return generation_; }
    bool isCurrentGeneration(unsigned long generation) const { return generation == generation_; }
    void clear();

    // Active-output resolver order:
    // focusedFrameOutput -> pointerOutput -> primaryOutput -> firstActive.
    void setFocusedFrameOutput(const OutputId& id);
    void setPointerOutput(const OutputId& id);
    void setPrimaryOutput(const OutputId& id);
    void clearFocusedFrameOutput();
    void clearPointerOutput();
    OutputId activeOutput() const;
    OutputId primaryOutput() const { return primary_; }
    OutputId focusedFrameOutput() const { return focusedFrame_; }
    OutputId pointerOutput() const { return pointer_; }

    // Ensure one-output fallback when topology empty or single fallback needed.
    // Returns true if fallback was synthesized.
    bool ensureOneOutputBehavior(const OutputId& fallbackId, const Rect& fallbackRect);

    // Edge/geometry transaction.
    bool validateOutputEdge(const OutputId& id, PanelEdge edge) const;
    bool setEdge(const OutputId& id, PanelEdge edge);
    Rect panelRectFor(const OutputId& id) const;
    Strut strutFor(const OutputId& id) const;
    Rect workAreaFor(const OutputId& id) const;
    bool childAxisIsHorizontal(const OutputId& id) const;

    // Coalesced work areas for wmmgr hook (one per output).
    std::vector<Rect> coalescedWorkAreas() const;

    // First active output deterministic (map order). Used as final fallback.
    OutputId firstActive() const;

    OutputPanel* panelFor(const OutputId& id);
    const OutputPanel* panelFor(const OutputId& id) const;

    // Transaction guard — single owner. Returns false if already active.
    bool beginTransaction();
    void endTransaction();
    bool inTransaction() const { return inTransaction_; }

private:
    std::map<OutputId, OutputPanel*> panels_;
    OutputId primary_;
    OutputId focusedFrame_;
    OutputId pointer_;
    bool inTransaction_;
    bool hasFocusedFrame_;
    bool hasPointer_;
    bool hasPrimary_;
    unsigned long generation_;
};

} // namespace panel
} // namespace flamewm

#endif // FLAMEWM_PANEL_PANELMANAGER_H
