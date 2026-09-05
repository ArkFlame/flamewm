//! FlameWM's independent X11 window manager.
//!
//! This crate is the product-owned native X11 window manager.
//! Historical implementations under `.vendor/` are reference material only.

mod atoms;
mod chrome;
mod classifier;
mod client;
mod geometry;
mod runtime;
mod wm;

pub use runtime::run;
pub use wm::WmConfig;
