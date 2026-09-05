use std::collections::BTreeSet;

use flamewm_api::system::{
    NetworkAccessPointSnapshot, NetworkKind, NetworkSnapshot, ServiceAvailability,
};
use flamewm_api::{ErrorCode, FlameError, FlameResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessPoint {
    pub path: String,
    pub ssid: String,
    pub strength_percent: u8,
    pub secured: bool,
    pub known: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkStatus {
    pub snapshot: NetworkSnapshot,
    pub active_path: String,
    pub visible_access_points: Vec<AccessPoint>,
    pub networking_enabled: bool,
}

impl Default for NetworkStatus {
    fn default() -> Self {
        Self {
            snapshot: NetworkSnapshot::default(),
            active_path: String::new(),
            visible_access_points: Vec::new(),
            networking_enabled: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkIntent {
    SetWifiEnabled(bool),
    ConnectKnown {
        access_point_path: String,
    },
    /// The password is intentionally absent. Secret acquisition belongs to NetworkManager's
    /// secret-agent/host-settings path and must never enter argv, logs or FlameWM plaintext state.
    ConnectNewSecure {
        access_point_path: String,
    },
    Disconnect,
    Scan,
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
            initial_ms: 250,
            maximum_ms: 8_000,
            maximum_attempts: 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkController {
    generation: u64,
    status: NetworkStatus,
    available: bool,
    reconnect_attempts: u8,
    reconnect_pending: bool,
    backoff: BackoffConfig,
    ap_refresh_pending: bool,
    known_profiles: BTreeSet<String>,
}

impl Default for NetworkController {
    fn default() -> Self {
        Self {
            generation: 0,
            status: NetworkStatus::default(),
            available: false,
            reconnect_attempts: 0,
            reconnect_pending: false,
            backoff: BackoffConfig::default(),
            ap_refresh_pending: false,
            known_profiles: BTreeSet::new(),
        }
    }
}

impl NetworkController {
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.available
    }

    #[must_use]
    pub fn status(&self) -> &NetworkStatus {
        &self.status
    }

    /// Produce the complete shell-facing snapshot without exposing NetworkManager adapter state.
    #[must_use]
    pub fn snapshot(&self) -> NetworkSnapshot {
        let mut snapshot = self.status.snapshot.clone();
        snapshot.generation = self.generation;
        snapshot.networking_enabled = self.status.networking_enabled;
        snapshot.active_path = self.status.active_path.clone();
        snapshot.access_points = self
            .status
            .visible_access_points
            .iter()
            .map(|ap| NetworkAccessPointSnapshot {
                path: ap.path.clone(),
                ssid: ap.ssid.clone(),
                strength_percent: ap.strength_percent.min(100),
                secured: ap.secured,
                known: ap.known,
            })
            .collect();
        snapshot
    }

    pub fn service_appeared(&mut self) {
        self.generation = self.generation.saturating_add(1).max(1);
        self.available = true;
        self.reconnect_attempts = 0;
        self.reconnect_pending = false;
        self.status.snapshot.availability = ServiceAvailability::Available;
        self.status.snapshot.generation = self.generation;
    }

    pub fn service_vanished(&mut self) {
        self.generation = self.generation.saturating_add(1).max(1);
        self.available = false;
        self.status = NetworkStatus {
            snapshot: NetworkSnapshot {
                availability: ServiceAvailability::Unavailable,
                generation: self.generation,
                kind: NetworkKind::Unavailable,
                ..NetworkSnapshot::default()
            },
            ..NetworkStatus::default()
        };
        self.known_profiles.clear();
        self.ap_refresh_pending = false;
        self.reconnect_pending = self.reconnect_attempts < self.backoff.maximum_attempts;
    }

    pub fn inject_status(&mut self, mut status: NetworkStatus, captured_generation: u64) -> bool {
        if captured_generation != self.generation {
            return false;
        }
        status.snapshot.generation = self.generation;
        status.snapshot.strength_percent = status.snapshot.strength_percent.min(100);
        for ap in &mut status.visible_access_points {
            ap.strength_percent = ap.strength_percent.min(100);
            if ap.known {
                self.known_profiles.insert(ap.path.clone());
            }
        }
        self.status = status;
        true
    }

    pub fn request_set_wifi_enabled(&self, enabled: bool) -> FlameResult<NetworkIntent> {
        self.ensure_available()?;
        Ok(NetworkIntent::SetWifiEnabled(enabled))
    }

    pub fn request_connect_known(
        &self,
        access_point_path: &str,
        caller_generation: u64,
    ) -> FlameResult<NetworkIntent> {
        self.validate_generation(caller_generation)?;
        self.ensure_available()?;
        if access_point_path.is_empty() || !self.known_profiles.contains(access_point_path) {
            return Err(FlameError::new(
                ErrorCode::NotFound,
                "saved NetworkManager access point was not found",
            ));
        }
        Ok(NetworkIntent::ConnectKnown {
            access_point_path: access_point_path.to_owned(),
        })
    }

    pub fn request_connect_new_secure(
        &self,
        access_point_path: &str,
        caller_generation: u64,
    ) -> FlameResult<NetworkIntent> {
        self.validate_generation(caller_generation)?;
        self.ensure_available()?;
        let Some(ap) = self
            .status
            .visible_access_points
            .iter()
            .find(|ap| ap.path == access_point_path)
        else {
            return Err(FlameError::new(
                ErrorCode::NotFound,
                "NetworkManager access point was not found",
            ));
        };
        if !ap.secured || ap.known {
            return Err(FlameError::new(
                ErrorCode::InvalidArgument,
                "secure-new-network intent requires an unknown secured access point",
            ));
        }
        Ok(NetworkIntent::ConnectNewSecure {
            access_point_path: access_point_path.to_owned(),
        })
    }

    pub fn request_disconnect(&self, caller_generation: u64) -> FlameResult<NetworkIntent> {
        self.validate_generation(caller_generation)?;
        self.ensure_available()?;
        Ok(NetworkIntent::Disconnect)
    }

    pub fn request_scan(&mut self) -> FlameResult<Option<NetworkIntent>> {
        self.ensure_available()?;
        if self.ap_refresh_pending {
            return Ok(None);
        }
        self.ap_refresh_pending = true;
        Ok(Some(NetworkIntent::Scan))
    }

    pub fn complete_scan(&mut self, captured_generation: u64) -> bool {
        if captured_generation != self.generation {
            return false;
        }
        self.ap_refresh_pending = false;
        true
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

    fn validate_generation(&self, caller_generation: u64) -> FlameResult<()> {
        if caller_generation != self.generation {
            Err(FlameError::new(
                ErrorCode::StaleRevision,
                "stale NetworkManager service generation",
            ))
        } else {
            Ok(())
        }
    }

    fn ensure_available(&self) -> FlameResult<()> {
        if self.available {
            Ok(())
        } else {
            Err(FlameError::new(
                ErrorCode::Unavailable,
                "NetworkManager is unavailable",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_loss_clears_stale_paths_and_invalidates_callbacks() {
        let mut controller = NetworkController::default();
        controller.service_appeared();
        let generation = controller.generation();
        controller.service_vanished();
        assert!(!controller.inject_status(NetworkStatus::default(), generation));
    }

    #[test]
    fn protected_network_intent_never_contains_secret() {
        let intent = NetworkIntent::ConnectNewSecure {
            access_point_path: "/ap/1".to_owned(),
        };
        assert_eq!(
            format!("{intent:?}"),
            "ConnectNewSecure { access_point_path: \"/ap/1\" }"
        );
    }

    #[test]
    fn shell_snapshot_carries_visible_access_points_without_secrets() {
        let mut controller = NetworkController::default();
        controller.service_appeared();
        let generation = controller.generation();
        let _ = controller.inject_status(
            NetworkStatus {
                snapshot: NetworkSnapshot {
                    wifi_enabled: true,
                    ..NetworkSnapshot::default()
                },
                active_path: "/ap/1".to_owned(),
                visible_access_points: vec![AccessPoint {
                    path: "/ap/1".to_owned(),
                    ssid: "FlameNet".to_owned(),
                    strength_percent: 140,
                    secured: true,
                    known: true,
                }],
                networking_enabled: true,
            },
            generation,
        );
        assert_eq!(controller.snapshot().access_points[0].strength_percent, 100);
    }

    #[test]
    fn reconnect_attempts_are_bounded() {
        let mut controller = NetworkController::default();
        controller.service_vanished();
        let mut attempts = 0;
        while controller.next_reconnect_delay_ms().is_some() {
            attempts += 1;
        }
        assert_eq!(attempts, 8);
    }
}
