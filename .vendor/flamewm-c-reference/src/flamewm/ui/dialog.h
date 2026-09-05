#ifndef FLAMEWM_UI_DIALOG_H
#define FLAMEWM_UI_DIALOG_H

#include "flamewm/ui/window.h"

#include <string>

namespace flamewm {
namespace ui {

class Dialog : public Window {
public:
    virtual void setTitle(const std::string& title) = 0;
    virtual std::string title() const = 0;
    virtual int exec() = 0;
    virtual void accept() = 0;
    virtual void reject() = 0;
    virtual ~Dialog() {}
};

} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_DIALOG_H
