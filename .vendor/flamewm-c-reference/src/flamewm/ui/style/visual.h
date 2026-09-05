#ifndef FLAMEWM_UI_STYLE_VISUAL_H
#define FLAMEWM_UI_STYLE_VISUAL_H

namespace flamewm {
namespace ui {
namespace style {

enum VisualRole {
    VisualRoleSurface = 0,
    VisualRoleControl,
    VisualRoleNavigation,
    VisualRoleTask,
    VisualRoleMenu,
    VisualRoleOverlay,
    VisualRoleText,
    VisualRoleIcon,
    VisualRoleBackground,
    VisualRolePanel,
    VisualRoleRaised,
    VisualRoleBorder,
    VisualRoleStrong,
    VisualRoleDeep,
    VisualRoleIndicator
};

enum VisualState {
    VisualNormal = 0,
    VisualHovered = 1 << 0,
    VisualPressed = 1 << 1,
    VisualFocused = 1 << 2,
    VisualSelected = 1 << 3,
    VisualDisabled = 1 << 4
};

typedef unsigned VisualStateMask;

inline VisualStateMask operator|(VisualState left, VisualState right) {
    return static_cast<VisualStateMask>(left) | static_cast<VisualStateMask>(right);
}

inline VisualStateMask operator|(VisualStateMask left, VisualState right) {
    return left | static_cast<VisualStateMask>(right);
}

inline VisualStateMask operator|(VisualState left, VisualStateMask right) {
    return static_cast<VisualStateMask>(left) | right;
}

inline bool hasVisualState(VisualStateMask states, VisualState state) {
    if (state == VisualNormal) return states == 0;
    return (states & static_cast<VisualStateMask>(state)) != 0;
}

struct Visual {
    VisualRole role;
    VisualStateMask state;
    bool enabled;

    Visual() : role(VisualRoleSurface), state(VisualNormal), enabled(true) {}
    explicit Visual(VisualState state_)
        : role(VisualRoleSurface), state(state_), enabled(state_ != VisualDisabled) {}
    Visual(VisualRole role_, VisualState state_)
        : role(role_), state(static_cast<VisualStateMask>(state_)), enabled(state_ != VisualDisabled) {}

    Visual(VisualRole role_, VisualStateMask states_)
        : role(role_), state(states_), enabled(!hasVisualState(states_, VisualDisabled)) {}

    bool hasState(VisualState value) const { return hasVisualState(state, value); }
    bool interactive() const { return enabled && !hasState(VisualDisabled); }
};

} // namespace style
} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_STYLE_VISUAL_H
