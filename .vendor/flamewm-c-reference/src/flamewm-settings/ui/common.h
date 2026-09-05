#ifndef FLAMEWM_SETTINGS_UI_COMMON_H
#define FLAMEWM_SETTINGS_UI_COMMON_H
#include "flamewm/api/geometry.h"
#include <string>
namespace flamewm { namespace ui { class Window; class Label; class Button; } }
namespace flamewm { namespace settings { namespace ui {
void place(flamewm::ui::Window*, const flamewm::api::Rect&);
void show(flamewm::ui::Window*);
void showLabel(flamewm::ui::Label*, const flamewm::api::Rect&, const std::string&, bool);
void settingsNav(flamewm::ui::Window*);
void settingsNavItem(flamewm::ui::Button*, bool selected);
void settingsCard(flamewm::ui::Window*);
void settingsRow(flamewm::ui::Window*, bool selected);
}}}
#endif
