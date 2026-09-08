use crate::{UiEvent, WidgetId};
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextInput {
    pub text: String,
    pub placeholder: String,
    pub enabled: bool,
}
impl TextInput {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            placeholder: String::new(),
            enabled: true,
        }
    }
    pub fn set_text(&mut self, text: impl Into<String>) -> bool {
        let text = text.into();
        if self.text == text {
            return false;
        }
        self.text = text;
        true
    }
    #[must_use]
    pub fn event(&self, widget: impl Into<WidgetId>) -> UiEvent {
        UiEvent::TextChanged {
            widget: widget.into(),
            text: self.text.clone(),
        }
    }
}
