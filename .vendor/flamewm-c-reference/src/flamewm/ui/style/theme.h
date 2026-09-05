#ifndef FLAMEWM_UI_STYLE_THEME_H
#define FLAMEWM_UI_STYLE_THEME_H

#include "flamewm/ui/shellstyle.h"
#include "flamewm/ui/style/typography.h"
#include "flamewm/ui/style/visual.h"

#include <string>

namespace flamewm {
namespace ui {
namespace style {

// Engine-neutral color token. Value uses CSS/X11-compatible notation.
struct Color {
    std::string value;

    Color() {}
    explicit Color(const std::string& value_) : value(value_) {}

    bool empty() const { return value.empty(); }
    bool operator==(const Color& other) const { return value == other.value; }
    bool operator!=(const Color& other) const { return !(*this == other); }
};

// ShellStyle already owns the established shell metric/token contract.
typedef ::flamewm::ShellStyle ShellMetrics;

struct SettingsMetrics {
    int navigationWidth;
    int pagePadding;
    int cardRadius;
    int rowHeight;
    int controlHeight;

    SettingsMetrics();
    static SettingsMetrics defaults();
};

struct Theme {
    Color accent;
    Color accentHover;
    Color accentPressed;
    Color accentMuted;
    Color background;
    Color panel;
    Color navigation;
    Color surface;
    Color surfaceRaised;
    Color hover;
    Color pressed;
    Color border;
    Color strong;
    Color deep;
    Color text;
    Color textBright;
    Color muted;
    Color mutedDeep;
    Color indicator;
    Color indicatorFocused;
    Color danger;
    Color textPrimary;
    Color textSecondary;
    Typography typography;
    ShellMetrics shell;
    SettingsMetrics settings;

    static Theme defaults();

    const Color& color(VisualRole role, VisualState state) const;
    const Color& color(VisualRole role, VisualStateMask states) const;

    const Color& primaryText() const { return textPrimary.empty() ? text : textPrimary; }
    const Color& secondaryText() const { return textSecondary.empty() ? muted : textSecondary; }
};

} // namespace style
} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_STYLE_THEME_H
