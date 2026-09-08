//! FlameWM XSettings producer owned by the window-manager process.
//!
//! Owns the `_XSETTINGS_S0` manager selection only when no owner exists yet.
//! Refuses to replace an existing owner; callers provide X11 selection state
//! through the [`SelectionOwner`] port so this crate stays engine-neutral.

use std::collections::BTreeMap;

use flamewm_api::settings::AppearanceMode;
use flamewm_api::{FlameError, FlameResult};

pub const XSETTINGS_SELECTION: &str = "_XSETTINGS_S0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XSettingsValue {
    Integer(i32),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XSettingsSnapshot {
    pub serial: u32,
    pub settings: BTreeMap<String, XSettingsValue>,
}

impl XSettingsSnapshot {
    #[must_use]
    pub fn get(&self, name: &str) -> Option<XSettingsValue> {
        self.settings.get(name).cloned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XSettingsInput {
    pub appearance: AppearanceMode,
    pub icon_theme: String,
    pub cursor_theme: String,
    pub cursor_size: u32,
    pub font_family: String,
    pub font_size_points: u32,
    pub dpi_x1024: u32,
    pub antialias: bool,
    pub hinting: bool,
    pub hint_style: String,
    pub rgba_order: String,
}

impl Default for XSettingsInput {
    fn default() -> Self {
        Self {
            appearance: AppearanceMode::Dark,
            icon_theme: "breeze-dark".to_owned(),
            cursor_theme: "Breeze".to_owned(),
            cursor_size: 24,
            font_family: "IBM Plex Sans".to_owned(),
            font_size_points: 10,
            dpi_x1024: 96 * 1024,
            antialias: true,
            hinting: true,
            hint_style: "hintslight".to_owned(),
            rgba_order: "rgb".to_owned(),
        }
    }
}

#[must_use]
pub fn theme_name_for(appearance: AppearanceMode) -> &'static str {
    if appearance.prefers_dark() {
        "Flame-Dark"
    } else {
        "Flame-Light"
    }
}

#[must_use]
pub fn gtk_theme_name_for(appearance: AppearanceMode) -> &'static str {
    theme_name_for(appearance)
}

#[must_use]
pub fn build_settings(input: &XSettingsInput) -> XSettingsSnapshot {
    let dark = input.appearance.prefers_dark();
    let mut settings = BTreeMap::new();
    settings.insert(
        "Net/ThemeName".to_owned(),
        XSettingsValue::Text(gtk_theme_name_for(input.appearance).to_owned()),
    );
    settings.insert(
        "Net/IconThemeName".to_owned(),
        XSettingsValue::Text(input.icon_theme.clone()),
    );
    settings.insert(
        "Xft/Antialias".to_owned(),
        XSettingsValue::Integer(i32::from(input.antialias)),
    );
    settings.insert(
        "Xft/Hinting".to_owned(),
        XSettingsValue::Integer(i32::from(input.hinting)),
    );
    settings.insert(
        "Xft/HintStyle".to_owned(),
        XSettingsValue::Text(input.hint_style.clone()),
    );
    settings.insert(
        "Xft/RGBA".to_owned(),
        XSettingsValue::Text(input.rgba_order.clone()),
    );
    settings.insert(
        "Xft/DPI".to_owned(),
        XSettingsValue::Integer(input.dpi_x1024 as i32),
    );
    settings.insert(
        "Gtk/PreferDarkTheme".to_owned(),
        XSettingsValue::Integer(i32::from(dark)),
    );
    XSettingsSnapshot {
        serial: 0,
        settings,
    }
}

/// String table published alongside the integer settings above. XSettings
/// strings (theme names) travel in the settings blob; keep them next to the
/// builder so producers cannot drift from `build_settings` keys.
#[must_use]
pub fn string_table(input: &XSettingsInput) -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "Net/ThemeName".to_owned(),
            gtk_theme_name_for(input.appearance).to_owned(),
        ),
        ("Net/IconThemeName".to_owned(), input.icon_theme.clone()),
        ("Xft/HintStyle".to_owned(), input.hint_style.clone()),
        ("Xft/RGBA".to_owned(), input.rgba_order.clone()),
        (
            "Gtk/FontName".to_owned(),
            format!("{} {}", input.font_family, input.font_size_points),
        ),
        ("Xft/CursorTheme".to_owned(), input.cursor_theme.clone()),
    ])
}

/// Port the engine implements: report whether the selection is already owned.
pub trait SelectionOwner {
    fn selection_owner_exists(&self, selection: &str) -> bool;
}

/// Claim `_XSETTINGS_S0` only when no owner exists. Never steals ownership.
pub fn claim_manager<O: SelectionOwner>(owner: &O) -> FlameResult<()> {
    if owner.selection_owner_exists(XSETTINGS_SELECTION) {
        return Err(FlameError::unavailable(
            "XSettings manager already owned; refusing to replace existing owner",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeOwner {
        owned: bool,
    }

    impl SelectionOwner for FakeOwner {
        fn selection_owner_exists(&self, selection: &str) -> bool {
            assert_eq!(selection, XSETTINGS_SELECTION);
            self.owned
        }
    }

    #[test]
    fn dark_is_default_and_publishes_dark_preference() {
        assert_eq!(AppearanceMode::default(), AppearanceMode::Dark);
        let snapshot = build_settings(&XSettingsInput::default());
        assert_eq!(
            snapshot.get("Gtk/PreferDarkTheme"),
            Some(XSettingsValue::Integer(1))
        );
        assert!(snapshot.settings.contains_key("Net/ThemeName"));
        assert!(snapshot.settings.contains_key("Net/IconThemeName"));
        assert!(snapshot.settings.contains_key("Xft/DPI"));
        assert!(snapshot.settings.contains_key("Xft/Antialias"));
        assert!(snapshot.settings.contains_key("Xft/Hinting"));
        let strings = string_table(&XSettingsInput::default());
        assert_eq!(
            strings.get("Net/ThemeName").map(String::as_str),
            Some("Flame-Dark")
        );
        assert_eq!(
            strings.get("Net/IconThemeName").map(String::as_str),
            Some("breeze-dark")
        );
    }

    #[test]
    fn refuses_to_replace_existing_manager_owner() {
        let owned = FakeOwner { owned: true };
        let error = claim_manager(&owned).expect_err("must refuse existing owner");
        assert_eq!(error.code, flamewm_api::ErrorCode::Unavailable);
        let free = FakeOwner { owned: false };
        claim_manager(&free).expect("free selection may be claimed");
    }

    #[test]
    fn light_snapshot_clears_dark_preference() {
        let input = XSettingsInput {
            appearance: AppearanceMode::Light,
            ..XSettingsInput::default()
        };
        let snapshot = build_settings(&input);
        assert_eq!(
            snapshot.get("Gtk/PreferDarkTheme"),
            Some(XSettingsValue::Integer(0))
        );
        let strings = string_table(&input);
        assert_eq!(
            strings.get("Net/ThemeName").map(String::as_str),
            Some("Flame-Light")
        );
    }
}
