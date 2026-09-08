//! Renderer-neutral taskbar feature composition.

pub mod clock;
pub mod context_menu;
pub mod controller;
pub mod docking;
pub mod intent;
pub mod model;
pub mod status;
pub mod tasks;
pub mod view;
pub mod workspaces;

pub use controller::TaskbarController;
pub use intent::{ContextMenuIntent, DockingIntent, StatusIntent, TaskbarIntent};
pub use model::TaskbarModel;
pub use view::TaskbarView;
