use std::fs;
use std::path::{Path, PathBuf};

use flamewm_api::{ErrorCode, FlameError, FlameResult};

#[must_use]
pub fn next_new_folder_path(desktop: &Path) -> PathBuf {
    let first = desktop.join("New Folder");
    if !first.exists() {
        return first;
    }
    for number in 2_u32..=u32::MAX {
        let candidate = desktop.join(format!("New Folder ({number})"));
        if !candidate.exists() {
            return candidate;
        }
    }
    desktop.join("New Folder (overflow)")
}

pub fn create_new_folder(desktop: &Path) -> FlameResult<PathBuf> {
    let path = next_new_folder_path(desktop);
    fs::create_dir(&path).map_err(|error| {
        FlameError::new(
            ErrorCode::IoFailure,
            format!("create {}: {error}", path.display()),
        )
    })?;
    Ok(path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandIntent {
    pub program: String,
    pub argv: Vec<String>,
    pub cwd: Option<PathBuf>,
}

#[must_use]
pub fn open_path(path: &Path) -> CommandIntent {
    CommandIntent {
        program: "xdg-open".to_owned(),
        argv: vec![path.to_string_lossy().into_owned()],
        cwd: None,
    }
}

#[must_use]
pub fn open_terminal(program: &str, desktop: &Path) -> CommandIntent {
    CommandIntent {
        program: program.to_owned(),
        argv: Vec::new(),
        cwd: Some(desktop.to_path_buf()),
    }
}

#[must_use]
pub fn desktop_settings() -> CommandIntent {
    CommandIntent {
        program: "flamewm-settings".to_owned(),
        argv: vec!["--page".to_owned(), "desktop".to_owned()],
        cwd: None,
    }
}

pub const DEFAULT_TERMINAL: &str = "x-terminal-emulator";

/// Terminal program without shell evaluation: `$TERMINAL` or compiled default.
#[must_use]
pub fn terminal_program() -> String {
    std::env::var("TERMINAL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_TERMINAL.to_owned())
}

/// Entry context rows. Trash exposes Empty Trash only; normal entries expose
/// Create Shortcut plus Delete (which the caller must confirm first).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryMenuAction {
    EmptyTrash,
    CreateShortcut,
    Delete,
}

impl EntryMenuAction {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::EmptyTrash => "Empty Trash",
            Self::CreateShortcut => "Create Shortcut",
            Self::Delete => "Delete",
        }
    }
}

#[must_use]
pub fn entry_context_menu(is_trash: bool) -> Vec<EntryMenuAction> {
    if is_trash {
        vec![EntryMenuAction::EmptyTrash]
    } else {
        vec![EntryMenuAction::CreateShortcut, EntryMenuAction::Delete]
    }
}

/// How a desktop item opens. `.desktop` entries launch via `DesktopEntry`;
/// regular files and directories use `xdg-open`. Never inverted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherOpen {
    DesktopEntry,
    RegularFile,
}

#[must_use]
pub const fn launcher_open_kind(is_desktop_entry: bool) -> LauncherOpen {
    if is_desktop_entry {
        LauncherOpen::DesktopEntry
    } else {
        LauncherOpen::RegularFile
    }
}

/// Collision-safe shortcut destination inside the desktop directory.
/// `.desktop` sources are copied with a ` link` suffix before the extension;
/// other sources get a symlinked sibling of the same scheme. Pure path
/// computation, no shell strings.
#[must_use]
pub fn shortcut_destination(desktop: &Path, file_name: &str) -> PathBuf {
    let (stem, extension) = match file_name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => {
            (stem.to_owned(), Some(extension.to_owned()))
        }
        _ => (file_name.to_owned(), None),
    };
    let join = |stem: &str| match &extension {
        Some(extension) => desktop.join(format!("{stem}.{extension}")),
        None => desktop.join(stem),
    };
    let first = join(&format!("{stem} link"));
    if !first.exists() {
        return first;
    }
    for number in 2_u32..=u32::MAX {
        let candidate = join(&format!("{stem} link ({number})"));
        if !candidate.exists() {
            return candidate;
        }
    }
    join(&format!("{stem} link (overflow)"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_menu_maps_trash_to_empty_trash_only() {
        assert_eq!(entry_context_menu(true), vec![EntryMenuAction::EmptyTrash]);
        assert_eq!(
            entry_context_menu(false),
            vec![EntryMenuAction::CreateShortcut, EntryMenuAction::Delete]
        );
    }

    #[test]
    fn launcher_open_selects_desktop_entry_for_desktop_files() {
        assert_eq!(launcher_open_kind(true), LauncherOpen::DesktopEntry);
        assert_eq!(launcher_open_kind(false), LauncherOpen::RegularFile);
    }

    #[test]
    fn shortcut_destination_appends_link_suffix_collision_free() {
        let root = std::env::temp_dir().join(format!(
            "flamewm-shortcut-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        assert_eq!(
            shortcut_destination(&root, "app.desktop"),
            root.join("app link.desktop")
        );
        fs::write(root.join("app link.desktop"), b"x").unwrap();
        assert_eq!(
            shortcut_destination(&root, "app.desktop"),
            root.join("app link (2).desktop")
        );
        assert_eq!(
            shortcut_destination(&root, "notes"),
            root.join("notes link")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
