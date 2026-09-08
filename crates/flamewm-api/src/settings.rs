use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingValue {
    Boolean(bool),
    Integer(i64),
    Percent(u8),
    Rgb(u32),
    Text(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceMode {
    System,
    Light,
    Dark,
}

impl Default for AppearanceMode {
    fn default() -> Self {
        Self::Dark
    }
}

impl AppearanceMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "system" => Some(Self::System),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    #[must_use]
    pub const fn prefers_dark(self) -> bool {
        matches!(self, Self::Dark)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    pub revision: u64,
    pub values: BTreeMap<String, SettingValue>,
}

impl SettingsSnapshot {
    #[must_use]
    pub fn new(revision: u64) -> Self {
        Self {
            revision,
            values: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsChange {
    pub key: String,
    pub value: Option<SettingValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsTransaction {
    pub expected_revision: u64,
    pub changes: Vec<SettingsChange>,
    pub reset_section: Option<String>,
}

impl SettingsTransaction {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty() && self.reset_section.is_none()
    }
}
