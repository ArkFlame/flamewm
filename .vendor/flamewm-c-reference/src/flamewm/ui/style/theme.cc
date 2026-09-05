#include "theme.h"

namespace flamewm {
namespace ui {
namespace style {

SettingsMetrics::SettingsMetrics()
    : navigationWidth(192), pagePadding(24), cardRadius(8), rowHeight(68), controlHeight(36) {
}

SettingsMetrics SettingsMetrics::defaults() {
    return SettingsMetrics();
}

Theme Theme::defaults() {
    Theme theme;
    theme.accent = Color("#EF4048");
    theme.accentHover = Color("#F2575E");
    theme.accentPressed = Color("#D9363E");
    theme.accentMuted = Color("#6C3034");
    theme.background = Color("#202326");
    theme.panel = Color("#191B1D");
    theme.navigation = Color("#1B1E20");
    theme.surface = Color("#202326");
    theme.surfaceRaised = Color("#25282B");
    theme.hover = Color("#303438");
    theme.pressed = Color("#383D42");
    theme.border = Color("#3A3F44");
    theme.strong = Color("#51565B");
    theme.deep = Color("#17191B");
    theme.text = Color("#F1F2F3");
    theme.textBright = Color("#FFFFFF");
    theme.muted = Color("#AEB4BB");
    theme.mutedDeep = Color("#737A82");
    theme.indicator = Color("#87919B");
    theme.indicatorFocused = Color("#C9CDD1");
    theme.danger = Color("#E53935");
    theme.textPrimary = theme.text;
    theme.textSecondary = theme.muted;
    theme.typography = Typography::defaults();
    theme.shell.panelHeight = 44;
    theme.shell.panelPadX = 2;
    theme.shell.panelColor = theme.panel.value;
    theme.shell.panelOpacityPercent = 99;
    theme.shell.accent = theme.accent.value;
    theme.shell.text = theme.text.value;
    theme.shell.muted = theme.muted.value;
    theme.shell.runningIndicator = theme.indicator.value;
    theme.shell.focusedIndicator = theme.indicatorFocused.value;
    theme.shell.workspaceBorder = theme.border.value;
    theme.shell.workspaceBg = theme.surfaceRaised.value;
    theme.shell.workspaceText = theme.indicatorFocused.value;
    theme.shell.clockDate = theme.indicatorFocused.value;
    theme.shell.popoverBg = theme.surfaceRaised.value;
    theme.shell.popoverBorderAlpha = 0.15;
    theme.shell.radius = 4;
    theme.shell.startButtonWidth = 43;
    theme.shell.startIconSize = 24;
    theme.shell.taskButtonWidth = 42;
    theme.shell.taskIconSize = 25;
    theme.shell.taskIndicatorInset = 9;
    theme.shell.taskIndicatorHeight = 2;
    theme.shell.taskFocusedIndicatorInset = 5;
    theme.shell.taskFocusedIndicatorHeight = 3;
    theme.shell.workspaceButtonWidth = 22;
    theme.shell.workspaceButtonHeight = 16;
    theme.shell.workspaceColumnGap = 1;
    theme.shell.workspaceRowGap = 1;
    theme.shell.workspacePadX = 3;
    theme.shell.workspacePadY = 4;
    theme.shell.workspaceRows = 2;
    theme.shell.trayButtonWidth = 30;
    theme.shell.trayIconSize = 18;
    theme.shell.clockMinWidth = 70;
    theme.shell.clockPadLeft = 3;
    theme.shell.clockPadRight = 5;
    theme.shell.clockPadTop = 2;
    theme.shell.clockPadBottom = 1;
    theme.shell.clockLineHeight = 16;
    theme.shell.popoverOffset = 4;
    theme.shell.popoverPadding = 13;
    theme.shell.startMenuWidth = 292;
    theme.shell.startMenuMinHeight = 349;
    theme.shell.startSubmenuWidth = 258;
    theme.shell.trayPopoverWidth = 310;
    theme.shell.clockPopoverWidth = 286;
    theme.settings = SettingsMetrics::defaults();
    return theme;
}

const Color& Theme::color(VisualRole role, VisualState state) const {
    return color(role, static_cast<VisualStateMask>(state));
}

const Color& Theme::color(VisualRole role, VisualStateMask states) const {
    if (hasVisualState(states, VisualDisabled)) return muted;
    if (hasVisualState(states, VisualPressed)) {
        return role == VisualRoleText || role == VisualRoleIcon ? accentPressed : pressed;
    }
    if (hasVisualState(states, VisualHovered)) {
        return role == VisualRoleText || role == VisualRoleIcon ? accentHover : hover;
    }
    if (hasVisualState(states, VisualSelected) || hasVisualState(states, VisualFocused)) return accent;
    switch (role) {
        case VisualRoleText:
            return primaryText();
        case VisualRoleIcon: return indicatorFocused;
        case VisualRoleNavigation: return navigation;
        case VisualRoleControl: return surfaceRaised;
        case VisualRoleTask: return panel;
        case VisualRoleMenu:
        case VisualRoleOverlay:
        case VisualRoleRaised: return surfaceRaised;
        case VisualRoleBackground: return background;
        case VisualRolePanel: return panel;
        case VisualRoleBorder: return border;
        case VisualRoleStrong: return strong;
        case VisualRoleDeep: return deep;
        case VisualRoleIndicator: return indicator;
        case VisualRoleSurface:
        default: return surface;
    }
}

} // namespace style
} // namespace ui
} // namespace flamewm
