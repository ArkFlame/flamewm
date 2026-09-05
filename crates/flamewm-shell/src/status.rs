use flamewm_api::system::SystemSnapshot;
use flamewm_shell_core::StatusViews;

pub fn from_snapshot(system: &SystemSnapshot) -> StatusViews {
    StatusViews::from_snapshot(system)
}
