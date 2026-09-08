use flamewm_render_core::RuntimeDocument;

pub struct UiDocument {
    pub(crate) document: RuntimeDocument,
}

pub struct UiDocumentView<'a> {
    pub(crate) document: &'a mut RuntimeDocument,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeImage {
    pub source: String,
    pub width: u32,
    pub height: u32,
    /// Straight (non-premultiplied) RGBA8 in row-major order. Dynamic/app
    /// icons carry real alpha here; no magenta color-key is consulted.
    pub pixels: Vec<u8>,
}

impl RuntimeImage {
    #[must_use]
    pub fn is_fully_opaque(&self) -> bool {
        self.pixels.chunks_exact(4).all(|pixel| pixel[3] == 255)
    }
}

pub trait UiDocumentAccess {
    fn text(&mut self, id: &str, text: String) -> Result<(), String>;
    fn image_rgb8(&mut self, id: &str, image: RuntimeImage) -> Result<(), String>;
    fn image_rgba8(&mut self, id: &str, image: RuntimeImage) -> Result<(), String>;
    fn visible(&mut self, id: &str, visible: bool) -> Result<(), String>;
    fn toggle(&mut self, track_id: &str, knob_id: &str, checked: bool) -> Result<(), String>;
    fn position(&mut self, id: &str, left: f32, top: f32) -> Result<(), String>;
    fn size(&mut self, id: &str, width: f32, height: f32) -> Result<(), String>;
    fn background(&mut self, id: &str, color: UiColor) -> Result<(), String>;
    fn border(&mut self, id: &str, color: UiColor) -> Result<(), String>;
    fn background_clear(&mut self, id: &str) -> Result<(), String>;
    fn border_clear(&mut self, id: &str) -> Result<(), String>;
    fn overflow(
        &mut self,
        id: &str,
        x: flamewm_render_core::Overflow,
        y: flamewm_render_core::Overflow,
    ) -> Result<(), String>;
}

impl UiDocument {
    pub fn text(&mut self, id: &str, text: String) -> Result<(), String> {
        self.document.set_text(id, text)
    }

    pub fn visible(&mut self, id: &str, visible: bool) -> Result<(), String> {
        self.document.set_visible(id, visible)
    }

    pub fn image_rgb8(&mut self, id: &str, image: RuntimeImage) -> Result<(), String> {
        self.image_rgba8(id, image)
    }

    pub fn image_rgba8(&mut self, id: &str, image: RuntimeImage) -> Result<(), String> {
        validate_rgba8(&image)?;
        self.document.replace_image_for_node(
            id,
            image.source,
            image.width,
            image.height,
            image.pixels,
        )
    }

    pub fn toggle(&mut self, track_id: &str, knob_id: &str, checked: bool) -> Result<(), String> {
        let recipe = flamewm_skin::recipes::toggle::TOGGLE;
        let color = if checked { recipe.on } else { recipe.off };
        self.document.set_background_color(track_id, rgb(color.0))?;
        let left = if checked {
            recipe.width - recipe.knob - 2
        } else {
            2
        };
        self.document.set_position_px(knob_id, left as f32, 2.0)
    }

    pub fn position(&mut self, id: &str, left: f32, top: f32) -> Result<(), String> {
        self.document.set_position_px(id, left, top)
    }

    pub fn size(&mut self, id: &str, width: f32, height: f32) -> Result<(), String> {
        self.document.set_size_px(id, width, height)
    }

    pub fn background(&mut self, id: &str, color: UiColor) -> Result<(), String> {
        self.document.set_background_color(id, color.into())
    }

    pub fn border(&mut self, id: &str, color: UiColor) -> Result<(), String> {
        self.document.set_border_color(id, color.into())
    }

    pub fn background_clear(&mut self, id: &str) -> Result<(), String> {
        self.document.clear_background_color(id)
    }

    pub fn border_clear(&mut self, id: &str) -> Result<(), String> {
        self.document.clear_border_color(id)
    }

    pub fn overflow(
        &mut self,
        id: &str,
        x: flamewm_render_core::Overflow,
        y: flamewm_render_core::Overflow,
    ) -> Result<(), String> {
        self.document.set_overflow(id, x, y)
    }
}

impl UiDocumentAccess for UiDocument {
    fn text(&mut self, id: &str, text: String) -> Result<(), String> {
        self.text(id, text)
    }

    fn image_rgb8(&mut self, id: &str, image: RuntimeImage) -> Result<(), String> {
        self.image_rgb8(id, image)
    }

    fn image_rgba8(&mut self, id: &str, image: RuntimeImage) -> Result<(), String> {
        self.image_rgba8(id, image)
    }

