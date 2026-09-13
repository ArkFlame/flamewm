use super::VolumeIntegration;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// PipeWire's native session-manager client. `wpctl` talks to WirePlumber,
/// rather than assuming a PulseAudio compatibility server is present.
#[derive(Default)]
struct State {
    level: f32,
    muted: bool,
}

pub struct PipeWire {
    state: Arc<Mutex<State>>,
}

impl PipeWire {
    pub fn detect() -> Option<Self> {
        volume_output().map(|_| Self::new())
    }

    fn new() -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let cache = state.clone();
        thread::spawn(move || loop {
            if let Some((level, muted)) = volume_output() {
                if let Ok(mut current) = cache.lock() {
                    *current = State { level, muted };
                }
            }
            thread::sleep(Duration::from_secs(1));
        });
        Self { state }
    }
}

impl VolumeIntegration for PipeWire {
    fn level(&self) -> f32 {
        self.state.lock().map(|state| state.level).unwrap_or(0.0)
    }

    fn muted(&self) -> bool {
        self.state.lock().map(|state| state.muted).unwrap_or(false)
    }

    fn set_level(&self, level: f32) {
        let value = format!("{:.3}", level.clamp(0.0, 1.0));
        if let Ok(mut state) = self.state.lock() {
            state.level = level.clamp(0.0, 1.0);
        }
        thread::spawn(move || {
            let _ = wpctl(&["set-volume", "@DEFAULT_AUDIO_SINK@", &value]);
        });
    }
}

fn volume_output() -> Option<(f32, bool)> {
    let output = wpctl(&["get-volume", "@DEFAULT_AUDIO_SINK@"])?;
    let level = output
        .split_whitespace()
        .find_map(|part| part.parse::<f32>().ok())?;
    Some((level.clamp(0.0, 1.0), output.contains("MUTED")))
}

fn wpctl(args: &[&str]) -> Option<String> {
    Command::new("timeout")
        .args(["2s", "wpctl"])
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
