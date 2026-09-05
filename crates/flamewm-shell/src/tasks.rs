use flamewm_api::applications::DesktopApplication;
use flamewm_api::panels::PanelsSnapshot;
use flamewm_api::window::WindowSnapshot;
use flamewm_shell_core::{task_visual_states, TaskVisualState};

pub fn visual_states(
    panels: &PanelsSnapshot,
    windows: &[WindowSnapshot],
    apps: &[DesktopApplication],
) -> Vec<TaskVisualState> {
    task_visual_states(panels, windows, apps)
}
