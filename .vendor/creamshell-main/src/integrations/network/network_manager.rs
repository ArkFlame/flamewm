use super::NetworkIntegration;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// NetworkManager integration, through its supported `nmcli` client.
#[derive(Default)]
struct State {
    connected: bool,
    enabled: bool,
    name: Option<String>,
    strength: Option<u8>,
}

pub struct NetworkManager {
    state: Arc<Mutex<State>>,
}

impl NetworkManager {
    pub fn detect() -> Option<Self> {
        command(&["-t", "-f", "RUNNING", "general"])
            .is_some_and(|state| state.eq_ignore_ascii_case("running"))
            .then(Self::new)
    }

    fn new() -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let cache = state.clone();
        thread::spawn(move || loop {
            let next = State {
                connected: command(&["-t", "-f", "STATE", "general"])
                    .is_some_and(|state| state == "connected"),
                enabled: command(&["radio", "wifi"])
                    .is_some_and(|state| state.eq_ignore_ascii_case("enabled")),
                name: Self::active_wifi()
                    .map(|(name, _)| name)
                    .or_else(active_connection),
                strength: Self::active_wifi().map(|(_, strength)| strength),
            };
            if let Ok(mut current) = cache.lock() {
                *current = next;
            }
            thread::sleep(Duration::from_secs(2));
        });
        Self { state }
    }

    fn active_wifi() -> Option<(String, u8)> {
        // `-t` makes this stable across locales.  It escapes separators inside
        // SSIDs, so parse its colon-delimited records accordingly.
        command(&["-t", "-f", "ACTIVE,SSID,SIGNAL", "device", "wifi"])?
            .lines()
            .find_map(|line| {
                let fields = nmcli_fields(line);
                let active = fields.first()?;
                let ssid = fields.get(1)?.to_owned();
                let signal = fields.get(2)?.parse().ok()?;
                (active == "yes" && !ssid.is_empty()).then_some((ssid, signal))
            })
    }
}

fn nmcli_fields(line: &str) -> Vec<String> {
    let mut fields = vec![String::new()];
    let mut escaped = false;
    for character in line.chars() {
        if escaped {
            fields.last_mut().expect("nmcli field").push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == ':' {
            fields.push(String::new());
        } else {
            fields.last_mut().expect("nmcli field").push(character);
        }
    }
    if escaped {
        fields.last_mut().expect("nmcli field").push('\\');
    }
    fields
}

impl NetworkIntegration for NetworkManager {
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
            let _ = command(&["radio", "wifi", if enabled { "on" } else { "off" }]);
        });
    }
}

fn active_connection() -> Option<String> {
    command(&["-t", "-f", "NAME", "connection", "show", "--active"])
        .and_then(|names| names.lines().next().map(str::to_owned))
        .filter(|name| !name.is_empty())
}

fn command(args: &[&str]) -> Option<String> {
    Command::new("timeout")
        .args(["2s", "nmcli"])
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
