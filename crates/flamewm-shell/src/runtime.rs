use std::collections::VecDeque;

use flamewm_api::applications::DesktopApplication;
use flamewm_api::display::DisplaySnapshot;
use flamewm_api::panels::PanelsSnapshot;
use flamewm_api::system::SystemSnapshot;
use flamewm_api::window::WindowSnapshot;

/// Complete renderer input. No UI component owns a competing copy of this state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSnapshot {
    pub displays: DisplaySnapshot,
    pub panels: PanelsSnapshot,
    pub windows: Vec<WindowSnapshot>,
    pub applications: Vec<DesktopApplication>,
    pub system: SystemSnapshot,
}

impl Default for ShellSnapshot {
    fn default() -> Self {
        Self {
            displays: DisplaySnapshot {
                generation: 0,
                outputs: Vec::new(),
                pending: None,
            },
            panels: PanelsSnapshot {
                revision: 0,
                panels: Vec::new(),
                tasks: Vec::new(),
                pinned_apps: Vec::new(),
            },
            windows: Vec::new(),
            applications: Vec::new(),
            system: SystemSnapshot::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellControl {
    ToggleStart,
    CloseStart,
    Launch(String),
    ActivateWindow(flamewm_api::WindowRef),
    MinimizeWindow(flamewm_api::WindowRef),
    RestoreWindow(flamewm_api::WindowRef),
    OpenPopover(String),
    ClosePopovers,
}

#[derive(Debug, Clone, Default)]
pub struct ShellRuntime {
    snapshot: ShellSnapshot,
    controls: VecDeque<ShellControl>,
}

impl ShellRuntime {
    #[must_use]
    pub fn snapshot(&self) -> &ShellSnapshot {
        &self.snapshot
    }

    pub fn replace_snapshot(&mut self, snapshot: ShellSnapshot) {
        self.snapshot = snapshot;
    }

    pub fn dispatch(&mut self, action: &str) {
        let control = match action {
            "start.toggle" => Some(ShellControl::ToggleStart),
            "start.close" => Some(ShellControl::CloseStart),
            "media.open" | "media.toggle" | "volume.open" | "network.open" | "clock.open" => Some(
                ShellControl::OpenPopover(action.split('.').next().unwrap_or(action).to_owned()),
            ),
            "popover.close" => Some(ShellControl::ClosePopovers),
            value if value.starts_with("start.") => {
                Some(ShellControl::Launch(value[6..].to_owned()))
            }
            _ => None,
        };
        if let Some(control) = control {
            self.controls.push_back(control);
        }
    }

    pub fn drain_controls(&mut self) -> impl Iterator<Item = ShellControl> + '_ {
        self.controls.drain(..)
    }
}
