use super::VolumeIntegration;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};

pub struct WindowsVolume {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    level: f32,
    muted: bool,
}

impl WindowsVolume {
    pub fn detect() -> Option<Self> {
        Some(Self::new())
    }

    fn new() -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let cache = state.clone();
        thread::spawn(move || {
            initialize_com();
            loop {
                if let Some(next) = read_state() {
                    if let Ok(mut current) = cache.lock() {
                        *current = next;
                    }
                }
                thread::sleep(Duration::from_secs(1));
            }
        });
        Self { state }
    }
}

impl VolumeIntegration for WindowsVolume {
    fn level(&self) -> f32 {
        self.state.lock().map(|state| state.level).unwrap_or(0.0)
    }
    fn muted(&self) -> bool {
        self.state.lock().map(|state| state.muted).unwrap_or(false)
    }

    fn set_level(&self, level: f32) {
        let level = level.clamp(0.0, 1.0);
        if let Ok(mut state) = self.state.lock() {
            state.level = level;
        }
        thread::spawn(move || {
            initialize_com();
            if let Some(endpoint) = endpoint() {
                let _ = unsafe { endpoint.SetMasterVolumeLevelScalar(level, std::ptr::null()) };
            }
        });
    }
}

fn endpoint() -> Option<IAudioEndpointVolume> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        device.Activate(CLSCTX_ALL, None).ok()
    }
}

fn initialize_com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

fn read_state() -> Option<State> {
    let endpoint = endpoint()?;
    unsafe {
        Some(State {
            level: endpoint.GetMasterVolumeLevelScalar().ok()?.clamp(0.0, 1.0),
            muted: endpoint
                .GetMute()
                .ok()
                .map(|value| value.as_bool())
                .unwrap_or(false),
        })
    }
}
