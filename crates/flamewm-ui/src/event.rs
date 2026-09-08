use crate::id::{ActionId, WidgetId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEvent {
    Activate {
        widget: WidgetId,
        action: ActionId,
    },
    Toggle {
        widget: WidgetId,
        checked: bool,
    },
    TextChanged {
        widget: WidgetId,
        text: String,
    },
    Submit {
        widget: WidgetId,
    },
    SelectionChanged {
        widget: WidgetId,
        index: Option<usize>,
    },
    ValueChanged {
        widget: WidgetId,
        value: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    Invoke(ActionId),
    SetToggle {
        widget: WidgetId,
        checked: bool,
    },
    SetText {
        widget: WidgetId,
        text: String,
    },
    Submit {
        widget: WidgetId,
    },
    Select {
        widget: WidgetId,
        index: Option<usize>,
    },
    SetValue {
        widget: WidgetId,
        value: i32,
    },
}
