use crate::components::icon::Icon;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuRow {
    pub label: String,
    pub icon: Option<Icon>,
}
