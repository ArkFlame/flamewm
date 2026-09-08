use crate::palette::{Rgb, STICKY_BACKGROUND, STICKY_TEXT};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickyRecipe {
    pub metrics: crate::metrics::StickyMetrics,
    pub background: Rgb,
    pub text: Rgb,
    pub title_height: u16,
    pub line_height: u16,
}

/// CSS `.sticky-note`, `.sticky-title`, and `.sticky-line` exact geometry.
pub const STICKY: StickyRecipe = StickyRecipe {
    metrics: crate::DEFAULT.sticky,
    background: STICKY_BACKGROUND,
    text: STICKY_TEXT,
    title_height: 22,
    line_height: 18,
};
