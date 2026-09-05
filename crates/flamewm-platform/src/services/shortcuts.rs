use flamewm_api::ports::ShortcutPort;
use flamewm_api::shortcuts::{KeyBinding, ShortcutSnapshot};
use flamewm_api::{ErrorCode, FlameError, FlameResult};

use crate::shortcuts::ShortcutRegistry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutService {
    registry: ShortcutRegistry,
    revision: u64,
}

impl Default for ShortcutService {
    fn default() -> Self {
        Self {
            registry: ShortcutRegistry::default(),
            revision: 1,
        }
    }
}

impl ShortcutService {
    #[must_use]
    pub fn snapshot(&self) -> ShortcutSnapshot {
        ShortcutSnapshot {
            revision: self.revision,
            bindings: self.registry.bindings().clone(),
        }
    }

    pub fn set_binding<P: ShortcutPort>(
        &mut self,
        port: &mut P,
        action: &str,
        binding: &str,
        expected_revision: u64,
    ) -> FlameResult<()> {
        self.require_revision(expected_revision)?;
        let mut candidate = self.registry.clone();
        candidate.set(action, binding)?;
        self.commit_candidate(port, candidate)
    }

    pub fn clear_binding<P: ShortcutPort>(
        &mut self,
        port: &mut P,
        action: &str,
        expected_revision: u64,
    ) -> FlameResult<()> {
        self.require_revision(expected_revision)?;
        let mut candidate = self.registry.clone();
        candidate.clear(action)?;
        self.commit_candidate(port, candidate)
    }

    pub fn reset_binding<P: ShortcutPort>(
        &mut self,
        port: &mut P,
        action: &str,
        expected_revision: u64,
    ) -> FlameResult<()> {
        self.require_revision(expected_revision)?;
        let mut candidate = self.registry.clone();
        candidate.reset_one(action)?;
        self.commit_candidate(port, candidate)
    }

    pub fn apply<P: ShortcutPort>(
        &mut self,
        port: &mut P,
        bindings: &std::collections::BTreeMap<String, KeyBinding>,
        expected_revision: u64,
    ) -> FlameResult<()> {
        self.require_revision(expected_revision)?;
        let mut candidate = self.registry.clone();
        candidate.replace_all(bindings)?;
        self.commit_candidate(port, candidate)
    }

    fn commit_candidate<P: ShortcutPort>(
        &mut self,
        port: &mut P,
        candidate: ShortcutRegistry,
    ) -> FlameResult<()> {
        if candidate == self.registry {
            return Ok(());
        }
        if let Err(error) = port.prepare_shortcuts(candidate.bindings()) {
            port.rollback_shortcuts();
            return Err(error);
        }
        if let Err(error) = port.commit_shortcuts() {
            port.rollback_shortcuts();
            return Err(error);
        }
        self.registry = candidate;
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    fn require_revision(&self, expected: u64) -> FlameResult<()> {
        if expected == self.revision {
            Ok(())
        } else {
            Err(FlameError::new(
                ErrorCode::StaleRevision,
                "shortcut revision is stale",
            ))
        }
    }
}
