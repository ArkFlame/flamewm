//! Runtime shell adapter. Product state arrives as snapshots; UI actions leave as controls.

pub mod calendar;
pub mod panel;
pub mod popovers;
pub mod runtime;
pub mod start;
pub mod status;
pub mod tasks;

pub use runtime::{ShellControl, ShellRuntime, ShellSnapshot};
