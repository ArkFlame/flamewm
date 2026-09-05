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
