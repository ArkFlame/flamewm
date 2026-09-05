#include "flamewm/engine/icewm/snap_bridge.h"

namespace flamewm {
namespace engine {
namespace icewm {

SnapBridge::SnapBridge()
    : service_(nullptr), overlay_(), window_(), generation_(0), active_(false) {}

SnapBridge::~SnapBridge() {
    cancel();
}

SnapBridge& SnapBridge::instance() {
    static SnapBridge bridge;
    return bridge;
}

bool SnapBridge::begin(platform::snap::SnapService* service, void* display,
                       api::WindowRef window, api::Rect floatingGeometry) {
    cancel();
    if (!service || !window.valid() || !floatingGeometry.valid()) return false;
    if (!service->moveBegin(window, floatingGeometry).ok()) return false;
    service_ = service;
    window_ = window;
    generation_ = window.generation;
    active_ = true;
    overlay_.setDisplay(display);
    return true;
}

bool SnapBridge::motion(api::Point pointer, api::Rect outputRect) {
    if (!active_ || !service_ || window_.generation != generation_) return false;
    if (!service_->moveMotion(window_, pointer, outputRect).ok()) return false;
    if (!service_->hasPreview(window_)) {
        overlay_.hide();
        return true;
    }
    api::Result<api::Rect> geometry = service_->previewGeometryForWindow(
        window_, service_->previewTarget(window_));
    if (!geometry.ok()) return false;
    if (overlay_.isVisible()) overlay_.move(geometry.value());
    else if (!overlay_.show(geometry.value())) return false;
    return true;
}

void SnapBridge::setWorkspaceDwellCallback(
    const std::function<void(platform::snap::SnapTarget)>& callback) {
    if (service_)
        service_->setWorkspaceDwellCallback(callback);
}

api::Result<api::Rect> SnapBridge::dragAway() {
    if (!active_ || !service_ || window_.generation != generation_)
        return api::Result<api::Rect>::Err(api::Error::StaleRevision,
                                           "stale snap bridge");
    return service_->dragAway(window_);
}

api::Result<api::Rect> SnapBridge::end(bool cancelMove) {
    if (!active_ || !service_ || window_.generation != generation_) {
        return api::Result<api::Rect>::Err(api::Error::StaleRevision, "stale snap bridge");
    }
    api::Result<api::Rect> result;
    if (cancelMove) {
        api::Status status = service_->moveCancel(window_);
        result = status.ok() ? api::Result<api::Rect>::Err(api::Error::NotFound, "snap cancelled")
                             : api::Result<api::Rect>::Err(status);
    } else {
        result = service_->moveEnd(window_);
    }
    service_->setWorkspaceDwellCallback(
        std::function<void(platform::snap::SnapTarget)>());
    hidePreview();
    active_ = false;
    service_ = nullptr;
    window_ = api::WindowRef();
    generation_ = 0;
    return result;
}

void SnapBridge::cancel() {
    if (active_ && service_ && window_.generation == generation_) {
        service_->moveCancel(window_);
        service_->setWorkspaceDwellCallback(
            std::function<void(platform::snap::SnapTarget)>());
    }
    hidePreview();
    active_ = false;
    service_ = nullptr;
    window_ = api::WindowRef();
    generation_ = 0;
}

void SnapBridge::hidePreview() { overlay_.hide(); }
bool SnapBridge::active() const { return active_; }
bool SnapBridge::previewVisible() const { return overlay_.isVisible(); }
api::Rect SnapBridge::previewGeometry() const { return overlay_.geometry(); }
api::WindowRef SnapBridge::window() const { return window_; }

} // namespace icewm
} // namespace engine
} // namespace flamewm
