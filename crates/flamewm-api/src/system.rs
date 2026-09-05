//! Typed shell-facing state for optional host integrations.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ServiceAvailability {
    #[default]
    Unknown,
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NetworkKind {
    #[default]
    Unavailable,
    Disconnected,
    Connecting,
    Wired,
    Wireless,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NetworkAccessPointSnapshot {
    pub path: String,
    pub ssid: String,
    pub strength_percent: u8,
    pub secured: bool,
    pub known: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkSnapshot {
    pub availability: ServiceAvailability,
    pub generation: u64,
    pub kind: NetworkKind,
    pub label: String,
    pub strength_percent: u8,
    pub wifi_enabled: bool,
    pub networking_enabled: bool,
    pub active_path: String,
    pub access_points: Vec<NetworkAccessPointSnapshot>,
}

impl Default for NetworkSnapshot {
    fn default() -> Self {
        Self {
            availability: ServiceAvailability::Unknown,
            generation: 0,
            kind: NetworkKind::Unavailable,
            label: String::new(),
            strength_percent: 0,
            wifi_enabled: false,
            networking_enabled: false,
            active_path: String::new(),
            access_points: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaSnapshot {
    pub availability: ServiceAvailability,
    pub generation: u64,
    pub active_bus_name: String,
    pub identity: String,
    pub title: String,
    pub artist: String,
    pub playback: PlaybackState,
    pub can_play: bool,
    pub can_pause: bool,
    pub can_next: bool,
    pub can_previous: bool,
}

impl Default for MediaSnapshot {
    fn default() -> Self {
        Self {
            availability: ServiceAvailability::Unknown,
            generation: 0,
            active_bus_name: String::new(),
            identity: String::new(),
            title: String::new(),
            artist: String::new(),
            playback: PlaybackState::Unavailable,
            can_play: false,
            can_pause: false,
            can_next: false,
            can_previous: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSnapshot {
    pub availability: ServiceAvailability,
    pub generation: u64,
    pub server_generation: u64,
    pub sink_name: String,
    pub volume_percent: u8,
    pub muted: bool,
}

impl Default for AudioSnapshot {
    fn default() -> Self {
        Self {
            availability: ServiceAvailability::Unknown,
            generation: 0,
            server_generation: 0,
            sink_name: String::new(),
            volume_percent: 0,
            muted: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemSnapshot {
    pub network: NetworkSnapshot,
    pub media: MediaSnapshot,
    pub audio: AudioSnapshot,
}
