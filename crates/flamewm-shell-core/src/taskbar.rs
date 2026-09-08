use super::status::STATUS_SLOT_COUNT;
use flamewm_api::applications::DesktopApplication;
use flamewm_api::display::DisplaySnapshot;
use flamewm_api::panels::{PanelsSnapshot, TaskEntry, TaskEntryKind};
use flamewm_api::window::{WindowSnapshot, WindowState};
use flamewm_api::{DesktopAppId, OutputId, Rect, TaskEntryId, WindowRef};
use flamewm_ui_core::style::ShellMetrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelSlot {
    Start = 0,
    Tasks = 1,
    Spacer = 2,
    Workspaces = 3,
    Status = 4,
    Tray = 5,
    Clock = 6,
}
pub const PANEL_SLOT_COUNT: usize = 7;
#[must_use]
pub fn default_panel_slot_footprints(
    metrics: ShellMetrics,
) -> ([i32; PANEL_SLOT_COUNT], [i32; PANEL_SLOT_COUNT]) {
    (
        [
            i32::from(metrics.start_button_width),
            0,
            12,
            i32::from(metrics.workspace_button_width) * 2,
            i32::from(metrics.tray_button_width) * STATUS_SLOT_COUNT,
            i32::from(metrics.tray_button_width),
            i32::from(metrics.clock_min_width),
        ],
        [
            i32::from(metrics.start_button_width),
            i32::from(metrics.task_button_width),
            0,
            i32::from(metrics.workspace_button_width),
            i32::from(metrics.tray_button_width),
            i32::from(metrics.tray_button_width),
            i32::from(metrics.clock_min_width),
        ],
    )
}
#[must_use]
pub fn solve_panel_slot_sizes(
    axis: i32,
    preferred: [i32; PANEL_SLOT_COUNT],
    minimum: [i32; PANEL_SLOT_COUNT],
) -> [i32; PANEL_SLOT_COUNT] {
    let axis = axis.max(0);
    let mut sizes = preferred.map(|value| value.max(0));
    let mut total: i32 = sizes.iter().sum();
    if total > axis {
        let mut excess = total - axis;
        while excess > 0 {
            let mut reduced = false;
            for index in 0..PANEL_SLOT_COUNT {
                let floor = minimum[index].max(0);
                if excess > 0 && sizes[index] > floor {
                    sizes[index] -= 1;
                    excess -= 1;
                    reduced = true;
                }
            }
            if !reduced {
                break;
            }
        }
        while excess > 0 {
            let mut reduced = false;
            for size in &mut sizes {
                if excess > 0 && *size > 0 {
                    *size -= 1;
                    excess -= 1;
                    reduced = true;
                }
            }
            if !reduced {
                break;
            }
        }
    } else if total < axis {
        let mut extra = axis - total;
        while extra > 0 {
            for index in [PanelSlot::Tasks as usize, PanelSlot::Spacer as usize] {
                if extra == 0 {
                    break;
                }
                sizes[index] += 1;
                extra -= 1;
            }
        }
    }
    total = sizes.iter().sum();
    debug_assert!(total <= axis || axis == 0);
    sizes
}
#[must_use]
pub fn panel_slot_rects(
    horizontal: bool,
    axis: i32,
    cross: i32,
    preferred: [i32; PANEL_SLOT_COUNT],
    minimum: [i32; PANEL_SLOT_COUNT],
) -> [Rect; PANEL_SLOT_COUNT] {
    let sizes = solve_panel_slot_sizes(axis, preferred, minimum);
    let mut result = [Rect::default(); PANEL_SLOT_COUNT];
    let mut cursor = 0;
    for (index, size) in sizes.into_iter().enumerate() {
        result[index] = if horizontal {
            Rect::new(cursor, 0, size, cross.max(0))
        } else {
            Rect::new(0, cursor, cross.max(0), size)
        };
        cursor += size;
    }
    result
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimaryPresentationRouter {
    primary: Option<OutputId>,
}
impl PrimaryPresentationRouter {
    #[must_use]
    pub fn from_displays(displays: &DisplaySnapshot) -> Self {
        let primary = displays
            .outputs
            .iter()
            .find(|o| o.connected && o.primary)
            .or_else(|| displays.outputs.iter().find(|o| o.connected))
            .map(|o| o.id.clone());
        Self { primary }
    }
    #[must_use]
    pub fn primary(&self) -> Option<&OutputId> {
        self.primary.as_ref()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskVisualState {
    pub id: TaskEntryId,
    pub app_id: DesktopAppId,
    pub window: Option<WindowRef>,
    pub icon_name: String,
    pub running: bool,
    pub focused: bool,
    pub minimized: bool,
}
#[must_use]
pub fn task_visual_states(
    panels: &PanelsSnapshot,
    windows: &[WindowSnapshot],
    applications: &[DesktopApplication],
) -> Vec<TaskVisualState> {
    panels
        .tasks
        .iter()
        .map(|entry| {
            let window = match entry.kind {
                TaskEntryKind::PinnedSlot { window } => window,
                TaskEntryKind::Window { window } => Some(window),
            };
            let snapshot = window
                .and_then(|reference| windows.iter().find(|item| item.reference == reference));
            let icon_name = applications
                .iter()
                .find(|app| app.id == entry.app_id)
                .map(|app| app.icon_name.clone())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| entry.app_id.as_str().to_owned());
            TaskVisualState {
                id: entry.id.clone(),
                app_id: entry.app_id.clone(),
                window,
                icon_name,
                running: window.is_some(),
                focused: snapshot.is_some_and(|item| item.focused),
                minimized: snapshot.is_some_and(|item| item.state == WindowState::Minimized),
            }
        })
        .collect()
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskClickAction {
    Launch(DesktopAppId),
    RestoreAndActivate(WindowRef),
    Minimize(WindowRef),
    Activate(WindowRef),
}
#[must_use]
pub fn task_click_action(entry: &TaskEntry, windows: &[WindowSnapshot]) -> TaskClickAction {
    let window = match entry.kind {
        TaskEntryKind::PinnedSlot { window } => window,
        TaskEntryKind::Window { window } => Some(window),
    };
    let Some(window) = window else {
        return TaskClickAction::Launch(entry.app_id.clone());
    };
    let Some(snapshot) = windows.iter().find(|item| item.reference == window) else {
        return TaskClickAction::Activate(window);
    };
    if snapshot.state == WindowState::Minimized {
        TaskClickAction::RestoreAndActivate(window)
    } else if snapshot.focused {
        TaskClickAction::Minimize(window)
    } else {
        TaskClickAction::Activate(window)
    }
}
