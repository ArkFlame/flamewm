#include "startcontroller.h"

namespace flamewm {
namespace panel {

StartController::StartController() : isOpen_(false) {}
StartController::~StartController() {}

bool StartController::toggle(const OutputId& activeOutput) {
    if (activeOutput.empty()) {
        // No valid target — close if open.
        if (isOpen_) { isOpen_ = false; openOutput_ = OutputId(); return false; }
        return false;
    }
    if (isOpen_ && openOutput_ == activeOutput) {
        // Same target -> close.
        isOpen_ = false;
        openOutput_ = OutputId();
        return false;
    }
    // Different target or closed -> close prior then open target (one globally).
    isOpen_ = true;
    openOutput_ = activeOutput;
    return true;
}

void StartController::close() {
    isOpen_ = false;
    openOutput_ = OutputId();
}

void StartController::onOutputRemoved(const OutputId& id) {
    if (isOpen_ && openOutput_ == id) close();
}

} // namespace panel
} // namespace flamewm
