//! Glyph rasterization via `fontdue`, resolving faces through
//! `creamui-fonts`'s registry.

use creamui_fonts::{FontWeight, DEFAULT_FAMILY};
use fontdue::layout::{
    CoordinateSystem, GlyphRasterConfig, HorizontalAlign, Layout, LayoutSettings, TextStyle,
    VerticalAlign,
};
use fontdue::Font as FontdueFont;
use std::collections::HashMap;
use std::rc::Rc;

/// A loaded font ready for rasterization, plus a reusable text layout buffer.
///
/// Rasterized glyph bitmaps are cached by [`GlyphRasterConfig`] (glyph +
/// pixel size): every repaint re-lays-out and re-walks the same glyphs (the
/// render loop repaints the whole scene on any change, e.g. scrolling), and
/// without this cache each of those repaints re-rasterized every visible
/// glyph from scratch via `fontdue`, which was the dominant cost behind
/// laggy scrolling on any screen with a non-trivial amount of text.
pub struct Font {
    inner: Rc<FontdueFont>,
    layout: Layout,
    glyph_cache: HashMap<GlyphRasterConfig, (fontdue::Metrics, Rc<Vec<u8>>)>,
}

/// One rasterized glyph, positioned in the coordinate space passed to
/// [`Font::layout_text`].
pub struct PositionedGlyph {
    pub byte_offset: usize,
    pub x: i32,
    pub y: i32,
    pub width: usize,
    pub height: usize,
    /// Per-pixel coverage (`0..=255`), row-major, `width * height` long.
    /// Shared with [`Font`]'s glyph cache rather than copied per paint.
    pub coverage: Rc<Vec<u8>>,
}

impl Font {
    /// Loads CreamUI's bundled default font.
    pub fn load() -> Self {
        Self::from_family(DEFAULT_FAMILY, FontWeight::Regular)
    }

    pub fn bold() -> Self {
        Self::from_family(DEFAULT_FAMILY, FontWeight::Bold)
    }

    /// Resolves `spec` (a CSS-style family stack) against the font registry.
    pub fn from_family(spec: &str, weight: FontWeight) -> Self {
        Self::from_face(creamui_fonts::resolve(spec, weight))
    }

    fn from_face(inner: Rc<FontdueFont>) -> Self {
        Font {
            inner,
            layout: Layout::new(CoordinateSystem::PositiveYDown),
            glyph_cache: HashMap::new(),
        }
    }

    /// Lays out `text` inside a box of `max_width` starting at `(x, y)`,
    /// centering it both horizontally and vertically, and rasterizes each
    /// glyph into a coverage mask.
    pub fn layout_text(
        &mut self,
        text: &str,
        font_size: f32,
        x: f32,
        y: f32,
        max_width: f32,
        max_height: f32,
        align: HorizontalAlign,
    ) -> Vec<PositionedGlyph> {
        self.layout.reset(&LayoutSettings {
            x,
            y,
            max_width: Some(max_width),
            max_height: Some(max_height),
            horizontal_align: align,
            vertical_align: VerticalAlign::Middle,
            ..LayoutSettings::default()
        });
        self.layout
            .append(&[self.inner.as_ref()], &TextStyle::new(text, font_size, 0));

        let inner = &self.inner;
        let cache = &mut self.glyph_cache;
        self.layout
            .glyphs()
            .iter()
            .filter(|g| g.width > 0 && g.height > 0)
            .map(|g| {
                let (metrics, coverage) = cache
                    .entry(g.key)
                    .or_insert_with(|| {
                        let (metrics, bitmap) = inner.rasterize_config(g.key);
                        (metrics, Rc::new(bitmap))
                    })
                    .clone();
                PositionedGlyph {
                    byte_offset: g.byte_offset,
                    x: g.x as i32,
                    y: g.y as i32,
                    width: metrics.width,
                    height: metrics.height,
                    coverage,
                }
            })
            .collect()
    }
}
