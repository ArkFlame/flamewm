#ifndef FLAMEWM_SHELL_PANEL_SURFACE_H
#define FLAMEWM_SHELL_PANEL_SURFACE_H

#include "flamewm/api/geometry.h"
#include "flamewm/api/ids.h"
#include "flamewm/api/panels.h"
#include "flamewm/api/ports.h"
#include "flamewm/ui/window.h"

#include <vector>

namespace flamewm { namespace engine { namespace icewm { namespace ui { class PanelRegistry; } } } }

// Engine adaptation (IceWM WorkArea/Display) is intentionally guarded so
// syntax-only builds with -I src succeed without pulling engine headers.
#if defined(__has_include)
#if __has_include("flamewm/engine/icewm/workarea_adapter.h")
#include "flamewm/engine/icewm/workarea_adapter.h"
#endif
#endif

// UiBackend forward for window creation; header stays X11-free.
namespace flamewm { namespace ui { class Window; } }

namespace flamewm {
namespace shell {

// PanelSurface owns one native panel window per output.
// Native window is a lightweight ui::Window created via UiBackend when
// available, otherwise via YWindow when FLAMEWM_PRODUCT_BUILD is defined.
// This header remains api/ui only so `g++ -I src -fsyntax-only` succeeds
// without X11 dev headers.
class PanelSurface {
public:
    enum class PanelSlot {
        Start,
        Tasks,
        FlexibleSpacer,
        WorkspacePager,
        MediaAudioNetwork,
        Status = MediaAudioNetwork,
        Tray,
        Clock
    };

    PanelSurface(const api::OutputId& output, api::PanelEdge edge, const api::Rect& outputGeometry);
    ~PanelSurface();

    // Non-copyable (owns native window).
    PanelSurface(const PanelSurface&) = delete;
    PanelSurface& operator=(const PanelSurface&) = delete;

    const api::OutputId& output() const;
    api::PanelEdge edge() const;
    void setEdge(api::PanelEdge edge);

    api::Rect outputGeometry() const;
    void setOutputGeometry(const api::Rect& rect);

    api::Rect geometry() const;
    void setGeometry(const api::Rect& rect);

    int thickness() const;
    // Sets logical panel thickness. Product taskbar settings allow 34..72.
    bool setThickness(int thickness);
    bool isHorizontal() const;
    bool isVertical() const;

    bool visible() const;
    void show();
    void hide();

    // Native window accessor (owned, may be null before configure).
    ui::Window* nativeWindow() const;
    ui::Window* panelRoot() const;
    ui::Window* container(PanelSlot slot) const;
    std::vector<ui::Window*> containers() const;

    // Configure native window (position/size) from outputGeometry_+edge+thickness.
    void configure();

    // Strut reservation via WorkAreaPort (IceWM remains authority).
    // Returns panel geometry when visible, empty otherwise.
    api::Rect strutRect() const;
    void requestStrut(api::WorkAreaPort* workArea);

    // Orientation helpers.
    static bool isHorizontalEdge(api::PanelEdge e);
    static bool isVerticalEdge(api::PanelEdge e);

private:
    api::Rect computePanelRect() const;
    void ensureNative();
    void ensureContainers();
    void applyGeometryToNative();
    void applyContainerGeometry();
    static std::vector<int> solveSlotSizes(int axis, const std::vector<int>& preferred,
                                           const std::vector<int>& minimum);

    api::OutputId output_;
    api::PanelEdge edge_;
    api::Rect outputGeometry_;
    api::Rect geometry_;
    bool visible_;
    int thickness_;
    bool requestingStrut_;
    ui::Window* window_;
    engine::icewm::ui::PanelRegistry* registry_;
    ui::Window* containers_[7];
};

} // namespace shell
} // namespace flamewm

#endif // FLAMEWM_SHELL_PANEL_SURFACE_H
