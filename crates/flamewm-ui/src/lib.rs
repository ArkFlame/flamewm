//! Product-facing, renderer-neutral retained UI contracts.

pub mod components;
pub mod composites;
pub mod event;
pub mod id;
pub mod tree;

pub use event::{UiAction, UiEvent};
pub use flamewm_ui_core as core;
pub use id::{ActionId, WidgetId};
pub use tree::{UiNode, UiTree, WidgetKind};
