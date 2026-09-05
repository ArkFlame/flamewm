#ifndef FLAMEWM_UI_SLIDER_H
#define FLAMEWM_UI_SLIDER_H

#include "flamewm/ui/window.h"

#include <functional>

namespace flamewm {
namespace ui {

class Slider : public Window {
public:
    virtual void setRange(int minV, int maxV) = 0;
    virtual int minimum() const = 0;
    virtual int maximum() const = 0;
    virtual void setValue(int value) = 0;
    virtual int value() const = 0;
    virtual void setOnChanged(std::function<void(int)> cb) = 0;
    virtual ~Slider() {}
};

} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_SLIDER_H
