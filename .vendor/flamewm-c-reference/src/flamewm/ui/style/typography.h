#ifndef FLAMEWM_UI_STYLE_TYPOGRAPHY_H
#define FLAMEWM_UI_STYLE_TYPOGRAPHY_H

#include <string>

namespace flamewm {
namespace ui {
namespace style {

enum TypographyRole {
    TypographyBody = 0,
    TypographyTitle,
    TypographyCaption,
    TypographyButton
};

struct TypographyPreset {
    std::string family;
    int size;
    bool bold;
    int lineHeight;
    int letterSpacing;

    TypographyPreset() : size(13), bold(false), lineHeight(16), letterSpacing(0) {}
    TypographyPreset(const std::string& family_, int size_, bool bold_, int lineHeight_, int letterSpacing_)
        : family(family_), size(size_), bold(bold_), lineHeight(lineHeight_), letterSpacing(letterSpacing_) {}
};

struct Typography {
    std::string family;
    int size;
    int sizeOffset;
    bool bold;
    int lineHeight;
    int letterSpacing;

    Typography() : size(13), sizeOffset(0), bold(false), lineHeight(16), letterSpacing(0) {}
    static Typography defaults();

    int effectiveSize() const { return size + sizeOffset > 1 ? size + sizeOffset : 1; }
    int sizeFor(TypographyRole role) const {
        int delta = role == TypographyTitle ? 8 : role == TypographyCaption ? -2 : 0;
        int result = effectiveSize() + delta;
        return result > 1 ? result : 1;
    }
    TypographyPreset presetFor(TypographyRole role) const {
        const int roleSize = sizeFor(role);
        return TypographyPreset(family, roleSize, boldFor(role),
                                role == TypographyTitle ? 24 : role == TypographyCaption ? 14 : 16,
                                letterSpacing);
    }
    bool boldFor(TypographyRole role) const { return bold || role == TypographyTitle || role == TypographyButton; }
    int lineHeightFor(TypographyRole role) const { return presetFor(role).lineHeight; }
    int letterSpacingFor(TypographyRole role) const { return presetFor(role).letterSpacing; }
};

} // namespace style
} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_STYLE_TYPOGRAPHY_H
