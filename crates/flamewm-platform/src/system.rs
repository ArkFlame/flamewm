use flamewm_api::system::{
    AudioSnapshot, MediaSnapshot, NetworkKind, NetworkSnapshot, PlaybackState, ServiceAvailability,
    SystemSnapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectBackoff {
    attempt: u8,
    initial_ms: u64,
    maximum_ms: u64,
}

impl Default for ReconnectBackoff {
    fn default() -> Self {
        Self {
            attempt: 0,
            initial_ms: 250,
            maximum_ms: 30_000,
        }
    }
}

impl ReconnectBackoff {
    #[must_use]
    pub fn next_delay_ms(&mut self) -> u64 {
        let shift = u32::from(self.attempt.min(7));
        let delay = self
            .initial_ms
            .saturating_mul(1_u64 << shift)
            .min(self.maximum_ms);
        self.attempt = self.attempt.saturating_add(1);
        delay
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SystemService {
    snapshot: SystemSnapshot,
    pulse_backoff: ReconnectBackoff,
}

impl SystemService {
    #[must_use]
    pub fn snapshot(&self) -> &SystemSnapshot {
        &self.snapshot
    }

    pub fn update_network(&mut self, mut update: NetworkSnapshot) -> bool {
        if update.generation < self.snapshot.network.generation {
            return false;
        }
        update.strength_percent = update.strength_percent.min(100);
        for access_point in &mut update.access_points {
            access_point.strength_percent = access_point.strength_percent.min(100);
        }
        self.snapshot.network = update;
        true
    }

    pub fn update_media(&mut self, update: MediaSnapshot) -> bool {
        if update.generation < self.snapshot.media.generation {
            return false;
        }
        self.snapshot.media = update;
        true
    }

    pub fn update_audio(&mut self, mut update: AudioSnapshot) -> bool {
        if update.generation < self.snapshot.audio.generation
            || update.server_generation < self.snapshot.audio.server_generation
        {
            return false;
        }
        update.volume_percent = update.volume_percent.min(100);
        if update.availability == ServiceAvailability::Available {
            self.pulse_backoff.reset();
        }
        self.snapshot.audio = update;
        true
    }

    pub fn network_owner_lost(&mut self) {
        let generation = self.snapshot.network.generation.saturating_add(1);
        self.snapshot.network = NetworkSnapshot {
            availability: ServiceAvailability::Unavailable,
            generation,
            kind: NetworkKind::Unavailable,
            ..NetworkSnapshot::default()
        };
    }

    pub fn mpris_owner_lost(&mut self) {
        let generation = self.snapshot.media.generation.saturating_add(1);
        self.snapshot.media = MediaSnapshot {
            availability: ServiceAvailability::Unavailable,
            generation,
            playback: PlaybackState::Unavailable,
            ..MediaSnapshot::default()
        };
    }

    pub fn pulse_disconnected(&mut self) -> u64 {
        let generation = self.snapshot.audio.generation.saturating_add(1);
        let server_generation = self.snapshot.audio.server_generation.saturating_add(1);
        self.snapshot.audio = AudioSnapshot {
            availability: ServiceAvailability::Unavailable,
            generation,
            server_generation,
            ..AudioSnapshot::default()
        };
        self.pulse_backoff.next_delay_ms()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_audio_callback_from_old_server_generation_is_ignored() {
        let mut service = SystemService::default();
        let _ = service.pulse_disconnected();
        let accepted = service.update_audio(AudioSnapshot {
            availability: ServiceAvailability::Available,
            generation: 0,
            server_generation: 0,
            volume_percent: 90,
            ..AudioSnapshot::default()
        });
        assert!(!accepted);
    }

    #[test]
    fn reconnect_backoff_is_bounded() {
        let mut backoff = ReconnectBackoff::default();
        let mut last = 0;
        for _ in 0..32 {
            last = backoff.next_delay_ms();
        }
        assert_eq!(last, 30_000);
    }
}
