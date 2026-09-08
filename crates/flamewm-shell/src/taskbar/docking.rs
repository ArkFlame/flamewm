use super::intent::{DockingIntent, TaskbarIntent};
use flamewm_api::OutputId;
use flamewm_api::panels::{PanelEdge, PanelsSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockingAction {
    MoveTo {
        output: OutputId,
        edge: PanelEdge,
        expected_revision: u64,
    },
    ToggleVisibility {
        output: OutputId,
        expected_revision: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockingView {
    pub edge: PanelEdge,
    pub orientation: Orientation,
    pub visible: bool,
}

#[must_use]
pub fn project(snapshot: &PanelsSnapshot) -> Option<DockingView> {
    snapshot.panels.first().map(|panel| DockingView {
        edge: panel.edge,
        orientation: if panel.edge.is_horizontal() {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        },
        visible: panel.visible,
    })
}

#[must_use]
pub fn move_intent(snapshot: &PanelsSnapshot, edge: PanelEdge) -> Option<DockingAction> {
    let panel = snapshot.panels.first()?;
    Some(DockingAction::MoveTo {
        output: panel.output.clone(),
        edge,
        expected_revision: snapshot.revision,
    })
}

#[must_use]
pub fn visibility_intent(snapshot: &PanelsSnapshot) -> Option<DockingAction> {
    let panel = snapshot.panels.first()?;
    Some(DockingAction::ToggleVisibility {
        output: panel.output.clone(),
        expected_revision: snapshot.revision,
    })
}

#[must_use]
pub fn intent(intent: DockingIntent) -> TaskbarIntent {
    TaskbarIntent::Dock(intent)
}
