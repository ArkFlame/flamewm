use flamewm_api::FlameResult;
use flamewm_control_core::{ControlError, ControlRequest, ControlResponse};
use flamewm_control_dbus::ControlClient;
use flamewm_dbus_reactor::BusKind;
use flamewm_settings_core::ControlTransport;

pub struct SettingsControl {
    client: ControlClient,
}

impl SettingsControl {
    pub fn connect_session() -> FlameResult<Self> {
        Ok(Self {
            client: ControlClient::connect(BusKind::Session)?,
        })
    }
}

impl ControlTransport for SettingsControl {
    fn call(&mut self, request: ControlRequest) -> Result<ControlResponse, ControlError> {
        self.client.call(&request)
    }
}
