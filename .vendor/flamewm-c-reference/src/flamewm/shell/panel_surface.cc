#include "flamewm/shell/panel_surface.h"
#include "flamewm/ui/backend.h"
#include "flamewm/ui/shellstyle.h"
#include "flamewm/engine/icewm/ui/panel_registry.h"

#include <algorithm>

namespace {

std::vector<int> solveSizes(int axis, const std::vector<int>& preferred,
                            const std::vector<int>& minimum) {
    std::vector<int> sizes = preferred;
    int total = 0;
    for (size_t i = 0; i < sizes.size(); ++i) {
        sizes[i] = std::max(0, sizes[i]);
        total += sizes[i];
    }

    if (total > axis) {
        int excess = total - axis;
        while (excess > 0) {
            bool reduced = false;
            for (size_t i = 0; i < sizes.size() && excess > 0; ++i) {
                const int floor = i < minimum.size() ? std::max(0, minimum[i]) : 0;
                if (sizes[i] > floor) {
                    --sizes[i];
                    --excess;
                    reduced = true;
                }
            }
            if (!reduced) break;
        }
        // Minimum content footprints can exceed a very narrow output. Keep
        // slots disjoint rather than allowing the last group to overlap.
        while (excess > 0) {
            bool reduced = false;
            for (size_t i = 0; i < sizes.size() && excess > 0; ++i) {
                if (sizes[i] > 0) {
                    --sizes[i];
                    --excess;
                    reduced = true;
                }
            }
            if (!reduced) break;
        }
    } else if (total < axis) {
        int extra = axis - total;
        // Tasks and spacer are the content-bearing flexible slots.
        const size_t flexible[] = {1, 2};
        while (extra > 0) {
            bool added = false;
            for (size_t i = 0; i < sizeof(flexible) / sizeof(flexible[0]) && extra > 0; ++i) {
                if (flexible[i] < sizes.size()) {
                    ++sizes[flexible[i]];
                    --extra;
                    added = true;
                }
            }
            if (!added) break;
        }
    }
    return sizes;
}

}

