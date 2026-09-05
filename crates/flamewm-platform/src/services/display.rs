use flamewm_api::display::{DisplaySnapshot, PendingModeChange};
use flamewm_api::ports::DisplayPort;
use flamewm_api::{ErrorCode, FlameError, FlameResult, ModeId, OutputId, TransactionId};

use crate::display::DisplayCoordinator;
use crate::scale::is_supported_scale;

#[derive(Debug, Default)]
pub struct DisplayService {
    coordinator: DisplayCoordinator,
    cached: Option<DisplaySnapshot>,
    engine_generation: u64,
}

impl DisplayService {
    pub fn refresh<P: DisplayPort>(&mut self, port: &mut P) -> FlameResult<&DisplaySnapshot> {
        let mut fresh = port.display_snapshot()?;
        let raw_generation = fresh.generation;
        let pending_output_missing = self.coordinator.pending_output_missing(
            fresh
                .outputs
                .iter()
                .filter(|output| output.connected)
                .map(|output| &output.id),
        );
        if self.coordinator.topology_is_stale(raw_generation) || pending_output_missing {
            let transaction = self
                .coordinator
                .pending()
                .map(|pending| pending.transaction)
                .ok_or_else(|| {
                    FlameError::new(ErrorCode::InternalFailure, "missing display transaction")
                })?;
            port.revert_mode(transaction)?;
            self.coordinator.revert(transaction)?;
            fresh = port.display_snapshot()?;
        }
        let raw_generation = fresh.generation;
        let next_product_generation = match self.cached.as_ref() {
            None => raw_generation.max(1),
            Some(previous) if self.engine_generation != raw_generation => {
                previous.generation.saturating_add(1).max(1)
            }
            Some(previous) => previous.generation,
        };
        if let Some(previous) = self.cached.as_ref() {
            for output in &mut fresh.outputs {
                if let Some(old) = previous
                    .outputs
                    .iter()
                    .find(|candidate| candidate.id == output.id)
                {
                    output.shell_scale_percent = old.shell_scale_percent;
                }
            }
        }
        fresh.generation = next_product_generation;
        fresh.pending = self.coordinator.pending().map(to_api_pending);
        self.engine_generation = raw_generation;
        self.cached = Some(fresh);
        self.cached.as_ref().ok_or_else(|| {
            FlameError::new(ErrorCode::InternalFailure, "display snapshot cache failed")
        })
    }

    #[must_use]
    pub fn cached(&self) -> Option<&DisplaySnapshot> {
        self.cached.as_ref()
    }

    pub fn begin_mode<P: DisplayPort>(
        &mut self,
        port: &mut P,
        output: &OutputId,
        mode: ModeId,
        expected_generation: u64,
        now_ms: u64,
    ) -> FlameResult<TransactionId> {
        if self.coordinator.pending().is_some() {
            return Err(FlameError::new(
                ErrorCode::Busy,
                "display transaction already pending",
            ));
        }
        if self.cached.is_none() {
            self.refresh(port)?;
        }
        let cached = self.cached.as_ref().ok_or_else(|| {
            FlameError::new(ErrorCode::InternalFailure, "missing display snapshot")
        })?;
        require_generation(cached, expected_generation)?;
        let fresh = port.display_snapshot()?;
        if self.engine_generation != 0 && fresh.generation != self.engine_generation {
            self.refresh(port)?;
            return Err(FlameError::new(
                ErrorCode::StaleRevision,
                "display topology changed before mode apply",
            ));
        }
        let selected = fresh
            .outputs
            .iter()
            .find(|candidate| candidate.connected && candidate.id == *output)
            .ok_or_else(|| FlameError::new(ErrorCode::NotFound, "display output not found"))?;
        if !selected.modes.iter().any(|candidate| candidate.id == mode) {
            return Err(FlameError::new(
                ErrorCode::InvalidArgument,
                "mode does not belong to output",
            ));
        }
        if selected.current_mode == mode {
            return Err(FlameError::new(
                ErrorCode::InvalidArgument,
                "display mode is already active",
            ));
        }

        let product_generation = cached.generation;
        let scale_by_output = cached
            .outputs
            .iter()
            .map(|candidate| (candidate.id.clone(), candidate.shell_scale_percent))
            .collect::<std::collections::BTreeMap<_, _>>();

        let transaction = port.apply_mode(output, mode)?;
        if let Err(error) =
            self.coordinator
                .begin(transaction, output.clone(), mode, fresh.generation, now_ms)
        {
            let _ = port.revert_mode(transaction);
            return Err(error);
        }
        let mut post = port.display_snapshot()?;
        self.engine_generation = post.generation;
        restore_shell_scales(&mut post, &scale_by_output);
        post.generation = product_generation.saturating_add(1).max(1);
        post.pending = self.coordinator.pending().map(to_api_pending);
        self.cached = Some(post);
        Ok(transaction)
    }

