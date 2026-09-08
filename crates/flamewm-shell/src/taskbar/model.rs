use flamewm_api::applications::DesktopApplication;
use flamewm_api::panels::PanelsSnapshot;
use flamewm_api::system::SystemSnapshot;
use flamewm_api::window::WindowSnapshot;
use flamewm_api::workspace::WorkspaceSnapshot;

/// Inputs owned by the platform and projected by the taskbar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskbarModel {
    pub panels: PanelsSnapshot,
    pub windows: Vec<WindowSnapshot>,
    pub applications: Vec<DesktopApplication>,
    pub workspaces: Option<WorkspaceSnapshot>,
    pub system: SystemSnapshot,
}

impl TaskbarModel {
    #[must_use]
    pub fn new(
        panels: PanelsSnapshot,
        windows: Vec<WindowSnapshot>,
        applications: Vec<DesktopApplication>,
        workspaces: Option<WorkspaceSnapshot>,
        system: SystemSnapshot,
    ) -> Self {
        Self {
            panels,
            windows,
            applications,
            workspaces,
            system,
        }
    }
}
