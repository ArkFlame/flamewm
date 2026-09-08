#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopRecipe {
    pub metrics: crate::metrics::DesktopMetrics,
}

/// CSS `.desktop-icon`, `.desktop-icon img`, and `.desktop-label` exact geometry.
pub const DESKTOP: DesktopRecipe = DesktopRecipe {
    metrics: crate::DEFAULT.desktop,
};
