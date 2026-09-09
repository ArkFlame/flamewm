//! ARGB surface format validation (J3/O3).
//! Pure checks shared by the X11 surface owner: depth-32 TrueColor visual
//! plus a non-zero XRender alpha mask select the composited path.

use crate::xlib::TRUE_COLOR_CLASS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceAlphaMode {
    CompositedArgb32,
    ShapeBackedArgb32,
    OpaqueFallback,
}

/// Validate a matched visual + XRender alpha mask.
/// Rejects `alpha_mask == 0` (no real alpha channel) into the shape/opaque path.
/// Also requires a compositing manager selection owner for the composited path;
/// without a compositor the caller must use [`fallback_mode`].
pub fn validate_argb32_visual(
    depth: i32,
    class: i32,
    alpha_mask: u32,
) -> Result<SurfaceAlphaMode, String> {
    if depth != 32 || class != TRUE_COLOR_CLASS {
        return Err(format!(
            "ARGB32 visual requires depth 32 TrueColor(4); got depth={depth} class={class}"
        ));
    }
    if alpha_mask == 0 {
        return Err("ARGB32 visual has zero alphaMask; no real alpha channel".to_string());
    }
    Ok(SurfaceAlphaMode::CompositedArgb32)
}

/// Downgrade decision when composited alpha is unavailable.
pub fn fallback_mode(shape_available: bool) -> SurfaceAlphaMode {
    if shape_available {
        SurfaceAlphaMode::ShapeBackedArgb32
    } else {
        SurfaceAlphaMode::OpaqueFallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xlib::TRANSPARENT_CLEAR_PIXEL;

    #[test]
    fn rejects_zero_alpha_mask() {
        assert!(validate_argb32_visual(32, TRUE_COLOR_CLASS, 0).is_err());
    }

    #[test]
    fn accepts_real_argb32_visual() {
        assert_eq!(
            validate_argb32_visual(32, TRUE_COLOR_CLASS, 0xFF00_0000).unwrap(),
            SurfaceAlphaMode::CompositedArgb32
        );
    }

    #[test]
    fn rejects_wrong_depth_or_class() {
        assert!(validate_argb32_visual(24, TRUE_COLOR_CLASS, 0xFF00_0000).is_err());
        assert!(validate_argb32_visual(32, 1, 0xFF00_0000).is_err());
    }

    #[test]
    fn transparent_clear_is_zero() {
        assert_eq!(TRANSPARENT_CLEAR_PIXEL, 0);
    }
}
