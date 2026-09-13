use crate::applications::DesktopApplication;
use crate::display::DisplaySnapshot;
use crate::panels::PanelsSnapshot;
use crate::session::SessionCapabilities;
use crate::system::SystemSnapshot;
use crate::window::WindowSnapshot;
use crate::workspace::WorkspaceSnapshot;

/// Consistent shell startup projection returned by one control request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellBootstrapSnapshot {
    pub windows_revision: u64,
    pub windows: Vec<WindowSnapshot>,
    pub workspaces: WorkspaceSnapshot,
    pub panels: PanelsSnapshot,
    pub applications: Vec<DesktopApplication>,
    pub session: SessionCapabilities,
    pub displays: DisplaySnapshot,
    pub system: SystemSnapshot,
}
