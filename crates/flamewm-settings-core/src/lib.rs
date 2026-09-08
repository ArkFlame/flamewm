//! FlameWM Settings application state and control-client contracts.
//!
//! The GUI process is intentionally separate from the WM process. It owns only transient UI state
//! and cached immutable snapshots; all authoritative mutations cross Flame Control.

use flamewm_api::display::DisplaySnapshot;
use flamewm_api::panels::PanelsSnapshot;
use flamewm_api::settings::{
    AppearanceMode, SettingValue, SettingsChange, SettingsSnapshot, SettingsTransaction,
};
use flamewm_api::shortcuts::ShortcutSnapshot;
use flamewm_api::{ModeId, OutputId, PanelEdge, TransactionId};
use flamewm_control_core::{ControlError, ControlRequest, ControlResponse};
use flamewm_ui_core::IconRole;

pub trait ControlTransport {
    fn call(&mut self, request: ControlRequest) -> Result<ControlResponse, ControlError>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SettingsPage {
    #[default]
    Appearance,
    Desktop,
    Taskbar,
    Displays,
    Fonts,
    Hotkeys,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSpec {
    pub page: SettingsPage,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: IconRole,
}

#[must_use]
pub const fn page_specs() -> &'static [PageSpec; 7] {
    &[
        PageSpec {
            page: SettingsPage::Appearance,
            name: "Appearance",
            description: "Accent, icon theme and live preview",
            icon: IconRole::Appearance,
        },
        PageSpec {
            page: SettingsPage::Desktop,
            name: "Desktop",
            description: "Wallpaper, selection and snap preview",
            icon: IconRole::Desktop,
        },
        PageSpec {
            page: SettingsPage::Taskbar,
            name: "Taskbar",
            description: "Color, opacity, height and Start branding",
            icon: IconRole::Taskbar,
        },
        PageSpec {
            page: SettingsPage::Displays,
            name: "Displays",
            description: "Output mode, scale and reversible confirmation",
            icon: IconRole::Displays,
        },
        PageSpec {
            page: SettingsPage::Fonts,
            name: "Fonts",
            description: "Family, bold and size offset",
            icon: IconRole::Fonts,
        },
        PageSpec {
            page: SettingsPage::Hotkeys,
            name: "Hotkeys",
            description: "Capture, clear and reset shortcut rows",
            icon: IconRole::Hotkeys,
        },
        PageSpec {
            page: SettingsPage::About,
            name: "About",
            description: "A lightweight desktop by ArkFlame Studios",
            icon: IconRole::About,
        },
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyCapture {
    pub action: String,
    pub captured: Option<String>,
}

impl HotkeyCapture {
    #[must_use]
    pub fn new(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            captured: None,
        }
    }

    /// Escape means "Not assigned" in the V5 product contract.
    pub fn capture(&mut self, normalized_key: &str) {
        self.captured = if normalized_key.eq_ignore_ascii_case("Escape") {
            Some(String::new())
        } else {
            Some(normalized_key.to_owned())
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsUiState {
    pub page: SettingsPage,
    pub selected_output: Option<OutputId>,
    pub hotkey_capture: Option<HotkeyCapture>,
    pub pending_display_transaction: Option<TransactionId>,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        Self {
            page: SettingsPage::Appearance,
            selected_output: None,
            hotkey_capture: None,
            pending_display_transaction: None,
        }
    }
}

pub struct SettingsClient<T: ControlTransport> {
    transport: T,
    ui: SettingsUiState,
    settings: Option<SettingsSnapshot>,
    displays: Option<DisplaySnapshot>,
    shortcuts: Option<ShortcutSnapshot>,
    panels: Option<PanelsSnapshot>,
}

impl<T: ControlTransport> SettingsClient<T> {
    #[must_use]
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            ui: SettingsUiState::default(),
            settings: None,
            displays: None,
            shortcuts: None,
            panels: None,
        }
    }

    #[must_use]
    pub fn ui(&self) -> &SettingsUiState {
        &self.ui
    }

    #[must_use]
    pub fn settings(&self) -> Option<&SettingsSnapshot> {
        self.settings.as_ref()
    }

    #[must_use]
    pub fn displays(&self) -> Option<&DisplaySnapshot> {
        self.displays.as_ref()
    }

    #[must_use]
    pub fn shortcuts(&self) -> Option<&ShortcutSnapshot> {
        self.shortcuts.as_ref()
    }

    #[must_use]
    pub fn panels(&self) -> Option<&PanelsSnapshot> {
        self.panels.as_ref()
    }

    pub fn set_page(&mut self, page: SettingsPage) {
        self.ui.page = page;
    }

    pub fn refresh_all(&mut self) -> Result<(), ControlError> {
        self.refresh_settings()?;
        self.refresh_displays()?;
        self.refresh_shortcuts()?;
        self.refresh_panels()?;
        Ok(())
    }

    pub fn refresh_settings(&mut self) -> Result<(), ControlError> {
        let response = self.transport.call(ControlRequest::GetSettings)?;
        let ControlResponse::Settings(snapshot) = response else {
            return Err(unexpected_response("settings snapshot"));
        };
        self.settings = Some(snapshot);
        Ok(())
    }

    pub fn refresh_displays(&mut self) -> Result<(), ControlError> {
        let response = self.transport.call(ControlRequest::GetDisplays)?;
        let ControlResponse::Displays(snapshot) = response else {
            return Err(unexpected_response("display snapshot"));
        };
        self.reconcile_selected_output(&snapshot);
        self.ui.pending_display_transaction =
            snapshot.pending.as_ref().map(|pending| pending.transaction);
        self.displays = Some(snapshot);
        Ok(())
    }

    pub fn refresh_shortcuts(&mut self) -> Result<(), ControlError> {
        let response = self.transport.call(ControlRequest::GetShortcuts)?;
        let ControlResponse::Shortcuts(snapshot) = response else {
            return Err(unexpected_response("shortcut snapshot"));
        };
        self.shortcuts = Some(snapshot);
        Ok(())
    }

    pub fn refresh_panels(&mut self) -> Result<(), ControlError> {
        let response = self.transport.call(ControlRequest::GetPanels)?;
        let ControlResponse::Panels(snapshot) = response else {
            return Err(unexpected_response("panels snapshot"));
        };
        self.panels = Some(snapshot);
        Ok(())
    }

    pub fn select_output(&mut self, output: &OutputId) -> bool {
        let exists = self.displays.as_ref().is_some_and(|snapshot| {
            snapshot
                .outputs
                .iter()
                .any(|candidate| &candidate.id == output)
        });
        if exists {
            self.ui.selected_output = Some(output.clone());
        }
        exists
    }

    pub fn apply_setting(
        &mut self,
        key: impl Into<String>,
        value: SettingValue,
    ) -> Result<(), ControlError> {
        self.apply_changes(vec![SettingsChange {
            key: key.into(),
            value: Some(value),
        }])
    }

    /// Apply one native Settings-page transaction and replace the cached snapshot only after the
    /// authoritative WM process accepts it. This is the common immediate-apply path.
    pub fn apply_changes(&mut self, changes: Vec<SettingsChange>) -> Result<(), ControlError> {
        let expected_revision = self
            .settings
            .as_ref()
            .map_or(0, |snapshot| snapshot.revision);
        let transaction = SettingsTransaction {
            expected_revision,
            changes,
            reset_section: None,
        };
        let response = self
            .transport
            .call(ControlRequest::ApplySettings(transaction))?;
        let ControlResponse::Settings(snapshot) = response else {
            return Err(unexpected_response("updated settings snapshot"));
        };
        self.settings = Some(snapshot);
        Ok(())
    }

    pub fn set_appearance(
        &mut self,
        accent: impl Into<String>,
        icon_theme: impl Into<String>,
        appearance: AppearanceMode,
    ) -> Result<(), ControlError> {
        self.apply_changes(vec![
            SettingsChange {
                key: "accent".to_owned(),
                value: Some(SettingValue::Text(accent.into())),
            },
            SettingsChange {
                key: "iconTheme".to_owned(),
                value: Some(SettingValue::Text(icon_theme.into())),
            },
            SettingsChange {
                key: "appearance".to_owned(),
                value: Some(SettingValue::Text(appearance.as_str().to_owned())),
            },
        ])
    }

    pub fn set_wallpaper(&mut self, path: impl Into<String>) -> Result<(), ControlError> {
        self.apply_setting("wallpaper", SettingValue::Text(path.into()))
    }

    pub fn set_desktop_selection_opacity(&mut self, percent: u8) -> Result<(), ControlError> {
        self.apply_setting(
            "DesktopSelectionFillOpacity",
            SettingValue::Percent(percent),
        )
    }

    pub fn set_snap_preview_opacity(&mut self, percent: u8) -> Result<(), ControlError> {
        self.apply_setting(
            "WindowSnapPreviewFillOpacity",
            SettingValue::Percent(percent),
        )
    }

    pub fn set_sticky_notes_enabled(&mut self, enabled: bool) -> Result<(), ControlError> {
        self.apply_setting("stickyNotesEnabled", SettingValue::Boolean(enabled))
    }

    pub fn set_taskbar_appearance(
        &mut self,
        color: impl Into<String>,
        opacity: u8,
        height: u16,
    ) -> Result<(), ControlError> {
        self.apply_changes(vec![
            SettingsChange {
                key: "taskbarColor".to_owned(),
                value: Some(SettingValue::Text(color.into())),
            },
            SettingsChange {
                key: "taskbarOpacity".to_owned(),
                value: Some(SettingValue::Percent(opacity)),
            },
            SettingsChange {
                key: "taskbarHeight".to_owned(),
                value: Some(SettingValue::Integer(i64::from(height))),
            },
        ])
    }

    pub fn set_start_branding(
        &mut self,
        text: impl Into<String>,
        icon_path: impl Into<String>,
    ) -> Result<(), ControlError> {
        self.apply_changes(vec![
            SettingsChange {
                key: "startButtonText".to_owned(),
                value: Some(SettingValue::Text(text.into())),
            },
            SettingsChange {
                key: "customStartIcon".to_owned(),
                value: Some(SettingValue::Text(icon_path.into())),
            },
        ])
    }

    pub fn set_fonts(
        &mut self,
        family: impl Into<String>,
        bold: bool,
        size_offset: i8,
    ) -> Result<(), ControlError> {
        self.apply_changes(vec![
            SettingsChange {
                key: "fontFamily".to_owned(),
                value: Some(SettingValue::Text(family.into())),
            },
            SettingsChange {
                key: "fontBold".to_owned(),
                value: Some(SettingValue::Boolean(bold)),
            },
            SettingsChange {
                key: "fontSizeOffset".to_owned(),
                value: Some(SettingValue::Integer(i64::from(size_offset))),
            },
        ])
    }

    pub fn begin_hotkey_capture(&mut self, action: impl Into<String>) {
        self.ui.hotkey_capture = Some(HotkeyCapture::new(action));
    }

    pub fn capture_hotkey(&mut self, normalized_key: &str) -> bool {
        let Some(capture) = self.ui.hotkey_capture.as_mut() else {
            return false;
        };
        capture.capture(normalized_key);
        true
    }

    pub fn commit_hotkey_capture(&mut self) -> Result<(), ControlError> {
        let Some(capture) = self.ui.hotkey_capture.take() else {
            return Ok(());
        };
        let expected_revision = self
            .shortcuts
            .as_ref()
            .map_or(0, |snapshot| snapshot.revision);
        let binding = capture.captured.unwrap_or_default();
        let request = if binding.is_empty() {
            ControlRequest::ClearShortcut {
                action: capture.action,
                expected_revision,
            }
        } else {
            ControlRequest::SetShortcut {
                action: capture.action,
                binding,
                expected_revision,
            }
        };
        expect_unit(self.transport.call(request)?)?;
        self.refresh_shortcuts()
    }

    pub fn reset_hotkey(&mut self, action: impl Into<String>) -> Result<(), ControlError> {
        let expected_revision = self
            .shortcuts
            .as_ref()
            .map_or(0, |snapshot| snapshot.revision);
        expect_unit(self.transport.call(ControlRequest::ResetShortcut {
            action: action.into(),
            expected_revision,
        })?)?;
        self.refresh_shortcuts()
    }

    pub fn begin_display_mode(
        &mut self,
        mode: ModeId,
        now_ms: u64,
    ) -> Result<TransactionId, ControlError> {
        let snapshot = self
            .displays
            .as_ref()
            .ok_or_else(|| missing_snapshot("display"))?;
        let output = self
            .ui
            .selected_output
            .clone()
            .ok_or_else(|| missing_snapshot("selected output"))?;
        let response = self.transport.call(ControlRequest::BeginDisplayMode {
            output,
            mode,
            expected_generation: snapshot.generation,
            now_ms,
        })?;
        let ControlResponse::Transaction(transaction) = response else {
            return Err(unexpected_response("display transaction"));
        };
        self.ui.pending_display_transaction = Some(transaction);
        Ok(transaction)
    }

    pub fn keep_display_mode(&mut self) -> Result<(), ControlError> {
        let Some(transaction) = self.ui.pending_display_transaction else {
            return Ok(());
        };
        expect_unit(
            self.transport
                .call(ControlRequest::KeepDisplayMode { transaction })?,
        )?;
        self.ui.pending_display_transaction = None;
        self.refresh_displays()
    }

    pub fn revert_display_mode(&mut self) -> Result<(), ControlError> {
        let Some(transaction) = self.ui.pending_display_transaction else {
            return Ok(());
        };
        expect_unit(
            self.transport
                .call(ControlRequest::RevertDisplayMode { transaction })?,
        )?;
        self.ui.pending_display_transaction = None;
        self.refresh_displays()
    }

    pub fn set_shell_scale(&mut self, percent: u16) -> Result<bool, ControlError> {
        let snapshot = self
            .displays
            .as_ref()
            .ok_or_else(|| missing_snapshot("display"))?;
        let output = self
            .ui
            .selected_output
            .clone()
            .ok_or_else(|| missing_snapshot("selected output"))?;
        let response = self.transport.call(ControlRequest::SetShellScale {
            output,
            percent,
            expected_revision: snapshot.generation,
        })?;
        let ControlResponse::Changed(changed) = response else {
            return Err(unexpected_response("changed flag"));
        };
        self.refresh_displays()?;
        Ok(changed)
    }

    pub fn set_panel_edge(
        &mut self,
        output: OutputId,
        edge: PanelEdge,
    ) -> Result<(), ControlError> {
        let expected_revision = self.panels.as_ref().map_or(0, |snapshot| snapshot.revision);
        expect_unit(self.transport.call(ControlRequest::SetPanelEdge {
            output,
            edge,
            expected_revision,
        })?)?;
        self.refresh_panels()
    }

    fn reconcile_selected_output(&mut self, snapshot: &DisplaySnapshot) {
        let selected_still_exists =
            self.ui.selected_output.as_ref().is_some_and(|selected| {
                snapshot.outputs.iter().any(|output| &output.id == selected)
            });
        if selected_still_exists {
            return;
        }
        self.ui.selected_output = snapshot
            .outputs
            .iter()
            .find(|output| output.primary && output.connected)
            .or_else(|| snapshot.outputs.iter().find(|output| output.connected))
            .map(|output| output.id.clone());
    }
}

fn expect_unit(response: ControlResponse) -> Result<(), ControlError> {
    if response == ControlResponse::Unit {
        Ok(())
    } else {
        Err(unexpected_response("unit response"))
    }
}

fn unexpected_response(expected: &str) -> ControlError {
    ControlError {
        name: "com.arkflame.FlameWM1.Error.InternalFailure",
        code: flamewm_api::ErrorCode::InternalFailure,
        message: format!("control transport returned unexpected response; expected {expected}"),
    }
}

fn missing_snapshot(name: &str) -> ControlError {
    ControlError {
        name: "com.arkflame.FlameWM1.Error.Unavailable",
        code: flamewm_api::ErrorCode::Unavailable,
        message: format!("{name} snapshot is not loaded"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use flamewm_api::display::{DisplayMode, OutputSnapshot};
    use flamewm_api::{Rect, Size};

    use super::*;

    #[derive(Default)]
    struct FakeTransport {
        responses: VecDeque<Result<ControlResponse, ControlError>>,
        requests: Vec<ControlRequest>,
    }

    impl ControlTransport for FakeTransport {
        fn call(&mut self, request: ControlRequest) -> Result<ControlResponse, ControlError> {
            self.requests.push(request);
            self.responses.pop_front().expect("queued response")
        }
    }

    fn display_snapshot() -> DisplaySnapshot {
        let mode = ModeId(7);
        DisplaySnapshot {
            generation: 3,
            outputs: vec![OutputSnapshot {
                id: OutputId::new("HDMI-1"),
                connector: "HDMI-1".to_owned(),
                edid_identity: "edid".to_owned(),
                connected: true,
                primary: true,
                geometry: Rect::new(0, 0, 1920, 1080),
                current_mode: mode,
                modes: vec![DisplayMode {
                    id: mode,
                    resolution: Size::new(1920, 1080),
                    refresh_millihz: 60_000,
                    preferred: true,
                }],
                shell_scale_percent: 100,
            }],
            pending: None,
        }
    }

    #[test]
    fn display_refresh_selects_primary_output() {
        let mut transport = FakeTransport::default();
        transport
            .responses
            .push_back(Ok(ControlResponse::Displays(display_snapshot())));
        let mut client = SettingsClient::new(transport);
        client.refresh_displays().expect("refresh");
        assert_eq!(
            client.ui().selected_output.as_ref().map(|id| id.0.as_str()),
            Some("HDMI-1")
        );
    }

    #[test]
    fn escape_capture_commits_clear_binding() {
        let mut transport = FakeTransport::default();
        transport.responses.push_back(Ok(ControlResponse::Unit));
        transport
            .responses
            .push_back(Ok(ControlResponse::Shortcuts(ShortcutSnapshot {
                revision: 2,
                bindings: Default::default(),
            })));
        let mut client = SettingsClient::new(transport);
        client.begin_hotkey_capture("ToggleStartMenu");
        assert!(client.capture_hotkey("Escape"));
        client.commit_hotkey_capture().expect("commit");
        assert_eq!(
            client.shortcuts().map(|snapshot| snapshot.revision),
            Some(2)
        );
    }

    #[test]
    fn native_settings_navigation_order_matches_current_root() {
        let names = page_specs()
            .iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "Appearance",
                "Desktop",
                "Taskbar",
                "Displays",
                "Fonts",
                "Hotkeys",
                "About"
            ]
        );
    }
}
