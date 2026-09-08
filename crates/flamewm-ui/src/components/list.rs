#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct List {
    pub rows: Vec<String>,
    pub selected: Option<usize>,
}
