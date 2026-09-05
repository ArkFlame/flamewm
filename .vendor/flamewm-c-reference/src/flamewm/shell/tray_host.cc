#include "flamewm/shell/tray_host.h"

namespace flamewm {
namespace shell {

TrayHost::TrayHost(api::TrayPort* t)
    : tray_(t), container_(nullptr), owner_(), hasOwner_(false) {}

TrayHost::~TrayHost() {}

void TrayHost::setTrayPort(api::TrayPort* t) {
    tray_ = t;
    owner_ = api::OutputId();
    hasOwner_ = false;
    if (tray_) {
        api::Result<api::OutputId> current = tray_->owner();
        if (current.ok()) {
            owner_ = current.value();
            hasOwner_ = owner_.valid();
        } else {
            owner_ = api::OutputId();
            hasOwner_ = false;
        }
    }
}

api::TrayPort* TrayHost::trayPort() const {
    return tray_;
}

void TrayHost::setContainer(flamewm::ui::Window* c) {
    container_ = c;
    if (container_) {
        container_->repaint();
    }
}

flamewm::ui::Window* TrayHost::container() const {
    return container_;
}

api::OutputId TrayHost::owner() const {
    return owner_;
}

bool TrayHost::hasOwner() const {
    return hasOwner_ && owner_.valid();
}

api::Status TrayHost::setOwner(const api::OutputId& o) {
    if (hasOwner_ && owner_ == o) {
        return api::Status::Ok();
    }
    if (tray_) {
        api::Status result = tray_->setOwner(o);
        if (!result.ok()) return result;
    }
    owner_ = o;
    hasOwner_ = o.valid();
    if (container_) {
        container_->repaint();
    }
    return api::Status::Ok();
}

void TrayHost::adopt(api::OutputId o) {
    // PanelService owns the native tray move; this only mirrors presentation state.
    setOwner(o);
}

void TrayHost::render() {
    // TrayAdapter is presentation authority; local state only mirrors it.
    if (tray_) {
        api::Result<api::OutputId> current = tray_->owner();
        if (current.ok()) {
            owner_ = current.value();
            hasOwner_ = owner_.valid();
        } else {
            owner_ = api::OutputId();
            hasOwner_ = false;
        }
    }
    if (container_) {
        container_->repaint();
    }
}

void TrayHost::invalidate() {
    if (container_) {
        container_->repaint();
    }
}

} // namespace shell
} // namespace flamewm
