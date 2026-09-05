#ifndef FLAMEWM_UI_POPOVER_H
#define FLAMEWM_UI_POPOVER_H

#include "flamewm/api/geometry.h"
#include "flamewm/ui/window.h"

#include <functional>

namespace flamewm {
namespace ui {

class Popover : public Window {
public:
    virtual void setAnchor(const api::Rect& rect) = 0;
    virtual api::Rect anchor() const = 0;
    virtual void showAt(const api::Rect& anchor) = 0;
    virtual void close() = 0;
    virtual void setOnClosed(std::function<void()> cb) = 0;
    virtual ~Popover() {}
};

} // namespace ui
} // namespace flamewm

#endif // FLAMEWM_UI_POPOVER_H
