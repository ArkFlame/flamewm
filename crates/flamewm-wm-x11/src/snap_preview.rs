//! Serialized snap-preview overlay (J11).
//!
//! The preview is a compiled `snap-preview.html` document hosted on a
//! `SurfaceRole::Overlay` + `SurfaceInputMode::PassThrough` surface, hidden
//! at init. It never duplicates snap math: candidates come from
//! `flamewm_window_core::snap_target` and geometry from
//! `flamewm_window_core::snap_geometry`, both resolved by the caller in
//! `wm.rs`. This module only owns surface lifecycle and the skip-redraw
//! guard (no polling, no timers).
//!
//! Opacity: the compiled CSS bakes the 20% default fill
//! (`--snap-fill:#ef404833`). Live settings are not plumbed to the WM event
//! loop and no new protocol is introduced here, so callers pass
//! `DEFAULT_PREVIEW_OPACITY_PERCENT` unless they already hold a live value.

use flamewm_api::Rect;
use flamewm_render_x11::backdrop;
use flamewm_ui_x11::{
    RuntimeImage, SurfaceConfig, SurfaceHandle, SurfaceInputMode, SurfaceRole, SurfaceRuntime,
    UiColor, UiDocumentAccess, UiTemplate, decode_document,
};

use flamewm_window_core::SnapTarget;

const COMPILED_UI: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/flamewm-snap-preview.rwr"));

/// Default fill opacity when live settings are unavailable (no new protocol).
pub const DEFAULT_PREVIEW_OPACITY_PERCENT: u8 = 20;

/// Accent red backing the preview border/fill (`#ef4048`): shared with
/// the desktop-selection material owner, one material two consumers.
pub const ACCENT_RGB: (u8, u8, u8) = backdrop::BACKDROP_ACCENT_RGB;

/// Node in the compiled document that carries the preview fill.
const FILL_NODE: &str = "snap-preview";

/// Image node filling the preview rect with composited backdrop pixels.
const BACKDROP_NODE: &str = "snap-preview-backdrop";

/// Source label for the retained composited backdrop override.
const BACKDROP_SOURCE: &str = "snap-preview-backdrop-retained";

/// Strong accent border over the captured backdrop (shared accent family).
fn border_color() -> UiColor {
    UiColor {
        r: ACCENT_RGB.0,
        g: ACCENT_RGB.1,
        b: ACCENT_RGB.2,
        a: backdrop::BACKDROP_BORDER_ALPHA,
    }
}

/// Translucent accent fill for an opacity percent. Pure and headless-testable.
/// Delegates to the shared desktop-selection material owner.
#[cfg(test)]
#[must_use]
pub fn fill_color(opacity_percent: u8) -> UiColor {
    let (r, g, b, a) = backdrop::fill_color(opacity_percent);
    UiColor { r, g, b, a }
}

/// Material for one upload: composited RGBA sized to the captured rect,
/// or `None` when capture failed (border-only preview, no fill, no image).
#[must_use]
pub fn preview_image(
    captured: &Result<backdrop::BackdropCapture, String>,
    opacity: u8,
) -> Option<RuntimeImage> {
    let captured = captured.as_ref().ok()?;
    let (fill, border) = backdrop::selection_material(opacity);
    let pixels = backdrop::composite_selection_material(captured, Some(fill), border);
    if captured.width == 0 || captured.height == 0 {
        return None;
    }
    if pixels.len() != (captured.width as usize) * (captured.height as usize) * 4 {
        return None;
    }
    Some(RuntimeImage {
        source: BACKDROP_SOURCE.to_string(),
        width: captured.width,
        height: captured.height,
        pixels,
    })
}

fn checked_size(value: i32) -> Option<u32> {
    u32::try_from(value).ok().filter(|size| *size > 0)
}

/// Serialized preview surface: hidden until a drag candidate arrives.
pub struct SnapPreviewSurface {
    runtime: Option<SurfaceRuntime>,
    surface: Option<SurfaceHandle>,
    visible: bool,
    candidate: Option<SnapTarget>,
    geometry: Option<Rect>,
    opacity: u8,
}

