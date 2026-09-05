use crate::{DesktopAppId, OutputId, Rect, TaskEntryId, WindowRef};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PanelEdge {
    #[default]
    Bottom,
    Top,
    Left,
    Right,
}

impl PanelEdge {
    #[must_use]
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Bottom | Self::Top)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelSnapshot {
    pub output: OutputId,
    pub edge: PanelEdge,
    pub logical_size: u16,
    pub visible: bool,
    pub geometry: Rect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskEntryKind {
    PinnedSlot { window: Option<WindowRef> },
    Window { window: WindowRef },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskEntry {
    pub id: TaskEntryId,
    pub app_id: DesktopAppId,
    pub kind: TaskEntryKind,
    pub order_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelsSnapshot {
    pub revision: u64,
    pub panels: Vec<PanelSnapshot>,
    pub tasks: Vec<TaskEntry>,
    pub pinned_apps: Vec<DesktopAppId>,
}
