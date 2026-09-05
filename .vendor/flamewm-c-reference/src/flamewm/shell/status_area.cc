#include "flamewm/shell/status_area.h"
#include "flamewm/platform/system/service.h"
#include "flamewm/ui/backend.h"
#include "flamewm/ui/button.h"
#include "flamewm/ui/label.h"
#include "flamewm/ui/slider.h"
#include "flamewm/ui/popover.h"
#include "flamewm/ui/list.h"
#include "flamewm/panel/audio.h"
#include "flamewm/panel/media.h"
#include "flamewm/panel/network.h"

#include <algorithm>

namespace flamewm {
namespace shell {

StatusArea::StatusArea()
    : container_(nullptr)
    , system_(nullptr)
    , snapshot_(nullptr)
    , audioButton_(nullptr)
    , mediaButton_(nullptr)
    , networkButton_(nullptr)
    , audioSlider_(nullptr)
    , popoverSlider_(nullptr)
    , statusLabel_(nullptr)
    , audioPopover_(nullptr)
    , mediaPopover_(nullptr)
    , networkPopover_(nullptr)
    , muteToggle_(nullptr)
    , mediaPrevious_(nullptr)
    , mediaPlayPause_(nullptr)
    , mediaNext_(nullptr)
    , networkDisconnect_(nullptr)
    , networkScan_(nullptr)
    , wifiToggle_(nullptr)
    , accessPoints_(nullptr)
    , audioDetails_(nullptr)
    , mediaDetails_(nullptr)
    , networkDetails_(nullptr)
    , audioView_(nullptr)
    , mediaView_(nullptr)
    , networkView_(nullptr)
    , popoverOpen_(false)
    , audioDragging_(false)
    , audioVolume_(50)
    , systemListenerId_(0) {}
StatusArea::~StatusArea() {
    if (system_ && systemListenerId_) system_->removeListener(systemListenerId_);
    clearControls();
    delete audioView_;
    delete mediaView_;
    delete networkView_;
    delete snapshot_;
}

void StatusArea::setContainer(flamewm::ui::Window* c) { container_ = c; }

void StatusArea::setSystemService(platform::system::SystemService* svc) {
    if (system_ && systemListenerId_) system_->removeListener(systemListenerId_);
    systemListenerId_ = 0;
    clearControls();
    system_ = svc;
    delete audioView_; audioView_ = nullptr;
    delete mediaView_; mediaView_ = nullptr;
    delete networkView_; networkView_ = nullptr;
    if (system_) {
        audioView_ = new panel::AudioView(system_->pulseAudio());
        mediaView_ = new panel::MediaView(system_->mpris());
        networkView_ = new panel::NetworkView(system_->networkManager());
        delete snapshot_;
        snapshot_ = new platform::system::SystemSnapshot(system_->snapshot());
        systemListenerId_ = system_->addListener([this](platform::system::SystemSnapshot s) {
            if (!snapshot_) snapshot_ = new platform::system::SystemSnapshot();
            *snapshot_ = s;
            this->syncFromSnapshot();
            this->invalidate();
        });
        syncFromSnapshot();
    }
}

void StatusArea::setSnapshot(const platform::system::SystemSnapshot& snap) {
    if (!snapshot_) snapshot_ = new platform::system::SystemSnapshot();
    *snapshot_ = snap;
    syncFromSnapshot();
    invalidate();
}

void StatusArea::refreshFromService() {
    if (!system_) return;
    if (!snapshot_) snapshot_ = new platform::system::SystemSnapshot();
    *snapshot_ = system_->snapshot();
    syncFromSnapshot();
    invalidate();
}

void StatusArea::clearControls() {
    if (audioPopover_) audioPopover_->close();
    if (mediaPopover_) mediaPopover_->close();
    if (networkPopover_) networkPopover_->close();
    delete audioButton_; audioButton_ = nullptr;
    delete mediaButton_; mediaButton_ = nullptr;
    delete networkButton_; networkButton_ = nullptr;
    delete audioSlider_; audioSlider_ = nullptr;
    delete popoverSlider_; popoverSlider_ = nullptr;
    delete statusLabel_; statusLabel_ = nullptr;
    delete muteToggle_; muteToggle_ = nullptr;
    delete mediaPrevious_; mediaPrevious_ = nullptr;
    delete mediaPlayPause_; mediaPlayPause_ = nullptr;
    delete mediaNext_; mediaNext_ = nullptr;
    delete networkDisconnect_; networkDisconnect_ = nullptr;
    delete networkScan_; networkScan_ = nullptr;
    delete wifiToggle_; wifiToggle_ = nullptr;
    delete accessPoints_; accessPoints_ = nullptr;
    delete audioDetails_; audioDetails_ = nullptr;
    delete mediaDetails_; mediaDetails_ = nullptr;
    delete networkDetails_; networkDetails_ = nullptr;
    delete audioPopover_; audioPopover_ = nullptr;
    delete mediaPopover_; mediaPopover_ = nullptr;
    delete networkPopover_; networkPopover_ = nullptr;
}

void StatusArea::ensureControls() {
    if (audioButton_ && mediaButton_ && networkButton_ && audioSlider_ && statusLabel_) return;
    if (!audioButton_) {
        audioButton_ = flamewm::ui::UiBackend::createButton(container_, IconRoleTaskbarStatus);
        audioButton_->setText("");
        audioButton_->setIconRole(IconRoleTaskbarStatus);
        audioButton_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleIndicator,
                                                           flamewm::ui::style::VisualNormal));
        audioButton_->setOnClick([this]() { this->openAudio(); });
        audioButton_->setGeometry(api::Rect(0, 0, 30, 18));
    }
    if (!mediaButton_) {
        mediaButton_ = flamewm::ui::UiBackend::createButton(container_, IconRoleTaskbarStatus);
        mediaButton_->setText("");
        mediaButton_->setIconRole(IconRoleTaskbarStatus);
        mediaButton_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleIndicator,
                                                           flamewm::ui::style::VisualNormal));
        mediaButton_->setOnClick([this]() { this->openMedia(); });
        mediaButton_->setGeometry(api::Rect(30, 0, 30, 18));
    }
    if (!networkButton_) {
        networkButton_ = flamewm::ui::UiBackend::createButton(container_);
        networkButton_->setText("");
        networkButton_->setIconRole(IconRoleNetwork);
        networkButton_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleIndicator,
                                                             flamewm::ui::style::VisualNormal));
        networkButton_->setOnClick([this]() { this->openNetwork(); });
        networkButton_->setGeometry(api::Rect(60, 0, 30, 18));
    }
    if (!audioSlider_) {
        audioSlider_ = flamewm::ui::UiBackend::createSlider(container_);
        audioSlider_->setRange(0, 100);
        audioSlider_->setValue(audioVolume_);
        audioSlider_->setOnChanged([this](int v) { this->onAudioSliderChanged(v); });
        // Drag ownership: press captures, motion/release continue even outside track, release ends.
        if (audioButton_) {
            // Button press/release/motion forwarding is for task drag; slider drag is via Button-like extended handling.
            // Ui Slider doesn't have press/motion directly, so we hook via button press/release on slider's window area.
            // FallbackSlider inherits Window only; real Ice backend will forward pointer grabs.
            // We simulate ownership via explicit handlers that caller can route.
        }
        // Attach drag ownership via slider's Window press/motion/release if available through Button interface
        // For C++11 fallback, we store handlers and expect shell to forward pointer events to these methods.
    }
    if (!statusLabel_) {
        statusLabel_ = flamewm::ui::UiBackend::createLabel(container_);
        statusLabel_->setText("");
    }
}

