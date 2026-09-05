#include "settingsapp.h"
#include "../ylocale.h"
#include "../intl.h"
#include <iostream>
#include <cstdlib>
#ifdef FLAMEWM_PRODUCT_BUILD
#include "flamewm/engine/icewm/font_bootstrap.h"
#endif

// On-demand entry. Only touches Flame typed config.
// Exits when closed — zero resident cost.
int main(int argc, char** argv){
    std::string configPath;
    flamewm::settings::SettingsPageId page = flamewm::settings::PageAppearance;
    std::string commandLineError;
    if (!flamewm::settings::SettingsApp::parseCommandLine(argc, argv, &configPath, &page, &commandLineError)) {
        std::cerr << "settings argument error: " << commandLineError << "\n";
        return 2;
    }
    YLocale locale;
    bindtextdomain(PACKAGE, LOCDIR);
    textdomain(PACKAGE);
#ifdef FLAMEWM_PRODUCT_BUILD
    flamewm::engine::icewm::FontBootstrap::initialize();
#endif
    flamewm::settings::SettingsApp app;
    std::string err;
    if(!app.init(configPath, &err)){
        std::cerr<<"settings init failed: "<<err<<"\n";
        return 1;
    }
    app.setPage(page);
    int result = app.runNative(argc, argv);
    app.shutdown();
    return result;
}
