//! Decoration cache: WM-owned glyph/raster caches, no per-frame rediscovery.
//!
//! Control SVGs decode once per role, the generic fallback raster once per
//! slot edge, catalog `Icon=` rasters once per icon name. Thread-unsafe by
//! design: owned by the single-threaded WM event loop.

use std::collections::HashMap;

use crate::chrome::ControlRole;

/// WM-owned frame caches: control glyphs, icon rasters, title measures,
/// profiler hook. Title measures cache the canonical Xft estimate
/// (`external_text_measure`, IBM Plex Sans) so layout never re-measures
/// per frame; native pixmap bytes track `external_pixmap_byte_estimate`.
#[derive(Debug, Default)]
pub struct DecorationCache {
    control_glyphs: HashMap<ControlRole, flamewm_image_core::RgbaImage>,
    icon_rasters: HashMap<String, flamewm_integrations_linux::icons::Rgba8Raster>,
    title_measures: HashMap<(String, u32), i32>,
    image_cache_bytes: usize,
}

impl DecorationCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn control_glyph(&self, role: ControlRole) -> Option<flamewm_image_core::RgbaImage> {
        self.control_glyphs.get(&role).cloned()
    }

    pub fn insert_control_glyph(
        &mut self,
        role: ControlRole,
        glyph: flamewm_image_core::RgbaImage,
    ) {
        self.control_glyphs.insert(role, glyph);
    }

    #[must_use]
    pub fn icon_raster(&self, key: &str) -> Option<flamewm_integrations_linux::icons::Rgba8Raster> {
        self.icon_rasters.get(key).cloned()
    }

    pub fn insert_icon_raster(
        &mut self,
        key: String,
        raster: flamewm_integrations_linux::icons::Rgba8Raster,
    ) {
        self.icon_rasters.insert(key, raster);
    }

    #[must_use]
    pub fn title_measure(&self, title: &str, size_bits: u32) -> Option<i32> {
        self.title_measures
            .get(&(title.to_owned(), size_bits))
            .copied()
    }

    pub fn insert_title_measure(&mut self, title: String, size_bits: u32, width: i32) {
        self.title_measures.insert((title, size_bits), width);
    }

    /// Native pixmap bytes for the live-target image cache (w*h*4 per
    /// cached entry, canonical `external_pixmap_byte_estimate`).
    #[must_use]
    pub fn image_cache_bytes(&self) -> usize {
        self.image_cache_bytes
    }

    pub fn add_image_cache_bytes(&mut self, bytes: usize) {
        self.image_cache_bytes = self.image_cache_bytes.saturating_add(bytes);
    }

    #[allow(dead_code)]
    pub fn clear_image_cache_bytes(&mut self) {
        self.image_cache_bytes = 0;
    }

    #[must_use]
    pub fn control_glyph_count(&self) -> usize {
        self.control_glyphs.len()
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn title_measure_count(&self) -> usize {
        self.title_measures.len()
    }

    #[must_use]
    pub fn icon_raster_count(&self) -> usize {
        self.icon_rasters.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t24_title_measure_cache_pins_canonical_width() {
        let mut cache = DecorationCache::new();
        let size_bits = 12.0_f32.to_bits();
        assert_eq!(cache.title_measure("hello", size_bits), None);
        let (expected, _) = flamewm_render_x11::external_text_measure("hello", 12.0);
        cache.insert_title_measure("hello".to_owned(), size_bits, expected.round() as i32);
        assert_eq!(
            cache.title_measure("hello", size_bits),
            Some(expected.round() as i32)
        );
        assert_eq!(cache.title_measure_count(), 1);
    }

    #[test]
    fn icon_bytes_track_pixmap_estimate() {
        let mut cache = DecorationCache::new();
        cache.add_image_cache_bytes(flamewm_render_x11::external_pixmap_byte_estimate(2, 3));
        assert_eq!(cache.image_cache_bytes(), 24);
        cache.clear_image_cache_bytes();
        assert_eq!(cache.image_cache_bytes(), 0);
    }
}