impl SnapPreviewSurface {
    #[must_use]
    pub fn new() -> Self {
        Self {
            runtime: None,
            surface: None,
            visible: false,
            candidate: None,
            geometry: None,
            opacity: DEFAULT_PREVIEW_OPACITY_PERCENT,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn visible(&self) -> bool {
        self.visible
    }

    #[cfg(test)]
    #[must_use]
    pub fn candidate(&self) -> Option<SnapTarget> {
        self.candidate
    }

    #[cfg(test)]
    #[must_use]
    pub fn geometry(&self) -> Option<Rect> {
        self.geometry
    }

    /// True when candidate, geometry, and palette are unchanged and the
    /// surface is already shown, so the caller can skip the redraw.
    #[must_use]
    pub fn is_current(&self, candidate: SnapTarget, geometry: Rect, opacity: u8) -> bool {
        self.visible
            && self.candidate == Some(candidate)
            && self.geometry == Some(geometry)
            && self.opacity == opacity
    }

    /// Show or move the preview for a live drag candidate. `None` hides.
    /// Total without a display: records intent, leaves `visible` false.
    pub fn update(&mut self, candidate: Option<SnapTarget>, geometry: Option<Rect>, opacity: u8) {
        let (Some(candidate), Some(geometry)) = (candidate, geometry) else {
            self.hide();
            return;
        };
        if self.is_current(candidate, geometry, opacity) {
            return;
        }
        if !self.ensure_surface() {
            self.candidate = Some(candidate);
            self.geometry = Some(geometry);
            self.opacity = opacity;
            self.visible = false;
            return;
        }
        let (Some(runtime), Some(surface)) = (self.runtime.as_mut(), self.surface) else {
            self.visible = false;
            return;
        };
        let (Some(width), Some(height)) =
            (checked_size(geometry.width), checked_size(geometry.height))
        else {
            self.hide();
            return;
        };
        if runtime
            .move_resize(surface, geometry.x, geometry.y, width, height)
            .is_err()
        {
            self.hide();
            return;
        }
        // C12 flow, gated by is_current above: capture only on target/rect
        // change. Retained composited pixels land as exact RGBA on the
        // backdrop image node filling the preview rect; the accent border
        // stays as the only extra tint. No second translucent red bg.
        // Capture failure -> clear/hide the image, clear the fill, keep
        // the border.
        let captured = backdrop::capture_root_rect_auto((geometry.x, geometry.y, width, height));
        let image = preview_image(&captured, opacity);
        let styled = runtime.with_document(surface, |document| {
            document
                .border(FILL_NODE, border_color())
                .map_err(|error: String| error)?;
            document
                .background_clear(FILL_NODE)
                .map_err(|error: String| error)?;
            match image {
                Some(image) => {
                    document
                        .image_rgba8(BACKDROP_NODE, image)
                        .map_err(|error: String| error)?;
                    document
                        .visible(BACKDROP_NODE, true)
                        .map_err(|error: String| error)?;
                }
                None => {
                    // Failure path: hide retained pixels, keep border only.
                    let _ = document.visible(BACKDROP_NODE, false);
                    document
                        .image_rgba8(
                            BACKDROP_NODE,
                            RuntimeImage {
                                source: BACKDROP_SOURCE.to_string(),
                                width: 1,
                                height: 1,
                                pixels: vec![0, 0, 0, 0],
                            },
                        )
                        .map_err(|error: String| error)?;
                }
            }
            Ok(())
        });
        if styled.is_err() {
            self.hide();
            return;
        }
        if runtime.redraw(surface).is_err() {
            self.hide();
            return;
        }
        if runtime.show(surface).is_err() {
            self.hide();
            return;
        }
        let _ = runtime.raise(surface);
        self.candidate = Some(candidate);
        self.geometry = Some(geometry);
        self.opacity = opacity;
        self.visible = true;
    }

    /// Hide on release, cancel, unmanage, or shutdown. Never fails.
    pub fn hide(&mut self) {
        if let (Some(runtime), Some(surface)) = (self.runtime.as_mut(), self.surface) {
            let _ = runtime.hide(surface);
        }
        self.visible = false;
        self.candidate = None;
        self.geometry = None;
    }

    /// Lazily create the overlay surface. `false` without a display.
    fn ensure_surface(&mut self) -> bool {
        if self.surface.is_some() {
            return true;
        }
        let mut runtime = match SurfaceRuntime::new() {
            Ok(runtime) => runtime,
            Err(_) => return false,
        };
        let document = match decode_document(COMPILED_UI) {
            Ok(document) => document,
            Err(_) => return false,
        };
        let config = SurfaceConfig {
            width: 1,
            height: 1,
            title: "FlameWM snap preview".to_owned(),
            role: SurfaceRole::Overlay,
            input: SurfaceInputMode::PassThrough,
            initially_visible: false,
            x: 0,
            y: 0,
        };
        match runtime.create_surface(UiTemplate::new(document), config) {
            Ok(handle) => {
                self.runtime = Some(runtime);
                self.surface = Some(handle);
                true
            }
            Err(_) => false,
        }
    }
}

impl Default for SnapPreviewSurface {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> Rect {
        Rect::new(0, 0, 960, 1080)
    }

    #[test]
    fn hidden_at_init() {
        let preview = SnapPreviewSurface::new();
        assert!(!preview.visible());
        assert_eq!(preview.candidate(), None);
        assert_eq!(preview.geometry(), None);
    }

    #[test]
    fn none_candidate_hides() {
        let mut preview = SnapPreviewSurface::new();
        preview.update(None, None, DEFAULT_PREVIEW_OPACITY_PERCENT);
        assert!(!preview.visible());
        assert_eq!(preview.candidate(), None);
    }

    #[test]
    fn unchanged_candidate_geometry_palette_skips_redraw() {
        let preview = SnapPreviewSurface {
            runtime: None,
            surface: None,
            visible: true,
            candidate: Some(SnapTarget::LeftHalf),
            geometry: Some(geometry()),
            opacity: DEFAULT_PREVIEW_OPACITY_PERCENT,
        };
        assert!(preview.is_current(
            SnapTarget::LeftHalf,
            geometry(),
            DEFAULT_PREVIEW_OPACITY_PERCENT
        ));
        assert!(!preview.is_current(
            SnapTarget::RightHalf,
            geometry(),
            DEFAULT_PREVIEW_OPACITY_PERCENT
        ));
        assert!(!preview.is_current(
            SnapTarget::LeftHalf,
            Rect::new(960, 0, 960, 1080),
            DEFAULT_PREVIEW_OPACITY_PERCENT
        ));
        assert!(!preview.is_current(SnapTarget::LeftHalf, geometry(), 40));
    }

    #[test]
    fn hide_clears_candidate_and_visibility() {
        let mut preview = SnapPreviewSurface {
            runtime: None,
            surface: None,
            visible: true,
            candidate: Some(SnapTarget::LeftHalf),
            geometry: Some(geometry()),
            opacity: DEFAULT_PREVIEW_OPACITY_PERCENT,
        };
        preview.hide();
        assert!(!preview.visible());
        assert_eq!(preview.candidate(), None);
        assert_eq!(preview.geometry(), None);
    }

    #[test]
    fn default_opacity_is_twenty_percent() {
        assert_eq!(DEFAULT_PREVIEW_OPACITY_PERCENT, 20);
        assert_eq!(
            fill_color(20),
            UiColor {
                r: 0xef,
                g: 0x40,
                b: 0x48,
                a: 51,
            }
        );
    }

    #[test]
    fn preview_image_consumes_shared_material_pixels() {
        let captured: Result<backdrop::BackdropCapture, String> = Ok(backdrop::BackdropCapture {
            width: 6,
            height: 6,
            pixels: vec![0, 0, 0, 255].repeat(36),
        });
        let image = preview_image(&captured, 20).expect("composited image");
        assert_eq!((image.width, image.height), (6, 6));
        assert_eq!(image.pixels.len(), 36 * 4);
        // Interior carries the shared translucent fill; border ring strong.
        let inner = &image.pixels[(3 * 6 + 3) * 4..][..4];
        assert_eq!((inner[0], inner[1], inner[2], inner[3]), (48, 13, 14, 255));
        assert!(image.pixels[0] > 200);
        // Failure -> no image: caller clears/hides the node, keeps border.
        let failed: Result<backdrop::BackdropCapture, String> = Err("no display".to_string());
        assert!(preview_image(&failed, 20).is_none());
    }

    #[test]
    fn update_without_display_stays_hidden_but_records_intent() {
        if SurfaceRuntime::new().is_ok() {
            return;
        }
        let mut preview = SnapPreviewSurface::new();
        preview.update(
            Some(SnapTarget::LeftHalf),
            Some(geometry()),
            DEFAULT_PREVIEW_OPACITY_PERCENT,
        );
        assert!(!preview.visible());
        assert_eq!(preview.candidate(), Some(SnapTarget::LeftHalf));
    }
}
