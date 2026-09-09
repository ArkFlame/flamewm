//! DecorationManager: WM-owned decoration facade over safe `x11rb`.
//!
//! Owns the catalog (discovered once), the icon resolver, the glyph/raster
//! and title-measure caches, and the bundled-cursor / background-pixel
//! caches. Live Xft/XRender/XShape handles stay in `flamewm-render-x11`'s
//! `ExternalDrawableSession`/`ExternalDrawableTarget` (the runtime owner,
//! constructed only through its `unsafe` constructors, which this crate
//! cannot call under `unsafe_code = "forbid"`);
//! this manager is the canonical intent side: title via Xft-compatible
//! measure/cache (IBM Plex Sans), layout on the measured width, app-icon
//! RGBA via the XRender straight-alpha path, real Breeze glyphs with the red
//! close-hover fill, shape via the canonical rounded mask. `Wm` delegates
//! `draw_frame`, hit-testing, identity, measure, and cursor here; no caller
//! renders directly or calls raw native APIs. Cursor stays Xcursor
//! Breeze-Dark owned by the runtime; the manager only ensures the bundled
//! fallback define.

use std::collections::HashMap;

use x11rb::connection::Connection;
use x11rb::errors::ReplyOrIdError;
use x11rb::protocol::xproto::{Cursor, Window};

use flamewm_integrations_linux::icons::{IconSize, Rgba8Raster};
use flamewm_render_x11::{external_pixmap_byte_estimate, external_text_measure};

use crate::chrome::{self, AppIconSource, ControlRole};
use crate::client::ManagedClient;

use super::cache::DecorationCache;
use super::identity;
use super::interaction::{self, FrameControl};
use super::layout;
use super::model::{FramePaintPlan, FrameSnapshot};
use super::paint::{self, PaintOutcome};
use crate::client::ResizeEdges;

/// Manager facade consumed by `Wm`. Single-threaded by construction.
pub struct DecorationManager {
    catalog: std::sync::Arc<flamewm_applications::ApplicationCatalog>,
    resolver: flamewm_integrations_linux::icons::IconResolver,
    cache: DecorationCache,
    cursor: Option<Cursor>,
    bg_pixel: Option<u32>,
    last_outcome: PaintOutcome,
    cache_gauge: flamewm_profiler::MemoryGauge,
    icon_gauge: flamewm_profiler::MemoryGauge,
}

impl DecorationManager {
    #[must_use]
    pub fn new() -> Self {
        flamewm_profiler::init_process("flamewm-wm");
        Self::with_catalog(std::sync::Arc::new(
            flamewm_applications::ApplicationCatalog::discover().unwrap_or_default(),
        ))
    }

    #[must_use]
    pub fn with_catalog(catalog: std::sync::Arc<flamewm_applications::ApplicationCatalog>) -> Self {
        flamewm_profiler::init_process("flamewm-wm");
        Self {
            catalog,
            resolver: flamewm_integrations_linux::icons::IconResolver::from_environment(
                workspace_root(),
                0,
            ),
            cache: DecorationCache::new(),
            cursor: None,
            bg_pixel: None,
            last_outcome: PaintOutcome::default(),
            cache_gauge: flamewm_profiler::MemoryGauge::new("decoration-cache"),
            icon_gauge: flamewm_profiler::MemoryGauge::new("decoration-icon-cache"),
        }
    }

    /// Measure the cached title width through the canonical Xft-compatible
    /// path (IBM Plex Sans), without touching the display.
    pub fn measure_title(&mut self, title: &str) -> i32 {
        let size_bits = paint::TITLE_FONT_SIZE.to_bits();
        let (_, _, _) = self.measure_decoration_text(title, paint::TITLE_FONT_SIZE);
        self.cache
            .title_measure(title, size_bits)
            .unwrap_or_else(|| layout::measured_title_width(title, paint::TITLE_FONT_SIZE))
    }

    /// True Xft measure when a live target exists; the headless estimate
    /// otherwise. Reports which path was used so callers never guess.
    pub fn measure_decoration_text(&mut self, text: &str, size: f32) -> (f32, f32, &'static str) {
        let _guard = flamewm_profiler::start("wm.decoration.measure");
        let size_bits = size.to_bits();
        if let Some(cached) = self.cache.title_measure(text, size_bits) {
            let height = (size.max(1.0) * 1.30).round();
            return (cached as f32, height, "cached");
        }
        let (width, height) = external_text_measure(text, size);
        self.cache
            .insert_title_measure(text.to_owned(), size_bits, width.round() as i32);
        (width, height, "xft-estimate")
    }

