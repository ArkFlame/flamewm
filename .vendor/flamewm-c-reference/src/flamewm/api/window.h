#ifndef FLAMEWM_API_WINDOW_H
#define FLAMEWM_API_WINDOW_H

#include "flamewm/api/ids.h"
#include "flamewm/api/geometry.h"

#include <cstdint>
#include <string>
#include <vector>

namespace flamewm {
namespace api {

enum class WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
    Hidden
};

struct WindowSnapshot {
    WindowRef ref;
    std::string title;
    std::string appId;
    Rect outerGeometry;
    Rect restoreGeometry;
    bool minimized;
    bool maximized;
    bool fullscreen;
    bool sticky;
    bool focused;
    WorkspaceRef workspace;
    OutputId output;
    uint64_t stateGeneration;

    WindowSnapshot()
        : minimized(false)
        , maximized(false)
        , fullscreen(false)
        , sticky(false)
        , focused(false)
        , stateGeneration(0)
    {}

    WindowState derivedState() const {
        if (fullscreen) return WindowState::Fullscreen;
        if (maximized) return WindowState::Maximized;
        if (minimized) return WindowState::Minimized;
        // Hidden is explicit via state, not inferred from geometry; caller maps if needed.
        return WindowState::Normal;
    }
};

// Optional helpers for filtering snapshots without exposing engine internals.

struct WindowFilter {
    bool includeMinimized;
    bool includeMaximized;
    bool includeFullscreen;
    bool includeSticky;
    bool includeHidden;

    WindowFilter()
        : includeMinimized(true)
        , includeMaximized(true)
        , includeFullscreen(true)
        , includeSticky(true)
        , includeHidden(false)
    {}

    bool matches(const WindowSnapshot& s) const {
        if (!includeMinimized && s.minimized) return false;
        if (!includeMaximized && s.maximized) return false;
        if (!includeFullscreen && s.fullscreen) return false;
        if (!includeSticky && s.sticky) return false;
        // Hidden is not a bool on snapshot; mapped via derived state if caller uses it.
        if (!includeHidden && s.derivedState() == WindowState::Hidden) return false;
        return true;
    }

    static WindowFilter all() {
        WindowFilter f;
        f.includeHidden = true;
        return f;
    }

    static WindowFilter visibleOnly() {
        WindowFilter f;
        f.includeMinimized = false;
        f.includeHidden = false;
        return f;
    }

    static WindowFilter onWorkspace(const WorkspaceRef& ws) {
        // Convenience marker; actual workspace comparison is done by caller
        // via predicate composition to keep this struct engine-free.
        (void)ws;
        return WindowFilter();
    }

    static WindowFilter onOutput(const OutputId& out) {
        (void)out;
        return WindowFilter();
    }
};

inline bool windowIsVisible(const WindowSnapshot& s) {
    return !s.minimized && s.derivedState() != WindowState::Hidden;
}

inline std::vector<WindowSnapshot> filterWindows(
    const std::vector<WindowSnapshot>& in,
    const WindowFilter& filter) {
    std::vector<WindowSnapshot> out;
    out.reserve(in.size());
    for (std::vector<WindowSnapshot>::const_iterator it = in.begin(); it != in.end(); ++it) {
        if (filter.matches(*it)) out.push_back(*it);
    }
    return out;
}

} // namespace api
} // namespace flamewm

#endif // FLAMEWM_API_WINDOW_H
