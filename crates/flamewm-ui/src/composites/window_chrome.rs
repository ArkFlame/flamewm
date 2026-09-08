use crate::ActionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowControlRole {
    Minimize,
    Maximize,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowControl {
    pub role: WindowControlRole,
    pub action: ActionId,
}

impl WindowControl {
    #[must_use]
    pub fn new(role: WindowControlRole, action: impl Into<ActionId>) -> Self {
        Self {
            role,
            action: action.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowChrome {
    pub title: String,
    pub controls: Vec<WindowControl>,
}

impl Default for WindowChrome {
    fn default() -> Self {
        Self {
            title: String::new(),
            controls: vec![
                WindowControl::new(WindowControlRole::Minimize, "window.minimize"),
                WindowControl::new(WindowControlRole::Maximize, "window.maximize"),
                WindowControl::new(WindowControlRole::Close, "window.close"),
            ],
        }
    }
}
