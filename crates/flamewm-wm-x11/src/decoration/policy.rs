//! Pure decoration-policy decision (no X calls, no rendering).
//!
//! J12 ruling: no verified GTK CSD atom exists in source/reference, so no
//! CSD indicator is invented here. `ClientDecorated` is reserved and
//! currently unreachable; it exists so a future verified hint can map to
//! "managed without Flame frame" without a redesign. `_GTK_FRAME_EXTENTS`
//! alone is never a decoration signal, and `WM_CLASS` is never consulted.

/// Who draws window borders/titlebar for a managed client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DecorationPolicy {
    /// Flame creates and paints its frame; full EWMH action set applies.
    #[default]
    ServerDecorated,
    /// Reserved: client draws its own decorations. Managed without a Flame
    /// frame. Unreachable until a verified CSD indicator lands.
    #[cfg(test)]
    ClientDecorated,
    /// No Flame frame (override-redirect, non-normal EWMH type, or an
    /// explicit Motif `decorations == 0` no-decoration request).
    #[cfg(test)]
    Undecorated,
}

impl DecorationPolicy {
    #[cfg(test)]
    #[must_use]
    pub const fn has_flame_frame(self) -> bool {
        matches!(self, Self::ServerDecorated)
    }
}

/// Inputs to the pure decision. All values are already-read hints.
#[cfg(test)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DecorationRequest {
    /// ICCCM `override_redirect` attribute.
    pub override_redirect: bool,
    /// True when the window carries no EWMH type or only
    /// `_NET_WM_WINDOW_TYPE_NORMAL`.
    pub is_normal_type: bool,
    /// True when `_MOTIF_WM_HINTS` carries `flags & (1 << 1) != 0`
    /// with `decorations == 0` (explicit no-decoration request).
    pub motif_no_decorations: bool,
}

/// Pure mapping. Order matters: override-redirect wins, then non-normal
/// types, then the Motif no-decoration request. Anything else keeps the
/// server frame.
#[cfg(test)]
#[must_use]
pub const fn decide(request: DecorationRequest) -> DecorationPolicy {
    if request.override_redirect {
        return DecorationPolicy::Undecorated;
    }
    if !request.is_normal_type {
        return DecorationPolicy::Undecorated;
    }
    if request.motif_no_decorations {
        return DecorationPolicy::Undecorated;
    }
    DecorationPolicy::ServerDecorated
}

/// ICCCM `WM_NORMAL_HINTS` flag bits (subset used here). Canonical
/// definitions live in [`crate::size_hints`]; re-exported here so existing
/// `policy::` test sites keep compiling.
#[cfg(test)]
pub use crate::size_hints::{HINT_P_MAX_SIZE, HINT_P_MIN_SIZE};

#[cfg(test)]
use crate::size_hints::ClientSizeHints;

/// Pure resizable test from already-read `WM_NORMAL_HINTS` cardinals.
///
/// `values[0]` is flags; `PMinSize` occupies indices 5..=6 and `PMaxSize`
/// indices 7..=8 when their bits are set. A window is fixed-size only when
/// both are present and equal (and non-degenerate); every other
/// hint-free or partial state stays resizable.
#[cfg(test)]
#[must_use]
pub fn normal_hints_resizable(values: &[u32]) -> bool {
    ClientSizeHints::parse(values).resizable
}

/// Parse raw `_MOTIF_WM_HINTS` cardinals (`flags, functions, decorations,
/// input_mode, status`). Returns true only for an explicit no-decoration
/// request: decorations flag present (`flags & (1 << 1)`) and
/// `decorations == 0`.
#[cfg(test)]
#[must_use]
pub fn motif_no_decorations(values: &[u32]) -> bool {
    if values.len() < 3 {
        return false;
    }
    values[0] & (1 << 1) != 0 && values[2] == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_redirect_is_undecorated() {
        let policy = decide(DecorationRequest {
            override_redirect: true,
            is_normal_type: true,
            motif_no_decorations: false,
        });
        assert_eq!(policy, DecorationPolicy::Undecorated);
        assert!(!policy.has_flame_frame());
    }

    #[test]
    fn non_normal_type_is_undecorated() {
        let policy = decide(DecorationRequest {
            override_redirect: false,
            is_normal_type: false,
            motif_no_decorations: false,
        });
        assert_eq!(policy, DecorationPolicy::Undecorated);
    }

    #[test]
    fn motif_zero_decorations_is_undecorated() {
        let policy = decide(DecorationRequest {
            override_redirect: false,
            is_normal_type: true,
            motif_no_decorations: true,
        });
        assert_eq!(policy, DecorationPolicy::Undecorated);
    }

    #[test]
    fn normal_hint_free_is_server_decorated() {
        let policy = decide(DecorationRequest {
            override_redirect: false,
            is_normal_type: true,
            motif_no_decorations: false,
        });
        assert_eq!(policy, DecorationPolicy::ServerDecorated);
        assert!(policy.has_flame_frame());
    }

    #[test]
    fn client_decorated_variant_has_no_frame() {
        assert!(!DecorationPolicy::ClientDecorated.has_flame_frame());
    }

    #[test]
    fn empty_hints_stay_resizable() {
        assert!(normal_hints_resizable(&[]));
        assert!(normal_hints_resizable(&[0]));
    }

    #[test]
    fn equal_min_max_is_fixed() {
        let values = vec![
            HINT_P_MIN_SIZE | HINT_P_MAX_SIZE,
            0,
            0,
            0,
            0,
            400,
            300,
            400,
            300,
        ];
        assert!(!normal_hints_resizable(&values));
    }

    #[test]
    fn unequal_min_max_stays_resizable() {
        let values = vec![
            HINT_P_MIN_SIZE | HINT_P_MAX_SIZE,
            0,
            0,
            0,
            0,
            100,
            100,
            800,
            600,
        ];
        assert!(normal_hints_resizable(&values));
    }

    #[test]
    fn motif_parse_requires_flag_and_zero() {
        assert!(motif_no_decorations(&[2, 0, 0, 0, 0]));
        assert!(!motif_no_decorations(&[0, 0, 0, 0, 0]));
        assert!(!motif_no_decorations(&[2, 0, 1, 0, 0]));
        assert!(!motif_no_decorations(&[2, 0]));
    }
}
