use std::process::Command;

pub mod control;
pub mod runtime;
pub mod view;

use flamewm_api::settings::SettingValue;
use flamewm_control_core::ControlError;
use flamewm_settings_core::{ControlTransport, SettingsClient, SettingsPage};
use flamewm_ui_x11::{UiActionEvent, UiActionPhase, UiDocumentAccess, UiWindowConfig};

const DONATE_URI: &str = "https://paypal.me/LinsaFTW";
const SOURCE_URI: &str = "https://github.com/ArkFlame/flamewm";
const WEBSITE_URI: &str = "https://wm.arkflame.com";

#[must_use]
pub fn normal_window_config(width: u32, height: u32) -> UiWindowConfig {
    UiWindowConfig {
        width,
        height,
        x: 0,
        y: 0,
        title: "System Settings".to_owned(),
    }
}

pub struct SettingsApplication<T: ControlTransport> {
    client: SettingsClient<T>,
}

impl<T: ControlTransport> SettingsApplication<T> {
    pub fn new(transport: T) -> Self {
        Self {
            client: SettingsClient::new(transport),
        }
    }

    pub fn client(&self) -> &SettingsClient<T> {
        &self.client
    }
    pub fn client_mut(&mut self) -> &mut SettingsClient<T> {
        &mut self.client
    }

    pub fn refresh(&mut self, document: &mut impl UiDocumentAccess) -> Result<(), String> {
        self.client.refresh_all().map_err(control_error)?;
        self.sync(document)
    }

    pub fn handle_action(
        &mut self,
        event: &UiActionEvent,
        document: &mut impl UiDocumentAccess,
    ) -> Result<(), String> {
        if event.phase != UiActionPhase::Release || !event.inside {
            return Ok(());
        }
        let action = event.action.as_str();
        if let Some(page) = action.strip_prefix("settings.page.").and_then(page) {
            self.client.set_page(page);
        } else if let Some(name) = action.strip_prefix("settings.accent.") {
            let accent = match name {
                "red" => "#EF4048",
                "blue" => "#3DAEE9",
                "purple" => "#9B59B6",
                "teal" => "#1ABC9C",
                _ => return Err(format!("unknown accent '{name}'")),
            };
            self.client
                .apply_setting("accent", SettingValue::Text(accent.to_owned()))
                .map_err(control_error)?;
        } else if let Some(slot) = action
            .strip_prefix("settings.display.select.")
            .and_then(|value| value.parse::<usize>().ok())
        {
            let output_id = self
                .client
                .displays()
                .and_then(|snapshot| snapshot.outputs.get(slot))
                .map(|output| output.id.clone());
            if let Some(output_id) = output_id {
                self.client.select_output(&output_id);
            }
        } else if let Some(key) = action.strip_prefix("settings.toggle.") {
            let enabled = self
                .client
                .settings()
                .and_then(|snapshot| snapshot.values.get(key))
                .and_then(|value| match value {
                    SettingValue::Boolean(value) => Some(!value),
                    _ => None,
                })
                .ok_or_else(|| format!("toggle '{key}' has no authoritative boolean value"))?;
            self.client
                .apply_setting(key, SettingValue::Boolean(enabled))
                .map_err(control_error)?;
        } else if let Some(percent) = action
            .strip_prefix("settings.display.scale.")
            .and_then(|value| value.parse::<u16>().ok())
        {
            self.client
                .set_shell_scale(percent)
                .map_err(control_error)?;
        } else if let Some(uri) = about_uri(action) {
            open_uri(uri)?;
        }
        self.sync(document)
    }

    fn sync(&self, document: &mut impl UiDocumentAccess) -> Result<(), String> {
        for (name, page) in [
            ("appearance", SettingsPage::Appearance),
            ("desktop", SettingsPage::Desktop),
            ("taskbar", SettingsPage::Taskbar),
            ("displays", SettingsPage::Displays),
            ("fonts", SettingsPage::Fonts),
            ("hotkeys", SettingsPage::Hotkeys),
            ("about", SettingsPage::About),
        ] {
            document.visible(&format!("page-{name}"), self.client.ui().page == page)?;
        }
        for slot in 0..4 {
            let id = format!("display-output-{slot}");
            let Some(output) = self
                .client
                .displays()
                .and_then(|snapshot| snapshot.outputs.get(slot))
            else {
                document.visible(&id, false)?;
                continue;
            };
            document.visible(&id, true)?;
            document.text(
                &format!("display-output-name-{slot}"),
                output.connector.clone(),
            )?;
            let resolution = output
                .modes
                .iter()
                .find(|mode| mode.id == output.current_mode)
                .map_or(output.geometry.size(), |mode| mode.resolution);
            document.text(
                &format!("display-output-size-{slot}"),
                format!(
                    "{} x {}  |  {}%",
                    resolution.width, resolution.height, output.shell_scale_percent
                ),
            )?;
            let selected = self.client.ui().selected_output.as_ref() == Some(&output.id);
            document.visible(&format!("display-output-selected-{slot}"), selected)?;
            document.visible(&format!("display-output-unselected-{slot}"), !selected)?;
        }
        if let Some(output) = self.client.ui().selected_output.as_ref().and_then(|id| {
            self.client
                .displays()
                .and_then(|snapshot| snapshot.outputs.iter().find(|output| &output.id == id))
        }) {
            document.text(
                "display-selected-title",
                format!("Selected: {}", output.connector),
            )?;
            let resolution = output
                .modes
                .iter()
                .find(|mode| mode.id == output.current_mode)
                .map_or(output.geometry.size(), |mode| mode.resolution);
            document.text(
                "display-selected-resolution",
                format!("{} x {}", resolution.width, resolution.height),
            )?;
            document.text(
                "display-selected-scale",
                format!("{}%", output.shell_scale_percent),
            )?;
        }
        for (key, on_id, off_id) in [
            (
                "stickyNotesEnabled",
                "sticky-toggle-on",
                "sticky-toggle-off",
            ),
            ("fontBold", "font-toggle-on", "font-toggle-off"),
        ] {
            let enabled = self
                .client
                .settings()
                .and_then(|snapshot| snapshot.values.get(key))
                .and_then(|value| match value {
                    SettingValue::Boolean(value) => Some(*value),
                    _ => None,
                });
            document.visible(on_id, enabled == Some(true))?;
            document.visible(off_id, enabled == Some(false))?;
        }
        Ok(())
    }
}

fn page(value: &str) -> Option<SettingsPage> {
    match value {
        "appearance" => Some(SettingsPage::Appearance),
        "desktop" => Some(SettingsPage::Desktop),
        "taskbar" => Some(SettingsPage::Taskbar),
        "displays" => Some(SettingsPage::Displays),
        "fonts" => Some(SettingsPage::Fonts),
        "hotkeys" => Some(SettingsPage::Hotkeys),
        "about" => Some(SettingsPage::About),
        _ => None,
    }
}

fn control_error(error: ControlError) -> String {
    format!("{}: {}", error.name, error.message)
}

fn about_uri(action: &str) -> Option<&'static str> {
    match action {
        "about.donate" => Some(DONATE_URI),
        "about.source" => Some(SOURCE_URI),
        "about.website" => Some(WEBSITE_URI),
        _ => None,
    }
}

fn open_uri(uri: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(uri)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open {uri}: {error}"))
}
