#ifndef FLAMEWM_SETTINGS_APP_H
#define FLAMEWM_SETTINGS_APP_H

#include <string>
#include "../flamewm/core/config.h"
#include <stdint.h>

namespace flamewm {
namespace control { class ControlClient; }
namespace settings {

enum SettingsPageId { PageAppearance=0, PageDesktop, PageTaskbar, PageDisplays, PageFonts, PageHotkeys, PageAbout, PageCount };

class SettingsApp {
public:
    SettingsApp();
    ~SettingsApp();
    bool init(const std::string& configPath, std::string* error);
    void shutdown();
    SettingsPageId currentPage() const { return current_; }
    void setPage(SettingsPageId p) { current_ = p; }
    const SettingsSnapshot& snapshot() const { return draft_; }
    const std::string& configPath() const { return configPath_; }
    bool applySnapshot(const SettingsSnapshot& snap, std::string* error);
    bool keepDisplayTx(std::string* error);
    bool revertDisplayTx(std::string* error);
    int runNative(int argc, char **argv);
    static bool parseCommandLine(int argc, char **argv, std::string* configPath,
                                 SettingsPageId* page, std::string* error);
    bool reloadFromControl(std::string* error);
    flamewm::control::ControlClient* controlClient();

private:
    SettingsSnapshot draft_;
    std::string configPath_;
    SettingsPageId current_;
    bool inited_;
    flamewm::control::ControlClient* controlClient_;
};

} // namespace settings
} // namespace flamewm
#endif
