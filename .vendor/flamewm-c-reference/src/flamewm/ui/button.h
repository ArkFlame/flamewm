#ifndef FLAMEWM_UI_BUTTON_H
#define FLAMEWM_UI_BUTTON_H

#include "flamewm/ui/iconroles.h"
#include "flamewm/ui/window.h"

#include <functional>
#include <string>

namespace flamewm {
namespace ui {

class Button : public Window {
public:
    virtual void setText(const std::string& text) = 0;
    virtual std::string text() const = 0;
    virtual void setIconRole(IconRole role) = 0;
    virtual IconRole iconRole() const = 0;
    virtual void setIconName(const std::string&) {}
    virtual std::string iconName() const { return std::string(); }
    virtual void setVisualState(unsigned) {}
    virtual unsigned visualState() const { return 0; }
    virtual void setEnabled(bool enabled) = 0;
    virtual bool isEnabled() const = 0;
    virtual void setOnClick(std::function<void()> cb) = 0;
    virtual void setOnContextMenu(std::function<void(int,int)> cb) = 0;
    // Press/release/motion for task drag forwarding
    virtual void setOnPress(std::function<void(int,int,int)> cb) = 0;
    virtual void setOnRelease(std::function<void(int,int,int)> cb) = 0;
    virtual void setOnMotion(std::function<void(int,int)> cb) = 0;
    virtual ~Button() {}
};

class Toggle : public Button {
public:
    virtual void setChecked(bool checked) = 0;
    virtual bool isChecked() const = 0;
    virtual void setOnToggled(std::function<void(bool)> cb) = 0;
    virtual ~Toggle() {}
};

} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_BUTTON_H
