#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    pub columns: u16,
}
impl Default for Grid {
    fn default() -> Self {
        Self { columns: 1 }
    }
}
