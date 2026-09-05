#ifndef FLAMEWM_SHELL_POPOVERS_H
#define FLAMEWM_SHELL_POPOVERS_H

#include "flamewm/api/geometry.h"
#include "flamewm/api/panels.h"
#include "flamewm/ui/window.h"

#include <vector>

namespace flamewm {
namespace shell {

struct PopoverAnchor {
    api::Rect anchorRect;
    api::PanelEdge edge;
    api::Rect outputRect;
    int panelThickness;
    int gapPx;

    PopoverAnchor() : edge(api::PanelEdge::Bottom), panelThickness(0), gapPx(4) {}
    PopoverAnchor(const api::Rect& anchor, api::PanelEdge e, const api::Rect& output,
                  int thickness, int gap)
        : anchorRect(anchor), edge(e), outputRect(output), panelThickness(thickness), gapPx(gap) {}
};

// Popover anchoring — positions popover window relative to panel edge + anchor.
// Edge-aware: Bottom/Top/Left/Right each have distinct anchor arithmetic,
// then result is clamped into outputRect (handles negative origins and
// popovers larger than output). One helper serves blank-panel context,
// task contexts and workspace popover.
class Popovers {
public:
    Popovers();
    ~Popovers();

    Popovers(const Popovers&) = delete;
    Popovers& operator=(const Popovers&) = delete;

    // Compute top-left for a popover of given size anchored to anchor.
    static api::Rect anchoredRect(const PopoverAnchor& anchor, const api::Size& popoverSize);

    // Convenience: anchor from output/panel geometry + gap.
    static api::Rect anchoredRect(const api::Rect& outputRect, const api::Rect& anchorRect,
                                  api::PanelEdge edge, int panelThickness,
                                  const api::Size& popoverSize, int gapPx);

    void show(flamewm::ui::Window* popover, const PopoverAnchor& anchor, const api::Size& size);
    void hide(flamewm::ui::Window* popover);
    void hideAll();
    bool isVisible(const flamewm::ui::Window* popover) const;

    size_t visibleCount() const;
    void reanchorVisible();

private:
    struct Entry {
        flamewm::ui::Window* window;
        PopoverAnchor anchor;
        api::Size size;
        Entry(flamewm::ui::Window* w, const PopoverAnchor& a, const api::Size& s)
            : window(w), anchor(a), size(s) {}
    };
    std::vector<Entry> entries_;

    int findIndex(const flamewm::ui::Window* w) const;
};

} // namespace shell
} // namespace flamewm

#endif // FLAMEWM_SHELL_POPOVERS_H
