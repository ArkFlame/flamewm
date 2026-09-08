use flamewm_api::applications::{ApplicationLaunchOptions, DesktopApplication};
use flamewm_api::{DesktopAppId, ErrorCode, FlameError, FlameResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopEntry {
    pub id: DesktopAppId,
    pub application: DesktopApplication,
    pub path: PathBuf,
    exec: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ApplicationCatalog {
    applications: Vec<DesktopEntry>,
}

pub mod catalog {
    pub use super::{ApplicationCatalog, DesktopEntry, xdg_application_dirs};
}

pub mod exec {
    pub use super::{expand_exec, tokenize_exec};
}

pub mod launcher {
    pub use super::launch_argv;
}

impl DesktopEntry {
    pub fn from_file(path: &Path) -> FlameResult<Option<Self>> {
        let file_name = path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .ok_or_else(|| FlameError::invalid("desktop entry path has no file name"))?;
        Self::from_file_with_id(path, DesktopAppId::new(file_name))
    }

    pub fn from_file_with_id(path: &Path, id: DesktopAppId) -> FlameResult<Option<Self>> {
        parse_desktop_entry(path, id)
    }

    #[must_use]
    pub fn id(&self) -> &DesktopAppId {
        &self.id
    }

    #[must_use]
    pub fn application(&self) -> &DesktopApplication {
        &self.application
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.application.name
    }

    #[must_use]
    pub fn icon(&self) -> &str {
        &self.application.icon_name
    }

    #[must_use]
    pub fn exec(&self) -> &str {
        &self.exec
    }

    pub fn argv(&self, options: &ApplicationLaunchOptions) -> FlameResult<Vec<String>> {
        expand_exec_with_path(&self.exec, &self.application, &self.path, options)
    }

    pub fn launch(&self, options: &ApplicationLaunchOptions) -> FlameResult<Child> {
        let argv = self.argv(options)?;
        launch_argv(&argv, &ApplicationLaunchOptions::default())
    }
}

impl ApplicationCatalog {
    pub fn discover() -> FlameResult<Self> {
        Self::from_dirs(&xdg_application_dirs())
    }

    pub fn from_dirs(dirs: &[PathBuf]) -> FlameResult<Self> {
        let mut entries = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for dir in dirs {
            if !dir.is_dir() {
                continue;
            }
            let mut files = Vec::new();
            collect_desktop_files(dir, dir, &mut files)?;
            files.sort();
            for path in files {
                let id = desktop_id(dir, &path);
                if !seen.insert(id.clone()) {
                    continue;
                }
                if let Some(entry) = parse_desktop_entry(&path, id)? {
                    entries.push(entry);
                }
            }
        }
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Self {
            applications: entries,
        })
    }

    #[must_use]
    pub fn all(&self) -> Vec<DesktopApplication> {
        self.applications
            .iter()
            .map(|entry| entry.application.clone())
            .collect()
    }

    #[must_use]
    pub fn entries(&self) -> &[DesktopEntry] {
        &self.applications
    }

    pub fn find(&self, id: &DesktopAppId) -> FlameResult<&DesktopEntry> {
        self.applications
            .iter()
            .find(|entry| &entry.id == id)
            .ok_or_else(|| FlameError::new(ErrorCode::NotFound, "application id not found"))
    }

    #[must_use]
    pub fn search(&self, query: &str) -> Vec<DesktopApplication> {
        let needle = query.trim().to_lowercase();
        self.applications
            .iter()
            .filter(|entry| {
                needle.is_empty() || {
                    let app = &entry.application;
                    app.id.as_str().to_lowercase().contains(&needle)
                        || app.name.to_lowercase().contains(&needle)
                        || app.generic_name.to_lowercase().contains(&needle)
                        || app.comment.to_lowercase().contains(&needle)
                        || app
                            .keywords
                            .iter()
                            .any(|v| v.to_lowercase().contains(&needle))
                        || app
                            .categories
                            .iter()
                            .any(|v| v.to_lowercase().contains(&needle))
                }
            })
            .map(|entry| entry.application.clone())
            .collect()
    }

    pub fn launch(
        &self,
        id: &DesktopAppId,
        options: &ApplicationLaunchOptions,
    ) -> FlameResult<Child> {
        let entry = self.find(id)?;
        let argv = expand_exec_with_path(&entry.exec, &entry.application, &entry.path, options)?;
        launch_argv(&argv, &ApplicationLaunchOptions::default())
    }
}

#[must_use]
pub fn xdg_application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.map(|path| path.join(".local/share")));
    if let Some(path) = data_home {
        dirs.push(path.join("applications"));
    }
    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_owned());
    for path in data_dirs.split(':').filter(|value| !value.is_empty()) {
        let path = PathBuf::from(path).join("applications");
        if !dirs.contains(&path) {
            dirs.push(path);
        }
    }
    dirs
}

