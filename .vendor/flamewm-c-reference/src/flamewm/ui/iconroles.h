#ifndef FLAMEWM_UI_ICONROLES_H
#define FLAMEWM_UI_ICONROLES_H

namespace flamewm {

enum IconRole {
    IconRoleAppearance = 0,
    IconRoleDesktop,
    IconRoleDisplays,
    IconRoleFonts,
    IconRoleHotkeys,
    IconRoleAbout,
    IconRoleTitleButton,
    IconRoleTaskbar,
    IconRoleTaskbarStatus,
    IconRoleMenu,
    IconRoleLauncher,
    IconRoleSearch,
    IconRoleSession,
    IconRoleNetwork,
    IconRoleFileAction,
    IconRoleSettingsCategory // alias semantic
};

enum RenderMode {
    RenderSymbolic = 0,
    RenderFullColor,
    RenderBrand
};

inline const char* iconRoleName(IconRole r) {
    switch (r) {
        case IconRoleAppearance: return "Appearance";
        case IconRoleDesktop: return "Desktop";
        case IconRoleDisplays: return "Displays";
        case IconRoleFonts: return "Fonts";
        case IconRoleHotkeys: return "Hotkeys";
        case IconRoleAbout: return "About";
        case IconRoleTitleButton: return "TitleButton";
        case IconRoleTaskbar: return "Taskbar";
        case IconRoleTaskbarStatus: return "TaskbarStatus";
        case IconRoleMenu: return "Menu";
        case IconRoleLauncher: return "Launcher";
        case IconRoleSearch: return "Search";
        case IconRoleSession: return "Session";
        case IconRoleNetwork: return "Network";
        case IconRoleFileAction: return "FileAction";
        case IconRoleSettingsCategory: return "SettingsCategory";
        default: return "Unknown";
    }
}

} // namespace flamewm
#endif