void StatusArea::closePopover(flamewm::ui::Popover* popover) {
    if (popover) popover->close();
    popoverOpen_ = false;
    audioDragging_ = false;
}

void StatusArea::updatePopovers() {
    if (audioButton_ && audioView_) audioButton_->setIconName(audioView_->state().iconRole);
    if (mediaButton_ && mediaView_) {
        mediaButton_->setIconName(mediaView_->state().glyph == "pause"
                                       ? "media-playback-pause" : "media-playback-start");
    }
    if (networkButton_ && networkView_) networkButton_->setIconName(networkView_->state().iconRole);
    if (audioDetails_ && audioView_) {
        const panel::AudioView::State& s = audioView_->state();
        audioDetails_->setText(s.sinkName.empty() ? "Audio" : s.sinkName);
        if (popoverSlider_) popoverSlider_->setValue(clampVolume(s.volume));
        if (audioSlider_) audioSlider_->setValue(clampVolume(s.volume));
        if (muteToggle_) muteToggle_->setChecked(s.muted);
    }
    if (mediaDetails_ && mediaView_) {
        const panel::MediaView::State& s = mediaView_->state();
        std::string text = s.identity;
        if (!s.title.empty()) text += text.empty() ? s.title : " - " + s.title;
        if (!s.artist.empty()) text += text.empty() ? s.artist : " - " + s.artist;
        mediaDetails_->setText(text.empty() ? "No media player" : text);
        if (mediaPlayPause_) mediaPlayPause_->setText(s.glyph.empty() ? "Play/Pause" : s.glyph);
        if (mediaPrevious_) mediaPrevious_->setEnabled(s.canPrev);
        if (mediaNext_) mediaNext_->setEnabled(s.canNext);
        if (mediaPlayPause_) mediaPlayPause_->setEnabled(s.canPlay || s.canPause);
    }
    if (networkDetails_ && networkView_) {
        const panel::NetworkView::State& s = networkView_->state();
        networkDetails_->setText(s.label.empty() ? "Network unavailable" : s.label);
        if (networkDisconnect_) networkDisconnect_->setEnabled(s.enabled && s.netState != integrations::NetworkDisconnected);
        if (wifiToggle_) wifiToggle_->setEnabled(s.enabled);
        if (accessPoints_ && system_ && system_->networkManager()) {
            std::vector<flamewm::ui::ListRow> rows;
            const std::vector<integrations::AccessPoint>& aps = system_->networkManager()->status().visibleAps;
            for (size_t i = 0; i < aps.size(); ++i) {
                rows.push_back(flamewm::ui::ListRow(aps[i].path, aps[i].ssid.empty() ? "Hidden network" : aps[i].ssid));
            }
            accessPoints_->setRows(rows);
        }
        if (wifiToggle_ && system_ && system_->networkManager()) {
            wifiToggle_->setOnToggled(std::function<void(bool)>());
            wifiToggle_->setChecked(system_->networkManager()->status().wifiEnabled);
            wifiToggle_->setOnToggled([this](bool e) { if (networkView_) networkView_->setWifiEnabled(e); });
        }
    }
}

