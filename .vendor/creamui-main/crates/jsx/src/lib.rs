//! Runtime component protocol used by CreamUI JSX.

use creamui_core::BoxedWidget;

/// Converts a component-function result into a native JSX child node.
///
/// Native JSX intrinsics are boxed by the macro itself. Application
/// components return `BoxedWidget`, which avoids a blanket implementation
/// that would overlap with future `Widget for Box<dyn Widget>` impls.
pub trait IntoWidget {
    fn into_widget(self) -> BoxedWidget;
}

impl IntoWidget for BoxedWidget {
    fn into_widget(self) -> BoxedWidget {
        self
    }
}
