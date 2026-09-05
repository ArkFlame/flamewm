#ifndef FLAMEWM_SHELL_STATUS_AREA_H
#define FLAMEWM_SHELL_STATUS_AREA_H

#include "flamewm/api/ports.h"
#include "flamewm/ui/window.h"

#include <string>

namespace flamewm {
namespace platform { namespace system { class SystemService; struct SystemSnapshot; } }
namespace panel { class AudioView; class MediaView; class NetworkView; }
namespace ui { class Button; class Slider; class Label; class Popover; class Toggle; class List; }
namespace shell {

// StatusArea hosts audio/media/network affordances.
// Each control delegates to its popover; no engine types leak.
// Consumes SystemService real snapshots (network/media/pulse).
class StatusArea {
public:
    StatusArea();
    ~StatusArea();

    StatusArea(const StatusArea&) = delete;
    StatusArea& operator=(const StatusArea&) = delete;

    void setContainer(flamewm::ui::Window* c);
    void setSystemService(platform::system::SystemService* svc);
    void setSnapshot(const platform::system::SystemSnapshot& snap);
    void refreshFromService();

    void render();
    void invalidate();

    // Delegate entry points — popover anchoring handled via Popovers.
    void openAudio();
    void openMedia();
    void openNetwork();
    void closeAll();

    bool hasOpenPopover() const;

    // Slider callbacks for test/drag ownership
    void onAudioSliderChanged(int value);
    void onAudioSliderPress(int x, int y, int button);
    void onAudioSliderRelease(int x, int y, int button);
    void onAudioSliderMotion(int x, int y);

private:
    void ensureControls();
    void syncFromSnapshot();
    void clearControls();
    void closePopover(flamewm::ui::Popover* popover);
    void updatePopovers();
    int clampVolume(int v) const;

    flamewm::ui::Window* container_;
    platform::system::SystemService* system_;
    platform::system::SystemSnapshot* snapshot_;
    flamewm::ui::Button* audioButton_;
    flamewm::ui::Button* mediaButton_;
    flamewm::ui::Button* networkButton_;
    flamewm::ui::Slider* audioSlider_;
    flamewm::ui::Slider* popoverSlider_;
    flamewm::ui::Label* statusLabel_;
    flamewm::ui::Popover* audioPopover_;
    flamewm::ui::Popover* mediaPopover_;
    flamewm::ui::Popover* networkPopover_;
    flamewm::ui::Toggle* muteToggle_;
    flamewm::ui::Button* mediaPrevious_;
    flamewm::ui::Button* mediaPlayPause_;
    flamewm::ui::Button* mediaNext_;
    flamewm::ui::Button* networkDisconnect_;
    flamewm::ui::Button* networkScan_;
    flamewm::ui::Toggle* wifiToggle_;
    flamewm::ui::List* accessPoints_;
    flamewm::ui::Label* audioDetails_;
    flamewm::ui::Label* mediaDetails_;
    flamewm::ui::Label* networkDetails_;
    panel::AudioView* audioView_;
    panel::MediaView* mediaView_;
    panel::NetworkView* networkView_;
    bool popoverOpen_;
    bool audioDragging_;
    int audioVolume_;
    int systemListenerId_;
};

} // namespace shell
} // namespace flamewm

#endif // FLAMEWM_SHELL_STATUS_AREA_H
