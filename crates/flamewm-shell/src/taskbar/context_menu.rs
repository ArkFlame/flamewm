use super::intent::{ContextMenuIntent, TaskbarIntent};
use flamewm_api::panels::{PanelsSnapshot, TaskEntryKind};
use flamewm_api::window::WindowSnapshot;
use flamewm_api::workspace::WorkspaceSnapshot;
use flamewm_api::{TaskEntryId, WindowRef};
use flamewm_ui_core::MenuEntry;

/// Stable context-menu row ids. Rows are keyed by id, never by label.
pub const TASK_ROW_ACTIVATE: &str = "activate";
pub const TASK_ROW_CLOSE: &str = "close";
pub const TASK_ROW_UNPIN: &str = "unpin";
pub const WORKSPACE_ROW_ACTIVATE: &str = "activate";
pub const WORKSPACE_ROW_INSERT_AFTER: &str = "insert-after";
pub const WORKSPACE_ROW_REMOVE: &str = "remove";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextAction {
    ActivateWindow(WindowRef),
    CloseWindow(WindowRef),
    MinimizeWindow(WindowRef),
    RestoreWindow(WindowRef),
    PinApp {
        app: flamewm_api::DesktopAppId,
        expected_revision: u64,
    },
    UnpinApp {
        app: flamewm_api::DesktopAppId,
        expected_revision: u64,
    },
    ActivateWorkspace {
        index: usize,
        expected_revision: u64,
    },
    InsertWorkspaceAfter {
        index: usize,
        expected_revision: u64,
    },
    RemoveWorkspace {
        index: usize,
        expected_revision: u64,
    },
}

impl ContextAction {
    #[must_use]
    pub fn into_control_request(self) -> flamewm_control_core::ControlRequest {
        use flamewm_control_core::ControlRequest;
        match self {
            Self::ActivateWindow(window) => ControlRequest::ActivateWindow(window),
            Self::CloseWindow(window) => ControlRequest::CloseWindow(window),
            Self::MinimizeWindow(window) => ControlRequest::MinimizeWindow(window),
            Self::RestoreWindow(window) => ControlRequest::RestoreWindow(window),
            Self::PinApp {
                app,
                expected_revision,
            } => ControlRequest::PinApp {
                app,
                expected_revision,
            },
            Self::UnpinApp {
                app,
                expected_revision,
            } => ControlRequest::UnpinApp {
                app,
                expected_revision,
            },
            Self::ActivateWorkspace {
                index,
                expected_revision,
            } => ControlRequest::ActivateWorkspace {
                index,
                expected_revision,
            },
            Self::InsertWorkspaceAfter {
                index,
                expected_revision,
            } => ControlRequest::InsertWorkspaceAfter {
                index,
                expected_revision,
            },
            Self::RemoveWorkspace {
                index,
                expected_revision,
            } => ControlRequest::RemoveWorkspace {
                index,
                expected_revision,
            },
        }
    }
}

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

#[must_use]
pub fn task_menu(id: TaskEntryId) -> (Vec<MenuEntry>, TaskbarIntent) {
    (
        vec![MenuEntry::Action {
            id: TASK_ROW_ACTIVATE.to_owned(),
            label: "Activate".to_owned(),
            icon: None,
            enabled: true,
        }],
        TaskbarIntent::OpenContextMenu(ContextMenuIntent::Task(id)),
    )
}

/// Map a stable task row id to a typed control. Row identity is the id,
/// never the label. Pinned entries resolve Unpin only; running non-pinned
/// entries resolve Activate/Close.
#[must_use]
pub fn task_action(
    snapshot: &PanelsSnapshot,
    windows: &[WindowSnapshot],
    id: &TaskEntryId,
    row: &str,
) -> Option<ContextAction> {
    let entry = snapshot.tasks.iter().find(|entry| &entry.id == id)?;
    if entry.id.is_pinned() {
        if row != TASK_ROW_UNPIN {
            return None;
        }
        return Some(ContextAction::UnpinApp {
            app: entry.app_id.clone(),
            expected_revision: snapshot.revision,
        });
    }
    let window = match entry.kind {
        TaskEntryKind::PinnedSlot { window } => window,
        TaskEntryKind::Window { window } => Some(window),
    }?;
    let known = windows.iter().any(|snapshot| snapshot.reference == window);
    if !known {
        return None;
    }
    match row {
        TASK_ROW_ACTIVATE => Some(ContextAction::ActivateWindow(window)),
        TASK_ROW_CLOSE => Some(ContextAction::CloseWindow(window)),
        _ => None,
    }
}

