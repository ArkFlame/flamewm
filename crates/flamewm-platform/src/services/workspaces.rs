use flamewm_api::ports::WorkspacePort;
use flamewm_api::workspace::WorkspaceSnapshot;
use flamewm_api::{ErrorCode, FlameError, FlameResult, WindowRef};

use crate::workspace::{TwoRowTopology, WorkspacePlanner};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WorkspaceService;

impl WorkspaceService {
    pub fn snapshot<P: WorkspacePort>(&self, port: &P) -> FlameResult<WorkspaceSnapshot> {
        let snapshot = port.workspace_snapshot()?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn activate<P: WorkspacePort>(
        &self,
        port: &mut P,
        index: usize,
        expected_revision: u64,
    ) -> FlameResult<()> {
        let snapshot = self.snapshot(port)?;
        require_revision(&snapshot, expected_revision)?;
        if index >= snapshot.count {
            return Err(FlameError::new(
                ErrorCode::InvalidArgument,
                "workspace index is invalid",
            ));
        }
        port.activate_workspace(index, expected_revision)
    }

    pub fn insert_after<P: WorkspacePort>(
        &self,
        port: &mut P,
        index: usize,
        expected_revision: u64,
    ) -> FlameResult<()> {
        let snapshot = self.snapshot(port)?;
        require_revision(&snapshot, expected_revision)?;
        let plan = WorkspacePlanner::insert_after(&snapshot, index)?;
        port.apply_workspace_plan(&plan)
    }

    pub fn remove<P: WorkspacePort>(
        &self,
        port: &mut P,
        index: usize,
        expected_revision: u64,
    ) -> FlameResult<()> {
        let snapshot = self.snapshot(port)?;
        require_revision(&snapshot, expected_revision)?;
        let plan = WorkspacePlanner::remove(&snapshot, index)?;
        port.apply_workspace_plan(&plan)
    }

    pub fn navigate<P: WorkspacePort>(
        &self,
        port: &mut P,
        direction: NavigationDirection,
        expected_revision: u64,
    ) -> FlameResult<()> {
        let snapshot = self.snapshot(port)?;
        require_revision(&snapshot, expected_revision)?;
        let target = match direction {
            NavigationDirection::Left => {
                TwoRowTopology::move_left(snapshot.active_index, snapshot.count)
            }
            NavigationDirection::Right => {
                TwoRowTopology::move_right(snapshot.active_index, snapshot.count)
            }
            NavigationDirection::Up => {
                TwoRowTopology::move_up(snapshot.active_index, snapshot.count)
            }
            NavigationDirection::Down => {
                TwoRowTopology::move_down(snapshot.active_index, snapshot.count)
            }
        };
        if target == snapshot.active_index {
            return Ok(());
        }
        port.activate_workspace(target, expected_revision)
    }

    pub fn move_window<P: WorkspacePort>(
        &self,
        port: &mut P,
        window: WindowRef,
        target: usize,
    ) -> FlameResult<()> {
        if !window.is_valid() {
            return Err(FlameError::new(
                ErrorCode::InvalidArgument,
                "invalid WindowRef",
            ));
        }
        let snapshot = self.snapshot(port)?;
        if target >= snapshot.count {
            return Err(FlameError::new(
                ErrorCode::InvalidArgument,
                "workspace index is invalid",
            ));
        }
        port.move_window_to_workspace(window, target)
    }
}

fn require_revision(snapshot: &WorkspaceSnapshot, expected: u64) -> FlameResult<()> {
    if snapshot.revision == expected {
        Ok(())
    } else {
        Err(FlameError::new(
            ErrorCode::StaleRevision,
            "workspace revision is stale",
        ))
    }
}
