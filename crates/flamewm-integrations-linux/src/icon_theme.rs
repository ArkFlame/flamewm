//! Freedesktop `index.theme` parser and inheritance expansion (J2).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconDirType {
    Fixed,
    Scalable,
    Threshold,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconDir {
    pub size: u32,
    pub scale: u32,
    pub dir_type: IconDirType,
    pub min_size: u32,
    pub max_size: u32,
    pub threshold: u32,
}

impl Default for IconDir {
    fn default() -> Self {
        Self {
            size: 0,
            scale: 1,
            dir_type: IconDirType::Threshold,
            min_size: 0,
            max_size: 0,
            threshold: 2,
        }
    }
}

impl IconDir {
    /// Distance from `requested` to this directory. Lower is better; 0 is exact.
    #[must_use]
    pub fn distance(&self, requested: u32) -> u64 {
        let req = u64::from(requested);
        match self.dir_type {
            IconDirType::Fixed => {
                let nominal = u64::from(self.size) * u64::from(self.scale.max(1));
                req.abs_diff(nominal)
            }
            IconDirType::Scalable => {
                let (mut lo, mut hi) = (u64::from(self.min_size), u64::from(self.max_size));
                if lo == 0 && hi == 0 {
                    let nominal = u64::from(self.size) * u64::from(self.scale.max(1));
                    return req.abs_diff(nominal);
                }
                if lo > hi {
                    std::mem::swap(&mut lo, &mut hi);
                }
                if req < lo {
                    lo - req
                } else if req > hi {
                    req - hi
                } else {
                    0
                }
            }
            IconDirType::Threshold => {
                let nominal = u64::from(self.size) * u64::from(self.scale.max(1));
                let t = u64::from(self.threshold);
                if req.abs_diff(nominal) <= t {
                    0
                } else if req < nominal {
                    nominal - t - req
                } else {
                    req - (nominal + t)
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct IconThemeIndex {
    pub name: String,
    pub inherits: Vec<String>,
    pub directories: Vec<String>,
    pub dirs: HashMap<String, IconDir>,
}

/// Bounded per-resolver theme lookup caches (J03 warm path).
///
/// All entries are scoped by `theme_generation`; a generation bump makes old
/// entries miss without clearing live-raster bytes. No raw map is exposed;
/// use [`IconThemeCaches::stats`] / `memory_estimate_bytes`.
#[derive(Clone, Debug, Default)]
pub struct IconThemeCaches {
    roots: HashMap<(String, u64), Vec<PathBuf>>,
    indexes: HashMap<(String, u64), Option<IconThemeIndex>>,
    inheritance: HashMap<(String, u64), Vec<String>>,
    exists: HashMap<(PathBuf, u64), bool>,
    hits: u64,
    misses: u64,
}

impl IconThemeCaches {
    fn roots(&mut self, theme: &str, generation: u64) -> Option<Vec<PathBuf>> {
        let key = (theme.to_owned(), generation);
        if let Some(hit) = self.roots.get(&key) {
            self.hits += 1;
            return Some(hit.clone());
        }
        self.misses += 1;
        None
    }

    pub(crate) fn cached_roots(&mut self, theme: &str, generation: u64) -> Option<Vec<PathBuf>> {
        self.roots(theme, generation)
    }

    pub(crate) fn store_roots(&mut self, theme: &str, generation: u64, roots: Vec<PathBuf>) {
        bound_insert(&mut self.roots, (theme.to_owned(), generation), roots, 512);
    }

    fn index(&mut self, theme: &str, generation: u64) -> Option<Option<IconThemeIndex>> {
        let key = (theme.to_owned(), generation);
        if let Some(hit) = self.indexes.get(&key) {
            self.hits += 1;
            return Some(hit.clone());
        }
        self.misses += 1;
        None
    }

    pub(crate) fn cached_index(
        &mut self,
        theme: &str,
        generation: u64,
    ) -> Option<Option<IconThemeIndex>> {
        self.index(theme, generation)
    }

    pub(crate) fn store_index(
        &mut self,
        theme: &str,
        generation: u64,
        index: Option<IconThemeIndex>,
    ) {
        bound_insert(
            &mut self.indexes,
            (theme.to_owned(), generation),
            index,
            256,
        );
    }

    fn inheritance(&mut self, seeds: &str, generation: u64) -> Option<Vec<String>> {
        let key = (seeds.to_owned(), generation);
        if let Some(hit) = self.inheritance.get(&key) {
            self.hits += 1;
            return Some(hit.clone());
        }
        self.misses += 1;
        None
    }

    pub(crate) fn cached_inheritance(
        &mut self,
        seeds: &str,
        generation: u64,
    ) -> Option<Vec<String>> {
        self.inheritance(seeds, generation)
    }

    pub(crate) fn store_inheritance(&mut self, seeds: &str, generation: u64, ordered: Vec<String>) {
        bound_insert(
            &mut self.inheritance,
            (seeds.to_owned(), generation),
            ordered,
            128,
        );
    }

    fn exists(&mut self, path: &Path, generation: u64) -> Option<bool> {
        let key = (path.to_path_buf(), generation);
        if let Some(hit) = self.exists.get(&key) {
            self.hits += 1;
            return Some(*hit);
        }
        self.misses += 1;
        None
    }

    pub(crate) fn cached_exists(&mut self, path: &Path, generation: u64) -> Option<bool> {
        self.exists(path, generation)
    }

    pub(crate) fn store_exists(&mut self, path: &Path, generation: u64, exists: bool) {
        bound_insert(
            &mut self.exists,
            (path.to_path_buf(), generation),
            exists,
            4096,
        );
    }

    /// (hits, misses) across roots/index/inheritance/exists lookups.
    #[must_use]
    pub fn stats(&self) -> (u64, u64) {
        (self.hits, self.misses)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.roots.len() + self.indexes.len() + self.inheritance.len() + self.exists.len()
    }

    #[must_use]
    pub fn memory_estimate_bytes(&self) -> usize {
        let mut bytes = 0;
        for ((theme, _), roots) in &self.roots {
            bytes += theme.len() + 8;
            for root in roots {
                bytes += root.as_os_str().len() + 32;
            }
        }
        for ((theme, _), index) in &self.indexes {
            bytes += theme.len() + 8 + 64;
            if let Some(index) = index {
                bytes += index.name.len();
                for parent in &index.inherits {
                    bytes += parent.len();
                }
                for dir in &index.directories {
                    bytes += dir.len() + 48;
                }
            }
        }
        for ((seeds, _), ordered) in &self.inheritance {
            bytes += seeds.len() + 8;
            for name in ordered {
                bytes += name.len();
            }
        }
        for ((path, _), _) in &self.exists {
            bytes += path.as_os_str().len() + 16;
        }
        bytes
    }

    fn clear_generation(&mut self) {
        // Generations are key-scoped so no clear is required; this hook keeps
        // the bound small if a host cycles generations rapidly.
        while self.len() > 4096 + 512 + 256 + 128 {
            if let Some(first) = self.exists.keys().next().cloned() {
                self.exists.remove(&first);
            } else {
                break;
            }
        }
    }

    pub(crate) fn note_generation_bump(&mut self) {
        self.clear_generation();
    }
}

fn bound_insert<K: std::hash::Hash + Eq + Clone, V>(
    map: &mut HashMap<K, V>,
    key: K,
    value: V,
    cap: usize,
) {
    if map.len() >= cap && !map.contains_key(&key) {
        // Bounded: drop an arbitrary entry rather than growing without limit.
        if let Some(first) = map.keys().next().cloned() {
            map.remove(&first);
        }
    }
    map.insert(key, value);
}

impl IconThemeIndex {
    #[must_use]
    pub fn parse(name: &str, text: &str) -> Self {
        let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut current = String::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                current = line[1..line.len() - 1].trim().to_owned();
                sections.entry(current.clone()).or_default();
                continue;
            }
            if current.is_empty() {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                sections
                    .entry(current.clone())
                    .or_default()
                    .insert(key.trim().to_owned(), value.trim().to_owned());
            }
        }
        let header = sections.get("Icon Theme").cloned().unwrap_or_default();
        let directories = split_list(header.get("Directories").map(String::as_str).unwrap_or(""));
        let inherits = split_list(header.get("Inherits").map(String::as_str).unwrap_or(""));
        let mut dirs = HashMap::new();
        for dir in &directories {
            if let Some(section) = sections.get(dir.as_str()) {
                dirs.insert(dir.clone(), parse_dir(section));
            }
        }
        // ScaledDirectories entries share the same section format.
        if let Some(scaled) = header.get("ScaledDirectories") {
            for dir in split_list(scaled) {
                if !dirs.contains_key(&dir) {
                    if let Some(section) = sections.get(dir.as_str()) {
                        let mut entry = parse_dir(section);
                        if entry.scale == 1 {
                            entry.scale = 2;
                        }
                        dirs.insert(dir.clone(), entry);
                    }
                }
            }
        }
        Self {
            name: name.to_owned(),
            inherits,
            directories,
            dirs,
        }
    }

    #[must_use]
    pub fn load(theme_dir: &Path) -> Option<Self> {
        let name = theme_dir
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default();
        let text = fs::read_to_string(theme_dir.join("index.theme")).ok()?;
        Some(Self::parse(&name, &text))
    }

    /// Directories ordered by proximity to `requested` (exact first).
    #[must_use]
    pub fn ordered_dirs(&self, requested: u32) -> Vec<(String, u64)> {
        let mut ordered: Vec<(String, u64)> = self
            .directories
            .iter()
            .map(|dir| {
                let distance = self
                    .dirs
                    .get(dir)
                    .map(|entry| entry.distance(requested))
                    .unwrap_or(u64::MAX / 2);
                (dir.clone(), distance)
            })
            .collect();
        ordered.sort_by_key(|(_, distance)| *distance);
        ordered
    }
}

fn parse_dir(section: &HashMap<String, String>) -> IconDir {
    let size = section
        .get("Size")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let scale = section
        .get("Scale")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let dir_type = match section.get("Type").map(String::as_str).unwrap_or("") {
        "Fixed" => IconDirType::Fixed,
        "Scalable" => IconDirType::Scalable,
        _ => IconDirType::Threshold,
    };
    let threshold = section
        .get("Threshold")
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);
    let min_size = section
        .get("MinSize")
        .and_then(|v| v.parse().ok())
        .unwrap_or(size);
    let max_size = section
        .get("MaxSize")
        .and_then(|v| v.parse().ok())
        .unwrap_or(size);
    IconDir {
        size,
        scale: scale.max(1),
        dir_type,
        min_size,
        max_size,
        threshold,
    }
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// Cycle-safe recursive `Inherits` expansion. `hicolor` is always the final
/// fallback. `loader` maps a theme name to its parsed index (or `None`).
pub fn inheritance_chain(
    start: &str,
    loader: &dyn Fn(&str) -> Option<IconThemeIndex>,
) -> Vec<String> {
    let mut chain = Vec::new();
    let mut visiting: Vec<String> = Vec::new();
    expand(start, loader, &mut chain, &mut visiting);
    if !chain.iter().any(|name| name == "hicolor") {
        chain.push("hicolor".to_owned());
    }
    chain
}

fn expand(
    name: &str,
    loader: &dyn Fn(&str) -> Option<IconThemeIndex>,
    chain: &mut Vec<String>,
    visiting: &mut Vec<String>,
) {
    if name.is_empty() || chain.iter().any(|item| item == name) {
        return;
    }
    if visiting.iter().any(|item| item == name) {
        return;
    }
    visiting.push(name.to_owned());
    chain.push(name.to_owned());
    if let Some(index) = loader(name) {
        for parent in index.inherits {
            expand(&parent, loader, chain, visiting);
        }
    }
    visiting.pop();
}

/// Search roots: `$HOME/.icons`, `$XDG_DATA_HOME/icons`, each
/// `$XDG_DATA_DIRS/icons`. Returned roots may not exist; callers skip missing.
#[must_use]
pub fn search_roots(data_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        if !home.as_os_str().is_empty() {
            roots.push(home.join(".icons"));
        }
    }
    for data_dir in data_dirs {
        roots.push(data_dir.join("icons"));
    }
    let mut unique = Vec::new();
    for root in roots {
        if !unique.contains(&root) {
            unique.push(root);
        }
    }
    unique
}

/// Locate `index.theme` for `theme` under `data_dirs`.
#[must_use]
pub fn theme_dir(theme: &str, data_dirs: &[PathBuf]) -> Option<PathBuf> {
    for root in search_roots(data_dirs) {
        let dir = root.join(theme);
        if dir.join("index.theme").is_file() {
            return Some(dir);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_directories_and_inherits() {
        let text = "[Icon Theme]\nName=Test\nDirectories=16x16/apps,scalable/apps\nInherits=Breeze,hicolor\n\n[16x16/apps]\nSize=16\nType=Fixed\n\n[scalable/apps]\nSize=48\nType=Scalable\nMinSize=16\nMaxSize=512\n";
        let index = IconThemeIndex::parse("Test", text);
        assert_eq!(index.inherits, vec!["Breeze", "hicolor"]);
        assert_eq!(index.directories.len(), 2);
        assert_eq!(index.dirs["16x16/apps"].dir_type, IconDirType::Fixed);
        assert_eq!(index.dirs["scalable/apps"].min_size, 16);
    }

    #[test]
    fn cycle_terminates() {
        let loader = |name: &str| {
            let inherits = match name {
                "A" => vec!["B".to_owned()],
                "B" => vec!["A".to_owned()],
                _ => Vec::new(),
            };
            Some(IconThemeIndex {
                name: name.to_owned(),
                inherits,
                ..IconThemeIndex::default()
            })
        };
        let chain = inheritance_chain("A", &loader);
        assert_eq!(chain.first().map(String::as_str), Some("A"));
        assert!(chain.contains(&"B".to_owned()));
        assert_eq!(chain.last().map(String::as_str), Some("hicolor"));
        assert!(chain.iter().filter(|n| *n == "A").count() == 1);
    }
}
