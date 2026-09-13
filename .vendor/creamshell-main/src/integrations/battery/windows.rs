use super::BatteryIntegration;
use crate::integrations::windows::powershell;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct WindowsBattery {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    percentage: Option<u8>,
    charging: bool,
}

impl WindowsBattery {
    pub fn detect() -> Option<Self> {
        query().map(|_| Self::new())
    }

    fn new() -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let cache = state.clone();
        thread::spawn(move || loop {
            if let Some(next) = query() {
                if let Ok(mut current) = cache.lock() {
                    *current = next;
                }
            }
            thread::sleep(Duration::from_secs(15));
        });
        Self { state }
    }
}

impl BatteryIntegration for WindowsBattery {
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

fn query() -> Option<State> {
    let output = powershell("Get-CimInstance Win32_Battery | Select-Object -First 1 EstimatedChargeRemaining,BatteryStatus | ForEach-Object { \"$($_.EstimatedChargeRemaining)`t$($_.BatteryStatus)\" }")?;
    let (percentage, status) = output.trim().split_once('\t')?;
    Some(State {
        percentage: percentage.parse().ok(),
        charging: matches!(status.trim(), "2" | "6" | "7" | "8" | "9"),
    })
}
