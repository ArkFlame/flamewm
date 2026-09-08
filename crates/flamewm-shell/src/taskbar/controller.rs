use super::intent::TaskbarIntent;
use super::model::TaskbarModel;
use super::{tasks, workspaces};
use flamewm_api::TaskEntryId;

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskbarController;

impl TaskbarController {
    #[must_use]
    pub fn task_click(&self, model: &TaskbarModel, id: &TaskEntryId) -> Option<TaskbarIntent> {
        tasks::intent_for_click(id, &model.panels, &model.windows)
    }

    #[must_use]
    pub fn workspace_click(&self, model: &TaskbarModel, index: usize) -> Option<TaskbarIntent> {
        workspaces::intent_for_click(model.workspaces.as_ref(), index)
    }
}
