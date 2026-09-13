use super::intent::TaskbarIntent;
use flamewm_api::workspace::WorkspaceSnapshot;
use flamewm_control_core::ControlRequest;

pub use flamewm_shell_core::workspaces::{
    project as core_project, slot_position, visible_page, workspace_labels, WorkspaceView,
    PAGE_SIZE,
};

/// Pager projection with the J07 static shell span (`shell.pager.project`).
/// Single owner: wraps the core workspace projection; no second pager.
#[must_use]
pub fn project(snapshot: Option<&WorkspaceSnapshot>) -> Vec<WorkspaceView> {
    let _span = crate::runtime::shell_span("shell.pager.project").start();
    core_project(snapshot)
}

#[must_use]
pub fn intent_for_click(
    snapshot: Option<&WorkspaceSnapshot>,
    index: usize,
) -> Option<TaskbarIntent> {
    snapshot
        .filter(|snapshot| index < snapshot.count)
        .map(|_| TaskbarIntent::ActivateWorkspace(index))
}

#[must_use]
pub fn control_request_for_click(
    snapshot: Option<&WorkspaceSnapshot>,
    index: usize,
) -> Option<ControlRequest> {
    snapshot
        .filter(|snapshot| index < snapshot.count)
        .map(|snapshot| ControlRequest::ActivateWorkspace {
            index,
            expected_revision: snapshot.revision,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use flamewm_shell_core::workspaces as core;

    fn snapshot(count: usize, active_index: usize) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            revision: 1,
            count,
            active_index,
            last_index: None,
            names: Vec::new(),
        }
    }

    fn indices(page: &[&WorkspaceView]) -> Vec<usize> {
        page.iter().map(|view| view.index).collect()
    }

    #[test]
    fn single_workspace_projects_empty() {
        let snap = snapshot(1, 0);
        assert!(project(Some(&snap)).is_empty());
        assert!(visible_page(&project(Some(&snap))).is_empty());
    }

    #[test]
    fn four_workspaces_active_zero_shows_first_page() {
        let snap = snapshot(4, 0);
        let views = project(Some(&snap));
        assert_eq!(indices(&visible_page(&views)), vec![0, 1, 2, 3]);
    }

    #[test]
    fn eight_workspaces_active_five_shows_second_page() {
        let snap = snapshot(8, 5);
        let views = project(Some(&snap));
        assert_eq!(indices(&visible_page(&views)), vec![4, 5, 6, 7]);
        assert_eq!(slot_position(4, &views), Some((0, 0)));
        assert_eq!(slot_position(5, &views), Some((0, 1)));
        assert_eq!(slot_position(6, &views), Some((1, 0)));
        assert_eq!(slot_position(7, &views), Some((1, 1)));
        assert_eq!(slot_position(0, &views), None);
    }

    #[test]
    fn eighteen_workspaces_active_last_shows_tail_page() {
        let snap = snapshot(18, 17);
        let views = project(Some(&snap));
        assert_eq!(indices(&visible_page(&views)), vec![16, 17]);
        assert_eq!(slot_position(16, &views), Some((0, 0)));
        assert_eq!(slot_position(17, &views), Some((0, 1)));
        assert_eq!(slot_position(0, &views), None);
    }

    #[test]
    fn click_slot_maps_to_global_index() {
        let snap = snapshot(8, 5);
        let views = project(Some(&snap));
        for view in visible_page(&views) {
            assert_eq!(
                intent_for_click(Some(&snap), view.index),
                Some(TaskbarIntent::ActivateWorkspace(view.index))
            );
        }
        assert_eq!(intent_for_click(Some(&snap), 8), None);
        assert_eq!(core::workspace_labels(2), vec!["1", "2"]);
    }
}
