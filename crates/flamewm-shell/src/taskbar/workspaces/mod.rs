use super::intent::TaskbarIntent;
use flamewm_api::workspace::WorkspaceSnapshot;
use flamewm_control_core::ControlRequest;
use flamewm_shell_core::workspace_labels;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceView {
    pub index: usize,
    /// Visible pager label. Always numeric `index + 1` per the numeric pager
    /// caller contract (`runtime.rs` maps `workspace-N` to page slot `N - 1`).
    /// Protocol names never leak here; they are preserved in [`WorkspaceView::name`].
    pub label: String,
    /// Preserved protocol name from the snapshot (`""` when unnamed).
    pub name: String,
    pub active: bool,
    pub row: usize,
    pub column: usize,
}

/// Number of workspace slots visible in the compact taskbar page (2x2 grid).
pub const PAGE_SIZE: usize = 4;

/// Returns the visible 4-slot page containing the active workspace.
///
/// `page_base = (active_pos / PAGE_SIZE) * PAGE_SIZE` where `active_pos` is the
/// position of the active view in `views` (fallback `0` when none is active),
/// clamped to `views[page_base..min(page_base + PAGE_SIZE, len)]`.
#[must_use]
pub fn visible_page(views: &[WorkspaceView]) -> Vec<&WorkspaceView> {
    if views.is_empty() {
        return Vec::new();
    }
    let active_pos = views.iter().position(|view| view.active).unwrap_or(0);
    let page_base = (active_pos / PAGE_SIZE) * PAGE_SIZE;
    let start = page_base.min(views.len());
    let end = (start + PAGE_SIZE).min(views.len());
    views[start..end].iter().collect()
}

/// Maps a global workspace index to its `(row, column)` slot in the visible page.
///
/// Global [`WorkspaceView::row`]/[`WorkspaceView::column`] use global topology
/// (`index / 2`, `index % 2`). This function instead returns page-relative
/// coordinates: `position_in_page = global_pos - page_base` maps
/// `0 -> (0, 0)`, `1 -> (0, 1)`, `2 -> (1, 0)`, `3 -> (1, 1)`, so the left
/// column shows `visible[0]`, `visible[2]` and the right column shows
/// `visible[1]`, `visible[3]`. Returns `None` when the index is unknown or
/// outside the current visible page.
#[must_use]
pub fn slot_position(global_index: usize, views: &[WorkspaceView]) -> Option<(usize, usize)> {
    if views.is_empty() {
        return None;
    }
    let active_pos = views.iter().position(|view| view.active).unwrap_or(0);
    let page_base = (active_pos / PAGE_SIZE) * PAGE_SIZE;
    let target_pos = views.iter().position(|view| view.index == global_index)?;
    let window_end = (page_base + PAGE_SIZE).min(views.len());
    if target_pos < page_base || target_pos >= window_end {
        return None;
    }
    let position_in_page = target_pos - page_base;
    Some((position_in_page / 2, position_in_page % 2))
}

#[must_use]
pub fn project(snapshot: Option<&WorkspaceSnapshot>) -> Vec<WorkspaceView> {
    let Some(snapshot) = snapshot else {
        return Vec::new();
    };
    if snapshot.count < 2 {
        return Vec::new();
    }
    let names = if snapshot.names.is_empty() || snapshot.names.len() != snapshot.count {
        vec![String::new(); snapshot.count]
    } else {
        snapshot.names.clone()
    };
    let labels = workspace_labels(snapshot.count);
    names
        .into_iter()
        .take(snapshot.count)
        .zip(labels.into_iter().take(snapshot.count))
        .enumerate()
        .map(|(index, (name, label))| WorkspaceView {
            index,
            label,
            name,
            active: index == snapshot.active_index,
            row: index / 2,
            column: index % 2,
        })
        .collect()
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
    }
}