pub fn tokenize_exec(exec: &str) -> FlameResult<Vec<String>> {
    let mut result = Vec::new();
    let mut token = String::new();
    let mut quote = None;
    let mut chars = exec.chars().peekable();
    while let Some(ch) = chars.next() {
        match quote {
            Some('"') => match ch {
                '"' => quote = None,
                '\\' => token.push(
                    chars
                        .next()
                        .ok_or_else(|| FlameError::invalid("unterminated Exec escape"))?,
                ),
                _ => token.push(ch),
            },
            Some('\'') => match ch {
                '\'' => quote = None,
                _ => token.push(ch),
            },
            None => match ch {
                '\'' | '"' => quote = Some(ch),
                '\\' => token.push(
                    chars
                        .next()
                        .ok_or_else(|| FlameError::invalid("unterminated Exec escape"))?,
                ),
                c if c.is_whitespace() => {
                    if !token.is_empty() {
                        result.push(std::mem::take(&mut token));
                    }
                }
                _ => token.push(ch),
            },
            _ => unreachable!(),
        }
    }
    if quote.is_some() {
        return Err(FlameError::invalid("unterminated Exec quote"));
    }
    if !token.is_empty() {
        result.push(token);
    }
    if result.is_empty() {
        return Err(FlameError::invalid("empty Exec"));
    }
    Ok(result)
}

pub fn expand_exec(
    exec: &str,
    app: &DesktopApplication,
    options: &ApplicationLaunchOptions,
) -> FlameResult<Vec<String>> {
    expand_exec_with_path(exec, app, Path::new(""), options)
}

fn expand_exec_with_path(
    exec: &str,
    app: &DesktopApplication,
    desktop_path: &Path,
    options: &ApplicationLaunchOptions,
) -> FlameResult<Vec<String>> {
    let tokens = tokenize_exec(exec)?;
    let mut argv = Vec::new();
    for token in tokens {
        let mut output = String::new();
        let mut chars = token.chars();
        while let Some(ch) = chars.next() {
            if ch != '%' {
                output.push(ch);
                continue;
            }
            match chars.next() {
                Some('%') => output.push('%'),
                Some('c') => output.push_str(&app.name),
                Some('k') => output.push_str(&desktop_path.to_string_lossy()),
                Some('i') => {
                    if !app.icon_name.is_empty() {
                        argv.push("--icon".to_owned());
                        argv.push(app.icon_name.clone());
                    }
                }
                Some('f' | 'u') => {
                    if let Some(value) = options.uris.first() {
                        output.push_str(value);
                    }
                }
                Some(code @ ('F' | 'U')) => {
                    if token != format!("%{code}") {
                        return Err(FlameError::invalid(
                            "list Exec field code must be standalone",
                        ));
                    }
                    argv.extend(options.uris.iter().cloned());
                }
                Some(code) => {
                    return Err(FlameError::invalid(format!(
                        "unsupported Exec field code %{code}"
                    )));
                }
                None => return Err(FlameError::invalid("trailing Exec field marker")),
            }
        }
        if !output.is_empty() {
            argv.push(output);
        }
    }
    argv.extend(options.extra_args.iter().cloned());
    Ok(argv)
}

pub fn launch_argv(argv: &[String], options: &ApplicationLaunchOptions) -> FlameResult<Child> {
    let program = argv
        .first()
        .ok_or_else(|| FlameError::invalid("empty application argv"))?;
    let mut command = Command::new(program);
    command
        .args(&argv[1..])
        .args(&options.extra_args)
        .args(&options.uris);
    command.spawn().map_err(|error| {
        FlameError::new(ErrorCode::IoFailure, format!("launch application: {error}"))
    })
}

