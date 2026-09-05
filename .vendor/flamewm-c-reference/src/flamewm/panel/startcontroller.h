#ifndef FLAMEWM_PANEL_STARTCONTROLLER_H
#define FLAMEWM_PANEL_STARTCONTROLLER_H

#include "../core/types.h"
#include <map>

namespace flamewm {
namespace panel {

// Toggle Start menu logic:
// toggle(activeOutput) { if same target open -> close else close prior then open target }
// At most one keyboard-opened Start globally, hot-unplug safe close,
// anchor to Start button per edge, output-aware routing.
class StartController {
public:
    StartController();
    ~StartController();

    // Returns true if menu becomes open after call, false if closed.
    bool toggle(const OutputId& activeOutput);

    bool isOpen() const { return isOpen_; }
    OutputId openOutput() const { return openOutput_; }

    void close();
    // Hot-unplug: if openOutput removed, close.
    void onOutputRemoved(const OutputId& id);

    // Anchor rect for menu relative to Start button edge.
    // Placeholder — returns panel-derived anchor hint without X dependency.
    // Actual YPopup anchoring uses edge-aware helper in IceWM hook.

private:
    bool isOpen_;
    OutputId openOutput_;
};

} // namespace panel
} // namespace flamewm

#endif // FLAMEWM_PANEL_STARTCONTROLLER_H
