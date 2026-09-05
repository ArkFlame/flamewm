#include "common.h"
#include "flamewm/ui/label.h"
#include "flamewm/ui/button.h"
#include "flamewm/ui/window.h"
namespace flamewm { namespace settings { namespace ui {
void place(flamewm::ui::Window* w, const flamewm::api::Rect& r) { if (w) w->setGeometry(r); }
void show(flamewm::ui::Window* w) { if (w) w->show(); }
void showLabel(flamewm::ui::Label* l, const flamewm::api::Rect& r, const std::string& t, bool wrap) {
    if (!l) return; l->setGeometry(r); l->setText(t); l->setWrap(wrap); l->show();
}
void settingsNav(flamewm::ui::Window* w) {
    if (w) { w->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleNavigation)); w->show(); }
}
void settingsNavItem(flamewm::ui::Button* b, bool selected) {
    if (!b) return;
    const flamewm::ui::style::VisualState state = selected ? flamewm::ui::style::VisualSelected : flamewm::ui::style::VisualNormal;
    b->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleControl, state));
    b->setVisualState(static_cast<unsigned>(state));
}
void settingsCard(flamewm::ui::Window* w) {
    if (w) w->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleRaised));
}
void settingsRow(flamewm::ui::Window* w, bool selected) {
    if (w) w->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleControl,
        selected ? flamewm::ui::style::VisualSelected : flamewm::ui::style::VisualNormal));
}
}}}
