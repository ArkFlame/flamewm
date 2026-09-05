#ifndef FLAMEWM_UI_PALETTE_H
#define FLAMEWM_UI_PALETTE_H

#include <string>

namespace flamewm {

// Central Flame palette — single source for accent/surface tokens.
// Dark pitch-black shell, Flame red accent. No raw red scattered.
struct FlamePalette {
    // accent family
    std::string accent;        // #EF4048
    std::string accentHover;   // slightly lighter
    std::string accentPressed; // slightly darker
    std::string accentMuted;   // desaturated / transparent

    // surfaces
    std::string surface;       // pitch-black shell bg
    std::string surfaceRaised; // raised layer (menus/popovers)
    std::string border;
    std::string danger;

    // text
    std::string textPrimary;
    std::string textSecondary;

    static FlamePalette defaults();
    static FlamePalette defaultInstance() { return defaults(); }

    // Semantic accessors — callers use roles, not raw fields
    const std::string& accentForHover(bool hovered, bool pressed) const;
    const std::string& surfaceForRaised(bool raised) const;

    // Validation helper — single source of truth for Flame red
    static bool isFlameRed(const std::string& hex) { return hex == "#EF4048"; }
};

} // namespace flamewm
#endif // FLAMEWM_UI_PALETTE_H
