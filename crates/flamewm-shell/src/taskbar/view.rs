use super::model::TaskbarModel;
use super::workspaces::WorkspaceView;
use super::{status, tasks, workspaces};
use flamewm_shell_core::{StatusViews, TaskVisualState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskbarView {
    pub tasks: Vec<TaskVisualState>,
    pub workspaces: Vec<WorkspaceView>,
    pub status: StatusViews,
}

#[must_use]
pub fn project(model: &TaskbarModel) -> TaskbarView {
    TaskbarView {
        tasks: tasks::project(&model.panels, &model.windows, &model.applications),
        workspaces: workspaces::project(model.workspaces.as_ref()),
        status: status::project(&model.system).views,
    }
}
