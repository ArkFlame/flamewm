#include "palette.h"
#include "flamewm/ui/style/theme.h"

namespace flamewm {

FlamePalette FlamePalette::defaults() {
    const ::flamewm::ui::style::Theme theme =
        ::flamewm::ui::style::Theme::defaults();
    FlamePalette p;
    p.accent        = theme.accent.value;
    p.accentHover   = theme.accentHover.value;
    p.accentPressed = theme.accentPressed.value;
    p.accentMuted   = theme.accentMuted.value;
    p.surface       = theme.background.value;
    p.surfaceRaised = theme.surfaceRaised.value;
    p.border        = theme.border.value;
    p.danger        = theme.danger.value;
    p.textPrimary   = theme.text.value;
    p.textSecondary = theme.muted.value;
    return p;
}

const std::string& FlamePalette::accentForHover(bool hovered, bool pressed) const {
    if (pressed) return accentPressed;
    if (hovered) return accentHover;
    return accent;
}

const std::string& FlamePalette::surfaceForRaised(bool raised) const {
    return raised ? surfaceRaised : surface;
}

} // namespace flamewm
