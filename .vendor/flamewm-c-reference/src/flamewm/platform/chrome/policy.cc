#include "flamewm/platform/chrome/policy.h"

#include <algorithm>
#include <map>
#include <string>

namespace flamewm {
namespace platform {
namespace chrome {

namespace {

bool isAllowedScale(int pct) {
    return pct == 100 || pct == 125 || pct == 150 || pct == 175 || pct == 200;
}

bool parseBoolSetting(const api::SettingsSnapshot& snap, const std::string& key, bool def) {
    std::map<std::string, std::string>::const_iterator it = snap.values.find(key);
    if (it == snap.values.end()) return def;
    const std::string& v = it->second;
    if (v == "1" || v == "true" || v == "yes" || v == "on") return true;
    if (v == "0" || v == "false" || v == "no" || v == "off") return false;
    return def;
}

} // namespace

struct ChromePolicy::Impl {
    std::map<std::string, int> scales;
    api::SettingsSnapshot settings;
    api::PanelsSnapshot panels;
    uint64_t rev;
    uint64_t scaleRev;

    Impl() : scales(), settings(), panels(), rev(0), scaleRev(0) {}
};

ChromePolicy::ChromePolicy()
    : impl_(new Impl()) {
}

ChromePolicy::~ChromePolicy() {
    delete impl_;
}

void ChromePolicy::onSettingsSnapshot(const api::SettingsSnapshot& snap) {
    impl_->settings = snap;
    ++impl_->rev;
}

void ChromePolicy::onPanelsSnapshot(const api::PanelsSnapshot& snap) {
    impl_->panels = snap;
    ++impl_->rev;
}

void ChromePolicy::onScaleRevision(uint64_t rev) {
    impl_->scaleRev = rev;
    ++impl_->rev;
}

void ChromePolicy::clearSubscriptions() {
    // No listener map in minimal policy — clear stored snapshots' listener affinity
    // and reset revision tracking if needed. No X11.
}

int ChromePolicy::scaleFor(const api::OutputId& output) const {
    if (!impl_) return 100;
    if (!output.valid()) return 100;
    std::map<std::string, int>::const_iterator it = impl_->scales.find(output.key);
    if (it != impl_->scales.end()) return it->second;
    return 100;
}

void ChromePolicy::setScaleForOutput(const api::OutputId& output, int percent, api::Status* outStatus) {
    if (!output.valid()) {
        if (outStatus) *outStatus = api::Status::make(api::Error::InvalidArgument, "invalid output");
        return;
    }
    if (!isAllowedScale(percent)) {
        if (outStatus) *outStatus = api::Status::make(api::Error::InvalidArgument, "scale must be 100/125/150/175/200");
        return;
    }
    std::map<std::string, int>::iterator it = impl_->scales.find(output.key);
    if (it != impl_->scales.end() && it->second == percent) {
        if (outStatus) *outStatus = api::Status::Ok();
        return;
    }
    impl_->scales[output.key] = percent;
    ++impl_->rev;
    ++impl_->scaleRev;
    if (outStatus) *outStatus = api::Status::Ok();
}

ChromeMetrics ChromePolicy::metricsFor(const api::OutputId& output) const {
    int scale = scaleFor(output);
    // Inline FlameMetrics defaults (platform must not link ui/palette).
    // Logical tokens at 100%: titlebarSize=32, visualBorder=1.
    // Physical = (logical * scale + 50)/100 (round half up).
    int titlebarSize = (32 * scale + 50) / 100;
    int visualBorder = (1 * scale + 50) / 100;
    ChromeMetrics m;
    m.titleBarHeight = titlebarSize;
    m.titleY = titlebarSize;
    m.borderX = visualBorder;
    m.borderY = visualBorder;
    return m;
}

api::Result<WindowChrome> ChromePolicy::chromeFor(const api::WindowSnapshot& window) const {
    if (!window.ref.valid()) {
        return api::Result<WindowChrome>::Err(api::Error::InvalidArgument, "invalid window");
    }

    WindowChrome c;
    // Defaults from settings/panel snapshot — no X11.
    // Product rule: HideTitleBarWhenMaximized hides titlebar when maximized.
    // Respects fullscreen (no titlebar), panel top edge (strut already owned by panels),
    // and explicit settings overrides.
    c.hideTitleWhenMaximized = parseBoolSetting(impl_->settings, "HideTitleBarWhenMaximized", false);
    bool hideDecorWhenMaximized = parseBoolSetting(impl_->settings, "HideDecorWhenMaximized", false);

    // Fullscreen: no chrome.
    if (window.fullscreen) {
        c.decorated = false;
        c.titleBar = false;
        c.border = false;
        c.minimizeButton = false;
        c.maximizeButton = false;
        c.resizable = false;
        // close remains but frame won't show it
        return api::Result<WindowChrome>::Ok(c);
    }

    if (window.maximized) {
        if (c.hideTitleWhenMaximized || hideDecorWhenMaximized) {
            c.titleBar = false;
        }
        if (hideDecorWhenMaximized) {
            c.border = false;
            c.decorated = !c.titleBar && !c.border ? false : true;
        }
    }

    // Sticky/hidden do not affect chrome in this policy — IceWM owns visibility.

    // Button policy: always present unless explicitly hidden by maximized rule.
    // No X11 motif hint duplication — IceWM remains authority for actual hints.
    return api::Result<WindowChrome>::Ok(c);
}

api::Rect ChromePolicy::centeredTitleRect(api::Rect titleBar,
                                          int titleWidth,
                                          int buttonsLeftWidth,
                                          int buttonsRightWidth) const {
    if (titleBar.w <= 0 || titleBar.h <= 0) {
        return api::Rect(titleBar.x, titleBar.y, 0, titleBar.h);
    }
    if (buttonsLeftWidth < 0) buttonsLeftWidth = 0;
    if (buttonsRightWidth < 0) buttonsRightWidth = 0;
    if (titleWidth < 0) titleWidth = 0;

    int leftBound = titleBar.x + buttonsLeftWidth;
    int rightBound = titleBar.x + titleBar.w - buttonsRightWidth;
    int avail = rightBound - leftBound;
    if (avail <= 0) {
        return api::Rect(leftBound, titleBar.y, 0, titleBar.h);
    }
    if (titleWidth <= 0) {
        return api::Rect(leftBound, titleBar.y, 0, titleBar.h);
    }
    if (titleWidth >= avail) {
        return api::Rect(leftBound, titleBar.y, avail, titleBar.h);
    }
    int idealX = titleBar.x + (titleBar.w - titleWidth) / 2;
    if (idealX < leftBound) idealX = leftBound;
    if (idealX + titleWidth > rightBound) idealX = rightBound - titleWidth;
    return api::Rect(idealX, titleBar.y, titleWidth, titleBar.h);
}

std::string ChromePolicy::semanticAccent() const {
    // Settings override if present as #RRGGBB hex; fallback is Flame red constant.
    // Do not call FlamePalette::defaults() here — palette lives in ui/runtime;
    // platform must not depend on ui (would create circular/linkage failure).
    std::map<std::string, std::string>::const_iterator it = impl_->settings.values.find("AccentColor");
    if (it != impl_->settings.values.end() && !it->second.empty()) {
        const std::string& v = it->second;
        if (v.size() == 7 && v[0] == '#') {
            bool hex = true;
            for (size_t i = 1; i < 7; ++i) {
                char c = v[i];
                if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F'))) { hex = false; break; }
            }
            if (hex) return v;
        }
    }
    return std::string("#EF4048");
}

uint64_t ChromePolicy::revision() const {
    return impl_ ? impl_->rev : 0;
}

} // namespace chrome
} // namespace platform
} // namespace flamewm
