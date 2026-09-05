#ifndef FLAMEWM_UI_BACKEND_H
#define FLAMEWM_UI_BACKEND_H

#include "flamewm/ui/button.h"
#include "flamewm/ui/dialog.h"
#include "flamewm/ui/image.h"
#include "flamewm/ui/label.h"
#include "flamewm/ui/list.h"
#include "flamewm/ui/popover.h"
#include "flamewm/ui/slider.h"
#include "flamewm/ui/textfield.h"
#include "flamewm/ui/window.h"

#include <string>

namespace flamewm {
namespace ui {

// Provider owns the native widget implementation. The facade owns neither the
// provider nor widgets returned by it. A provider must be installed explicitly.
class UiProvider {
public:
    virtual bool init() = 0;
    virtual void shutdown() = 0;

    virtual Window* createWindow(Window* parent) = 0;
    virtual Button* createButton(Window* parent, IconRole role) = 0;
    virtual Toggle* createToggle(Window* parent, IconRole role) = 0;
    virtual Label* createLabel(Window* parent) = 0;
    virtual Slider* createSlider(Window* parent) = 0;
    virtual Dialog* createDialog(Window* parent) = 0;
    virtual TextField* createTextField(Window* parent) = 0;
    virtual List* createList(Window* parent) = 0;
    virtual Popover* createPopover(Window* parent) = 0;
    // Older providers may omit images; the facade supplies a headless fallback.
    virtual Image* createImage(Window*) { return 0; }
    virtual ~UiProvider() {}
};

// UiBackend is an X11-free process-wide provider facade.
class UiBackend {
public:
    // Installs one non-owned provider. Fails if another provider is installed
    // or if this provider is already initialized.
    static bool installProvider(UiProvider* provider, std::string* error = 0);
    static bool removeProvider(UiProvider* provider, std::string* error = 0);
    static bool installTestProvider(std::string* error = 0);

    static bool isAvailable();
    static bool init();
    static void shutdown();

    static Window* createWindow(Window* parent = 0);
    static Button* createButton(Window* parent = 0, IconRole role = IconRoleTaskbar);
    static Toggle* createToggle(Window* parent = 0, IconRole role = IconRoleTaskbar);
    static Label* createLabel(Window* parent = 0);
    static Slider* createSlider(Window* parent = 0);
    static Dialog* createDialog(Window* parent = 0);
    static TextField* createTextField(Window* parent = 0);
    static List* createList(Window* parent = 0);
    static Popover* createPopover(Window* parent = 0);
    static Image* createImage(Window* parent = 0);
};

} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_BACKEND_H
