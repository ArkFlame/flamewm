use super::BrightnessIntegration;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Clone)]
enum Backend {
    BrightnessCtl,
    Ddc,
}

pub struct BrightnessCtl {
    backend: Backend,
    level: Arc<Mutex<f32>>,
}

impl BrightnessCtl {
    pub fn detect() -> Option<Self> {
        if output("brightnessctl", &["get"]).is_some() {
            return Some(Self::new(Backend::BrightnessCtl));
        }
        // DDC is useful for external displays where the kernel backlight does
        // not exist.  `getvcp` also verifies that a monitor is reachable.
        output("ddcutil", &["getvcp", "10", "--terse"])
            .is_some()
            .then(|| Self::new(Backend::Ddc))
    }

    fn new(backend: Backend) -> Self {
        let level = Arc::new(Mutex::new(1.0));
        let cache = level.clone();
        let worker_backend = backend.clone();
        thread::spawn(move || loop {
            if let Some(next) = level_for(&worker_backend) {
                if let Ok(mut current) = cache.lock() {
                    *current = next;
                }
            }
            thread::sleep(Duration::from_secs(2));
        });
        Self { backend, level }
    }

    fn ddc_value() -> Option<(f32, f32)> {
        let line = output("ddcutil", &["getvcp", "10", "--terse"])?;
        let numbers: Vec<f32> = line
            .split_whitespace()
            .filter_map(|item| item.trim_matches(',').parse().ok())
            .collect();
        // `VCP 10 C current max` is the terse format.
        (numbers.len() >= 2).then(|| (numbers[numbers.len() - 2], numbers[numbers.len() - 1]))
    }
}

impl BrightnessIntegration for BrightnessCtl {
    fn level(&self) -> f32 {
        self.level.lock().map(|level| *level).unwrap_or(1.0)
    }

    fn set_level(&self, level: f32) {
        let level = level.clamp(0.0, 1.0);
        if let Ok(mut current) = self.level.lock() {
            *current = level;
        }
        let backend = self.backend.clone();
        thread::spawn(move || match backend {
            Backend::BrightnessCtl => {
                let _ = output(
                    "brightnessctl",
                    &["set", &format!("{}%", (level * 100.0).round())],
                );
            }
            Backend::Ddc => {
                if let Some((_, max)) = Self::ddc_value() {
                    let _ = output(
                        "ddcutil",
                        &["setvcp", "10", &((level * max).round() as u32).to_string()],
                    );
                }
            }
        });
    }
}

fn level_for(backend: &Backend) -> Option<f32> {
    match backend {
        Backend::BrightnessCtl => {
            let current = output("brightnessctl", &["get"]).and_then(|v| v.parse::<f32>().ok());
            let maximum = output("brightnessctl", &["max"]).and_then(|v| v.parse::<f32>().ok());
            current.zip(maximum).map(|(current, max)| current / max)
        }
        Backend::Ddc => BrightnessCtl::ddc_value().map(|(current, max)| current / max),
    }
    .map(|level| level.clamp(0.0, 1.0))
}

fn output(program: &str, args: &[&str]) -> Option<String> {
    Command::new("timeout")
        .args(["2s", program])
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
