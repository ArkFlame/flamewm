#ifndef FLAMEWM_ENGINE_ICEWM_SNAP_BRIDGE_H
#define FLAMEWM_ENGINE_ICEWM_SNAP_BRIDGE_H

#include "flamewm/api/ports.h"
#include "flamewm/platform/snap/service.h"
#include "flamewm/engine/icewm/native_overlay.h"
#include <functional>

namespace flamewm {
namespace engine {
namespace icewm {

class SnapBridge {
public:
    SnapBridge();
    ~SnapBridge();

    bool begin(platform::snap::SnapService* service, void* display,
               api::WindowRef window, api::Rect floatingGeometry);
    bool motion(api::Point pointer, api::Rect outputRect);
    api::Result<api::Rect> dragAway();
    void setWorkspaceDwellCallback(
        const std::function<void(platform::snap::SnapTarget)>& callback);
    api::Result<api::Rect> end(bool cancel);
    void cancel();
    void hidePreview();

    bool active() const;
    bool previewVisible() const;
    api::Rect previewGeometry() const;
    api::WindowRef window() const;

    static SnapBridge& instance();

private:
    SnapBridge(const SnapBridge&);
    SnapBridge& operator=(const SnapBridge&);

    platform::snap::SnapService* service_;
    NativeOverlay overlay_;
    api::WindowRef window_;
    uint64_t generation_;
    bool active_;
};

} // namespace icewm
} // namespace engine
} // namespace flamewm

#endif
