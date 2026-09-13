use super::DesktopIntegration;
use crate::platform::OpenWindow;
use creamui_render::WindowHandle;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

/// KWin Wayland exposes its window-control interface over the user D-Bus.
/// kdotool is a small D-Bus client for that interface; using it avoids the X11
/// APIs, which cannot enumerate native Wayland windows.
pub struct KWinDbus {
    windows: Arc<Mutex<Vec<OpenWindow>>>,
}

impl KWinDbus {
    pub fn detect() -> Option<Self> {
        busctl(&["status", "org.kde.KWin"])
            .and_then(|_| kdotool(&["--help"]))
            .map(|_| Self::new())
    }

    fn new() -> Self {
        let windows = Arc::new(Mutex::new(Vec::new()));
        let cache = windows.clone();
        thread::spawn(move || loop {
            let next = query_windows();
            if let Ok(mut current) = cache.lock() {
                *current = next;
            }
            thread::sleep(Duration::from_millis(700));
        });
        Self { windows }
    }
}

impl DesktopIntegration for KWinDbus {
    fn prepare_window(&self, window: &WindowHandle) {
        window.set_always_on_top(true);
    }

    fn windows(&self) -> Vec<OpenWindow> {
        self.windows
            .lock()
            .map(|windows| windows.clone())
            .unwrap_or_default()
    }

    fn activate_window(&self, id: &str) {
        let command = if self
            .windows()
            .iter()
            .any(|window| window.id == id && window.active)
        {
            "windowminimize"
        } else {
            "windowactivate"
        };
        let id = id.to_owned();
        thread::spawn(move || {
            let _ = kdotool(&[command, &id]);
        });
    }
}

fn query_windows() -> Vec<OpenWindow> {
    let active = kdotool(&["getactivewindow"]);
    kdotool(&["search", "--name", ""])
        .map(|ids| {
            ids.lines()
                .filter_map(|id| window_from_id(id.trim(), active.as_deref()))
                .filter(|window| !window.title.starts_with("CreamShell"))
                .collect()
        })
        .unwrap_or_default()
}

fn window_from_id(id: &str, active: Option<&str>) -> Option<OpenWindow> {
    let title = kdotool(&["getwindowname", id])?;
    if title.is_empty() {
        return None;
    }
    let app_name = kdotool(&["getwindowclassname", id])
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| title.clone());
    Some(OpenWindow {
        id: id.to_owned(),
        icon_path: find_icon(
            &app_name,
            kdotool(&["getwindowpid", id]).and_then(|pid| pid.parse().ok()),
        ),
        app_name,
        title,
        active: active == Some(id),
    })
}

/// KWin exposes an application identifier but not a ready-to-paint raster
/// icon. Resolve it once in the background worker and retain the result.
fn find_icon(app_name: &str, pid: Option<u32>) -> Option<PathBuf> {
    let key = format!("{}:{pid:?}", app_name.to_ascii_lowercase());
    if let Ok(mut cache) = icon_cache().lock() {
        if let Some(icon) = cache.get(&key) {
            return icon.clone();
        }
        // Avoid blocking KWin's polling worker on recursive icon discovery.
        // The task initially uses its text fallback, then picks up this cache.
        cache.insert(key.clone(), None);
    }
    let lookup_key = key.clone();
    let lookup_app_name = app_name.to_owned();
    thread::spawn(move || {
        let icon = pid
            .and_then(declared_process_icon)
            .or_else(|| scan_icon(&lookup_app_name));
        if let Ok(mut cache) = icon_cache().lock() {
            cache.insert(lookup_key, icon);
        }
    });
    None
}

#[derive(Clone)]
struct DesktopEntry {
    program: String,
    icon: String,
}

fn declared_process_icon(pid: u32) -> Option<PathBuf> {
    let process = process_program(pid)?;
    desktop_entries()
        .iter()
        .find(|entry| program_matches(&entry.program, &process))
        .and_then(|entry| resolve_declared_icon(&entry.icon))
}

fn process_program(pid: u32) -> Option<PathBuf> {
    fs::read_link(format!("/proc/{pid}/exe")).ok()
}

fn desktop_entries() -> &'static Vec<DesktopEntry> {
    static ENTRIES: OnceLock<Vec<DesktopEntry>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        desktop_entry_dirs()
            .into_iter()
            .filter_map(|directory| fs::read_dir(directory).ok())
            .flat_map(|entries| entries.flatten())
            .filter_map(|entry| parse_desktop_entry(&entry.path()))
            .collect()
    })
}

fn desktop_entry_dirs() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        directories.push(PathBuf::from(data_home).join("applications"));
    } else if let Some(home) = std::env::var_os("HOME") {
        directories.push(PathBuf::from(home).join(".local/share/applications"));
    }
    let data_dirs =
        std::env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    directories
        .extend(std::env::split_paths(&data_dirs).map(|directory| directory.join("applications")));
    if let Some(home) = std::env::var_os("HOME") {
        directories
            .push(PathBuf::from(home).join(".local/share/flatpak/exports/share/applications"));
    }
    directories.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    directories
}