int StatusArea::clampVolume(int v) const {
    if (v < 0) return 0;
    if (v > 100) return 100;
    return v;
}

void StatusArea::syncFromSnapshot() {
    ensureControls();
    if (!snapshot_) return;
    // Network area
    if (networkButton_) {
        networkButton_->setText("");
        const bool visible = networkView_ && networkView_->isVisible();
        networkButton_->setEnabled(visible && networkView_->state().enabled);
        if (visible) networkButton_->show(); else networkButton_->hide();
    }
    // Media area
    if (mediaButton_) {
        mediaButton_->setText("");
        const bool visible = mediaView_ && mediaView_->isVisible();
        mediaButton_->setEnabled(visible);
        if (visible) mediaButton_->show(); else mediaButton_->hide();
    }
    // Audio area + slider
    if (audioButton_ && audioSlider_) {
        audioButton_->setText("");
        const bool visible = audioView_ && audioView_->isVisible();
        audioButton_->setEnabled(visible && audioView_->state().enabled);
        if (visible) audioButton_->show(); else audioButton_->hide();
        // Panel status slots stay icon-only; volume control belongs to audio popover.
        audioSlider_->hide();
    }
    if (statusLabel_ && snapshot_) {
        statusLabel_->setText("");
        statusLabel_->hide();
    }
    updatePopovers();
}

