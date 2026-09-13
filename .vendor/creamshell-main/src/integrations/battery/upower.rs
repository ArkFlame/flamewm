use super::BatteryIntegration;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct UPower {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    percentage: Option<u8>,
    charging: bool,
}

impl UPower {
    pub fn detect() -> Option<Self> {
        let device = command(&["-e"])?
            .lines()
            .find(|line| line.contains("battery_"))?
            .to_owned();
        let state = Arc::new(Mutex::new(State::default()));
        let cache = state.clone();
        thread::spawn(move || loop {
            let properties = properties(&device);
            let next = State {
                percentage: property(properties.as_deref(), "percentage")
                    .and_then(|value| value.trim_end_matches('%').parse().ok()),
                charging: matches!(
                    property(properties.as_deref(), "state"),
                    Some("charging") | Some("fully-charged")
                ),
            };
            if let Ok(mut current) = cache.lock() {
                *current = next;
            }
            thread::sleep(Duration::from_secs(15));
        });
        Some(Self { state })
    }
}

impl BatteryIntegration for UPower {
    fn percentage(&self) -> Option<u8> {
        self.state.lock().ok().and_then(|state| state.percentage)
    }

    fn charging(&self) -> bool {
        self.state
            .lock()
            .map(|state| state.charging)
            .unwrap_or(false)
    }
}

fn properties(device: &str) -> Option<String> {
    command(&["-i", device])
}

fn property<'a>(properties: Option<&'a str>, name: &str) -> Option<&'a str> {
    properties?.lines().find_map(|line| {
        let (key, value) = line.trim().split_once(':')?;
        (key.trim() == name).then_some(value.trim())
    })
}

fn command(args: &[&str]) -> Option<String> {
    Command::new("timeout")
        .args(["2s", "upower"])
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
}
