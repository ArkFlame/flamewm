use std::collections::{BTreeMap, BTreeSet};

use flamewm_api::Rect;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionModel {
    selected: BTreeSet<String>,
    fill_opacity: u8,
    border_opacity: u8,
    rubber: Option<Rect>,
}

impl Default for SelectionModel {
    fn default() -> Self {
        Self {
            selected: BTreeSet::new(),
            fill_opacity: 20,
            border_opacity: 60,
            rubber: None,
        }
    }
}

impl SelectionModel {
    pub fn set_fill_opacity(&mut self, opacity: u8) {
        self.fill_opacity = opacity.min(60);
    }

    #[must_use]
    pub const fn fill_opacity(&self) -> u8 {
        self.fill_opacity
    }

    pub fn set_border_opacity(&mut self, opacity: u8) {
        self.border_opacity = opacity.min(100);
    }

    #[must_use]
    pub const fn border_opacity(&self) -> u8 {
        self.border_opacity
    }

    #[must_use]
    pub const fn fill_alpha(&self) -> u8 {
        ((self.fill_opacity as u16 * 255 + 50) / 100) as u8
    }

    pub fn select(&mut self, id: impl Into<String>) {
        self.selected.insert(id.into());
    }

    pub fn deselect(&mut self, id: &str) {
        self.selected.remove(id);
    }

    pub fn clear(&mut self) {
        self.selected.clear();
    }

    #[must_use]
    pub fn selected(&self) -> &BTreeSet<String> {
        &self.selected
    }

    pub fn set_rubber(&mut self, rect: Rect) {
        self.rubber = Some(rect);
    }

    pub fn clear_rubber(&mut self) {
        self.rubber = None;
    }

    #[must_use]
    pub const fn rubber(&self) -> Option<Rect> {
        self.rubber
    }

    #[must_use]
    pub fn hit_test(&self, item_rects: &BTreeMap<String, Rect>) -> BTreeSet<String> {
        let Some(rubber) = self.rubber else {
            return BTreeSet::new();
        };
        item_rects
            .iter()
            .filter_map(|(id, rect)| rect.intersects(rubber).then_some(id.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_fill_still_preserves_selection_state_for_visible_border_renderer() {
        let mut selection = SelectionModel::default();
        selection.set_fill_opacity(0);
        selection.select("a");
        assert_eq!(selection.fill_alpha(), 0);
        assert_eq!(selection.border_opacity(), 60);
        assert!(selection.selected().contains("a"));
    }

    #[test]
    fn rubber_band_uses_rectangle_intersection_not_origin_only() {
        let mut selection = SelectionModel::default();
        selection.set_rubber(Rect::new(50, 50, 20, 20));
        let rects = BTreeMap::from([("a".to_owned(), Rect::new(40, 40, 15, 15))]);
        assert!(selection.hit_test(&rects).contains("a"));
    }
}
