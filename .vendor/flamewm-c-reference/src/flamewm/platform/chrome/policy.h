#ifndef FLAMEWM_PLATFORM_CHROME_POLICY_H
#define FLAMEWM_PLATFORM_CHROME_POLICY_H

#include "flamewm/api/errors.h"
#include "flamewm/api/geometry.h"
#include "flamewm/api/ids.h"
#include "flamewm/api/window.h"
#include "flamewm/api/settings.h"
#include "flamewm/api/panels.h"

#include <string>

namespace flamewm {
namespace platform {
namespace chrome {

struct ChromeMetrics {
    int titleBarHeight;
    int borderX;
    int borderY;
    int titleY;

    ChromeMetrics() : titleBarHeight(0), borderX(0), borderY(0), titleY(0) {}
    ChromeMetrics(int th, int bx, int by, int ty)
        : titleBarHeight(th), borderX(bx), borderY(by), titleY(ty) {}
};

// Window chrome decision — no X11, pure policy.
// Derived from WindowSnapshot + SettingsSnapshot + PanelsSnapshot.
struct WindowChrome {
    bool decorated;
    bool titleBar;
    bool border;
    bool minimizeButton;
    bool maximizeButton;
    bool closeButton;
    bool resizable;
    bool hideTitleWhenMaximized;

    WindowChrome()
        : decorated(true)
        , titleBar(true)
        , border(true)
        , minimizeButton(true)
        , maximizeButton(true)
        , closeButton(true)
        , resizable(true)
        , hideTitleWhenMaximized(false) {}
};

class ChromePolicy {
public:
    ChromePolicy();
    ~ChromePolicy();

    ChromePolicy(const ChromePolicy&) = delete;
    ChromePolicy& operator=(const ChromePolicy&) = delete;

    // Snapshots consumed explicitly — no ports, no X11.
    void onSettingsSnapshot(const api::SettingsSnapshot& snap);
    void onPanelsSnapshot(const api::PanelsSnapshot& snap);
    void onScaleRevision(uint64_t rev);
    void clearSubscriptions();

    // Per-output scale — mirrors ScaleService (100 default). No X11.
    int scaleFor(const api::OutputId& output) const;
    void setScaleForOutput(const api::OutputId& output, int percent, api::Status* outStatus);

    // Metrics derived from scale — no X11.
    ChromeMetrics metricsFor(const api::OutputId& output) const;

    // Window chrome policy — validated, typed errors, no X11.
    api::Result<WindowChrome> chromeFor(const api::WindowSnapshot& window) const;

    // Title centering — validated inputs, no X11.
    api::Rect centeredTitleRect(api::Rect titleBar,
                                int titleWidth,
                                int buttonsLeftWidth,
                                int buttonsRightWidth) const;

    // Semantic tokens — no X11.
    std::string semanticAccent() const;

    uint64_t revision() const;

private:
    struct Impl;
    Impl* impl_;
};

} // namespace chrome
} // namespace platform
} // namespace flamewm

#endif // FLAMEWM_PLATFORM_CHROME_POLICY_H
