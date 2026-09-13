use super::NetworkIntegration;
use crate::integrations::windows::powershell;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct WindowsNetwork {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    connected: bool,
    enabled: bool,
    name: Option<String>,
    strength: Option<u8>,
}

impl WindowsNetwork {
    pub fn detect() -> Option<Self> {
        powershell("Get-NetAdapter -Physical | Select-Object -First 1 -ExpandProperty Status")
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

impl NetworkIntegration for WindowsNetwork {
    fn connected(&self) -> bool {
        self.state
            .lock()
            .map(|state| state.connected)
            .unwrap_or(false)
    }
    fn network_name(&self) -> Option<String> {
        self.state.lock().ok().and_then(|state| state.name.clone())
    }
    fn strength(&self) -> Option<u8> {
        self.state.lock().ok().and_then(|state| state.strength)
    }

    fn enabled(&self) -> bool {
        self.state
            .lock()
            .map(|state| state.enabled)
            .unwrap_or(false)
    }

    fn set_enabled(&self, enabled: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.enabled = enabled;
            if !enabled {
                state.connected = false;
                state.name = None;
                state.strength = None;
            }
        }
        thread::spawn(move || {
            let action = if enabled {
                "Enable-NetAdapter"
            } else {
                "Disable-NetAdapter"
            };
            let _ = powershell(&format!(
                "Get-NetAdapter | Where-Object {{ $_.MediaType -eq 'Native 802.11' -or $_.Name -match 'Wi-Fi|Wireless' }} | Select-Object -First 1 | {action} -Confirm:$false"
            ));
        });
    }
}

fn query() -> State {
    let output = powershell("netsh wlan show interfaces").unwrap_or_default();
    let mut name = None;
    let mut strength = None;
    let mut connected = false;
    for line in output.lines().map(str::trim) {
        if let Some(value) = line
            .strip_prefix("State")
            .and_then(|line| line.split_once(':'))
            .map(|(_, value)| value.trim())
        {
            connected = value.eq_ignore_ascii_case("connected");
        } else if let Some(value) = line
            .strip_prefix("SSID")
            .and_then(|line| line.split_once(':'))
            .map(|(_, value)| value.trim())
        {
            if !value.is_empty() && !line.starts_with("BSSID") {
                name = Some(value.to_owned());
            }
        } else if let Some(value) = line
            .strip_prefix("Signal")
            .and_then(|line| line.split_once(':'))
            .map(|(_, value)| value.trim().trim_end_matches('%'))
        {
            strength = value.parse().ok();
        }
    }
    if !connected {
        connected = powershell(
            "(Get-NetAdapter -Physical | Where-Object Status -eq 'Up' | Measure-Object).Count",
        )
        .is_some_and(|count| count != "0");
    }
    State {
        connected,
        enabled: connected
            || powershell(
                "(Get-NetAdapter | Where-Object { $_.MediaType -eq 'Native 802.11' -or $_.Name -match 'Wi-Fi|Wireless' } | Where-Object Status -ne 'Disabled' | Measure-Object).Count",
            )
            .is_some_and(|count| count != "0"),
        name,
        strength,
    }
}
