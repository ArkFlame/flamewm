#ifndef FLAMEWM_DESKTOP_FILEACTIONS_H
#define FLAMEWM_DESKTOP_FILEACTIONS_H

#include <string>
#include <vector>

namespace flamewm {
namespace desktop {

// Safe argv APIs, no shell interpolation.
// All execs via vector (execv/execvp or GAppInfo), never /bin/sh -c.
class FileActions {
public:
    // Create New Folder collision-safe, returns created path or empty on error
    static std::string createNewFolder(const std::string& desktopDir, std::string* error);
    // Open Terminal with cwd = desktopDir when safely supported (argv vector)
    static bool openTerminal(const std::string& desktopDir, std::string* error);
    static bool openPath(const std::string& path, std::string* error);
    static std::vector<std::string> buildTerminalArgv(const std::string& desktopDir);

    // Desktop and Wallpaper: open Settings on desktop page
    static bool openDesktopAndWallpaper(std::string* error);
    static std::vector<std::string> buildSettingsArgv();

    // Helpers exposed for testing: collision-safe name generation
    static std::string nextFolderName(const std::string& desktopDir);
};

} // namespace desktop
} // namespace flamewm

#endif
