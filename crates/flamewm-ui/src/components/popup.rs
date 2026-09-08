#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Popup {
    pub open: bool,
}
impl Default for Popup {
    fn default() -> Self {
        Self { open: false }
    }
}