namespace flamewm {
namespace shell {

PanelSurface::PanelSurface(const api::OutputId& output, api::PanelEdge edge, const api::Rect& g)
    : output_(output)
    , edge_(edge)
    , outputGeometry_(g)
    , geometry_()
    , visible_(false)
    , thickness_(ShellStyle::defaults().panelHeight)
    , requestingStrut_(false)
    , window_(0)
    , registry_(&engine::icewm::ui::PanelRegistry::instance()) {
    for (int i = 0; i < 7; ++i) containers_[i] = 0;
    geometry_ = computePanelRect();
}

PanelSurface::~PanelSurface() {
    for (int i = 6; i >= 0; --i) {
        delete containers_[i];
        containers_[i] = 0;
    }
    if (registry_ && window_) registry_->unregisterPanel(output_, window_);
    if (window_) {
        delete window_;
        window_ = 0;
    }
}

const api::OutputId& PanelSurface::output() const { return output_; }
api::PanelEdge PanelSurface::edge() const { return edge_; }
void PanelSurface::setEdge(api::PanelEdge e) {
    if (edge_ == e) return;
    edge_ = e;
    geometry_ = computePanelRect();
    applyGeometryToNative();
}

api::Rect PanelSurface::outputGeometry() const { return outputGeometry_; }
void PanelSurface::setOutputGeometry(const api::Rect& r) {
    if (outputGeometry_ == r) return;
    outputGeometry_ = r;
    geometry_ = computePanelRect();
    applyGeometryToNative();
}

api::Rect PanelSurface::geometry() const { return geometry_; }
void PanelSurface::setGeometry(const api::Rect& r) {
    geometry_ = r;
    applyGeometryToNative();
}

int PanelSurface::thickness() const { return thickness_; }
bool PanelSurface::setThickness(int thickness) {
    if (thickness < 34 || thickness > 72) return false;
    if (thickness_ == thickness) return true;
    thickness_ = thickness;
    geometry_ = computePanelRect();
    applyGeometryToNative();
    return true;
}
bool PanelSurface::isHorizontal() const { return isHorizontalEdge(edge_); }
bool PanelSurface::isVertical() const { return isVerticalEdge(edge_); }

bool PanelSurface::visible() const { return visible_; }

void PanelSurface::show() {
    visible_ = true;
    ensureNative();
    applyGeometryToNative();
    if (window_) {
        window_->show();
        window_->repaint();
    }
    for (int i = 0; i < 7; ++i) {
        if (containers_[i]) {
            containers_[i]->show();
            containers_[i]->repaint();
        }
    }
}

void PanelSurface::hide() {
    visible_ = false;
    if (window_) window_->hide();
    for (int i = 0; i < 7; ++i)
        if (containers_[i]) containers_[i]->hide();
}

ui::Window* PanelSurface::nativeWindow() const { return window_; }
ui::Window* PanelSurface::panelRoot() const { return window_; }

ui::Window* PanelSurface::container(PanelSlot slot) const {
    return containers_[static_cast<int>(slot)];
}

std::vector<ui::Window*> PanelSurface::containers() const {
    std::vector<ui::Window*> result;
    result.reserve(7);
    for (int i = 0; i < 7; ++i) result.push_back(containers_[i]);
    return result;
}

void PanelSurface::configure() {
    geometry_ = computePanelRect();
    ensureNative();
    if (visible_) show();
    else {
        applyGeometryToNative();
        hide();
    }
}

api::Rect PanelSurface::strutRect() const {
    if (!visible_) return api::Rect(0, 0, 0, 0);
    if (!geometry_.valid()) return api::Rect(0, 0, 0, 0);
    return geometry_;
}

void PanelSurface::requestStrut(api::WorkAreaPort* workArea) {
    // PanelService is sole authority for reservation side effects.
    (void)workArea;
}

bool PanelSurface::isHorizontalEdge(api::PanelEdge e) {
    return e == api::PanelEdge::Bottom || e == api::PanelEdge::Top;
}
bool PanelSurface::isVerticalEdge(api::PanelEdge e) {
    return e == api::PanelEdge::Left || e == api::PanelEdge::Right;
}

api::Rect PanelSurface::computePanelRect() const {
    if (!outputGeometry_.valid()) return api::Rect();
    int t = thickness_;
    switch (edge_) {
        case api::PanelEdge::Bottom:
            return api::Rect(outputGeometry_.x, outputGeometry_.y + outputGeometry_.h - t,
                             outputGeometry_.w, t);
        case api::PanelEdge::Top:
            return api::Rect(outputGeometry_.x, outputGeometry_.y,
                             outputGeometry_.w, t);
        case api::PanelEdge::Left:
            return api::Rect(outputGeometry_.x, outputGeometry_.y,
                             t, outputGeometry_.h);
        case api::PanelEdge::Right:
            return api::Rect(outputGeometry_.x + outputGeometry_.w - t, outputGeometry_.y,
                             t, outputGeometry_.h);
        default:
            return api::Rect(outputGeometry_.x, outputGeometry_.y + outputGeometry_.h - t,
                             outputGeometry_.w, t);
    }
}

void PanelSurface::ensureNative() {
    if (window_) return;
#if defined(FLAMEWM_PRODUCT_BUILD)
    // Product build: prefer UiBackend when initialized, otherwise fallback.
    // YWindow direct path is guarded to keep header X11-free; UiBackend
    // forwards to IceWM backend when available.
    if (ui::UiBackend::isAvailable()) {
        window_ = ui::UiBackend::createWindow();
        if (window_) {
            window_->setRole(ui::WindowRole::PanelDock);
            window_->setVisual(ui::style::Visual(ui::style::VisualRolePanel,
                                                  ui::style::VisualNormal));
        }
        if (registry_ && window_) registry_->registerPanel(output_, window_);
        ensureContainers();
        return;
    }
    window_ = ui::UiBackend::createWindow();
    if (window_) {
        window_->setRole(ui::WindowRole::PanelDock);
        window_->setVisual(ui::style::Visual(ui::style::VisualRolePanel,
                                              ui::style::VisualNormal));
        if (registry_) registry_->registerPanel(output_, window_);
        ensureContainers();
        return;
    }
#if defined(__has_include)
#if __has_include("ywindow.h")
    // Last resort direct YWindow would be instantiated here if UiBackend
    // unavailable; kept behind include guard to preserve syntax-only builds.
#endif
#endif
#else
    window_ = ui::UiBackend::createWindow();
    if (window_) {
        window_->setRole(ui::WindowRole::PanelDock);
        window_->setVisual(ui::style::Visual(ui::style::VisualRolePanel,
                                              ui::style::VisualNormal));
    }
    if (registry_ && window_) registry_->registerPanel(output_, window_);
#endif
    ensureContainers();
}

void PanelSurface::ensureContainers() {
    if (!window_) return;
    for (int i = 0; i < 7; ++i) {
        if (!containers_[i]) containers_[i] = ui::UiBackend::createWindow(window_);
        if (containers_[i]) {
            containers_[i]->setRole(ui::WindowRole::GenericChild);
            containers_[i]->setVisual(ui::style::Visual(ui::style::VisualRoleSurface,
                                                         ui::style::VisualNormal));
        }
    }
}

void PanelSurface::applyGeometryToNative() {
    if (!window_) return;
    if (!geometry_.valid()) return;
    window_->setGeometry(geometry_);
    applyContainerGeometry();
    window_->repaint();
}

void PanelSurface::applyContainerGeometry() {
    if (!geometry_.valid()) return;

    const ShellStyle style = ShellStyle::defaults();
    const int axis = isHorizontal() ? geometry_.w : geometry_.h;
    const int cross = isHorizontal() ? geometry_.h : geometry_.w;

    std::vector<int> preferred(7, 0);
    std::vector<int> minimum(7, 0);
    preferred[0] = style.startButtonWidth;
    preferred[1] = containers_[1] ? (isHorizontal() ? containers_[1]->geometry().w
                                                     : containers_[1]->geometry().h)
                                  : 0;
    preferred[2] = 12;
    preferred[3] = containers_[3] ? (isHorizontal() ? containers_[3]->geometry().w
                                                     : containers_[3]->geometry().h)
                                  : style.workspaceButtonWidth * 2;
    preferred[4] = containers_[4] ? (isHorizontal() ? containers_[4]->geometry().w
                                                     : containers_[4]->geometry().h)
                                  : style.trayButtonWidth * 3;
    preferred[5] = containers_[5] ? (isHorizontal() ? containers_[5]->geometry().w
                                                     : containers_[5]->geometry().h)
                                  : style.trayButtonWidth;
    preferred[6] = containers_[6] ? (isHorizontal() ? containers_[6]->geometry().w
                                                     : containers_[6]->geometry().h)
                                  : style.clockMinWidth;
    minimum[0] = style.startButtonWidth;
    minimum[1] = style.taskButtonWidth;
    minimum[3] = style.workspaceButtonWidth;
    minimum[4] = style.trayButtonWidth;
    minimum[5] = style.trayButtonWidth;
    minimum[6] = style.clockMinWidth;
    const std::vector<int> sizes = solveSlotSizes(axis, preferred, minimum);

    int cursor = 0;
    api::Rect slots[7];
    for (int i = 0; i < 4; ++i) {
        if (isHorizontal()) slots[i] = api::Rect(cursor, 0, sizes[i], cross);
        else slots[i] = api::Rect(0, cursor, cross, sizes[i]);
        cursor += sizes[i];
    }

    cursor = sizes[0] + sizes[1] + sizes[2] + sizes[3];
    for (int slot = 4; slot < 7; ++slot) {
        if (isHorizontal()) slots[slot] = api::Rect(cursor, 0, sizes[slot], cross);
        else slots[slot] = api::Rect(0, cursor, cross, sizes[slot]);
        cursor += sizes[slot];
    }

    for (int i = 0; i < 7; ++i) {
        if (containers_[i]) {
            containers_[i]->setGeometry(slots[i]);
            containers_[i]->repaint();
        }
    }
}

std::vector<int> PanelSurface::solveSlotSizes(int axis, const std::vector<int>& preferred,
                                              const std::vector<int>& minimum) {
    return solveSizes(axis, preferred, minimum);
}

} // namespace shell
} // namespace flamewm
