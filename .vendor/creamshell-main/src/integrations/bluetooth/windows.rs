use super::BluetoothIntegration;
use crate::integrations::windows::powershell;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct WindowsBluetooth {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    powered: bool,
    name: Option<String>,
}

impl WindowsBluetooth {
    pub fn detect() -> Option<Self> {
        powershell("Get-PnpDevice -Class Bluetooth | Select-Object -First 1 -ExpandProperty Status")
            .map(|_| Self::new())
    }

    fn new() -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let cache = state.clone();
        thread::spawn(move || loop {
            if let Ok(mut current) = cache.lock() {
                *current = query();
            }
            thread::sleep(Duration::from_secs(2));
        });
        Self { state }
    }
}

impl BluetoothIntegration for WindowsBluetooth {
    fn powered(&self) -> bool {
        self.state
            .lock()
            .map(|state| state.powered)
            .unwrap_or(false)
    }
    fn connected(&self) -> bool {
        self.state
            .lock()
            .map(|state| state.name.is_some())
            .unwrap_or(false)
    }
    fn device_name(&self) -> Option<String> {
        self.state.lock().ok().and_then(|state| state.name.clone())
    }

    fn set_powered(&self, powered: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.powered = powered;
            if !powered {
                state.name = None;
            }
        }
        thread::spawn(move || {
            let action = if powered {
                "Enable-PnpDevice"
            } else {
                "Disable-PnpDevice"
            };
            let _ = powershell(&format!(
                "Get-PnpDevice -Class Bluetooth | Where-Object {{ $_.FriendlyName -match 'Radio|Adapter' }} | Select-Object -First 1 | {action} -Confirm:$false"
            ));
        });
    }
}

fn query() -> State {
    let output = powershell("Get-PnpDevice -Class Bluetooth | Where-Object Status -eq 'OK' | ForEach-Object { \"$($_.FriendlyName)`t$($_.InstanceId)\" }").unwrap_or_default();
    let mut devices = output.lines();
    let first = devices
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let name = first
        .and_then(|line| line.split('\t').next())
        .map(str::to_owned);
    State {
        powered: name.is_some(),
        name,
    }
}