    pub fn keep<P: DisplayPort>(
        &mut self,
        port: &mut P,
        transaction: TransactionId,
    ) -> FlameResult<()> {
        require_transaction(&self.coordinator, transaction)?;
        let (product_generation, scales) = self.product_generation_and_scales();
        port.keep_mode(transaction)?;
        self.coordinator.keep(transaction)?;
        let mut fresh = port.display_snapshot()?;
        self.engine_generation = fresh.generation;
        restore_shell_scales(&mut fresh, &scales);
        fresh.generation = product_generation;
        fresh.pending = None;
        self.cached = Some(fresh);
        Ok(())
    }

    pub fn revert<P: DisplayPort>(
        &mut self,
        port: &mut P,
        transaction: TransactionId,
    ) -> FlameResult<()> {
        require_transaction(&self.coordinator, transaction)?;
        let (product_generation, scales) = self.product_generation_and_scales();
        port.revert_mode(transaction)?;
        self.coordinator.revert(transaction)?;
        let mut fresh = port.display_snapshot()?;
        self.engine_generation = fresh.generation;
        restore_shell_scales(&mut fresh, &scales);
        fresh.generation = product_generation.saturating_add(1).max(1);
        fresh.pending = None;
        self.cached = Some(fresh);
        Ok(())
    }

    pub fn tick<P: DisplayPort>(&mut self, port: &mut P, now_ms: u64) -> FlameResult<bool> {
        if !self.coordinator.is_expired(now_ms) {
            return Ok(false);
        }
        let transaction = self
            .coordinator
            .pending()
            .map(|pending| pending.transaction)
            .ok_or_else(|| {
                FlameError::new(ErrorCode::InternalFailure, "missing display transaction")
            })?;
        self.revert(port, transaction)?;
        Ok(true)
    }

    fn product_generation_and_scales(&self) -> (u64, std::collections::BTreeMap<OutputId, u16>) {
        let generation = self
            .cached
            .as_ref()
            .map_or(1, |snapshot| snapshot.generation.max(1));
        let scales = self
            .cached
            .as_ref()
            .map(|snapshot| {
                snapshot
                    .outputs
                    .iter()
                    .map(|output| (output.id.clone(), output.shell_scale_percent))
                    .collect()
            })
            .unwrap_or_default();
        (generation, scales)
    }

    pub fn set_shell_scale(
        &mut self,
        output: &OutputId,
        scale_percent: u16,
        expected_generation: u64,
    ) -> FlameResult<bool> {
        if !is_supported_scale(scale_percent) {
            return Err(FlameError::new(
                ErrorCode::InvalidArgument,
                "shell scale must be 100/125/150/175/200",
            ));
        }
        let snapshot = self.cached.as_mut().ok_or_else(|| {
            FlameError::new(
                ErrorCode::Unavailable,
                "display snapshot is not initialized",
            )
        })?;
        require_generation(snapshot, expected_generation)?;
        let selected = snapshot
            .outputs
            .iter_mut()
            .find(|candidate| candidate.connected && candidate.id == *output)
            .ok_or_else(|| FlameError::new(ErrorCode::NotFound, "display output not found"))?;
        if selected.shell_scale_percent == scale_percent {
            return Ok(false);
        }
        selected.shell_scale_percent = scale_percent;
        snapshot.generation = snapshot.generation.saturating_add(1);
        Ok(true)
    }
}

fn restore_shell_scales(
    snapshot: &mut DisplaySnapshot,
    scales: &std::collections::BTreeMap<OutputId, u16>,
) {
    for output in &mut snapshot.outputs {
        if let Some(scale) = scales.get(&output.id) {
            output.shell_scale_percent = *scale;
        }
    }
}

fn require_generation(snapshot: &DisplaySnapshot, expected: u64) -> FlameResult<()> {
    if snapshot.generation == expected {
        Ok(())
    } else {
        Err(FlameError::new(
            ErrorCode::StaleRevision,
            "display topology generation is stale",
        ))
    }
}

fn require_transaction(
    coordinator: &DisplayCoordinator,
    transaction: TransactionId,
) -> FlameResult<()> {
    match coordinator.pending() {
        Some(pending) if pending.transaction == transaction => Ok(()),
        Some(_) => Err(FlameError::new(
            ErrorCode::Conflict,
            "display transaction id mismatch",
        )),
        None => Err(FlameError::new(
            ErrorCode::NotFound,
            "no pending display transaction",
        )),
    }
}

fn to_api_pending(pending: &crate::display::PendingDisplayChange) -> PendingModeChange {
    PendingModeChange {
        transaction: pending.transaction,
        output: pending.output.clone(),
        mode: pending.mode,
        deadline_ms: pending.deadline_ms,
    }
}
