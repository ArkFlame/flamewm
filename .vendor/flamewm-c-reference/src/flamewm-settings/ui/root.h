#ifndef FLAMEWM_SETTINGS_UI_ROOT_H
#define FLAMEWM_SETTINGS_UI_ROOT_H
#include "../settingsapp.h"
#include "../pages/desktop.h"
#include "../pages/taskbar.h"
#include "../pages/displays.h"
#include "../pages/hotkeys.h"
#include "../pages/appearance.h"
#include "../pages/fonts.h"
#include "flamewm/ui/button.h"
#include "flamewm/ui/slider.h"
#include "flamewm/ui/textfield.h"
#include "flamewm/ui/image.h"
#include <vector>
namespace flamewm { namespace ui { class Window; class Label; class List; } }
namespace flamewm { namespace settings { namespace ui {
class Root {
public:
    explicit Root(SettingsApp*); ~Root();
    bool create(std::string*); void show();
private:
    void select(int);
    void add(flamewm::ui::Window*, int);
    flamewm::ui::Label* label(int, const flamewm::api::Rect&, const std::string&, bool);
    flamewm::ui::Button* button(int, const flamewm::api::Rect&, const std::string&, flamewm::IconRole);
    SettingsApp* app_; flamewm::ui::Window* window_; flamewm::ui::List* navigation_;
    flamewm::ui::Window* navigationSurface_; flamewm::ui::Image* wordmark_;
    std::vector<flamewm::ui::Button*> navigationItems_;
    flamewm::ui::Label* title_; flamewm::ui::Label* body_; flamewm::ui::Label* status_;
    std::vector<flamewm::ui::Window*> pageWidgets_[7];
    AppearancePage appearancePage_; DesktopPage desktopPage_; TaskbarPage taskbarPage_; DisplaysPage displaysPage_; FontsPage fontsPage_; HotkeysPage hotkeysPage_;
};
}}}
#endif