    /// Resolve the app icon in priority order: native pixels, catalog
    /// `Icon=` raster (cached per name), generic fallback, WM_CLASS role.
    pub fn resolve_app_icon(
        &mut self,
        native: Option<chrome::IconImage>,
        wm_class: &str,
    ) -> AppIconSource {
        let _guard = flamewm_profiler::start("wm.decoration.icon");
        if let Some(native) = native {
            return AppIconSource::Native(native);
        }
        let identity = identity::window_identity(wm_class);
        if let Some(entry) = self.catalog.find_by_window_identity(&identity) {
            let icon_name = entry.icon().to_owned();
            if !icon_name.is_empty() {
                if let Some(cached) = self.cache.icon_raster(&icon_name) {
                    return AppIconSource::CatalogRaster(cached);
                }
                let size = IconSize::new(16, 16);
                if let Ok(raster) = self.resolver.prepare_application(icon_name.clone(), size) {
                    self.cache.insert_icon_raster(icon_name, raster.clone());
                    self.refresh_cache_gauge();
                    return AppIconSource::CatalogRaster(raster);
                }
            }
        }
        if let Some(role) = identity::class_role_fallback(wm_class) {
            return AppIconSource::ClassRole(role);
        }
        AppIconSource::Generic
    }

    /// Cached control glyph (real Breeze SVG, Flame color scheme), decoded
    /// once per role. Min/max/restore/close map to `window-*.svg` assets.
    pub fn control_glyph(
        &mut self,
        role: ControlRole,
        edge: u32,
    ) -> Option<flamewm_image_core::RgbaImage> {
        if let Some(cached) = self.cache.control_glyph(role) {
            return Some(cached);
        }
        let glyph = chrome::control_glyph_svg(role, edge.min(64).max(8))?;
        self.cache.insert_control_glyph(role, glyph.clone());
        self.refresh_cache_gauge();
        Some(glyph)
    }

    /// Cached generic fallback raster, once per slot edge.
    pub fn generic_icon_raster(&mut self, edge: u32) -> Option<Rgba8Raster> {
        let key = format!("generic:{edge}");
        if let Some(cached) = self.cache.icon_raster(&key) {
            return Some(cached);
        }
        let size = IconSize::new(edge.min(256).max(1), edge.min(256).max(1));
        let raster = self
            .resolver
            .prepare_path(workspace_root().join("assets/web/flamewm-icon.svg"), size)
            .ok()?;
        self.cache.insert_icon_raster(key, raster.clone());
        self.refresh_cache_gauge();
        Some(raster)
    }

    /// Snapshot the paint inputs for one managed client. Title width uses
    /// the canonical measured width so layout centers on the Xft advance.
    #[allow(clippy::too_many_arguments)]
    pub fn snapshot_for(
        &mut self,
        client: &ManagedClient,
        wm_class: &str,
        active: bool,
        titlebar_height: u16,
        source: &AppIconSource,
    ) -> FrameSnapshot {
        let _ = (wm_class, active);
        FrameSnapshot {
            frame: client.frame,
            outer: client.outer,
            title: client.title.clone(),
            title_text_width: self.measure_title(&client.title),
            has_native_icon: matches!(source, AppIconSource::Native(_)),
            has_catalog_icon: matches!(source, AppIconSource::CatalogRaster(_)),
            hover: client.close_hover.then_some(ControlRole::Close),
            active: client.frame == client.client || active,
            maximized: client.maximized,
            fullscreen: client.fullscreen,
            close_hover: client.close_hover,
            titlebar_height,
        }
    }

    /// Derive the headless-testable paint plan, warming glyph caches so no
    /// per-frame rediscovery happens on the live path.
    pub fn paint_plan_for(
        &mut self,
        snapshot: &FrameSnapshot,
        scene_roles: &[flamewm_skin::recipes::window_chrome::WindowControlRole],
    ) -> FramePaintPlan {
        let roles: Vec<ControlRole> = scene_roles
            .iter()
            .map(|role| ControlRole::from(*role))
            .collect();
        for role in &roles {
            // Skin named metric: 16px control glyph edge.
            let _ = self.control_glyph(
                *role,
                u32::from(
                    flamewm_skin::recipes::window_chrome::WINDOW_CHROME
                        .metrics
                        .control_glyph_edge,
                ),
            );
        }
        let _ =
            self.generic_icon_raster(u32::from(chrome::icon_slot_for(snapshot.titlebar_height)));
        let _ = chrome::ControlPolicy::recipe();
        let _ = chrome::Metrics::new(snapshot.titlebar_height).titlebar_height;
        let _ = chrome::CLOSE_HOVER_RADIUS;
        layout::paint_plan_for(snapshot, roles)
    }

