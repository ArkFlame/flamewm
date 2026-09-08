#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub text: String,
}
impl Label {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}
