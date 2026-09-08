//! Transactional geometry commit + presentation state machine.
//! Pure state logic lives here; the X11 side-effects stay in `app/`.

/// Native presentation lifecycle. Only forward transitions are legal;
/// `Mapped` requires a prior `MapNotify` (see `note_mapped`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentationState {
    #[default]
    CreatedHidden,
    Projected,
    GeometryCommitted,
    Painted,
    Mapped,
    Presented,
}

impl PresentationState {
    /// Attempt a forward transition. Rejects skips and any backward move.
    pub fn advance_to(self, next: Self) -> Result<Self, String> {
        let order = |s: Self| match s {
            Self::CreatedHidden => 0,
            Self::Projected => 1,
            Self::GeometryCommitted => 2,
            Self::Painted => 3,
            Self::Mapped => 4,
            Self::Presented => 5,
        };
        if order(next) == order(self) + 1 {
            Ok(next)
        } else {
            Err(presenter_error(&format!(
                "invalid presentation transition {self:?} -> {next:?}"
            )))
        }
    }

    /// Record a MapNotify: only Painted -> Mapped is legal.
    pub fn note_mapped(self) -> Result<Self, String> {
        self.advance_to(Self::Mapped)
    }
}

/// Error constructor that never swallows context on first present.
pub fn presenter_error(detail: &str) -> String {
    format!("native presenter: {detail}")
}

/// Transactional geometry commit plan: retained size first, then backbuffer,
/// retarget, resize, shape, repaint, flush. Callers apply each step in order
/// and abort (no partial window resize) when any step fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeometryCommit {
    pub width: u32,
    pub height: u32,
}

impl GeometryCommit {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let (w, h) = (width.max(1), height.max(1));
        if w > 16384 || h > 16384 {
            return Err(presenter_error("geometry commit exceeds 16384px limit"));
        }
        Ok(Self {
            width: w,
            height: h,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_skipped_and_backward_transitions() {
        let s = PresentationState::CreatedHidden;
        assert!(s.advance_to(PresentationState::GeometryCommitted).is_err());
        let s = PresentationState::Painted;
        assert!(s.advance_to(PresentationState::GeometryCommitted).is_err());
        assert_eq!(
            s.advance_to(PresentationState::Mapped).unwrap(),
            PresentationState::Mapped
        );
    }

    #[test]
    fn mapped_requires_painted_first() {
        assert!(PresentationState::CreatedHidden.note_mapped().is_err());
        assert!(PresentationState::Painted.note_mapped().is_ok());
    }

    #[test]
    fn commit_clamps_and_rejects_huge() {
        assert_eq!(
            GeometryCommit::new(0, 0).unwrap(),
            GeometryCommit {
                width: 1,
                height: 1
            }
        );
        assert!(GeometryCommit::new(20000, 10).is_err());
    }
}
