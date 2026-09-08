use crate::{UiEvent, WidgetId};
use flamewm_ui_core::controls::ToggleState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toggle {
    pub checked: bool,
    pub enabled: bool,
}
impl Toggle {
    #[must_use]
    pub const fn new(checked: bool) -> Self {
        Self {
            checked,
            enabled: true,
        }
    }
    pub fn set_checked(&mut self, checked: bool) -> bool {
        let mut state = ToggleState::default();
        let _ = state.set_checked(self.checked);
        let changed = state.set_checked(checked);
        self.checked = state.checked();
        changed
    }
    pub fn toggle(&mut self) -> bool {
        self.set_checked(!self.checked)
    }
    #[must_use]
    pub fn event(&self, widget: impl Into<WidgetId>) -> UiEvent {
        UiEvent::Toggle {
            widget: widget.into(),
            checked: self.checked,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_changes_once_per_state_transition() {
        let mut toggle = Toggle::new(false);
        assert!(toggle.toggle());
        assert!(!toggle.set_checked(true));
        assert!(toggle.checked);
    }
}
