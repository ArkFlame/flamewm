#pragma once

#include "flamewm/api/geometry.h"
#include <cstdint>

#if __has_include(<X11/Xlib.h>)
#include <X11/Xlib.h>
#ifdef Status
#undef Status
#endif
#endif

namespace flamewm {
namespace engine {
namespace icewm {

class NativeOverlay {
public:
    NativeOverlay();
    explicit NativeOverlay(void* display);
    ~NativeOverlay();

    void setDisplay(void* display);
    void* display() const;

    bool isVisible() const;
    api::Rect geometry() const;

    bool show(const api::Rect& rect);
    void move(const api::Rect& rect);
    void hide();
    void destroy();

    void setAccentColor(uint32_t color);
    void setFillColor(uint32_t color);
    void setOpacity(unsigned percent);
    void setInset(unsigned inset);
    uint32_t accentColor() const;
    uint32_t fillColor() const;
    unsigned opacity() const;
    unsigned inset() const;

private:
    void* display_;
#if __has_include(<X11/Xlib.h>)
    Window window_;
#else
    unsigned long window_;
#endif
    bool visible_;
    api::Rect geometry_;
    bool hasDisplay_;
    uint32_t accentColor_;
    uint32_t fillColor_;
    unsigned opacity_;
    unsigned inset_;

    bool createWindow(const api::Rect& rect);
    void applyGeometry(const api::Rect& rect);
};

} // namespace icewm
} // namespace engine
} // namespace flamewm
