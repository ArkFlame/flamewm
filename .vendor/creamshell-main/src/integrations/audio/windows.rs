use super::AudioIntegration;
use crate::integrations::windows::powershell;
use crate::platform::Playback;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct WindowsMedia {
    playback: Arc<Mutex<Option<Playback>>>,
}

impl WindowsMedia {
    pub fn detect() -> Option<Self> {
        powershell("Add-Type -AssemblyName System.Runtime.WindowsRuntime; [Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager, Windows.Media.Control, ContentType=WindowsRuntime]::RequestAsync().GetAwaiter().GetResult() | Out-Null")
            .map(|_| Self::new())
    }

    fn new() -> Self {
        let playback = Arc::new(Mutex::new(None));
        let cache = playback.clone();
        thread::spawn(move || loop {
            if let Ok(mut current) = cache.lock() {
                *current = query_playback();
            }
            thread::sleep(Duration::from_secs(1));
        });
        Self { playback }
    }
}

impl AudioIntegration for WindowsMedia {
    fn playback(&self) -> Option<Playback> {
        self.playback.lock().ok().and_then(|value| value.clone())
    }

    fn toggle_playback(&self) {
        thread::spawn(|| {
            let _ = powershell(&media_script(
                "$session.TryTogglePlayPauseAsync().GetAwaiter().GetResult() | Out-Null",
            ));
        });
    }

    fn previous(&self) {
        thread::spawn(|| {
            let _ = powershell(&media_script(
                "$session.TrySkipPreviousAsync().GetAwaiter().GetResult() | Out-Null",
            ));
        });
    }

    fn next(&self) {
        thread::spawn(|| {
            let _ = powershell(&media_script(
                "$session.TrySkipNextAsync().GetAwaiter().GetResult() | Out-Null",
            ));
        });
    }
}

fn query_playback() -> Option<Playback> {
    let output = powershell(&media_script(
        "$properties = $session.TryGetMediaPropertiesAsync().GetAwaiter().GetResult(); \"$($session.GetPlaybackInfo().PlaybackStatus)`t$($properties.Artist)`t$($properties.Title)\"",
    ))?;
    let mut fields = output.splitn(3, '\t');
    let status = fields.next()?.trim().to_owned();
    let artist = fields.next().unwrap_or_default().trim().to_owned();
    let title = fields.next().unwrap_or_default().trim().to_owned();
    (!title.is_empty() || !artist.is_empty()).then_some(Playback {
        app_icon: None,
        title,
        artist,
        status,
        art_url: None,
        position: "0:00".into(),
        length: "0:00".into(),
    })
}

fn media_script(body: &str) -> String {
    format!(
        "Add-Type -AssemblyName System.Runtime.WindowsRuntime; $manager = [Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager, Windows.Media.Control, ContentType=WindowsRuntime]::RequestAsync().GetAwaiter().GetResult(); $session = $manager.GetCurrentSession(); if ($null -ne $session) {{ {body} }}"
    )
}
