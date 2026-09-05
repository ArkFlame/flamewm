#ifndef FLAMEWM_UI_TEXTFIELD_H
#define FLAMEWM_UI_TEXTFIELD_H

#include "flamewm/ui/window.h"

#include <functional>
#include <string>

namespace flamewm {
namespace ui {

class TextField : public Window {
public:
    virtual void setText(const std::string& text) = 0;
    virtual std::string text() const = 0;
    virtual void setPlaceholder(const std::string& placeholder) = 0;
    virtual std::string placeholder() const = 0;
    virtual void setOnChanged(std::function<void(const std::string&)> cb) = 0;
    virtual void setOnSubmit(std::function<void(const std::string&)> cb) = 0;
    virtual void focus() = 0;
    virtual bool hasFocus() const = 0;
    virtual ~TextField() {}
};

} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_TEXTFIELD_H