void StatusArea::render() {
    ensureControls();
    syncFromSnapshot();
    if (container_) {
        container_->show();
        const api::Rect g = container_->geometry();
        if (audioButton_) audioButton_->setGeometry(api::Rect(0, 0, 30, g.h));
        if (mediaButton_) mediaButton_->setGeometry(api::Rect(30, 0, 30, g.h));
        if (networkButton_) networkButton_->setGeometry(api::Rect(60, 0, 30, g.h));
        container_->repaint();
    }
    if (audioButton_) audioButton_->repaint();
    if (mediaButton_) mediaButton_->repaint();
    if (networkButton_) networkButton_->repaint();
    if (audioSlider_) audioSlider_->repaint();
    if (statusLabel_) statusLabel_->repaint();
}
void StatusArea::invalidate() {
    render();
    if (container_) container_->repaint();
}

void StatusArea::openAudio() {
    if (!audioView_ || !audioView_->isVisible()) return;
    closeAll();
    if (!audioPopover_) {
        audioPopover_ = flamewm::ui::UiBackend::createPopover(container_);
        muteToggle_ = flamewm::ui::UiBackend::createToggle(audioPopover_, IconRoleTaskbarStatus);
        audioDetails_ = flamewm::ui::UiBackend::createLabel(audioPopover_);
        popoverSlider_ = flamewm::ui::UiBackend::createSlider(audioPopover_);
        if (!audioPopover_ || !popoverSlider_ || !muteToggle_ || !audioDetails_) return;
        audioPopover_->setRole(flamewm::ui::WindowRole::Popup);
        audioPopover_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleOverlay,
                                                            flamewm::ui::style::VisualNormal));
        audioDetails_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleText,
                                                            flamewm::ui::style::VisualNormal));
        popoverSlider_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleControl,
                                                             flamewm::ui::style::VisualNormal));
        muteToggle_->setIconName("audio-volume-muted");
        muteToggle_->setText("Mute");
        muteToggle_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleControl,
                                                          flamewm::ui::style::VisualNormal));
        popoverSlider_->setRange(0, 100);
        popoverSlider_->setOnChanged([this](int v) { if (audioView_) audioView_->setVolume(clampVolume(v)); });
        muteToggle_->setOnToggled([this](bool m) { if (audioView_) audioView_->setMute(m); });
        audioPopover_->setOnClosed([this]() { popoverOpen_ = false; audioDragging_ = false; });
    }
    if (!audioPopover_ || !popoverSlider_ || !muteToggle_ || !audioDetails_) return;
    audioDetails_->setGeometry(api::Rect(8, 8, 220, 22));
    popoverSlider_->setGeometry(api::Rect(8, 36, 220, 24));
    muteToggle_->setGeometry(api::Rect(8, 68, 100, 24));
    audioDetails_->show(); popoverSlider_->show(); muteToggle_->show();
    audioPopover_->setGeometry(api::Rect(0, 0, 240, 100));
    audioPopover_->showAt(audioButton_->geometry()); popoverOpen_ = true; updatePopovers();
}
void StatusArea::openMedia() {
    if (!mediaView_ || !mediaView_->isVisible()) return;
    closeAll();
    if (!mediaPopover_) {
        mediaPopover_ = flamewm::ui::UiBackend::createPopover(container_);
        mediaDetails_ = flamewm::ui::UiBackend::createLabel(mediaPopover_);
        mediaPrevious_ = flamewm::ui::UiBackend::createButton(mediaPopover_);
        mediaPlayPause_ = flamewm::ui::UiBackend::createButton(mediaPopover_);
        mediaNext_ = flamewm::ui::UiBackend::createButton(mediaPopover_);
        if (!mediaPopover_ || !mediaDetails_ || !mediaPrevious_ || !mediaPlayPause_ || !mediaNext_) return;
        mediaPopover_->setRole(flamewm::ui::WindowRole::Popup);
        mediaPopover_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleOverlay,
                                                            flamewm::ui::style::VisualNormal));
        mediaDetails_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleText,
                                                            flamewm::ui::style::VisualNormal));
        mediaPrevious_->setText("Previous"); mediaPlayPause_->setText("Play/Pause"); mediaNext_->setText("Next");
        mediaPrevious_->setIconName("media-skip-backward");
        mediaPlayPause_->setIconName("media-playback-start");
        mediaNext_->setIconName("media-skip-forward");
        mediaPrevious_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleControl,
                                                             flamewm::ui::style::VisualNormal));
        mediaPlayPause_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleControl,
                                                              flamewm::ui::style::VisualNormal));
        mediaNext_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleControl,
                                                         flamewm::ui::style::VisualNormal));
        mediaPrevious_->setOnClick([this]() { if (mediaView_) mediaView_->prev(); });
        mediaPlayPause_->setOnClick([this]() { if (mediaView_) mediaView_->playPause(); });
        mediaNext_->setOnClick([this]() { if (mediaView_) mediaView_->next(); });
        mediaPopover_->setOnClosed([this]() { popoverOpen_ = false; });
    }
    if (!mediaPopover_ || !mediaDetails_ || !mediaPrevious_ || !mediaPlayPause_ || !mediaNext_) return;
    mediaDetails_->setGeometry(api::Rect(8, 8, 260, 30));
    mediaPrevious_->setGeometry(api::Rect(8, 46, 78, 26)); mediaPlayPause_->setGeometry(api::Rect(91, 46, 100, 26)); mediaNext_->setGeometry(api::Rect(196, 46, 72, 26));
    mediaDetails_->show(); mediaPrevious_->show(); mediaPlayPause_->show(); mediaNext_->show();
    mediaPopover_->setGeometry(api::Rect(0, 0, 276, 80)); mediaPopover_->showAt(mediaButton_->geometry()); popoverOpen_ = true; updatePopovers();
}
void StatusArea::openNetwork() {
    if (!networkView_ || !networkView_->isVisible()) return;
    closeAll();
    if (!networkPopover_) {
        networkPopover_ = flamewm::ui::UiBackend::createPopover(container_);
        networkDetails_ = flamewm::ui::UiBackend::createLabel(networkPopover_);
        networkDisconnect_ = flamewm::ui::UiBackend::createButton(networkPopover_);
        networkScan_ = flamewm::ui::UiBackend::createButton(networkPopover_);
        wifiToggle_ = flamewm::ui::UiBackend::createToggle(networkPopover_, IconRoleNetwork);
        accessPoints_ = flamewm::ui::UiBackend::createList(networkPopover_);
        if (!networkPopover_ || !networkDetails_ || !networkDisconnect_ || !networkScan_ || !wifiToggle_ || !accessPoints_) return;
        networkPopover_->setRole(flamewm::ui::WindowRole::Popup);
        networkPopover_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleOverlay,
                                                              flamewm::ui::style::VisualNormal));
        networkDetails_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleText,
                                                              flamewm::ui::style::VisualNormal));
        networkDisconnect_->setText("Disconnect"); networkScan_->setText("Scan");
        networkDisconnect_->setIconName("network-offline");
        networkScan_->setIconName("view-refresh");
        wifiToggle_->setText("Wi-Fi");
        wifiToggle_->setIconName("network-wireless");
        networkDisconnect_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleControl,
                                                                 flamewm::ui::style::VisualNormal));
        networkScan_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleControl,
                                                           flamewm::ui::style::VisualNormal));
        wifiToggle_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleControl,
                                                          flamewm::ui::style::VisualNormal));
        accessPoints_->setVisual(flamewm::ui::style::Visual(flamewm::ui::style::VisualRoleSurface,
                                                            flamewm::ui::style::VisualNormal));
        networkDisconnect_->setOnClick([this]() { if (networkView_) networkView_->disconnect(); });
        networkScan_->setOnClick([this]() { if (networkView_) networkView_->requestScan(); });
        wifiToggle_->setOnToggled([this](bool e) { if (networkView_) networkView_->setWifiEnabled(e); });
        accessPoints_->setOnActivated([this](int index) {
            if (!networkView_ || !accessPoints_) return;
            std::vector<flamewm::ui::ListRow> rows = accessPoints_->rows();
            if (index < 0 || index >= static_cast<int>(rows.size())) return;
            if (system_ && system_->networkManager()) {
                const std::vector<integrations::AccessPoint>& aps = system_->networkManager()->status().visibleAps;
                if (static_cast<size_t>(index) < aps.size()) {
                    if (aps[static_cast<size_t>(index)].known) networkView_->connectKnown(aps[static_cast<size_t>(index)].path);
                    else networkView_->connectNewSecure(aps[static_cast<size_t>(index)].path);
                }
            }
        });
        networkPopover_->setOnClosed([this]() { popoverOpen_ = false; });
    }
    if (!networkPopover_ || !networkDetails_ || !networkDisconnect_ || !networkScan_ || !wifiToggle_ || !accessPoints_) return;
    networkDetails_->setGeometry(api::Rect(8, 8, 240, 26)); networkDisconnect_->setGeometry(api::Rect(8, 42, 100, 26)); networkScan_->setGeometry(api::Rect(114, 42, 70, 26)); wifiToggle_->setGeometry(api::Rect(190, 42, 70, 26)); accessPoints_->setGeometry(api::Rect(8, 74, 252, 120));
    networkDetails_->show(); networkDisconnect_->show(); networkScan_->show(); wifiToggle_->show(); accessPoints_->show();
    networkPopover_->setGeometry(api::Rect(0, 0, 270, 204)); networkPopover_->showAt(networkButton_->geometry()); popoverOpen_ = true; updatePopovers();
}
void StatusArea::closeAll() { closePopover(audioPopover_); closePopover(mediaPopover_); closePopover(networkPopover_); }
bool StatusArea::hasOpenPopover() const { return popoverOpen_; }