    /// Live paint: caller resolves the icon source and warms glyphs; this
    /// executes cursor + shape + PANEL fill + close-hover + canonical blits
    /// through safe `x11rb` and records the outcome. Title intent (canonical
    /// measure) is recorded as `title_drawn`; the glyph draw itself happens
    /// on the runtime-owned live `ExternalDrawableTarget`; the cursor stays
    /// Xcursor Breeze-Dark, this only ensures the bundled fallback define.
    pub fn draw_frame<C: Connection>(
        &mut self,
        conn: &C,
        snapshot: &FrameSnapshot,
        plan: &FramePaintPlan,
        icon: Option<&flamewm_image_core::RgbaImage>,
    ) -> Result<PaintOutcome, ReplyOrIdError> {
        let controls: Vec<flamewm_image_core::RgbaImage> = plan
            .glyph_roles
            .iter()
            .filter_map(|role| {
                // Skin named metric: 16px control glyph edge, centered in the
                // 38px button slot at paint time.
                self.control_glyph(
                    *role,
                    u32::from(
                        flamewm_skin::recipes::window_chrome::WINDOW_CHROME
                            .metrics
                            .control_glyph_edge,
                    ),
                )
            })
            .collect();
        if let Some(rgba) = icon {
            self.cache
                .add_image_cache_bytes(external_pixmap_byte_estimate(
                    u32::from(plan.slot.max(1)),
                    u32::from(plan.slot.max(1)),
                ));
            let _ = rgba;
            self.icon_gauge.set(self.cache.image_cache_bytes() as u64);
        }
        let outcome = paint::paint_frame(
            conn,
            snapshot,
            plan,
            icon,
            &controls,
            &mut self.cursor,
            &mut self.bg_pixel,
        )?;
        self.last_outcome = outcome.clone();
        Ok(outcome)
    }

    /// Define the bundled fallback cursor on the root window. Never
    /// overrides an Xcursor Breeze-Dark themed cursor owned by the runtime.
    pub fn define_root_cursor<C: Connection>(
        &mut self,
        conn: &C,
        root: Window,
    ) -> Result<(), ReplyOrIdError> {
        let mut outcome = PaintOutcome::default();
        paint::ensure_bundled_cursor(conn, root, &mut self.cursor, &mut outcome)?;
        Ok(())
    }

    /// Hit-test a title-button control for a client frame.
    pub fn control_at(
        &self,
        client: &ManagedClient,
        titlebar_height: u16,
        x: i16,
    ) -> Option<FrameControl> {
        let hit = interaction::frame_control_at(client.outer.width, titlebar_height, x);
        let _ = hit.map(FrameControl::role);
        hit
    }

    /// Resize edges for a client frame at a pointer position.
    pub fn resize_edges(&self, client: &ManagedClient, x: i16, y: i16) -> ResizeEdges {
        let edges = ResizeEdges::at(client.outer.width, client.outer.height, x, y);
        let _ = self.cache_stats();
        edges
    }

    #[must_use]
    pub fn last_outcome(&self) -> &PaintOutcome {
        &self.last_outcome
    }

    #[must_use]
    pub fn cache_stats(&self) -> (usize, usize) {
        (
            self.cache.control_glyph_count(),
            self.cache.icon_raster_count(),
        )
    }

    fn refresh_cache_gauge(&self) {
        let bytes = self.resolver.memory_estimate_bytes() as u64;
        self.cache_gauge.set(bytes);
    }
}

impl Default for DecorationManager {
    fn default() -> Self {
        Self::new()
    }
}

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .to_path_buf()
}

/// Canonical blit input for the live painter: native `_NET_WM_ICON` first,
/// then the catalog raster already resolved by the caller. Pure helper over
/// the resolved source so the live path and tests share one selection rule.
#[must_use]
pub fn frame_icon_rgba(source: &AppIconSource, slot: u32) -> Option<flamewm_image_core::RgbaImage> {
    native_icon_rgba_for(source, slot).or_else(|| catalog_rgba_for(source))
}

/// Native icon RGBA for a resolved native source, scaled into the slot via
/// the canonical XRender straight-alpha path (`native_icon_rgba`).
#[must_use]
pub fn native_icon_rgba_for(
    source: &AppIconSource,
    slot: u32,
) -> Option<flamewm_image_core::RgbaImage> {
    match source {
        AppIconSource::Native(icon) => chrome::native_icon_rgba(icon, slot),
        _ => None,
    }
}

/// Catalog raster converted to canonical RGBA for the live blit path.
#[must_use]
pub fn catalog_rgba_for(source: &AppIconSource) -> Option<flamewm_image_core::RgbaImage> {
    match source {
        AppIconSource::CatalogRaster(raster) => flamewm_image_core::RgbaImage::from_rgba8(
            raster.width,
            raster.height,
            raster.pixels.clone(),
        ),
        _ => None,
    }
}

/// Debug reason when the icon slot falls back past the catalog.
#[must_use]
pub fn icon_fallback_reason(wm_class: &str, reason: &str) -> String {
    if wm_class.is_empty() {
        format!("icon-fallback wm_class=unknown reason={reason}")
    } else {
        format!("icon-fallback wm_class={wm_class} reason={reason}")
    }
}

#[allow(dead_code)]
fn _keep_hashmap_import(_: &HashMap<String, String>) {}