    fn visible(&mut self, id: &str, visible: bool) -> Result<(), String> {
        self.visible(id, visible)
    }

    fn toggle(&mut self, track_id: &str, knob_id: &str, checked: bool) -> Result<(), String> {
        self.toggle(track_id, knob_id, checked)
    }

    fn position(&mut self, id: &str, left: f32, top: f32) -> Result<(), String> {
        self.position(id, left, top)
    }

    fn size(&mut self, id: &str, width: f32, height: f32) -> Result<(), String> {
        self.size(id, width, height)
    }

    fn background(&mut self, id: &str, color: UiColor) -> Result<(), String> {
        self.background(id, color)
    }

    fn border(&mut self, id: &str, color: UiColor) -> Result<(), String> {
        self.border(id, color)
    }

    fn background_clear(&mut self, id: &str) -> Result<(), String> {
        self.background_clear(id)
    }

    fn border_clear(&mut self, id: &str) -> Result<(), String> {
        self.border_clear(id)
    }

    fn overflow(
        &mut self,
        id: &str,
        x: flamewm_render_core::Overflow,
        y: flamewm_render_core::Overflow,
    ) -> Result<(), String> {
        self.overflow(id, x, y)
    }
}

impl<'a> UiDocumentAccess for UiDocumentView<'a> {
    fn text(&mut self, id: &str, text: String) -> Result<(), String> {
        self.document.set_text(id, text)
    }

    fn image_rgb8(&mut self, id: &str, image: RuntimeImage) -> Result<(), String> {
        validate_rgba8(&image)?;
        self.document.replace_image_for_node(
            id,
            image.source,
            image.width,
            image.height,
            image.pixels,
        )
    }

    fn image_rgba8(&mut self, id: &str, image: RuntimeImage) -> Result<(), String> {
        validate_rgba8(&image)?;
        self.document.replace_image_for_node(
            id,
            image.source,
            image.width,
            image.height,
            image.pixels,
        )
    }

    fn visible(&mut self, id: &str, visible: bool) -> Result<(), String> {
        self.document.set_visible(id, visible)
    }

    fn toggle(&mut self, track_id: &str, knob_id: &str, checked: bool) -> Result<(), String> {
        let recipe = flamewm_skin::recipes::toggle::TOGGLE;
        let color = if checked { recipe.on } else { recipe.off };
        self.document.set_background_color(track_id, rgb(color.0))?;
        let left = if checked {
            recipe.width - recipe.knob - 2
        } else {
            2
        };
        self.document.set_position_px(knob_id, left as f32, 2.0)
    }

    fn position(&mut self, id: &str, left: f32, top: f32) -> Result<(), String> {
        self.document.set_position_px(id, left, top)
    }

    fn size(&mut self, id: &str, width: f32, height: f32) -> Result<(), String> {
        self.document.set_size_px(id, width, height)
    }

    fn background(&mut self, id: &str, color: UiColor) -> Result<(), String> {
        self.document.set_background_color(id, color.into())
    }

    fn border(&mut self, id: &str, color: UiColor) -> Result<(), String> {
        self.document.set_border_color(id, color.into())
    }

    fn background_clear(&mut self, id: &str) -> Result<(), String> {
        self.document.clear_background_color(id)
    }

    fn border_clear(&mut self, id: &str) -> Result<(), String> {
        self.document.clear_border_color(id)
    }

    fn overflow(
        &mut self,
        id: &str,
        x: flamewm_render_core::Overflow,
        y: flamewm_render_core::Overflow,
    ) -> Result<(), String> {
        self.document.set_overflow(id, x, y)
    }
}

impl From<UiColor> for flamewm_render_core::Color {
    fn from(color: UiColor) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    }
}

