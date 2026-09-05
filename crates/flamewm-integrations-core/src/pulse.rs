use flamewm_api::system::{AudioSnapshot, ServiceAvailability};
use flamewm_api::{ErrorCode, FlameError, FlameResult};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PulseState {
    #[default]
    Unavailable,
    Connecting,
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PulseStatus {
    pub state: PulseState,
    pub sink_name: String,
    pub volume_percent: u8,
    pub muted: bool,
}

impl Default for PulseStatus {
    fn default() -> Self {
        Self {
            state: PulseState::Unavailable,
            sink_name: String::new(),
            volume_percent: 0,
            muted: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PulseIntent {
    SetVolume(u8),
    SetMute(bool),
    ReconcileSink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackoffConfig {
    pub initial_ms: u64,
    pub maximum_ms: u64,
    pub maximum_attempts: u8,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_ms: 300,
            maximum_ms: 8_000,
            maximum_attempts: 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PulseController {
    generation: u64,
    server_generation: u64,
    status: PulseStatus,
    reconnect_attempts: u8,
    reconnect_pending: bool,
    backoff: BackoffConfig,
    sink_reconcile_pending: bool,
}

impl Default for PulseController {
    fn default() -> Self {
        Self {
            generation: 0,
            server_generation: 0,
            status: PulseStatus::default(),
            reconnect_attempts: 0,
            reconnect_pending: false,
            backoff: BackoffConfig::default(),
            sink_reconcile_pending: false,
        }
    }
}

impl PulseController {
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    #[must_use]
    pub const fn server_generation(&self) -> u64 {
        self.server_generation
    }

    pub fn connect_started(&mut self) {
        self.generation = self.generation.saturating_add(1).max(1);
        self.status.state = PulseState::Connecting;
    }

    pub fn ready(&mut self, captured_generation: u64) -> bool {
        if captured_generation != self.generation {
            return false;
        }
        self.status.state = PulseState::Ready;
        self.reconnect_attempts = 0;
        self.reconnect_pending = false;
        true
    }

    pub fn server_restarted(&mut self) {
        self.generation = self.generation.saturating_add(1).max(1);
        self.server_generation = self.server_generation.saturating_add(1).max(1);
        self.status = PulseStatus::default();
        self.sink_reconcile_pending = false;
        self.reconnect_pending = true;
    }

    pub fn sink_info(
        &mut self,
        sink_name: &str,
        volume_percent: u8,
        muted: bool,
        captured_generation: u64,
        captured_server_generation: u64,
    ) -> bool {
        if captured_generation != self.generation
            || captured_server_generation != self.server_generation
        {
            return false;
        }
        self.status = PulseStatus {
            state: PulseState::Ready,
            sink_name: sink_name.to_owned(),
            volume_percent: volume_percent.min(100),
            muted,
        };
        self.sink_reconcile_pending = false;
        true
    }

    pub fn subscription_event(&mut self, captured_generation: u64) -> bool {
        if captured_generation != self.generation || self.sink_reconcile_pending {
            return false;
        }
        self.sink_reconcile_pending = true;
        true
    }

    pub fn take_reconcile_intent(&mut self, captured_generation: u64) -> Option<PulseIntent> {
        if captured_generation != self.generation || !self.sink_reconcile_pending {
            return None;
        }
        self.sink_reconcile_pending = false;
        Some(PulseIntent::ReconcileSink)
    }

    pub fn set_volume(&self, percent: u8, caller_generation: u64) -> FlameResult<PulseIntent> {
        self.validate_ready(caller_generation)?;
        Ok(PulseIntent::SetVolume(percent.min(100)))
    }

    pub fn set_mute(&self, muted: bool, caller_generation: u64) -> FlameResult<PulseIntent> {
        self.validate_ready(caller_generation)?;
        Ok(PulseIntent::SetMute(muted))
    }

    pub fn next_reconnect_delay_ms(&mut self) -> Option<u64> {
        if !self.reconnect_pending || self.reconnect_attempts >= self.backoff.maximum_attempts {
            self.reconnect_pending = false;
            return None;
        }
        let shift = u32::from(self.reconnect_attempts.min(7));
        let delay = self
            .backoff
            .initial_ms
            .saturating_mul(1_u64 << shift)
            .min(self.backoff.maximum_ms);
        self.reconnect_attempts = self.reconnect_attempts.saturating_add(1);
        if self.reconnect_attempts >= self.backoff.maximum_attempts {
            self.reconnect_pending = false;
        }
        Some(delay)
    }

    #[must_use]
    pub fn snapshot(&self) -> AudioSnapshot {
        AudioSnapshot {
            availability: if self.status.state == PulseState::Ready {
                ServiceAvailability::Available
            } else {
                ServiceAvailability::Unavailable
            },
            generation: self.generation,
            server_generation: self.server_generation,
            sink_name: self.status.sink_name.clone(),
            volume_percent: self.status.volume_percent,
            muted: self.status.muted,
        }
    }

    fn validate_ready(&self, caller_generation: u64) -> FlameResult<()> {
        if caller_generation != self.generation {
            return Err(FlameError::new(
                ErrorCode::StaleRevision,
                "stale PulseAudio generation",
            ));
        }
        if self.status.state != PulseState::Ready {
            return Err(FlameError::new(
                ErrorCode::Unavailable,
                "PulseAudio/PipeWire-Pulse is unavailable",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_server_callback_is_rejected_after_restart() {
        let mut pulse = PulseController::default();
        pulse.connect_started();
        let generation = pulse.generation();
        let server_generation = pulse.server_generation();
        pulse.server_restarted();
        assert!(!pulse.sink_info("old", 90, false, generation, server_generation));
    }

    #[test]
    fn subscription_burst_coalesces_to_one_reconcile() {
        let mut pulse = PulseController::default();
        pulse.connect_started();
        let generation = pulse.generation();
        assert!(pulse.subscription_event(generation));
        assert!(!pulse.subscription_event(generation));
        assert_eq!(
            pulse.take_reconcile_intent(generation),
            Some(PulseIntent::ReconcileSink)
        );
    }
}
