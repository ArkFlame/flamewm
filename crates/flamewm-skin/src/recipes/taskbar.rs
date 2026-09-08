#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskbarRecipe {
    pub metrics: crate::metrics::TaskbarMetrics,
    pub task_button_icon: u16,
    pub tray_icon: u16,
    pub clock_width: u16,
}

/// CSS `.taskbar`, `.task-button`, `.tray-button`, and `.clock` exact sizes.
pub const TASKBAR: TaskbarRecipe = TaskbarRecipe {
    metrics: crate::DEFAULT.taskbar,
    task_button_icon: 25,
    tray_icon: 17,
    clock_width: 74,
};
