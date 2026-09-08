use flamewm_ui_core::IconRole;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Icon {
    pub role: IconRole,
}
impl Icon {
    #[must_use]
    pub const fn new(role: IconRole) -> Self {
        Self { role }
    }
}
