//! Native layout helpers: node_global_rect + document_intrinsic_size
//! reading the retained layout stored in X11App.

use super::*;

impl X11App {
    /// Window-pixel rect of a node from retained layout (scaled by document ui_scale).
    pub(crate) fn node_global_rect(&self, document: &RuntimeDocument, index: u32) -> Option<Rect> {
        let scale = document.ui_scale();
        self.layout
            .as_ref()?
            .boxes
            .get(index as usize)
            .map(|b| Rect {
                x: b.rect.x * scale,
                y: b.rect.y * scale,
                width: b.rect.width * scale,
                height: b.rect.height * scale,
            })
    }

    /// Intrinsic content size of the document root in device pixels.
    pub(crate) fn document_intrinsic_size(&self, document: &RuntimeDocument) -> Option<(f32, f32)> {
        let scale = document.ui_scale();
        let layout = self.layout.as_ref()?;
        let root = document.document.root as usize;
        let content = layout
            .contents
            .get(root)
            .copied()
            .unwrap_or_else(|| layout.boxes.get(root).map(|b| b.rect).unwrap_or_default());
        Some((content.width * scale, content.height * scale))
    }
}