/// Legacy single-arg lookup kept for the old caller shape: resolves the
/// first policy row for the entry (Unpin for pinned, Activate for running).
#[must_use]
pub fn task_action_legacy(
    snapshot: &PanelsSnapshot,
    windows: &[WindowSnapshot],
    id: TaskEntryId,
) -> Option<ContextAction> {
    let entry = snapshot.tasks.iter().find(|entry| entry.id == id)?;
    if entry.id.is_pinned() {
        return task_action(snapshot, windows, &entry.id.clone(), TASK_ROW_UNPIN);
    }
    task_action(snapshot, windows, &entry.id.clone(), TASK_ROW_ACTIVATE)
}

#[must_use]
pub fn workspace_menu(
    snapshot: &WorkspaceSnapshot,
    index: usize,
) -> Vec<(MenuEntry, ContextAction)> {
    if index >= snapshot.count {
        return Vec::new();
    }
    let mut entries = vec![(
        MenuEntry::Action {
            id: WORKSPACE_ROW_ACTIVATE.to_owned(),
            label: "Activate".to_owned(),
            icon: None,
            enabled: index != snapshot.active_index,
        },
        ContextAction::ActivateWorkspace {
            index,
            expected_revision: snapshot.revision,
        },
    )];
    entries.push((
        MenuEntry::Action {
            id: WORKSPACE_ROW_INSERT_AFTER.to_owned(),
            label: "Insert workspace after".to_owned(),
            icon: None,
            enabled: true,
        },
        ContextAction::InsertWorkspaceAfter {
            index,
            expected_revision: snapshot.revision,
        },
    ));
    if snapshot.count > 1 {
        entries.push((
            MenuEntry::Action {
                id: WORKSPACE_ROW_REMOVE.to_owned(),
                label: "Remove workspace".to_owned(),
                icon: None,
                enabled: true,
            },
            ContextAction::RemoveWorkspace {
                index,
                expected_revision: snapshot.revision,
            },
        ));
    }
    entries
}

/// Map a stable workspace row id to a typed control, revision-fenced.
/// Row identity is the id, never the label.
#[must_use]
pub fn workspace_action(
    snapshot: &WorkspaceSnapshot,
    index: usize,
    row: &str,
) -> Option<ContextAction> {
    if index >= snapshot.count {
        return None;
    }
    match row {
        WORKSPACE_ROW_ACTIVATE => Some(ContextAction::ActivateWorkspace {
            index,
            expected_revision: snapshot.revision,
        }),
        WORKSPACE_ROW_INSERT_AFTER => Some(ContextAction::InsertWorkspaceAfter {
            index,
            expected_revision: snapshot.revision,
        }),
        WORKSPACE_ROW_REMOVE => {
            if snapshot.count > 1 {
                Some(ContextAction::RemoveWorkspace {
                    index,
                    expected_revision: snapshot.revision,
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Visible context-menu row count for a menu kind. Row identity is the
/// row id, never the label. Used to size the native surface to its content
/// rows so no spare (black) area remains.
#[must_use]
pub fn visible_row_count_for_kind(
    panels: &PanelsSnapshot,
    windows: &[WindowSnapshot],
    workspaces: Option<&WorkspaceSnapshot>,
    kind: &crate::ContextMenuKind,
) -> usize {
    match kind {
        crate::ContextMenuKind::Task(id) => task_menu_for_entry(panels, windows, id).len(),
        crate::ContextMenuKind::Workspace { index } => workspaces
            .map(|snapshot| workspace_menu(snapshot, *index).len())
            .unwrap_or(0),
    }
}

/// Stable row identity for a task entry slot: entry id plus policy row id.
#[must_use]
pub fn row_identity(entry: &TaskEntryId, row: &str) -> String {
    format!("{}#{row}", entry.0)
}
