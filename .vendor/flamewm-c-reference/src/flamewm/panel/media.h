#ifndef FLAMEWM_PANEL_MEDIA_H
#define FLAMEWM_PANEL_MEDIA_H

#include "../integrations/mpris.h"
#include "../core/types.h"
#include "anchor.h"
#include <string>
#include <stdint.h>

namespace flamewm {
namespace panel {

// Media view model — slot collapses when no player (not permanently shown).
// Generation-validated callbacks; V4 glyph rule:
//   Playing -> Play glyph, Paused -> Pause glyph (state glyph)
class MediaView : public integrations::MprisListener {
public:
    explicit MediaView(integrations::Mpris* mpris);
    ~MediaView();

    struct State {
        bool visible; // false when no players -> collapses
        bool hasActive;
        std::string activeBusName;
        std::string identity;
        std::string title;
        std::string artist;
        std::string glyph; // "play" when Playing, "pause" when Paused (V4 contract)
        bool canPlay, canPause, canNext, canPrev;
        State():visible(false),hasActive(false),canPlay(false),canPause(false),canNext(false),canPrev(false){}
    };

    const State& state() const { return state_; }
    bool isVisible() const { return state_.visible; }
    uint64_t generation() const { return generation_; }

    PopoverPlacement popoverPlacement(const PanelRect& iconRect, int popoverW, int popoverH,
                                      PanelEdge edge, const WorkArea& wa, int gap) const;

    // Intents — generation guarded
    bool play();
    bool pause();
    bool playPause();
    bool next();
    bool prev();

private:
    void onPlayersChanged(const std::vector<integrations::PlayerInfo>& players,
                          const std::string& activeBusName,
                          uint64_t gen);
    void rebuild(const std::vector<integrations::PlayerInfo>& players, const std::string& active);

    integrations::Mpris* mpris_;
    State state_;
    uint64_t generation_;
};

} // namespace panel
} // namespace flamewm

#endif
