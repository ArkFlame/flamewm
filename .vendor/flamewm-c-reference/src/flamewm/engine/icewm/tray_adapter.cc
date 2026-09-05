#include "flamewm/engine/icewm/tray_adapter.h"
#include "flamewm/engine/icewm/ui/panel_registry.h"

#if defined(__has_include)
#if __has_include(<X11/Xlib.h>)
#include <X11/Xlib.h>
#define FLAMEWM_HAS_X11 1
#else
#define FLAMEWM_HAS_X11 0
#endif
#else
#define FLAMEWM_HAS_X11 0
#endif

// Real YXTray/YWindow integration only under product build with X11.
// Header remains X11-free for syntax-only and Autotools -std=c++11 checks.
#if defined(FLAMEWM_WITH_X11) || defined(FLAMEWM_PRODUCT_BUILD)
#if FLAMEWM_HAS_X11
#if defined(__has_include)
#if __has_include("ywindow.h") && __has_include("yxtray.h")
#include "ywindow.h"
#include "yxtray.h"
#define FLAMEWM_HAS_YXTRAY 1
#else
#define FLAMEWM_HAS_YXTRAY 0
#endif
#else
#define FLAMEWM_HAS_YXTRAY 0
#endif
#else
#define FLAMEWM_HAS_YXTRAY 0
#endif
#else
#define FLAMEWM_HAS_YXTRAY 0
#endif
#ifndef FLAMEWM_HAS_YXTRAY
#define FLAMEWM_HAS_YXTRAY 0
#endif
#ifdef Status
#undef Status
#endif

namespace flamewm {
namespace engine {
namespace icewm {

TrayAdapter::TrayAdapter()
    : display_(nullptr), tray_(nullptr), registry_(nullptr), owner_(), hasOwner_(false) {}

TrayAdapter::TrayAdapter(void* display)
    : display_(display), tray_(nullptr), registry_(nullptr), owner_(), hasOwner_(false) {}

TrayAdapter::~TrayAdapter() {}

void TrayAdapter::setDisplay(void* display) {
    display_ = display;
}

void* TrayAdapter::display() const {
    return display_;
}

void TrayAdapter::setTray(void* tray) {
    tray_ = tray;
}

void* TrayAdapter::tray() const {
    return tray_;
}

void TrayAdapter::setRegistry(ui::PanelRegistry* registry) {
    registry_ = registry;
}

ui::PanelRegistry* TrayAdapter::registry() const {
    return registry_;
}

api::Status TrayAdapter::setOwner(api::OutputId output) {
    if (!output.valid()) {
        // Empty owner clears logical placement. Selection remains owned by
        // the single tray instance; no second selection or reparent occurs.
        owner_ = api::OutputId();
        hasOwner_ = false;
        return api::Status::Ok();
    }

    // Idempotent: same owner is a no-op — must not reparent or touch selection.
    if (hasOwner_ && owner_ == output) {
        return api::Status::Ok();
    }

    // Selection ownership (XEmbed _NET_SYSTEM_TRAY_S0) lives in the single
    // YXTrayProxy singleton. We never claim a second selection on secondary
    // panels; relocating the tray means reparenting the one native YXTray
    // presentation into the target panel's container. Resolve registry
    // (explicit or singleton) and reparent before publishing owner.
    ui::PanelRegistry* reg = registry_ ? registry_ : &ui::PanelRegistry::instance();

#if FLAMEWM_HAS_YXTRAY
    if (tray_ != nullptr) {
        YXTray* yTray = static_cast<YXTray*>(tray_);
        // Secondary panels must already be registered to know where to
        // reparent. If the requested output has no native panel surface yet,
        // fail without changing owner — caller retains previous selection
        // locality. Logically this is still the same single selection owner.
        void* targetPanel = reg ? reg->panelFor(output) : nullptr;
        if (targetPanel == nullptr) {
            // No presentation to reparent — keep prior owner, report NotFound.
            // Panels are registered by engine TaskBar creation; absence means
            // the output is not materialized.
            return api::Status::make(api::Error::NotFound, "no panel surface for output");
        }
        YWindow* target = static_cast<YWindow*>(targetPanel);
        // One XEmbed owner: ownership remains with the single YXTrayProxy.
        // Visual relocation is a pure reparent; do not call XSetSelectionOwner
        // again and do not construct a second YXTrayProxy.
        yTray->reparent(target, 0, 0);
        // Ensure geometry/relayout is driven by panel after reparent.
        // YXTray::relayout is called by TaskBar layout; a minimal trigger
        // here guarantees visibility without requiring an extra layout pass.
        // Parent change itself does not fire configure; show so parent can lay out.
        yTray->show();
    }
#endif
#if FLAMEWM_HAS_X11
    // When built with X11 but no tray wired, there is still exactly one
    // selection owner. Record logical owner only; no second selection claim.
    // This still satisfies "do not claim second selection" even in stub builds.
    (void)display_;
    (void)reg;
    owner_ = output;
    hasOwner_ = true;
    return api::Status::Ok();
#else
    (void)display_;
    (void)reg;
    owner_ = output;
    hasOwner_ = true;
    return api::Status::Ok();
#endif
}

api::Result<api::OutputId> TrayAdapter::owner() {
    if (!hasOwner_ || !owner_.valid()) {
        return api::Result<api::OutputId>::Err(
            api::Status::make(api::Error::Unavailable, "no tray owner"));
    }
    return api::Result<api::OutputId>::Ok(owner_);
}

} // namespace icewm
} // namespace engine
} // namespace flamewm
