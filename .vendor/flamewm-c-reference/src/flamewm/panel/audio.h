#ifndef FLAMEWM_PANEL_AUDIO_H
#define FLAMEWM_PANEL_AUDIO_H

#include "../integrations/pulse.h"
#include "../core/types.h"
#include "anchor.h"
#include <string>
#include <stdint.h>

namespace flamewm {
namespace panel {

// Audio view model — graceful degrade when Pulse absent (hide/disable cleanly, no dead slot)
// Generation-validated callbacks, server generation guards.
class AudioView : public integrations::PulseListener {
public:
    explicit AudioView(integrations::PulseAudio* pulse);
    ~AudioView();

    struct State {
        bool visible; // false when Pulse unavailable and no fallback
        bool enabled;
        bool muted;
        int volume; // 0..100
        std::string sinkName;
        std::string iconRole; // "audio-volume-high/medium/low/muted"
        State():visible(false),enabled(false),muted(false),volume(0){}
    };

    const State& state() const { return state_; }
    bool isVisible() const { return state_.visible; }
    uint64_t generation() const { return generation_; }
    uint64_t serverGeneration() const { return serverGen_; }

    PopoverPlacement popoverPlacement(const PanelRect& iconRect, int popoverW, int popoverH,
                                      PanelEdge edge, const WorkArea& wa, int gap) const;

    bool setVolume(int pct);
    bool setMute(bool m);

    // Pointer gesture owns capture until release or backend invalidation.
    void beginPointerCapture();
    void releasePointerCapture();
    void cancelPointerCapture() { releasePointerCapture(); }
    // Map root pointer position to slider value; backend receives async intent.
    bool updatePointer(int pointerX, int trackX, int trackWidth);
    void reconcilePointerCapture();
    bool hasPointerCapture() const { return pointerCaptured_; }

private:
    void onPulseStatus(const integrations::PulseStatus& s, uint64_t gen);
    void rebuild(const integrations::PulseStatus& s);
    static std::string iconRoleFor(int vol, bool muted);

    integrations::PulseAudio* pulse_;
    State state_;
    uint64_t generation_;
    uint64_t serverGen_;
    bool pointerCaptured_;
    bool captureReleasePending_;
    bool xPointerGrabbed_;
};

} // namespace panel
} // namespace flamewm

#endif