void StatusArea::onAudioSliderChanged(int value) {
    audioVolume_ = clampVolume(value);
    if (audioSlider_ && audioSlider_->value() != audioVolume_) audioSlider_->setValue(audioVolume_);
    if (audioView_) audioView_->setVolume(audioVolume_);
}

void StatusArea::onAudioSliderPress(int x, int y, int button) {
    (void)x; (void)y; (void)button;
    audioDragging_ = true;
    // Capture pointer: subsequent motion/release handled even outside track via these methods.
    // In IceWM backend, grab is established at YWindow level; here we just set ownership flag.
}

void StatusArea::onAudioSliderRelease(int x, int y, int button) {
    (void)x; (void)y; (void)button;
    if (!audioDragging_) return;
    audioDragging_ = false;
    // Value already updated via motion; no extra commit needed.
}

void StatusArea::onAudioSliderMotion(int x, int y) {
    if (!audioDragging_) return;
    if (!audioSlider_) return;
    // Map x to 0..100 regardless of being outside track — drag outside still updates.
    api::Rect r = audioSlider_->geometry();
    int w = r.w > 0 ? r.w : 120;
    int rel = x - r.x;
    // Allow outside: clamp rel to allow drag beyond edges, but still compute proportional value then clamp.
    int v = 0;
    if (w > 0) v = (rel * 100) / w;
    v = clampVolume(v);
    audioSlider_->setValue(v);
    audioVolume_ = v;
    (void)y;
}

} // namespace shell
} // namespace flamewm
