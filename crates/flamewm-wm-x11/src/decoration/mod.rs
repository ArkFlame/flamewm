//! Window decorations: WM-owned frame state, layout, identity, cache,
//! paint, shape, and the [`manager::DecorationManager`] facade consumed by
//! `Wm`. Product code projects state through the manager; no caller renders
//! directly or calls raw native APIs.

pub mod cache;
pub mod identity;
pub mod interaction;
pub mod layout;
pub mod manager;
pub mod model;
pub mod paint;
pub mod shape;

pub use crate::client::ResizeEdges;
pub use identity::scene_role_for;
pub use interaction::FrameControl;
pub use layout::resized_rect;
pub use manager::{DecorationManager, frame_icon_rgba, icon_fallback_reason};
pub use shape::bounding_rectangles;
