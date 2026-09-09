//! Runtime shell adapter. Product state arrives as snapshots; UI actions leave as controls.

pub mod async_projection;
pub mod calendar;
pub mod icon_loader;
pub mod panel;
pub mod popovers;
pub mod projection;
pub mod runtime;
pub mod start;
pub mod status;
pub mod tasks;

/// The active taskbar status projection is compiled without activating stale
/// taskbar controllers that target an older control contract.
pub mod taskbar {
    #[path = "context_menu.rs"]
    pub mod context_menu;
    #[path = "intent.rs"]
    pub mod intent;
    #[path = "status/mod.rs"]
    pub mod status;
    #[path = "tasks/mod.rs"]
    pub mod tasks;
    #[path = "workspaces/mod.rs"]
    pub mod workspaces;
}

pub use runtime::{
    ContextMenuKind, ContextMenuState, ShellControl, ShellRuntime, ShellSnapshot, ShellSurfaces,
};
