use crate::components::button::Button;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusButton {
    pub button: Button,
    pub status: String,
}
