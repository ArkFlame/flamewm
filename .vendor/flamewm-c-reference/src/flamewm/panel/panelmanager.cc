#include "panelmanager.h"

namespace flamewm {
namespace panel {

PanelManager::PanelManager()
    : inTransaction_(false), hasFocusedFrame_(false), hasPointer_(false), hasPrimary_(false), generation_(0) {}

PanelManager::~PanelManager() { clear(); }

void PanelManager::clear() {
    for (std::map<OutputId, OutputPanel*>::iterator it = panels_.begin(); it != panels_.end(); ++it)
        delete it->second;
    panels_.clear();
    ++generation_;
}

bool PanelManager::addOutput(const OutputId& id, const Rect& rect, PanelEdge edge, int scalePct) {
    if (id.empty()) return false;
    if (panels_.find(id) != panels_.end()) {
        // update rect/edge/scale idempotently
        OutputPanel* p = panels_[id];
        bool changed = p->outputRect() != rect || p->edge() != edge || p->scalePct() != scalePct;
        p->setOutputRect(rect);
        p->setEdge(edge);
        p->setScalePct(scalePct);
        if (changed)
            ++generation_;
        return true;
    }
    OutputPanel* p = new OutputPanel(id, rect, edge, scalePct);
    panels_[id] = p;
    ++generation_;
    return true;
}

bool PanelManager::removeOutput(const OutputId& id) {
    std::map<OutputId, OutputPanel*>::iterator it = panels_.find(id);
    if (it == panels_.end()) return false;
    delete it->second;
    panels_.erase(it);
    ++generation_;
    if (focusedFrame_ == id) { focusedFrame_ = OutputId(); hasFocusedFrame_ = false; }
    if (pointer_ == id) { pointer_ = OutputId(); hasPointer_ = false; }
    if (primary_ == id) { primary_ = OutputId(); hasPrimary_ = false; }
    return true;
}

bool PanelManager::hasOutput(const OutputId& id) const {
    return panels_.find(id) != panels_.end();
}

size_t PanelManager::count() const { return panels_.size(); }

void PanelManager::setFocusedFrameOutput(const OutputId& id) { focusedFrame_ = id; hasFocusedFrame_ = !id.empty(); }
void PanelManager::setPointerOutput(const OutputId& id) { pointer_ = id; hasPointer_ = !id.empty(); }
void PanelManager::setPrimaryOutput(const OutputId& id) { primary_ = id; hasPrimary_ = !id.empty(); }
void PanelManager::clearFocusedFrameOutput() { focusedFrame_ = OutputId(); hasFocusedFrame_ = false; }
void PanelManager::clearPointerOutput() { pointer_ = OutputId(); hasPointer_ = false; }

OutputId PanelManager::firstActive() const {
    if (panels_.empty()) return OutputId();
    return panels_.begin()->first;
}

OutputId PanelManager::activeOutput() const {
    if (hasFocusedFrame_ && hasOutput(focusedFrame_)) return focusedFrame_;
    if (hasPointer_ && hasOutput(pointer_)) return pointer_;
    if (hasPrimary_ && hasOutput(primary_)) return primary_;
    return firstActive();
}

bool PanelManager::ensureOneOutputBehavior(const OutputId& fallbackId, const Rect& fallbackRect) {
    if (!panels_.empty()) return false;
    if (fallbackId.empty()) return false;
    addOutput(fallbackId, fallbackRect, PanelEdgeBottom, 100);
    setPrimaryOutput(fallbackId);
    return true;
}

bool PanelManager::validateOutputEdge(const OutputId& id, PanelEdge edge) const {
    if (!hasOutput(id)) return false;
    (void)edge;
    return true;
}

bool PanelManager::setEdge(const OutputId& id, PanelEdge edge) {
    if (!validateOutputEdge(id, edge)) return false;
    OutputPanel* p = panels_[id];
    if (p->edge() == edge) return false; // no-op guard
    // Transaction: stage child layout axis implicitly via edge -> isHorizontalAxis.
    // Coalesce workArea derived from new edge.
    p->setEdge(edge);
    return true;
}

Rect PanelManager::panelRectFor(const OutputId& id) const {
    std::map<OutputId, OutputPanel*>::const_iterator it = panels_.find(id);
    if (it == panels_.end()) return Rect();
    return it->second->panelRect();
}

Strut PanelManager::strutFor(const OutputId& id) const {
    std::map<OutputId, OutputPanel*>::const_iterator it = panels_.find(id);
    if (it == panels_.end()) return Strut();
    return it->second->strut();
}

Rect PanelManager::workAreaFor(const OutputId& id) const {
    std::map<OutputId, OutputPanel*>::const_iterator it = panels_.find(id);
    if (it == panels_.end()) return Rect();
    return it->second->workArea();
}

bool PanelManager::childAxisIsHorizontal(const OutputId& id) const {
    std::map<OutputId, OutputPanel*>::const_iterator it = panels_.find(id);
    if (it == panels_.end()) return true;
    return it->second->isHorizontalAxis();
}

std::vector<Rect> PanelManager::coalescedWorkAreas() const {
    std::vector<Rect> out;
    out.reserve(panels_.size());
    for (std::map<OutputId, OutputPanel*>::const_iterator it = panels_.begin(); it != panels_.end(); ++it)
        out.push_back(it->second->workArea());
    return out;
}

OutputPanel* PanelManager::panelFor(const OutputId& id) {
    std::map<OutputId, OutputPanel*>::iterator it = panels_.find(id);
    if (it == panels_.end()) return 0;
    return it->second;
}

const OutputPanel* PanelManager::panelFor(const OutputId& id) const {
    std::map<OutputId, OutputPanel*>::const_iterator it = panels_.find(id);
    if (it == panels_.end()) return 0;
    return it->second;
}

bool PanelManager::beginTransaction() {
    if (inTransaction_) return false;
    inTransaction_ = true;
    return true;
}

void PanelManager::endTransaction() { inTransaction_ = false; }

} // namespace panel
} // namespace flamewm
