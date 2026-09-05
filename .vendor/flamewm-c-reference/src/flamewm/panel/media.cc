#include "media.h"

namespace flamewm {
namespace panel {

MediaView::MediaView(integrations::Mpris* mpris)
    : mpris_(mpris), generation_(mpris ? mpris->generation() : 1)
{
    if (mpris_) mpris_->addListener(this);
    if (mpris_) {
        rebuild(mpris_->players(), mpris_->activePlayerBusName());
    }
}

MediaView::~MediaView() {
    if (mpris_) mpris_->removeListener(this);
}

PopoverPlacement MediaView::popoverPlacement(const PanelRect& iconRect, int popoverW, int popoverH,
                                             PanelEdge edge, const WorkArea& wa, int gap) const {
    return PopoverAnchor::anchor(iconRect, popoverW, popoverH, edge, wa, gap);
}

bool MediaView::play()      { return mpris_ && !mpris_->isStale(generation_) && mpris_->requestPlay(state_.activeBusName, generation_); }
bool MediaView::pause()     { return mpris_ && !mpris_->isStale(generation_) && mpris_->requestPause(state_.activeBusName, generation_); }
bool MediaView::playPause() { return mpris_ && !mpris_->isStale(generation_) && mpris_->requestPlayPause(state_.activeBusName, generation_); }
bool MediaView::next()      { return mpris_ && !mpris_->isStale(generation_) && mpris_->requestNext(state_.activeBusName, generation_); }
bool MediaView::prev()      { return mpris_ && !mpris_->isStale(generation_) && mpris_->requestPrev(state_.activeBusName, generation_); }

void MediaView::onPlayersChanged(const std::vector<integrations::PlayerInfo>& players,
                                 const std::string& activeBusName,
                                 uint64_t gen) {
    generation_ = gen;
    rebuild(players, activeBusName);
}

void MediaView::rebuild(const std::vector<integrations::PlayerInfo>& players, const std::string& active) {
    if (players.empty()) {
        state_.visible = false; // collapses when no players
        state_.hasActive = false;
        state_.activeBusName.clear();
        state_.identity.clear();
        state_.title.clear();
        state_.artist.clear();
        state_.glyph.clear();
        state_.canPlay = state_.canPause = state_.canNext = state_.canPrev = false;
        return;
    }
    state_.visible = true;
    state_.activeBusName = active;
    const integrations::PlayerInfo* ap = 0;
    for (size_t i = 0; i < players.size(); ++i) if (players[i].busName == active) { ap = &players[i]; break; }
    if (!ap) {
        state_.hasActive = false;
        state_.glyph.clear();
        return;
    }
    state_.hasActive = true;
    state_.identity = ap->identity;
    state_.title = ap->trackTitle;
    state_.artist = ap->artist;
    state_.canPlay = ap->canPlay;
    state_.canPause = ap->canPause;
    state_.canNext = ap->canNext;
    state_.canPrev = ap->canPrev;
    // V4 contract: Playing -> Play glyph, Paused -> Pause glyph
    if (ap->status == integrations::PlaybackPlaying) state_.glyph = "play";
    else if (ap->status == integrations::PlaybackPaused) state_.glyph = "pause";
    else state_.glyph.clear();
}

} // namespace panel
} // namespace flamewm
