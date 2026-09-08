use flamewm_api::system::{PlaybackState, ServiceAvailability, SystemSnapshot};
use flamewm_shell_core::{MediaPopoverModel, MediaView};

use super::super::intent::StatusIntent;

#[must_use]
pub fn project(snapshot: &SystemSnapshot) -> MediaView {
    let mut view = flamewm_shell_core::StatusViews::from_snapshot(snapshot).media;
    view.visible = snapshot.media.availability == ServiceAvailability::Available
        && matches!(
            snapshot.media.playback,
            PlaybackState::Playing | PlaybackState::Paused
        );
    view
}

#[must_use]
pub fn popover(snapshot: &SystemSnapshot) -> Option<MediaPopoverModel> {
    if snapshot.media.availability != ServiceAvailability::Available
        || !matches!(
            snapshot.media.playback,
            PlaybackState::Playing | PlaybackState::Paused
        )
    {
        return None;
    }
    flamewm_shell_core::StatusPopovers::from_snapshot(snapshot).media
}

#[must_use]
pub fn intent(snapshot: &SystemSnapshot) -> Option<StatusIntent> {
    (snapshot.media.availability == ServiceAvailability::Available
        && matches!(
            snapshot.media.playback,
            PlaybackState::Playing | PlaybackState::Paused
        ))
    .then_some(StatusIntent::Media)
}
