#include "flamewm/shell/popovers.h"

#include <algorithm>

namespace flamewm {
namespace shell {

Popovers::Popovers() {}
Popovers::~Popovers() {
    // Windows may be destroyed before Popovers (reverse declaration / shutdown).
    // Do not dispatch virtual hide() on potentially-destroyed windows.
    entries_.clear();
}

api::Rect Popovers::anchoredRect(const PopoverAnchor& a, const api::Size& sz) {
    return anchoredRect(a.outputRect, a.anchorRect, a.edge, a.panelThickness, sz, a.gapPx);
}

api::Rect Popovers::anchoredRect(const api::Rect& outputRect, const api::Rect& anchorRect,
                                 api::PanelEdge edge, int /*panelThickness*/,
                                 const api::Size& sz, int gapPx) {
    // Edge-aware popover placement with output clamping.
    // Bottom: above panel, left-aligned to anchor left.
    // Top:    below panel, left-aligned to anchor left.
    // Left:   to the right of panel, top-aligned to anchor top.
    // Right:  to the left of panel, top-aligned to anchor top.
    // Negative output origins and oversize popovers are clamped.

    if (!sz.valid()) {
        return api::Rect(anchorRect.x, anchorRect.y, 0, 0);
    }

    int w = sz.w;
    int h = sz.h;
    // Clamp size to output if larger — otherwise clamping position would be insufficient.
    if (outputRect.valid()) {
        if (w > outputRect.w) w = outputRect.w;
        if (h > outputRect.h) h = outputRect.h;
    }

    int desiredX = anchorRect.x;
    int desiredY = anchorRect.y;

    switch (edge) {
        case api::PanelEdge::Bottom:
            desiredX = anchorRect.x;
            desiredY = anchorRect.y - gapPx - h;
            break;
        case api::PanelEdge::Top:
            desiredX = anchorRect.x;
            desiredY = anchorRect.y + anchorRect.h + gapPx;
            break;
        case api::PanelEdge::Left:
            desiredX = anchorRect.x + anchorRect.w + gapPx;
            desiredY = anchorRect.y;
            break;
        case api::PanelEdge::Right:
            desiredX = anchorRect.x - gapPx - w;
            desiredY = anchorRect.y;
            break;
        default:
            desiredX = anchorRect.x;
            desiredY = anchorRect.y - gapPx - h;
            break;
    }

    if (!outputRect.valid()) {
        return api::Rect(desiredX, desiredY, w, h);
    }

    int minX = outputRect.x;
    int maxX = outputRect.x + outputRect.w - w;
    int minY = outputRect.y;
    int maxY = outputRect.y + outputRect.h - h;

    // Oversize already clamped above; pin to origin if still not fitting (defensive).
    if (w >= outputRect.w) { minX = outputRect.x; maxX = outputRect.x; }
    if (h >= outputRect.h) { minY = outputRect.y; maxY = outputRect.y; }

    int cx = std::max(minX, std::min(desiredX, maxX));
    int cy = std::max(minY, std::min(desiredY, maxY));
    return api::Rect(cx, cy, w, h);
}

int Popovers::findIndex(const flamewm::ui::Window* w) const {
    for (size_t i = 0; i < entries_.size(); ++i) {
        if (entries_[i].window == w) {
            return static_cast<int>(i);
        }
    }
    return -1;
}

void Popovers::show(flamewm::ui::Window* popover, const PopoverAnchor& anchor, const api::Size& size) {
    if (!popover) return;

    api::Rect rc = anchoredRect(anchor, size);
    // Avoid showing zero-size popovers.
    if (!rc.valid()) {
        rc.w = size.w;
        rc.h = size.h;
    }

    int idx = findIndex(popover);
    if (idx >= 0) {
        entries_[static_cast<size_t>(idx)].anchor = anchor;
        entries_[static_cast<size_t>(idx)].size = size;
    } else {
        entries_.push_back(Entry(popover, anchor, size));
    }

    popover->setGeometry(rc);
    popover->show();
    popover->repaint();

    // Ensure only one popover visible: close others (blank-panel / task / workspace
    // contexts are mutually exclusive).
    for (size_t i = 0; i < entries_.size(); ) {
        if (entries_[i].window != popover && entries_[i].window) {
            entries_[i].window->hide();
            entries_.erase(entries_.begin() + static_cast<int>(i));
        } else {
            ++i;
        }
    }
}

void Popovers::hide(flamewm::ui::Window* popover) {
    if (!popover) return;
    int idx = findIndex(popover);
    if (idx < 0) {
        // Still hide even if not tracked — caller may hide unmanaged window.
        popover->hide();
        return;
    }
    popover->hide();
    entries_.erase(entries_.begin() + idx);
}

void Popovers::hideAll() {
    for (size_t i = 0; i < entries_.size(); ++i) {
        if (entries_[i].window) {
            entries_[i].window->hide();
        }
    }
    entries_.clear();
}

bool Popovers::isVisible(const flamewm::ui::Window* popover) const {
    if (!popover) return false;
    int idx = findIndex(popover);
    if (idx < 0) return false;
    return popover->isVisible();
}

size_t Popovers::visibleCount() const {
    size_t n = 0;
    for (size_t i = 0; i < entries_.size(); ++i) {
        if (entries_[i].window && entries_[i].window->isVisible()) ++n;
    }
    return n;
}

void Popovers::reanchorVisible() {
    for (size_t i = 0; i < entries_.size(); ++i) {
        if (!entries_[i].window) continue;
        if (!entries_[i].window->isVisible()) continue;
        api::Rect rc = anchoredRect(entries_[i].anchor, entries_[i].size);
        entries_[i].window->setGeometry(rc);
        entries_[i].window->repaint();
    }
}

} // namespace shell
} // namespace flamewm