fn parse_desktop_entry(path: &Path) -> Option<DesktopEntry> {
    let contents = fs::read_to_string(path).ok()?;
    let mut in_desktop_entry = false;
    let mut program = None;
    let mut icon = None;
    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            if in_desktop_entry {
                break;
            }
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        if let Some(value) = line.strip_prefix("Exec=") {
            program = desktop_exec_program(value);
        } else if let Some(value) = line.strip_prefix("Icon=") {
            icon = Some(value.to_owned());
        }
    }
    Some(DesktopEntry {
        program: program?,
        icon: icon?,
    })
}

fn desktop_exec_program(exec: &str) -> Option<String> {
    let mut program = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for character in exec.chars() {
        if escaped {
            program.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if character.is_whitespace() && !quoted {
            break;
        } else {
            program.push(character);
        }
    }
    (!program.is_empty()).then_some(program)
}

fn program_matches(declared: &str, process: &Path) -> bool {
    let declared = Path::new(declared);
    if matches!(
        declared.file_name().and_then(|name| name.to_str()),
        Some("env" | "flatpak" | "sh" | "bash" | "gtk-launch" | "snap")
    ) {
        return false;
    }
    if declared.is_absolute() {
        return declared == process;
    }
    declared.file_name() == process.file_name()
}

fn resolve_declared_icon(icon: &str) -> Option<PathBuf> {
    let path = PathBuf::from(icon);
    if path.is_absolute() {
        return supported_image(&path).then_some(path);
    }
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(icon);
    scan_icon(name)
}

fn supported_image(path: &Path) -> bool {
    path.is_file()
        && path.extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("png")
                || extension.eq_ignore_ascii_case("jpg")
                || extension.eq_ignore_ascii_case("jpeg")
                || extension.eq_ignore_ascii_case("webp")
        })
}

fn icon_cache() -> &'static Mutex<HashMap<String, Option<PathBuf>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<PathBuf>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn scan_icon(app_name: &str) -> Option<PathBuf> {
    let key = app_name.to_ascii_lowercase();
    let mut names = vec![key.clone()];
    if let Some(last) = app_name.rsplit('.').next() {
        names.push(last.to_ascii_lowercase());
    }
    names.sort();
    names.dedup();
    icon_roots()
        .into_iter()
        .filter_map(|root| find_icon_in(&root, &names))
        .min_by_key(|path| icon_score(path, &names))
}

fn icon_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/share/pixmaps"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join(".local/share/icons"));
        roots.push(home.join(".local/share/flatpak/appstream"));
    }
    roots.push(PathBuf::from("/var/lib/flatpak/appstream"));
    roots
}

fn find_icon_in(root: &Path, names: &[String]) -> Option<PathBuf> {
    let mut matches = Vec::new();
    collect_icons(root, names, &mut matches);
    matches
        .into_iter()
        .min_by_key(|path| icon_score(path, names))
}

fn collect_icons(directory: &Path, names: &[String], matches: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_icons(&path, names, matches);
            continue;
        }
        let is_png = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"));
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_ascii_lowercase);
        if is_png && stem.is_some_and(|stem| names.iter().any(|name| stem.contains(name))) {
            matches.push(path);
        }
    }
}

fn icon_score(path: &Path, names: &[String]) -> (u8, u16) {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let exact = names.contains(&stem);
    let size = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .find_map(|part| {
            part.split('x')
                .next()
                .and_then(|size| size.parse::<u16>().ok())
        })
        .unwrap_or(256);
    (u8::from(!exact), size.abs_diff(64))
}

fn busctl(args: &[&str]) -> Option<String> {
    Command::new("timeout")
        .args(["2s", "busctl", "--user"])
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn kdotool(args: &[&str]) -> Option<String> {
    Command::new("timeout")
        .args(["2s", "kdotool"])
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::{desktop_exec_program, program_matches};
    use std::path::Path;

    #[test]
    fn desktop_exec_program_handles_quotes_and_arguments() {
        assert_eq!(
            desktop_exec_program("\"/opt/Example App/example\" --new-window %U"),
            Some("/opt/Example App/example".to_owned())
        );
    }

    #[test]
    fn desktop_exec_program_stops_before_arguments() {
        assert_eq!(
            desktop_exec_program("firefox %U"),
            Some("firefox".to_owned())
        );
    }

    #[test]
    fn program_matching_rejects_shared_launchers() {
        assert!(program_matches("firefox", Path::new("/usr/bin/firefox")));
        assert!(!program_matches("flatpak", Path::new("/usr/bin/flatpak")));
    }
}