fn rgb(value: u32) -> flamewm_render_core::Color {
    flamewm_render_core::Color::rgb((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

fn validate_rgba8(image: &RuntimeImage) -> Result<(), String> {
    if image.width == 0 || image.height == 0 {
        return Err("runtime image dimensions must be non-zero".to_string());
    }
    let expected = (image.width as usize)
        .checked_mul(image.height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "runtime image dimensions overflow".to_string())?;
    if image.pixels.len() != expected {
        return Err(format!(
            "runtime image has {} bytes; expected {expected} (straight RGBA8)",
            image.pixels.len()
        ));
    }
    Ok(())
}

/// Visible-shape mask geometry for a rounded popup/menu surface: row spans
/// (y, x_start, x_end_exclusive) in surface pixels, cut from the root
/// background radius plus content bounds. Render-side copy used by the
/// render-owned XShape bounding-mask bridge; kept in sync with the
/// identical pure function in `flamewm-render-core::paint` so the geometry
/// can be verified without a display.
#[cfg(test)]
pub fn rounded_rect_mask_spans(width: u32, height: u32, radius: u32) -> Vec<(u32, u32, u32)> {
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let radius = radius.min(width / 2).min(height / 2);
    if radius <= 1 {
        return (0..height).map(|row| (row, 0, width)).collect();
    }
    let mut spans = Vec::with_capacity(height as usize);
    for row in 0..height {
        let corner_row = row.min(height - 1 - row);
        let inset = if corner_row >= radius {
            0
        } else {
            let r = f64::from(radius);
            let dy = r - (f64::from(corner_row) + 0.5);
            let dx = (r * r - dy * dy).max(0.0).sqrt();
            // Floor matches the render-side corner rasterization: mask spans
            // and painted pixels use one formula.
            (r - dx).floor().clamp(0.0, r) as u32
        };
        let start = inset.min(width);
        let end = width.saturating_sub(inset).max(start);
        spans.push((row, start, end));
    }
    spans
}

pub fn decode_document(bytes: &[u8]) -> Result<UiDocument, String> {
    Ok(UiDocument {
        document: RuntimeDocument::new(flamewm_render_core::decode(bytes)?)?,
    })
}

/// A renderer document kept behind the UI/X11 boundary.
pub struct UiTemplate {
    pub(crate) document: RuntimeDocument,
}

impl UiTemplate {
    pub fn new(document: UiDocument) -> Self {
        Self {
            document: document.document,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_node_doc() -> UiDocument {
        let asset = flamewm_render_core::ImageAsset {
            source: "old".to_string(),
            width: 1,
            height: 1,
            pixels: vec![0, 0, 0, 255],
        };
        let node = flamewm_render_core::CompiledNode {
            kind: flamewm_render_core::NodeKind::Image,
            parent: None,
            first_child: None,
            next_sibling: None,
            id: "icon".to_string(),
            action: String::new(),
            text: String::new(),
            image: Some(0),
            style: flamewm_render_core::Style::default(),
            hover_style: None,
            active_style: None,
        };
        UiDocument {
            document: RuntimeDocument::new(flamewm_render_core::CompiledDocument {
                source_fingerprint: 0,
                root: 0,
                variables: Vec::new(),
                assets: vec![asset],
                nodes: vec![node],
            })
            .expect("fixture validates"),
        }
    }

    #[test]
    fn rgba8_replace_keeps_alpha_and_rejects_rgb8_length() {
        let mut doc = image_node_doc();
        doc.image_rgba8(
            "icon",
            RuntimeImage {
                source: "t".to_string(),
                width: 1,
                height: 1,
                pixels: vec![200, 30, 10, 128],
            },
        )
        .expect("rgba replace");
        assert_eq!(
            doc.document.document.assets[0].pixels,
            vec![200, 30, 10, 128]
        );
        let error = doc
            .image_rgba8(
                "icon",
                RuntimeImage {
                    source: "bad".to_string(),
                    width: 1,
                    height: 1,
                    pixels: vec![1, 2, 3],
                },
            )
            .expect_err("3-byte RGB8 must fail");
        assert!(error.contains("straight RGBA8"), "unexpected: {error}");
    }

    #[test]
    fn magenta_pixel_is_opaque_content_not_transparency() {
        let mut doc = image_node_doc();
        doc.image_rgba8(
            "icon",
            RuntimeImage {
                source: "m".to_string(),
                width: 1,
                height: 1,
                pixels: vec![255, 0, 255, 255],
            },
        )
        .expect("magenta replace");
        assert!(doc.document.document.assets[0].is_fully_opaque());
    }

    #[test]
    fn rounded_mask_geometry_matches_quarter_circle_cut() {
        // Floor of (r - dx) at pixel-row centers, matching the render-side
        // corner rasterization. 8x8 r=4: row 0 cuts 2px per side, rows 1..6
        // are full width under floor, row 7 mirrors row 0.
        let spans = rounded_rect_mask_spans(8, 8, 4);
        assert_eq!(
            spans,
            vec![
                (0, 2, 6),
                (1, 0, 8),
                (2, 0, 8),
                (3, 0, 8),
                (4, 0, 8),
                (5, 0, 8),
                (6, 0, 8),
                (7, 2, 6),
            ]
        );
    }

    #[test]
    fn rounded_mask_degrades_to_full_rect_for_small_radius() {
        assert_eq!(rounded_rect_mask_spans(4, 2, 0), vec![(0, 0, 4), (1, 0, 4)]);
        assert!(rounded_rect_mask_spans(0, 8, 4).is_empty());
    }
}
