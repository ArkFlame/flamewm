#include "flamewm/engine/icewm/bridge.h"

namespace flamewm {
namespace engine {
namespace icewm {

struct Bridge::Impl {
    flamewm::platform::PlatformHost* host;
    flamewm::api::EnginePorts ports;
    bool attached;

    Impl() : host(nullptr), attached(false) {
        // Zero ports for safe detached state.
        ports.window = nullptr;
        ports.workspace = nullptr;
        ports.display = nullptr;
        ports.shortcut = nullptr;
        ports.workArea = nullptr;
        ports.mainLoop = nullptr;
        ports.input = nullptr;
        ports.application = nullptr;
        ports.session = nullptr;
        ports.background = nullptr;
        ports.tray = nullptr;
    }
};

Bridge::Bridge() : impl_(new Impl()) {}

Bridge::~Bridge() {
    delete impl_;
    impl_ = nullptr;
}

Bridge& Bridge::instance() {
    static Bridge inst;
    return inst;
}

bool Bridge::attach(flamewm::platform::PlatformHost* host,
                    flamewm::api::EnginePorts ports) {
#ifdef FLAMEWM_PRODUCT_BUILD
    if (impl_->attached)
        return false;
    if (host == nullptr)
        return false;
    impl_->host = host;
    impl_->ports = ports;
    impl_->attached = true;
    return true;
#else
    (void)host;
    (void)ports;
    return false;
#endif
}

void Bridge::detach() {
    impl_->attached = false;
    impl_->host = nullptr;
    impl_->ports.window = nullptr;
    impl_->ports.workspace = nullptr;
    impl_->ports.display = nullptr;
    impl_->ports.shortcut = nullptr;
    impl_->ports.workArea = nullptr;
    impl_->ports.mainLoop = nullptr;
    impl_->ports.input = nullptr;
    impl_->ports.application = nullptr;
    impl_->ports.session = nullptr;
    impl_->ports.background = nullptr;
    impl_->ports.tray = nullptr;
}

bool Bridge::isAttached() const {
    return impl_->attached && impl_->host != nullptr;
}

flamewm::platform::PlatformHost* Bridge::host() const {
    return isAttached() ? impl_->host : nullptr;
}

} // namespace icewm
} // namespace engine
} // namespace flamewm
