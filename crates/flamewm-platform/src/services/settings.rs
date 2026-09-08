use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use flamewm_api::ports::ShortcutPort;
use flamewm_api::settings::AppearanceMode;
use flamewm_api::settings::{SettingValue, SettingsSnapshot, SettingsTransaction};
use flamewm_api::shortcuts::KeyBinding;
use flamewm_api::{ErrorCode, FlameError, FlameResult};

use crate::settings::{ConfigStore, ProductSettings};
use crate::shortcuts::ShortcutRegistry;

#[derive(Debug, Clone)]
pub struct SettingsService {
    store: ConfigStore,
    path: PathBuf,
}

impl SettingsService {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self {
            store: ConfigStore::default(),
            path,
        }
    }

    #[must_use]
    pub fn current(&self) -> &ProductSettings {
        self.store.current()
    }

    #[must_use]
    pub fn snapshot(&self) -> SettingsSnapshot {
        to_api_snapshot(self.store.current())
    }

    pub fn load_text(&mut self, text: &str) -> FlameResult<()> {
        let mut parsed = self.store.parse_snapshot(text)?;
        if parsed.revision <= self.store.current().revision {
            parsed.revision = self.store.current().revision.saturating_add(1);
        }
        self.store.apply_snapshot(parsed)
    }

    /// Apply one complete settings transaction with native shortcut state and persistence behaving
    /// as one transaction. Native grabs are rolled back if persistence fails; no SettingsChanged
    /// state is published until both native state and durable state succeed.
    pub fn apply<P: ShortcutPort>(
        &mut self,
        shortcut_port: &mut P,
        transaction: &SettingsTransaction,
    ) -> FlameResult<SettingsSnapshot> {
        if transaction.expected_revision != self.store.current().revision {
            return Err(FlameError::new(
                ErrorCode::StaleRevision,
                "settings revision is stale",
            ));
        }
        if transaction.is_empty() {
            return Ok(self.snapshot());
        }

        let mut candidate = self.store.current().clone();
        if let Some(section) = transaction.reset_section.as_deref() {
            reset_section(&mut candidate, section)?;
        }
        for change in &transaction.changes {
            apply_change(&mut candidate, &change.key, change.value.as_ref())?;
        }
        candidate.revision = self.store.current().revision.saturating_add(1);
        candidate.validate()?;

        let desired_shortcuts = shortcut_bindings(&candidate)?;
        if let Err(error) = shortcut_port.prepare_shortcuts(&desired_shortcuts) {
            shortcut_port.rollback_shortcuts();
            return Err(error);
        }
        if let Err(error) = shortcut_port.commit_shortcuts() {
            shortcut_port.rollback_shortcuts();
            return Err(error);
        }
        if let Err(error) = ConfigStore::persist(&self.path, &candidate) {
            shortcut_port.rollback_shortcuts();
            return Err(error);
        }
        self.store.apply_snapshot(candidate)?;
        Ok(self.snapshot())
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[must_use]
pub fn to_api_snapshot(settings: &ProductSettings) -> SettingsSnapshot {
    let mut values = BTreeMap::from([
        (
            "accent".to_owned(),
            SettingValue::Text(settings.accent.clone()),
        ),
        (
            "iconTheme".to_owned(),
            SettingValue::Text(settings.icon_theme.clone()),
        ),
        (
            "appearance".to_owned(),
            SettingValue::Text(settings.appearance.as_str().to_owned()),
        ),
        (
            "wallpaper".to_owned(),
            SettingValue::Text(settings.wallpaper.clone()),
        ),
        (
            "DesktopSelectionFillOpacity".to_owned(),
            SettingValue::Percent(settings.desktop_selection_fill_opacity),
        ),
        (
            "WindowSnapPreviewFillOpacity".to_owned(),
            SettingValue::Percent(settings.window_snap_preview_fill_opacity),
        ),
        (
            "fontFamily".to_owned(),
            SettingValue::Text(settings.font_family.clone()),
        ),
        (
            "fontBold".to_owned(),
            SettingValue::Boolean(settings.font_bold),
        ),
        (
            "fontSizeOffset".to_owned(),
            SettingValue::Integer(i64::from(settings.font_size_offset)),
        ),
        (
            "taskbarColor".to_owned(),
            SettingValue::Text(settings.taskbar_color.clone()),
        ),
        (
            "taskbarOpacity".to_owned(),
            SettingValue::Percent(settings.taskbar_opacity),
        ),
        (
            "taskbarHeight".to_owned(),
            SettingValue::Integer(i64::from(settings.taskbar_height)),
        ),
        (
            "startButtonText".to_owned(),
            SettingValue::Text(settings.start_button_text.clone()),
        ),
        (
            "customStartIcon".to_owned(),
            SettingValue::Text(settings.custom_start_icon.clone()),
        ),
        (
            "stickyNotesEnabled".to_owned(),
            SettingValue::Boolean(settings.sticky_notes_enabled),
        ),
    ]);
    values.extend(settings.per_output_scale.iter().map(|(output, scale)| {
        (
            format!("scale.{output}"),
            SettingValue::Integer(i64::from(*scale)),
        )
    }));
    values.extend(settings.hotkeys.iter().map(|(action, binding)| {
        (
            format!("hotkey.{action}"),
            SettingValue::Text(binding.clone()),
        )
    }));
    SettingsSnapshot {
        revision: settings.revision,
        values,
    }
}

fn apply_change(
    settings: &mut ProductSettings,
    key: &str,
    value: Option<&SettingValue>,
) -> FlameResult<()> {
    let defaults = ProductSettings::default();
    match key {
        "accent" => settings.accent = text_or(value, defaults.accent)?,
        "iconTheme" => settings.icon_theme = text_or(value, defaults.icon_theme)?,
        "appearance" => {
            let raw = text_or(value, defaults.appearance.as_str().to_owned())?;
            settings.appearance =
                AppearanceMode::parse(&raw).ok_or_else(|| FlameError::invalid("bad appearance"))?;
        }
        "wallpaper" => settings.wallpaper = text_or(value, defaults.wallpaper)?,
        "DesktopSelectionFillOpacity" => {
            settings.desktop_selection_fill_opacity =
                percent_or(value, defaults.desktop_selection_fill_opacity)?;
        }
        "WindowSnapPreviewFillOpacity" => {
            settings.window_snap_preview_fill_opacity =
                percent_or(value, defaults.window_snap_preview_fill_opacity)?;
        }
        "fontFamily" => settings.font_family = text_or(value, defaults.font_family)?,
        "fontBold" => settings.font_bold = bool_or(value, defaults.font_bold)?,
        "fontSizeOffset" => {
            settings.font_size_offset = integer_or(value, i64::from(defaults.font_size_offset))?
                .try_into()
                .map_err(|_| FlameError::invalid("fontSizeOffset is out of range"))?;
        }
        "taskbarColor" => settings.taskbar_color = text_or(value, defaults.taskbar_color)?,
        "taskbarOpacity" => settings.taskbar_opacity = percent_or(value, defaults.taskbar_opacity)?,
        "taskbarHeight" => {
            settings.taskbar_height = integer_or(value, i64::from(defaults.taskbar_height))?
                .try_into()
                .map_err(|_| FlameError::invalid("taskbarHeight is out of range"))?;
        }
        "startButtonText" => {
            settings.start_button_text = text_or(value, defaults.start_button_text)?
        }
        "customStartIcon" => {
            settings.custom_start_icon = text_or(value, defaults.custom_start_icon)?
        }
        "stickyNotesEnabled" => {
            settings.sticky_notes_enabled = bool_or(value, defaults.sticky_notes_enabled)?;
        }
        _ if key.starts_with("scale.") => {
            let output = key.trim_start_matches("scale.");
            if output.is_empty() {
                return Err(FlameError::invalid("scale output id is empty"));
            }
            match value {
                Some(SettingValue::Integer(scale)) => {
                    let scale: u16 = (*scale)
                        .try_into()
                        .map_err(|_| FlameError::invalid("scale is out of range"))?;
                    settings.per_output_scale.insert(output.to_owned(), scale);
                }
                None => {
                    settings.per_output_scale.remove(output);
                }
                Some(_) => return Err(FlameError::invalid("scale setting requires integer")),
            }
        }
        _ if key.starts_with("hotkey.") => {
            let action = key.trim_start_matches("hotkey.");
            match value {
                Some(SettingValue::Text(binding)) => {
                    settings.hotkeys.insert(action.to_owned(), binding.clone());
                }
                None => {
                    settings.hotkeys.remove(action);
                }
                Some(_) => return Err(FlameError::invalid("hotkey setting requires text")),
            }
        }
        _ => {
            return Err(FlameError::new(
                ErrorCode::NotFound,
                format!("unknown setting: {key}"),
            ));
        }
    }
    Ok(())
}

fn reset_section(settings: &mut ProductSettings, section: &str) -> FlameResult<()> {
    let defaults = ProductSettings::default();
    match section.to_ascii_lowercase().as_str() {
        "appearance" => {
            settings.accent = defaults.accent;
            settings.icon_theme = defaults.icon_theme;
            settings.appearance = defaults.appearance;
        }
        "desktop" => {
            settings.wallpaper = defaults.wallpaper;
            settings.desktop_selection_fill_opacity = defaults.desktop_selection_fill_opacity;
            settings.sticky_notes_enabled = defaults.sticky_notes_enabled;
        }
        "windows" => {
            settings.window_snap_preview_fill_opacity = defaults.window_snap_preview_fill_opacity;
        }
        "fonts" => {
            settings.font_family = defaults.font_family;
            settings.font_bold = defaults.font_bold;
            settings.font_size_offset = defaults.font_size_offset;
        }
        "taskbar" => {
            settings.taskbar_color = defaults.taskbar_color;
            settings.taskbar_opacity = defaults.taskbar_opacity;
            settings.taskbar_height = defaults.taskbar_height;
            settings.start_button_text = defaults.start_button_text;
            settings.custom_start_icon = defaults.custom_start_icon;
        }
        "displays" => settings.per_output_scale.clear(),
        "hotkeys" => settings.hotkeys.clear(),
        _ => {
            return Err(FlameError::new(
                ErrorCode::NotFound,
                "unknown settings section",
            ));
        }
    }
    Ok(())
}

fn shortcut_bindings(settings: &ProductSettings) -> FlameResult<BTreeMap<String, KeyBinding>> {
    let mut registry = ShortcutRegistry::default();
    for (action, value) in &settings.hotkeys {
        registry.set(action, value)?;
    }
    Ok(registry.bindings().clone())
}

fn text_or(value: Option<&SettingValue>, default: String) -> FlameResult<String> {
    match value {
        Some(SettingValue::Text(value)) => Ok(value.clone()),
        None => Ok(default),
        Some(_) => Err(FlameError::invalid("setting requires text")),
    }
}

fn bool_or(value: Option<&SettingValue>, default: bool) -> FlameResult<bool> {
    match value {
        Some(SettingValue::Boolean(value)) => Ok(*value),
        None => Ok(default),
        Some(_) => Err(FlameError::invalid("setting requires boolean")),
    }
}

fn percent_or(value: Option<&SettingValue>, default: u8) -> FlameResult<u8> {
    match value {
        Some(SettingValue::Percent(value)) => Ok(*value),
        None => Ok(default),
        Some(_) => Err(FlameError::invalid("setting requires percent")),
    }
}

fn integer_or(value: Option<&SettingValue>, default: i64) -> FlameResult<i64> {
    match value {
        Some(SettingValue::Integer(value)) => Ok(*value),
        None => Ok(default),
        Some(_) => Err(FlameError::invalid("setting requires integer")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_snapshot_uses_stable_native_configuration_keys() {
        let snapshot = to_api_snapshot(&ProductSettings::default());
        assert!(snapshot.values.contains_key("wallpaper"));
    }

    #[test]
    fn unknown_setting_is_rejected_instead_of_becoming_hidden_state() {
        let mut settings = ProductSettings::default();
        let error = apply_change(
            &mut settings,
            "invented",
            Some(&SettingValue::Boolean(true)),
        )
        .expect_err("unknown key must fail");
        assert_eq!(error.code, ErrorCode::NotFound);
    }
}