fn collect_desktop_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> FlameResult<()> {
    for item in fs::read_dir(dir).map_err(|error| {
        FlameError::new(
            ErrorCode::IoFailure,
            format!("read applications directory: {error}"),
        )
    })? {
        let path = item
            .map_err(|error| FlameError::new(ErrorCode::IoFailure, error.to_string()))?
            .path();
        if path.is_dir() {
            collect_desktop_files(root, &path, files)?;
        } else if path.extension().and_then(|v| v.to_str()) == Some("desktop") {
            files.push(path);
        }
    }
    Ok(())
}

fn desktop_id(root: &Path, path: &Path) -> DesktopAppId {
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('/', "-");
    DesktopAppId::new(rel)
}

fn parse_desktop_entry(path: &Path, id: DesktopAppId) -> FlameResult<Option<DesktopEntry>> {
    let text = fs::read_to_string(path).map_err(|error| {
        FlameError::new(ErrorCode::IoFailure, format!("read desktop entry: {error}"))
    })?;
    let mut values = std::collections::BTreeMap::new();
    let mut section = false;
    for line in text.lines().map(|line| line.trim_end_matches('\r')) {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = &line[1..line.len() - 1] == "Desktop Entry";
            continue;
        }
        if !section || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }
    if values
        .get("Type")
        .map(String::as_str)
        .unwrap_or("Application")
        != "Application"
        || is_true(values.get("Hidden"))
        || is_true(values.get("NoDisplay"))
    {
        return Ok(None);
    }
    if !desktop_environment_matches(&values) || !try_exec_is_available(values.get("TryExec")) {
        return Ok(None);
    }
    let exec = match values.get("Exec") {
        Some(value) => value,
        None => return Ok(None),
    };
    let argv = tokenize_exec(exec)?;
    if values
        .get("Name")
        .map(|value| value.trim())
        .unwrap_or("")
        .is_empty()
    {
        return Ok(None);
    }
    let app = DesktopApplication {
        id: id.clone(),
        name: values.get("Name").cloned().unwrap_or_default(),
        generic_name: values.get("GenericName").cloned().unwrap_or_default(),
        comment: values.get("Comment").cloned().unwrap_or_default(),
        startup_wm_class: values.get("StartupWMClass").cloned().unwrap_or_default(),
        argv,
        keywords: list_value(values.get("Keywords")),
        icon_name: values.get("Icon").cloned().unwrap_or_default(),
        categories: list_value(values.get("Categories")),
    };
    Ok(Some(DesktopEntry {
        id,
        application: app,
        path: path.to_owned(),
        exec: exec.clone(),
    }))
}

