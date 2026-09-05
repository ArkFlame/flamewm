use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchEventKind {
    Create,
    Delete,
    Modify,
    MoveFrom,
    MoveTo,
    Overflow,
    Invalidated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchEvent {
    pub kind: WatchEventKind,
    pub path: PathBuf,
    pub cookie: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WatchBatch {
    pub requires_rescan: bool,
    pub watcher_invalidated: bool,
    pub renames: Vec<(PathBuf, PathBuf)>,
    pub touched: Vec<PathBuf>,
}

#[must_use]
pub fn reduce(events: &[WatchEvent]) -> WatchBatch {
    let mut batch = WatchBatch::default();
    let mut moved_from = BTreeMap::<u32, PathBuf>::new();
    for event in events {
        match event.kind {
            WatchEventKind::Overflow => batch.requires_rescan = true,
            WatchEventKind::Invalidated => {
                batch.requires_rescan = true;
                batch.watcher_invalidated = true;
            }
            WatchEventKind::MoveFrom if event.cookie != 0 => {
                moved_from.insert(event.cookie, event.path.clone());
            }
            WatchEventKind::MoveTo if event.cookie != 0 => {
                if let Some(old) = moved_from.remove(&event.cookie) {
                    batch.renames.push((old, event.path.clone()));
                } else {
                    batch.touched.push(event.path.clone());
                }
            }
            WatchEventKind::Create
            | WatchEventKind::Delete
            | WatchEventKind::Modify
            | WatchEventKind::MoveFrom
            | WatchEventKind::MoveTo => batch.touched.push(event.path.clone()),
        }
    }
    batch.touched.extend(moved_from.into_values());
    batch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_move_cookie_is_one_rename_not_delete_plus_create() {
        let batch = reduce(&[
            WatchEvent {
                kind: WatchEventKind::MoveFrom,
                path: PathBuf::from("a"),
                cookie: 4,
            },
            WatchEvent {
                kind: WatchEventKind::MoveTo,
                path: PathBuf::from("b"),
                cookie: 4,
            },
        ]);
        assert_eq!(
            batch.renames,
            vec![(PathBuf::from("a"), PathBuf::from("b"))]
        );
    }

    #[test]
    fn overflow_forces_authoritative_rescan() {
        let batch = reduce(&[WatchEvent {
            kind: WatchEventKind::Overflow,
            path: PathBuf::new(),
            cookie: 0,
        }]);
        assert!(batch.requires_rescan);
    }
}
