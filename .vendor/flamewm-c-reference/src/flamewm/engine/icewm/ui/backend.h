#ifndef FLAMEWM_ENGINE_ICEWM_UI_BACKEND_H
#define FLAMEWM_ENGINE_ICEWM_UI_BACKEND_H

#include "flamewm/ui/window.h"
#include "flamewm/ui/button.h"
#include "flamewm/ui/label.h"
#include "flamewm/ui/slider.h"
#include "flamewm/ui/dialog.h"
#include "flamewm/ui/image.h"
#include "flamewm/ui/textfield.h"
#include "flamewm/ui/list.h"
#include "flamewm/ui/popover.h"

namespace flamewm {
namespace engine {
namespace icewm {
namespace ui {

class IceWMBackend {
public:
    IceWMBackend();
    ~IceWMBackend();

    IceWMBackend(const IceWMBackend&) = delete;
    IceWMBackend& operator=(const IceWMBackend&) = delete;

    bool isAvailable() const;
    bool init();
    void shutdown();

    flamewm::ui::Window* createWindow();
    flamewm::ui::Window* createWindow(flamewm::ui::WindowRole role,
                                      flamewm::ui::Window* parent = 0);
    flamewm::ui::Button* createButton();
    flamewm::ui::Toggle* createToggle();
    flamewm::ui::Label* createLabel();
    flamewm::ui::Slider* createSlider();
    flamewm::ui::Dialog* createDialog();
    flamewm::ui::TextField* createTextField();
    flamewm::ui::List* createList();
    flamewm::ui::Popover* createPopover();
    flamewm::ui::Image* createImage();

    static IceWMBackend& instance();

private:
    struct Impl;
    Impl* impl_;
};

} // namespace ui
} // namespace icewm
} // namespace engine
} // namespace flamewm

#endif // FLAMEWM_ENGINE_ICEWM_UI_BACKEND_H
