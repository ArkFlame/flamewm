use flamewm_api::applications::DesktopApplication;
use flamewm_shell_core::StartModel;

pub type StartState = StartModel;

pub fn state(applications: Vec<DesktopApplication>) -> StartState {
    StartModel::new(applications)
}
