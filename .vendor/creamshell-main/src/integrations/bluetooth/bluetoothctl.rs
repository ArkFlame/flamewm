use super::BluetoothIntegration;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// BlueZ integration through its `bluetoothctl` client.
#[derive(Default)]
struct State {
    powered: bool,
    name: Option<String>,
}

pub struct BluetoothCtl {
    state: Arc<Mutex<State>>,
}

impl BluetoothCtl {
    pub fn detect() -> Option<Self> {
        output(&["show"]).map(|_| Self::new())
    }

    fn new() -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let cache = state.clone();
        thread::spawn(move || loop {
            let controller = output(&["show"]).unwrap_or_default();
            let name = connected_device();
            let next = State {
                powered: property(&controller, "Powered") == Some("yes"),
                name,
            };
            if let Ok(mut current) = cache.lock() {
                *current = next;
            }
            thread::sleep(Duration::from_secs(2));
        });
        Self { state }
    }
}

impl BluetoothIntegration for BluetoothCtl {
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
            let _ = output(&["power", if powered { "on" } else { "off" }]);
        });
    }
}

fn connected_device() -> Option<String> {
    output(&["devices", "Connected"])?
        .lines()
        .find_map(|line| line.strip_prefix("Device "))
        .and_then(|line| line.split_once(' ').map(|(_, name)| name.to_owned()))
}

fn property<'a>(output: &'a str, name: &str) -> Option<&'a str> {
    output.lines().find_map(|line| {
        let (key, value) = line.trim().split_once(':')?;
        (key == name).then_some(value.trim())
    })
}

fn output(args: &[&str]) -> Option<String> {
    Command::new("timeout")
        .args(["2s", "bluetoothctl"])
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
}
