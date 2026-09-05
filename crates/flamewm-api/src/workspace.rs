use crate::{ErrorCode, FlameError, FlameResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSnapshot {
    pub revision: u64,
    pub count: usize,
    pub active_index: usize,
    pub last_index: Option<usize>,
    pub names: Vec<String>,
}

impl WorkspaceSnapshot {
    pub fn validate(&self) -> FlameResult<()> {
        if self.count == 0 {
            return Err(FlameError::invalid("workspace count must be at least one"));
        }
        if self.active_index >= self.count {
            return Err(FlameError::invalid(
                "active workspace is outside the topology",
            ));
        }
        if self.last_index.is_some_and(|index| index >= self.count) {
            return Err(FlameError::invalid(
                "last workspace is outside the topology",
            ));
        }
        if !self.names.is_empty() && self.names.len() != self.count {
            return Err(FlameError::invalid(
                "workspace names must be empty or match count",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspacePlanKind {
    Activate,
    Insert,
    Remove,
}

/// Complete engine-neutral workspace mutation plan.
///
/// `old_to_new[old_index]` gives the destination index for every old workspace. For removal,
/// windows from the removed workspace migrate through this same mapping. For insertion, all old
/// indices remain represented and the inserted workspace has no old source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePlan {
    pub kind: WorkspacePlanKind,
    pub expected_revision: u64,
    pub new_revision: u64,
    pub old_count: usize,
    pub new_count: usize,
    pub requested_index: usize,
    pub old_to_new: Vec<usize>,
    pub new_active: usize,
    pub new_last: Option<usize>,
    pub new_names: Vec<String>,
}

impl WorkspacePlan {
    pub fn validate(&self) -> FlameResult<()> {
        if self.old_count == 0 || self.new_count == 0 {
            return Err(FlameError::invalid(
                "workspace plan cannot contain zero workspaces",
            ));
        }
        if self.old_to_new.len() != self.old_count {
            return Err(FlameError::invalid(
                "workspace mapping length must equal old count",
            ));
        }
        if self.old_to_new.iter().any(|&index| index >= self.new_count) {
            return Err(FlameError::invalid(
                "workspace mapping points outside new topology",
            ));
        }
        if self.new_active >= self.new_count {
            return Err(FlameError::invalid("planned active workspace is invalid"));
        }
        if self.new_last.is_some_and(|index| index >= self.new_count) {
            return Err(FlameError::invalid("planned last workspace is invalid"));
        }
        if self.new_names.len() != self.new_count {
            return Err(FlameError::invalid("planned names must match new count"));
        }
        if self.new_revision <= self.expected_revision {
            return Err(FlameError::new(
                ErrorCode::StaleRevision,
                "new workspace revision must advance",
            ));
        }
        Ok(())
    }
}
