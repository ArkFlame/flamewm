use flamewm_api::{OutputId, Rect};

pub const MIN_TEXT_SIZE: u8 = 10;
pub const MAX_TEXT_SIZE: u8 = 32;
pub const DEFAULT_NOTE_WIDTH: i32 = 190;
pub const DEFAULT_NOTE_HEIGHT: i32 = 190;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Rgb {
    #[must_use]
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickyNote {
    pub id: String,
    pub workspace: usize,
    pub output: OutputId,
    pub rect: Rect,
    pub text: String,
    pub background: Rgb,
    pub foreground: Rgb,
    pub text_size: u8,
}

impl StickyNote {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        output: OutputId,
        workspace: usize,
        origin_x: i32,
        origin_y: i32,
    ) -> Self {
        Self {
            id: id.into(),
            workspace,
            output,
            rect: Rect::new(origin_x, origin_y, DEFAULT_NOTE_WIDTH, DEFAULT_NOTE_HEIGHT),
            text: String::new(),
            background: Rgb::new(0xff, 0xe5, 0x6b),
            foreground: Rgb::new(0x17, 0x17, 0x17),
            text_size: 15,
        }
    }

    pub fn set_text_size(&mut self, size: u8) -> bool {
        if !(MIN_TEXT_SIZE..=MAX_TEXT_SIZE).contains(&size) {
            return false;
        }
        self.text_size = size;
        true
    }

    pub fn clamp_to_work_area(&mut self, work_area: Rect) {
        self.rect = self.rect.clamp_inside(work_area);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickyNoteStore {
    enabled: bool,
    disable_pending: bool,
    notes: Vec<StickyNote>,
}

impl StickyNoteStore {
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    #[must_use]
    pub fn notes(&self) -> &[StickyNote] {
        &self.notes
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&StickyNote> {
        self.notes.iter().find(|note| note.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut StickyNote> {
        self.notes.iter_mut().find(|note| note.id == id)
    }

    pub fn set_text(&mut self, id: &str, text: impl Into<String>) -> bool {
        let Some(note) = self.get_mut(id) else {
            return false;
        };
        note.text = text.into();
        true
    }

    pub fn set_rect(&mut self, id: &str, rect: Rect) -> bool {
        if !rect.is_valid() {
            return false;
        }
        let Some(note) = self.get_mut(id) else {
            return false;
        };
        note.rect = rect;
        true
    }

    pub fn set_style(&mut self, id: &str, background: Rgb, foreground: Rgb, text_size: u8) -> bool {
        if !(MIN_TEXT_SIZE..=MAX_TEXT_SIZE).contains(&text_size) {
            return false;
        }
        let Some(note) = self.get_mut(id) else {
            return false;
        };
        note.background = background;
        note.foreground = foreground;
        note.text_size = text_size;
        true
    }

    pub fn enable(&mut self) {
        self.enabled = true;
        self.disable_pending = false;
    }

    pub fn create(&mut self, note: StickyNote) -> bool {
        if !self.enabled || self.notes.iter().any(|existing| existing.id == note.id) {
            return false;
        }
        self.notes.push(note);
        true
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.notes.len();
        self.notes.retain(|note| note.id != id);
        before != self.notes.len()
    }

    pub fn request_disable(&mut self) {
        if self.enabled {
            self.disable_pending = true;
        }
    }

    pub fn cancel_disable(&mut self) {
        self.disable_pending = false;
    }

    /// Disabling sticky notes is destructive by product contract.
    pub fn confirm_disable(&mut self) -> bool {
        if !self.disable_pending {
            return false;
        }
        self.notes.clear();
        self.enabled = false;
        self.disable_pending = false;
        true
    }

    pub fn apply_workspace_mapping(&mut self, old_to_new: &[Option<usize>]) {
        self.notes.retain_mut(|note| {
            let Some(mapping) = old_to_new.get(note.workspace) else {
                return false;
            };
            let Some(new_workspace) = *mapping else {
                return false;
            };
            note.workspace = new_workspace;
            true
        });
    }

    pub(crate) fn from_persisted(enabled: bool, notes: Vec<StickyNote>) -> Self {
        Self {
            enabled,
            disable_pending: false,
            notes,
        }
    }
}

impl Default for StickyNoteStore {
    fn default() -> Self {
        Self {
            enabled: true,
            disable_pending: false,
            notes: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disable_requires_explicit_confirmation_and_then_deletes_notes() {
        let mut store = StickyNoteStore::default();
        store.create(StickyNote::new("n1", OutputId::new("eDP-1"), 0, 10, 10));
        store.request_disable();
        assert_eq!(store.notes().len(), 1);
        assert!(store.confirm_disable());
        assert!(store.notes().is_empty());
        assert!(!store.enabled());
    }

    #[test]
    fn workspace_mapping_uses_shared_transform_contract() {
        let mut store = StickyNoteStore::default();
        store.create(StickyNote::new("n1", OutputId::new("eDP-1"), 2, 10, 10));
        store.apply_workspace_mapping(&[Some(0), None, Some(1)]);
        assert_eq!(store.notes()[0].workspace, 1);
    }
}
