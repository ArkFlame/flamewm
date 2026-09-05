#include <X11/Xlib.h>
#include <X11/Xutil.h>

#include <cstdlib>
#include <iostream>
#include <string>
#include <vector>

#include "../ui/style/box.h"
#include "../../flamewm-desktop/render/material_x11.h"

#define CHECK(condition, message) do { \
    if (!(condition)) { std::cerr << "FAIL: " << message << "\n"; return 1; } \
    std::cout << "PASS: " << message << "\n"; \
} while (0)

namespace {
struct Sample {
    Window window;
    const char* name;
    flamewm::ui::style::Material material;
};

bool allocate(Display* display, Colormap colormap, const std::string& value,
              XColor* color) {
    if (!XParseColor(display, colormap, value.c_str(), color)) return false;
    return XAllocColor(display, colormap, color) != 0;
}

bool waitMapped(Display* display, const std::vector<Window>& windows) {
    for (int attempt = 0; attempt < 40; ++attempt) {
        bool mapped = true;
        for (size_t i = 0; i < windows.size(); ++i) {
            XWindowAttributes attributes;
            if (!XGetWindowAttributes(display, windows[i], &attributes) ||
                attributes.map_state != IsViewable) {
                mapped = false;
                break;
            }
        }
        if (mapped) return true;
        XSync(display, False);
    }
    return false;
}
}

int main() {
    const char* displayName = std::getenv("DISPLAY");
    CHECK(displayName != 0 && *displayName != '\0', "DISPLAY is set by Xephyr harness");

    Display* display = XOpenDisplay(displayName);
    CHECK(display != 0, "open Xephyr display");
    const int screen = DefaultScreen(display);
    const Window root = RootWindow(display, screen);
    const Colormap colormap = DefaultColormap(display, screen);

    const flamewm::ui::style::Material surface(
        flamewm::ui::style::Color("#0A0A0A"), flamewm::ui::style::Color("#2A2A2A"),
        flamewm::ui::style::Color("#000000"), 8, 1);
    const flamewm::ui::style::Material raised(
        flamewm::ui::style::Color("#1A1A1A"), flamewm::ui::style::Color("#2A2A2A"),
        flamewm::ui::style::Color("#000000"), 4, 1);
    const flamewm::ui::style::Material accent(
        flamewm::ui::style::Color("#EF4048"), flamewm::ui::style::Color("#EF4048"),
        flamewm::ui::style::Color("#000000"), 4, 1);
    const flamewm::ui::style::Material border(
        flamewm::ui::style::Color("#2A2A2A"), flamewm::ui::style::Color("#2A2A2A"),
        flamewm::ui::style::Color("#000000"), 2, 1);

    XColor surfaceColor;
    XColor borderColor;
    CHECK(allocate(display, colormap, surface.fill.value, &surfaceColor), "allocate surface material");
    CHECK(allocate(display, colormap, border.border.value, &borderColor), "allocate border material");
    Window top = XCreateSimpleWindow(display, root, 80, 70, 400, 260, 1,
                                     borderColor.pixel, surfaceColor.pixel);
    CHECK(top != 0, "create deterministic top-level window");
    XSetWindowAttributes windowAttributes;
    windowAttributes.override_redirect = True;
    XChangeWindowAttributes(display, top, CWOverrideRedirect, &windowAttributes);
    XSelectInput(display, top, ExposureMask | StructureNotifyMask);

    Sample samples[] = {
        { 0, "raised", raised },
        { 0, "accent", accent },
        { 0, "border", border }
    };
    const int x[] = { 24, 152, 280 };
    for (size_t i = 0; i < sizeof(samples) / sizeof(samples[0]); ++i) {
        XColor color;
        CHECK(allocate(display, colormap, samples[i].material.fill.value, &color),
              std::string("allocate ") + samples[i].name + " material");
        samples[i].window = XCreateSimpleWindow(display, top, x[i], 80, 96, 96, 1,
                                                 borderColor.pixel, color.pixel);
        CHECK(samples[i].window != 0, std::string("create ") + samples[i].name + " child");
        XSelectInput(display, samples[i].window, ExposureMask | StructureNotifyMask);
    }

    GC gc = XCreateGC(display, top, 0, 0);
    CHECK(gc != 0, "create material graphics context");
    XMapWindow(display, top);
    for (size_t i = 0; i < sizeof(samples) / sizeof(samples[0]); ++i) {
        XMapWindow(display, samples[i].window);
    }
    XFlush(display);

    std::vector<Window> windows;
    windows.push_back(top);
    for (size_t i = 0; i < sizeof(samples) / sizeof(samples[0]); ++i) windows.push_back(samples[i].window);
    CHECK(waitMapped(display, windows), "top-level and children become viewable");

    flamewm::desktop::MaterialX11::fill(display, top, gc, 0, 0, 400, 260, surfaceColor.pixel);
    for (size_t i = 0; i < sizeof(samples) / sizeof(samples[0]); ++i) {
        XColor color;
        CHECK(allocate(display, colormap, samples[i].material.fill.value, &color),
              std::string("resolve ") + samples[i].name + " sample color");
        flamewm::desktop::MaterialX11::fill(display, samples[i].window, gc, 0, 0, 96, 96,
                                             color.pixel);
    }
    XSync(display, False);

    XImage* image = XGetImage(display, top, 0, 0, 400, 260, AllPlanes, ZPixmap);
    CHECK(image != 0, "capture top-level with XGetImage");
    CHECK(XGetPixel(image, 4, 4) == surfaceColor.pixel, "surface pixel matches material");
    XDestroyImage(image);
    for (size_t i = 0; i < sizeof(samples) / sizeof(samples[0]); ++i) {
        XColor color;
        CHECK(allocate(display, colormap, samples[i].material.fill.value, &color),
              std::string("allocate pixel expectation for ") + samples[i].name);
        image = XGetImage(display, samples[i].window, 0, 0, 96, 96, AllPlanes, ZPixmap);
        CHECK(image != 0, std::string("capture ") + samples[i].name + " child with XGetImage");
        CHECK(XGetPixel(image, 48, 48) == color.pixel,
              std::string("XGetImage pixel matches ") + samples[i].name + " child material");
        XDestroyImage(image);
    }
    XFreeGC(display, gc);
    for (size_t i = 0; i < sizeof(samples) / sizeof(samples[0]); ++i) XDestroyWindow(display, samples[i].window);
    XDestroyWindow(display, top);
    XCloseDisplay(display);
    std::cout << "ALL NATIVE UI PIXEL PASS\n";
    return 0;
}
