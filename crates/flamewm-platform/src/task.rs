use flamewm_api::{DesktopAppId, TaskEntryId, WindowRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskEntryKind {
    /// Persistent application slot. When `window` is set, the first live window of the pinned
    /// application is represented by the anchored pinned slot itself.
    Pinned { window: Option<WindowRef> },
    /// Independent live window entry. Same-application windows are never grouped.
    Window { window: WindowRef },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskEntry {
    pub app: DesktopAppId,
    pub kind: TaskEntryKind,
}

impl TaskEntry {
    #[must_use]
    pub fn pinned(app: DesktopAppId) -> Self {
        Self {
            app,
            kind: TaskEntryKind::Pinned { window: None },
        }
    }

    #[must_use]
    pub fn window(app: DesktopAppId, window: WindowRef) -> Self {
        Self {
            app,
            kind: TaskEntryKind::Window { window },
        }
    }

    #[must_use]
    pub fn id(&self) -> TaskEntryId {
        match self.kind {
            TaskEntryKind::Pinned { .. } => TaskEntryId::pinned(&self.app),
            TaskEntryKind::Window { window } => TaskEntryId::window(window),
        }
    }

    #[must_use]
    pub const fn represented_window(&self) -> Option<WindowRef> {
        match self.kind {
            TaskEntryKind::Pinned { window } => window,
            TaskEntryKind::Window { window } => Some(window),
        }
    }

    #[must_use]
    pub const fn is_pinned_slot(&self) -> bool {
        matches!(self.kind, TaskEntryKind::Pinned { .. })
    }
}

/// V8 task-strip authority.
///
/// The strip owns only FlameWM presentation ordering/pin identity. The engine remains authoritative
/// for the existence, focus, state and lifetime generation of windows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskStrip {
    entries: Vec<TaskEntry>,
}

impl TaskStrip {
    #[must_use]
    pub fn entries(&self) -> &[TaskEntry] {
        &self.entries
    }

    #[must_use]
    pub fn is_pinned(&self, app: &DesktopAppId) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.app == *app && entry.is_pinned_slot())
    }

    #[must_use]
    pub fn pinned_apps(&self) -> Vec<DesktopAppId> {
        self.entries
            .iter()
            .filter(|entry| entry.is_pinned_slot())
            .map(|entry| entry.app.clone())
            .collect()
    }

    pub fn set_pinned(&mut self, desired: &[DesktopAppId]) {
        // First unpin slots no longer requested. A represented live window remains in place.
        let mut index = 0;
        while index < self.entries.len() {
            let requested = desired.iter().any(|app| *app == self.entries[index].app);
            if self.entries[index].is_pinned_slot() && !requested {
                let replacement = match self.entries[index].kind {
                    TaskEntryKind::Pinned {
                        window: Some(window),
                    } => Some(TaskEntry::window(self.entries[index].app.clone(), window)),
                    TaskEntryKind::Pinned { window: None } => None,
                    TaskEntryKind::Window { window } => {
                        Some(TaskEntry::window(self.entries[index].app.clone(), window))
                    }
                };
                if let Some(replacement) = replacement {
                    self.entries[index] = replacement;
                    index += 1;
                } else {
                    self.entries.remove(index);
                }
            } else {
                index += 1;
            }
        }

        // Add missing pinned apps. Prefer converting the first existing live window in-place.
        for app in desired {
            if self.is_pinned(app) {
                continue;
            }
            if let Some(window_index) = self.first_window_index_for_app(app) {
                if let Some(window) = self.entries[window_index].represented_window() {
                    self.entries[window_index] = TaskEntry {
                        app: app.clone(),
                        kind: TaskEntryKind::Pinned {
                            window: Some(window),
                        },
                    };
                }
            } else {
                self.entries.push(TaskEntry::pinned(app.clone()));
            }
        }

        // Reorder only pinned slots to the requested relative order. Live non-pinned entries keep
        // their relative order and no window identity is modified.
        let mut anchor = 0;
        for app in desired {
            if let Some(pos) = self
                .entries
                .iter()
                .enumerate()
                .skip(anchor)
                .find_map(|(i, entry)| (entry.is_pinned_slot() && entry.app == *app).then_some(i))
            {
                let entry = self.entries.remove(pos);
                let insertion = anchor.min(self.entries.len());
                self.entries.insert(insertion, entry);
                anchor = insertion + 1;
            }
        }
    }

    pub fn add_window(&mut self, app: DesktopAppId, window: WindowRef) -> bool {
        if self.window_index(window).is_some() {
            return false;
        }
        if let Some(slot) = self.entries.iter_mut().find(|entry| {
            entry.app == app && matches!(entry.kind, TaskEntryKind::Pinned { window: None })
        }) {
            slot.kind = TaskEntryKind::Pinned {
                window: Some(window),
            };
            return true;
        }
        self.entries.push(TaskEntry::window(app, window));
        true
    }

    pub fn remove_window(&mut self, window: WindowRef) -> bool {
        let Some(index) = self.window_index(window) else {
            return false;
        };
        let app = self.entries[index].app.clone();
        let was_pinned_slot = self.entries[index].is_pinned_slot();

        if was_pinned_slot {
            // Keep the anchored slot. Promote one sibling window into it when available.
            if let Some(sibling) = self.entries.iter().enumerate().find_map(|(i, entry)| {
                (i != index
                    && entry.app == app
                    && matches!(entry.kind, TaskEntryKind::Window { .. }))
                .then_some(i)
            }) {
                let sibling_entry = self.entries.remove(sibling);
                if let Some(sibling_window) = sibling_entry.represented_window() {
                    let slot_index = if sibling < index { index - 1 } else { index };
                    self.entries[slot_index].kind = TaskEntryKind::Pinned {
                        window: Some(sibling_window),
                    };
                }
            } else {
                self.entries[index].kind = TaskEntryKind::Pinned { window: None };
            }
        } else {
            self.entries.remove(index);
        }
        true
    }

    pub fn pin_window(&mut self, window: WindowRef) -> bool {
        let Some(index) = self.window_index(window) else {
            return false;
        };
        let app = self.entries[index].app.clone();
        if self.is_pinned(&app) {
            return true;
        }
        self.entries[index].kind = TaskEntryKind::Pinned {
            window: Some(window),
        };
        true
    }

    pub fn pin_app(&mut self, app: DesktopAppId) -> bool {
        if self.is_pinned(&app) {
            return false;
        }
        if let Some(index) = self.first_window_index_for_app(&app) {
            if let Some(window) = self.entries[index].represented_window() {
                self.entries[index].kind = TaskEntryKind::Pinned {
                    window: Some(window),
                };
            }
        } else {
            self.entries.push(TaskEntry::pinned(app));
        }
        true
    }

    pub fn unpin_app(&mut self, app: &DesktopAppId) -> bool {
        let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.app == *app && entry.is_pinned_slot())
        else {
            return false;
        };
        match self.entries[index].kind {
            TaskEntryKind::Pinned {
                window: Some(window),
            } => {
                self.entries[index].kind = TaskEntryKind::Window { window };
            }
            TaskEntryKind::Pinned { window: None } => {
                self.entries.remove(index);
            }
            TaskEntryKind::Window { .. } => return false,
        }
        true
    }

    /// Commit insertion ordering on release. `to` is the final insertion index after removal.
    pub fn reorder_on_release(&mut self, from: usize, to: usize) -> bool {
        if from >= self.entries.len() || to >= self.entries.len() || from == to {
            return false;
        }
        let entry = self.entries.remove(from);
        self.entries.insert(to, entry);
        true
    }

    #[must_use]
    pub fn begin_reorder(&self, from: usize) -> Option<ReorderSession> {
        (from < self.entries.len()).then(|| ReorderSession {
            source: from,
            original_order: self.entries.iter().map(TaskEntry::id).collect(),
        })
    }

    #[must_use]
    pub fn current_ids(&self) -> Vec<TaskEntryId> {
        self.entries.iter().map(TaskEntry::id).collect()
    }

    fn window_index(&self, window: WindowRef) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.represented_window() == Some(window))
    }

    fn first_window_index_for_app(&self, app: &DesktopAppId) -> Option<usize> {
        self.entries
            .iter()
            .position(|entry| entry.app == *app && entry.represented_window().is_some())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReorderSession {
    source: usize,
    original_order: Vec<TaskEntryId>,
}

impl ReorderSession {
    #[must_use]
    pub const fn source(&self) -> usize {
        self.source
    }

    #[must_use]
    pub fn original_order(&self) -> &[TaskEntryId] {
        &self.original_order
    }

    /// Cancellation is intentionally a no-op because the strip is never mutated during drag.
    #[must_use]
    pub fn cancel(self, strip: &TaskStrip) -> bool {
        strip.current_ids() == self.original_order
    }

    pub fn commit(self, strip: &mut TaskStrip, insertion_index: usize) -> bool {
        strip.reorder_on_release(self.source, insertion_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(value: &str) -> DesktopAppId {
        DesktopAppId::new(value)
    }

    fn win(value: u64) -> WindowRef {
        WindowRef::new(value, 1)
    }

    #[test]
    fn pinned_app_uses_one_anchored_entry_and_same_app_windows_remain_independent() {
        let mut strip = TaskStrip::default();
        strip.pin_app(app("org.kde.dolphin"));
        strip.add_window(app("org.kde.dolphin"), win(1));
        strip.add_window(app("org.kde.dolphin"), win(2));
        strip.add_window(app("org.kde.dolphin"), win(3));
        assert_eq!(strip.entries().len(), 3);
        assert_eq!(strip.entries()[0].id().as_str(), "pin:org.kde.dolphin");
        assert_eq!(strip.entries()[1].id().as_str(), "win:2:1");
        assert_eq!(strip.entries()[2].id().as_str(), "win:3:1");
    }

    #[test]
    fn closing_pinned_representative_promotes_sibling_without_moving_slot() {
        let mut strip = TaskStrip::default();
        strip.pin_app(app("firefox"));
        strip.add_window(app("firefox"), win(1));
        strip.add_window(app("firefox"), win(2));
        strip.add_window(app("terminal"), win(9));
        assert!(strip.remove_window(win(1)));
        assert_eq!(strip.entries()[0].id().as_str(), "pin:firefox");
        assert_eq!(strip.entries()[0].represented_window(), Some(win(2)));
        assert_eq!(strip.entries()[1].represented_window(), Some(win(9)));
    }

    #[test]
    fn final_pinned_window_close_restores_placeholder() {
        let mut strip = TaskStrip::default();
        strip.pin_app(app("firefox"));
        strip.add_window(app("firefox"), win(1));
        assert!(strip.remove_window(win(1)));
        assert_eq!(strip.entries().len(), 1);
        assert_eq!(strip.entries()[0].represented_window(), None);
        assert_eq!(strip.entries()[0].id().as_str(), "pin:firefox");
    }

    #[test]
    fn unpin_active_app_preserves_live_window_identity_and_location() {
        let mut strip = TaskStrip::default();
        strip.add_window(app("terminal"), win(9));
        strip.add_window(app("firefox"), win(1));
        strip.pin_window(win(1));
        assert!(strip.unpin_app(&app("firefox")));
        assert_eq!(strip.entries()[1].id().as_str(), "win:1:1");
    }

    #[test]
    fn cancelled_drag_cannot_change_order() {
        let mut strip = TaskStrip::default();
        strip.pin_app(app("a"));
        strip.pin_app(app("b"));
        strip.add_window(app("c"), win(3));
        let session = strip.begin_reorder(0).expect("source exists");
        assert!(session.cancel(&strip));
        assert_eq!(
            strip
                .current_ids()
                .iter()
                .map(TaskEntryId::as_str)
                .collect::<Vec<_>>(),
            vec!["pin:a", "pin:b", "win:3:1"]
        );
    }

    #[test]
    fn release_commits_before_between_after_style_insertion() {
        let mut strip = TaskStrip::default();
        strip.pin_app(app("a"));
        strip.pin_app(app("b"));
        strip.pin_app(app("c"));
        let session = strip.begin_reorder(0).expect("source exists");
        assert!(session.commit(&mut strip, 2));
        assert_eq!(
            strip
                .current_ids()
                .iter()
                .map(TaskEntryId::as_str)
                .collect::<Vec<_>>(),
            vec!["pin:b", "pin:c", "pin:a"]
        );
    }
}
