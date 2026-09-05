#include "composer.h"

namespace flamewm {
namespace panel {

const char* Composer::sectionName(Section s) {
    switch (s) {
        case SectionStart: return "Start";
        case SectionPinnedRunning: return "PinnedRunning";
        case SectionFlexibleDrag: return "FlexibleDrag";
        case SectionDesktops: return "Desktops";
        case SectionMedia: return "Media";
        case SectionAudio: return "Audio";
        case SectionNetwork: return "Network";
        case SectionTray: return "Tray";
        case SectionClock: return "Clock";
        default: return "Unknown";
    }
}

std::vector<Composer::Section> Composer::order() {
    std::vector<Section> v;
    v.reserve(SectionCount);
    v.push_back(SectionStart);
    v.push_back(SectionPinnedRunning);
    v.push_back(SectionFlexibleDrag);
    v.push_back(SectionDesktops);
    v.push_back(SectionMedia);
    v.push_back(SectionAudio);
    v.push_back(SectionNetwork);
    v.push_back(SectionTray);
    v.push_back(SectionClock);
    return v;
}

bool Composer::isBlankDragSurface(Section hitSection, bool hitOnChild) {
    if (hitOnChild) return false; // child consumes input
    return hitSection == SectionFlexibleDrag;
}

} // namespace panel
} // namespace flamewm
