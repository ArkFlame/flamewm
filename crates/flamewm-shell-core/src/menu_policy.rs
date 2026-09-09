//! Shell context-menu row policy. Pure menu identity; no control or
//! runtime state here. Row identity is the stable row id, never the label.

use flamewm_api::TaskEntryId;
use flamewm_api::panels::{PanelsSnapshot, TaskEntryKind};
use flamewm_api::window::WindowSnapshot;
use flamewm_ui_core::MenuEntry;

/// Stable context-menu row ids. Rows are keyed by id, never by label.
pub const TASK_ROW_ACTIVATE: &str = "activate";
pub const TASK_ROW_CLOSE: &str = "close";
pub const TASK_ROW_UNPIN: &str = "unpin";
pub const WORKSPACE_ROW_ACTIVATE: &str = "activate";
pub const WORKSPACE_ROW_INSERT_AFTER: &str = "insert-after";
pub const WORKSPACE_ROW_REMOVE: &str = "remove";

/// Task context-menu policy (V4): pinned entries expose Unpin only;
/// running non-pinned entries expose Activate + Close.
#[must_use]
pub fn task_menu_for_entry(
    snapshot: &PanelsSnapshot,
    windows: &[WindowSnapshot],
    id: &TaskEntryId,
) -> Vec<MenuEntry> {
    let Some(entry) = snapshot.tasks.iter().find(|entry| &entry.id == id) else {
        return Vec::new();
    };
    if entry.id.is_pinned() {
        return vec![MenuEntry::Action {
            id: TASK_ROW_UNPIN.to_owned(),
            label: "Unpin".to_owned(),
            icon: None,
            enabled: true,
        }];
    }
    let window = match entry.kind {
        TaskEntryKind::PinnedSlot { window } => window,
        TaskEntryKind::Window { window } => Some(window),
    };
    let running =
        window.is_some_and(|window| windows.iter().any(|snapshot| snapshot.reference == window));
    if !running {
        return Vec::new();
    }
    vec![
        MenuEntry::Action {
            id: TASK_ROW_ACTIVATE.to_owned(),
            label: "Activate".to_owned(),
            icon: None,
            enabled: true,
        },
        MenuEntry::Action {
            id: TASK_ROW_CLOSE.to_owned(),
            label: "Close".to_owned(),
            icon: None,
            enabled: true,
        },
    ]
}

/// Stable row identity for a task entry slot: entry id plus policy row id.
#[must_use]
pub fn row_identity(entry: &TaskEntryId, row: &str) -> String {
    format!("{}#{row}", entry.0)
}
