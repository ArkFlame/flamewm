use crate::{components::button::Button, components::icon::Icon};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconButton {
    pub icon: Icon,
    pub button: Button,
}
