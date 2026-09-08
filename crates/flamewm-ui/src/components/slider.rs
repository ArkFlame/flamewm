use flamewm_ui_core::controls::SliderState;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slider {
    pub state: SliderState,
}
impl Slider {
    #[must_use]
    pub fn new(minimum: i32, maximum: i32, value: i32) -> Self {
        Self {
            state: SliderState::new(minimum, maximum, value),
        }
    }
}
