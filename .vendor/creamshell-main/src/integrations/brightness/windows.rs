use super::BrightnessIntegration;
use crate::integrations::windows::{powershell, run_powershell};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct WindowsBrightness {
    level: Arc<Mutex<f32>>,
}

impl WindowsBrightness {
    pub fn detect() -> Option<Self> {
        read_level().map(|_| Self::new())
    }

    fn new() -> Self {
        let level = Arc::new(Mutex::new(1.0));
        let cache = level.clone();
        thread::spawn(move || loop {
            if let Some(value) = read_level() {
                if let Ok(mut current) = cache.lock() {
                    *current = value;
                }
            }
            thread::sleep(Duration::from_secs(2));
        });
        Self { level }
    }
}

impl BrightnessIntegration for WindowsBrightness {
    fn level(&self) -> f32 {
        self.level.lock().map(|level| *level).unwrap_or(1.0)
    }

    fn set_level(&self, level: f32) {
        let level = level.clamp(0.0, 1.0);
        if let Ok(mut current) = self.level.lock() {
            *current = level;
        }
        let value = (level * 100.0).round() as u8;
        run_powershell(format!(
            "Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods | ForEach-Object {{ $_.WmiSetBrightness(1, {value}) | Out-Null }}"
        ));
    }
}

fn read_level() -> Option<f32> {
    powershell("(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness | Select-Object -First 1).CurrentBrightness")
        .and_then(|value| value.parse::<f32>().ok())
        .map(|value| (value / 100.0).clamp(0.0, 1.0))
}
