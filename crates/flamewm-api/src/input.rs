use crate::{OutputId, Point};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointerPosition {
    pub root: Point,
    pub output: Option<OutputId>,
}
