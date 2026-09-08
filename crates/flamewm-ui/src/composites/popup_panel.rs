use crate::components::{popup::Popup, stack::Stack};
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PopupPanel {
    pub popup: Popup,
    pub content: Stack,
}
