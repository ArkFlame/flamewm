#ifndef FLAMEWM_UI_SHELLSTYLE_H
#define FLAMEWM_UI_SHELLSTYLE_H

#include <string>

namespace flamewm {

// Product-owned shell presentation tokens. Values are logical pixels unless
// explicitly marked as a percentage or color.
struct ShellStyle {
    int panelHeight;
    int panelPadX;
    std::string panelColor;
    int panelOpacityPercent;

    std::string accent;
    std::string text;
    std::string muted;
    std::string runningIndicator;
    std::string focusedIndicator;
    std::string workspaceBorder;
    std::string workspaceBg;
    std::string workspaceText;
    std::string clockDate;
    std::string popoverBg;
    double popoverBorderAlpha;
    int radius;

    int startButtonWidth;
    int startIconSize;

    int taskButtonWidth;
    int taskIconSize;
    int taskIndicatorInset;
    int taskIndicatorHeight;
    int taskFocusedIndicatorInset;
    int taskFocusedIndicatorHeight;

    int workspaceButtonWidth;
    int workspaceButtonHeight;
    int workspaceColumnGap;
    int workspaceRowGap;
    int workspacePadX;
    int workspacePadY;
    int workspaceRows;

    int trayButtonWidth;
    int trayIconSize;

    int clockMinWidth;
    int clockPadLeft;
    int clockPadRight;
    int clockPadTop;
    int clockPadBottom;
    int clockLineHeight;

    int popoverOffset;
    int popoverPadding;
    int startMenuWidth;
    int startMenuMinHeight;
    int startSubmenuWidth;
    int trayPopoverWidth;
    int clockPopoverWidth;

    static ShellStyle defaults();
};

} // namespace flamewm

#endif // FLAMEWM_UI_SHELLSTYLE_H
