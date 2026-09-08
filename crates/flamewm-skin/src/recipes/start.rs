#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartRecipe {
    pub metrics: crate::metrics::StartMetrics,
    pub category_width: u16,
    pub category_height: u16,
    pub search_height: u16,
    pub app_width: u16,
}

/// CSS `.start-menu`, `.start-category`, `.start-search`, and `.start-app` exact sizes.
pub const START: StartRecipe = StartRecipe {
    metrics: crate::DEFAULT.start,
    category_width: 280,
    category_height: 37,
    search_height: 45,
    app_width: 248,
};
