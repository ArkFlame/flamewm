use super::AudioIntegration;
use crate::platform::Playback;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// MPRIS is a session D-Bus protocol. `playerctl` is its standard client and
/// keeps this integration independent from any particular media player.
pub struct Mpris {
    playback: Arc<Mutex<Option<Playback>>>,
}

impl Mpris {
    pub fn detect() -> Option<Self> {
        playerctl(&["--version"]).map(|_| Self::new())
    }

    fn new() -> Self {
        let playback = Arc::new(Mutex::new(None));
        let cache = playback.clone();
        thread::spawn(move || loop {
            let next = query_playback();
            if let Ok(mut current) = cache.lock() {
                *current = next;
            }
            thread::sleep(Duration::from_millis(500));
        });
        Self { playback }
    }
}

impl AudioIntegration for Mpris {
    fn playback(&self) -> Option<Playback> {
        self.playback
            .lock()
            .ok()
            .and_then(|current| current.clone())
    }

    fn toggle_playback(&self) {
        thread::spawn(|| {
            let _ = playerctl(&["play-pause"]);
        });
    }

    fn seek(&self, position: f64) {
        thread::spawn(move || {
            let position = position.max(0.0).to_string();
            let _ = playerctl(&["position", &position]);
        });
    }

    fn previous(&self) {
        thread::spawn(|| {
            let _ = playerctl(&["previous"]);
        });
    }

    fn next(&self) {
        thread::spawn(|| {
            let _ = playerctl(&["next"]);
        });
    }
}

fn query_playback() -> Option<Playback> {
    let fields = playerctl(&[
        "metadata",
        "--format",
        "{{status}}\t{{artist}}\t{{title}}\t{{mpris:artUrl}}\t{{position}}\t{{mpris:length}}",
    ])?;
    let mut values = fields.trim_end().split('\t');
    let status = values.next()?.to_owned();
    let artist = values.next().unwrap_or_default().to_owned();
    let title = values.next().unwrap_or_default().to_owned();
    if artist.is_empty() && title.is_empty() {
        return None;
    }
    Some(Playback {
        app_icon: None,
        title,
        artist,
        status,
        art_url: values.next().and_then(resolve_art),
        position: format_time(values.next().unwrap_or_default()),
        length: format_time(values.next().unwrap_or_default()),
    })
}

fn playerctl(args: &[&str]) -> Option<String> {
    Command::new("timeout")
        .args(["2s", "playerctl"])
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
}

fn format_time(value: &str) -> String {
    value
        .parse::<u64>()
        .map(|micros| format!("{}:{:02}", micros / 60_000_000, micros / 1_000_000 % 60))
        .unwrap_or_else(|_| "0:00".into())
}

fn resolve_art(value: &str) -> Option<PathBuf> {
    if let Some(path) = value.strip_prefix("file://") {
        return Some(PathBuf::from(path));
    }
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        return None;
    }
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    let path = std::env::temp_dir().join(format!("creamshell-art-{:x}", hasher.finish()));
    if path.is_file() {
        return Some(path);
    }
    let output = Command::new("timeout")
        .args(["4s", "curl", "-LfsS", "-A", "CreamShell/1.0", value])
        .output()
        .ok()
        .filter(|result| result.status.success())?;
    if output.stdout.is_empty() || output.stdout.len() > 8 * 1024 * 1024 {
        return None;
    }
    std::fs::write(&path, output.stdout).ok()?;
    Some(path)
}
