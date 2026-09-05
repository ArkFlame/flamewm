#include "audio.h"
#include "../core/runtime.h"
#include "../../yxapp.h"

namespace flamewm {
namespace panel {

AudioView::AudioView(integrations::PulseAudio* pulse)
    : pulse_(pulse), generation_(pulse ? pulse->generation() : 1), serverGen_(pulse ? pulse->serverGeneration() : 1)
    , pointerCaptured_(false), captureReleasePending_(false), xPointerGrabbed_(false)
{
    if (pulse_) pulse_->addListener(this);
    if (pulse_) rebuild(pulse_->status());
}

AudioView::~AudioView() {
    reconcilePointerCapture();
    if (pulse_) pulse_->removeListener(this);
}

PopoverPlacement AudioView::popoverPlacement(const PanelRect& iconRect, int popoverW, int popoverH,
                                             PanelEdge edge, const WorkArea& wa, int gap) const {
    return PopoverAnchor::anchor(iconRect, popoverW, popoverH, edge, wa, gap);
}

bool AudioView::setVolume(int pct) {
    if (!pulse_) return false;
    if (pulse_->isStale(generation_)) return false;
    if (pulse_->isServerStale(serverGen_)) return false;
    return pulse_->setVolume(pct, generation_);
}
bool AudioView::setMute(bool m) {
    if (!pulse_) return false;
    if (pulse_->isStale(generation_)) return false;
    if (pulse_->isServerStale(serverGen_)) return false;
    return pulse_->setMute(m, generation_);
}

void AudioView::onPulseStatus(const integrations::PulseStatus& s, uint64_t gen) {
    bool invalidated = pulse_ && (gen != generation_ || pulse_->isServerStale(serverGen_));
    generation_ = gen;
    if (pulse_) serverGen_ = pulse_->serverGeneration();
    rebuild(s);
    if (invalidated) reconcilePointerCapture();
}

void AudioView::beginPointerCapture() {
    if (pointerCaptured_) return;
    if (xapp && xapp->display()) {
        int result = XGrabPointer(xapp->display(), xapp->root(), True,
            ButtonPressMask | ButtonReleaseMask | PointerMotionMask,
            GrabModeAsync, GrabModeAsync, None, None, CurrentTime);
        if (result != GrabSuccess) return;
        xPointerGrabbed_ = true;
    }
    pointerCaptured_ = true;
    captureReleasePending_ = false;
}

void AudioView::releasePointerCapture() {
    if (!pointerCaptured_ || captureReleasePending_) return;
    captureReleasePending_ = true;
    pointerCaptured_ = false;
    if (xPointerGrabbed_ && xapp && xapp->display()) {
        XUngrabPointer(xapp->display(), CurrentTime);
        xPointerGrabbed_ = false;
    }
}

void AudioView::reconcilePointerCapture() {
    if (!pointerCaptured_ && !captureReleasePending_) return;
    releasePointerCapture();
    captureReleasePending_ = false;
}

bool AudioView::updatePointer(int pointerX, int trackX, int trackWidth) {
    if (!pointerCaptured_ || trackWidth <= 0) return false;
    int value = (pointerX - trackX) * 100 / trackWidth;
    if (value < 0) value = 0;
    if (value > 100) value = 100;
    return setVolume(value);
}

void AudioView::rebuild(const integrations::PulseStatus& s) {
    if (!s.available || s.state != integrations::PulseReady) {
        // hide/disable cleanly without dead slot when backend unavailable
        state_.visible = false;
        state_.enabled = false;
        state_.muted = s.muted;
        state_.volume = s.volumePercent;
        state_.sinkName = s.sinkName;
        state_.iconRole = iconRoleFor(s.volumePercent, s.muted);
        return;
    }
    state_.visible = true;
    state_.enabled = true;
    state_.muted = s.muted;
    state_.volume = s.volumePercent;
    state_.sinkName = s.sinkName;
    state_.iconRole = iconRoleFor(s.volumePercent, s.muted);
}

std::string AudioView::iconRoleFor(int vol, bool muted) {
    if (muted || vol == 0) return "audio-volume-muted";
    if (vol < 33) return "audio-volume-low";
    if (vol < 66) return "audio-volume-medium";
    return "audio-volume-high";
}

} // namespace panel
} // namespace flamewm
