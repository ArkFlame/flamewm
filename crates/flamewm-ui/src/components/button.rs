use crate::{ActionId, UiEvent, WidgetId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Button {
    pub label: String,
    pub action: ActionId,
    pub enabled: bool,
}
impl Button {
    #[must_use]
    pub fn new(label: impl Into<String>, action: impl Into<ActionId>) -> Self {
        Self {
            label: label.into(),
            action: action.into(),
            enabled: true,
        }
    }
    #[must_use]
    pub fn activate(&self, widget: impl Into<WidgetId>) -> Option<UiEvent> {
        self.enabled.then_some(UiEvent::Activate {
            widget: widget.into(),
            action: self.action.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_button_does_not_activate() {
        let mut button = Button::new("Open", "open");
        button.enabled = false;
        assert!(button.activate("open-button").is_none());
    }
}