fn is_true(value: Option<&String>) -> bool {
    value
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(false)
}
fn list_value(value: Option<&String>) -> Vec<String> {
    value
        .map(|v| {
            v.split(';')
                .filter(|item| !item.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn desktop_environment_matches(values: &std::collections::BTreeMap<String, String>) -> bool {
    let current = std::env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .or_else(|| std::env::var("DESKTOP_SESSION").ok());
    let Some(current) = current else {
        return true;
    };
    let names: Vec<&str> = current
        .split(':')
        .filter(|value| !value.is_empty())
        .collect();
    let only = list_value(values.get("OnlyShowIn"));
    let not = list_value(values.get("NotShowIn"));
    (only.is_empty()
        || only
            .iter()
            .any(|value| names.iter().any(|name| name.eq_ignore_ascii_case(value))))
        && !not
            .iter()
            .any(|value| names.iter().any(|name| name.eq_ignore_ascii_case(value)))
}

fn try_exec_is_available(value: Option<&String>) -> bool {
    let Some(program) = value
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    else {
        return true;
    };
    let path = Path::new(program);
    if path.components().count() > 1 {
        return path.is_file();
    }
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    paths
        .as_os_str()
        .to_string_lossy()
        .split(':')
        .filter(|directory| !directory.is_empty())
        .map(|directory| Path::new(directory).join(program))
        .any(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "flamewm-applications-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(path.join("applications/nested")).unwrap();
        fs::write(path.join("applications/nested/example.desktop"), "[Desktop Entry]\nType=Application\nName=Example Tool\nGenericName=Editor\nComment=Fixture\nKeywords=write;fixture;\nCategories=Utility;\nIcon=example\nExec=fixture %U --literal=DISPLAY=:9\n").unwrap();
        fs::write(
            path.join("applications/nested/hidden.desktop"),
            "[Desktop Entry]\nType=Application\nName=Hidden\nHidden=true\nExec=hidden\n",
        )
        .unwrap();
        path
    }

    #[test]
    fn catalog_precedence_search_and_desktop_id() {
        let root = fixture();
        let catalog = ApplicationCatalog::from_dirs(&[root.join("applications")]).unwrap();
        assert_eq!(catalog.entries().len(), 1);
        assert_eq!(catalog.entries()[0].id.as_str(), "nested-example.desktop");
        assert_eq!(catalog.search("fixture")[0].name, "Example Tool");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn strict_exec_argv_keeps_nested_display_literal() {
        let app = DesktopApplication {
            id: DesktopAppId::new("example.desktop"),
            name: "Example".into(),
            generic_name: String::new(),
            comment: String::new(),
            startup_wm_class: String::new(),
            argv: Vec::new(),
            keywords: Vec::new(),
            icon_name: "icon".into(),
            categories: Vec::new(),
        };
        let options = ApplicationLaunchOptions {
            extra_args: vec!["--extra".into()],
            uris: vec!["file:///tmp/a b".into()],
        };
        assert_eq!(
            expand_exec("program %U --literal=DISPLAY=:9", &app, &options).unwrap(),
            vec![
                "program",
                "file:///tmp/a b",
                "--literal=DISPLAY=:9",
                "--extra"
            ]
        );
    }

    fn write_entry(root: &Path, name: &str, body: &str) -> PathBuf {
        let path = root.join(name);
        fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn from_file_exposes_name_icon_exec() {
        let root = fixture();
        let path = write_entry(
            &root,
            "entry.desktop",
            "[Desktop Entry]\nType=Application\nName=Entry Name\nIcon=entry-icon\nExec=myapp --flag\n",
        );
        let entry = DesktopEntry::from_file(&path).unwrap().unwrap();
        assert_eq!(entry.name(), "Entry Name");
        assert_eq!(entry.icon(), "entry-icon");
        assert_eq!(entry.exec(), "myapp --flag");
        assert_eq!(entry.application().name, "Entry Name");
        assert_eq!(entry.path(), path.as_path());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn from_file_absolute_icon_path() {
        let root = fixture();
        let path = write_entry(
            &root,
            "absolute.desktop",
            "[Desktop Entry]\nType=Application\nName=Absolute\nIcon=/usr/share/icons/absolute.png\nExec=myapp\n",
        );
        let entry = DesktopEntry::from_file(&path).unwrap().unwrap();
        assert_eq!(entry.icon(), "/usr/share/icons/absolute.png");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn from_file_icon_name() {
        let root = fixture();
        let path = write_entry(
            &root,
            "named.desktop",
            "[Desktop Entry]\nType=Application\nName=Named\nIcon=text-editor\nExec=myapp\n",
        );
        let entry = DesktopEntry::from_file(&path).unwrap().unwrap();
        assert_eq!(entry.icon(), "text-editor");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn from_file_expands_field_codes() {
        let root = fixture();
        let path = write_entry(
            &root,
            "codes.desktop",
            "[Desktop Entry]\nType=Application\nName=Field Codes\nIcon=icon\nExec=myapp %c %k %f\n",
        );
        let entry = DesktopEntry::from_file(&path).unwrap().unwrap();
        let options = ApplicationLaunchOptions {
            extra_args: Vec::new(),
            uris: vec!["file:///tmp/input".into()],
        };
        let argv = entry.argv(&options).unwrap();
        let desktop = path.to_string_lossy().into_owned();
        assert_eq!(
            argv,
            vec![
                "myapp".to_owned(),
                "Field Codes".to_owned(),
                desktop,
                "file:///tmp/input".to_owned(),
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn from_file_missing_exec_is_none() {
        let root = fixture();
        let path = write_entry(
            &root,
            "noexec.desktop",
            "[Desktop Entry]\nType=Application\nName=No Exec\nIcon=icon\n",
        );
        assert!(DesktopEntry::from_file(&path).unwrap().is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn from_file_hidden_is_none() {
        let root = fixture();
        let path = write_entry(
            &root,
            "hidden-entry.desktop",
            "[Desktop Entry]\nType=Application\nName=Hidden Entry\nHidden=true\nExec=myapp\n",
        );
        assert!(DesktopEntry::from_file(&path).unwrap().is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
