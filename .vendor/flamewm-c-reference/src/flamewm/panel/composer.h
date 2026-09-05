#ifndef FLAMEWM_PANEL_COMPOSER_H
#define FLAMEWM_PANEL_COMPOSER_H

#include <string>
#include <vector>

namespace flamewm {
namespace panel {

// Composition order (V4 SHELL.md, no Activities, legacy clutter off):
// Start -> pinned+running -> flexible drag surface -> two-row desktops
// -> media -> audio -> network -> tray -> clock/date
// Only blank panel space starts dock drag — children consume input (documented).
// This is a pure model describing order and drag eligibility.
class Composer {
public:
    enum Section {
        SectionStart = 0,
        SectionPinnedRunning,
        SectionFlexibleDrag,
        SectionDesktops,
        SectionMedia,
        SectionAudio,
        SectionNetwork,
        SectionTray,
        SectionClock,
        SectionCount
    };

    static const char* sectionName(Section s);
    static std::vector<Section> order();

    // Drag helper: only blank space starts dock drag.
    // In model, "blank" means hit on SectionFlexibleDrag / empty gap.
    // Children (Start button, task buttons, status icons, tray, clock) consume input.
    static bool isBlankDragSurface(Section hitSection, bool hitOnChild);
};

} // namespace panel
} // namespace flamewm

#endif // FLAMEWM_PANEL_COMPOSER_H
