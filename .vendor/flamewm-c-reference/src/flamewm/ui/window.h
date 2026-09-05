#ifndef FLAMEWM_UI_WINDOW_H
#define FLAMEWM_UI_WINDOW_H

#include "flamewm/api/geometry.h"
#include "flamewm/ui/style/visual.h"

#include <functional>

namespace flamewm {
namespace ui {

enum class WindowRole {
    GenericChild,
    PanelDock,
    ApplicationWindow,
    Popup
};

class Window {
public:
    virtual void setGeometry(const api::Rect& rect) = 0;
    virtual api::Rect geometry() const = 0;
    virtual void show() = 0;
    virtual void hide() = 0;
    virtual bool isVisible() const = 0;
    virtual void repaint() = 0;
    virtual void setEnabled(bool) {}
    virtual bool isEnabled() const { return true; }
    virtual void setOnClose(std::function<void()> cb) = 0;

    virtual void setVisual(const style::Visual& visual) { visual_ = visual; }
    virtual style::Visual visual() const { return visual_; }

    // Role is applied before mapping so the window manager sees the intended
    // surface type. Providers predating role support retain GenericChild.
    virtual void setRole(WindowRole) {}
    virtual WindowRole role() const { return WindowRole::GenericChild; }

    virtual ~Window() {}

protected:
    style::Visual visual_;
};

} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_WINDOW_H
