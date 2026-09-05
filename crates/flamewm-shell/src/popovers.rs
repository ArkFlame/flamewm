use flamewm_api::system::SystemSnapshot;
use flamewm_shell_core::StatusPopovers;

pub fn from_snapshot(system: &SystemSnapshot) -> StatusPopovers {
    StatusPopovers::from_snapshot(system)
}
