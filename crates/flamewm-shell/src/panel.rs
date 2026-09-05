use flamewm_api::panels::PanelSnapshot;
use flamewm_shell_core::{start_surface_layout, StartSurfaceLayout};
use flamewm_ui_core::style::ShellMetrics;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelView {
    pub snapshot: PanelSnapshot,
    pub start: StartSurfaceLayout,
}

pub fn views(snapshot: &[PanelSnapshot], metrics: ShellMetrics) -> Vec<PanelView> {
    snapshot
        .iter()
        .map(|panel| PanelView {
            start: start_surface_layout(panel.output.clone(), panel.geometry, panel.edge, metrics),
            snapshot: panel.clone(),
        })
        .collect()
}
