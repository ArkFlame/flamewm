#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsRecipe {
    pub metrics: crate::metrics::SettingsMetrics,
    pub body_height: u16,
    pub brand_width: u16,
    pub brand_height: u16,
    pub nav_row_height: u16,
    pub nav_selection_width: u16,
    pub nav_selection_height: u16,
}

/// CSS `.settings-window`, `.settings-nav`, `.nav-row`, and `.nav-selection` exact sizes.
pub const SETTINGS: SettingsRecipe = SettingsRecipe {
    metrics: crate::DEFAULT.settings,
    body_height: 449,
    brand_width: 192,
    brand_height: 64,
    nav_row_height: 36,
    nav_selection_width: 176,
    nav_selection_height: 36,
};
