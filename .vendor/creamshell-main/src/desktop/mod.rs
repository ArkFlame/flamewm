use crate::bar::DOCK_HEIGHT;
use crate::icons::pixel_icon;
use creamui_core::layout::{
    Dimension, LengthPercentageAuto, Position, Rect as LayoutRect, Size as LayoutSize,
    Style as LayoutStyle,
};
use creamui_core::{BoxedWidget, Point, Size, StateStyle, Style, Styled};
use creamui_image::{Image, ImageData, ImageFit};
use creamui_macros::jsx;
use creamui_reactive::Signal;
use creamui_render::AppHandle;
use creamui_theme::Color;
use creamui_widgets::layout::{fixed, Align, Justify};
use creamui_widgets::{RawButton, RawView};
use std::cell::Cell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::OnceLock;

pub const BACKGROUND: Color = Color::rgb(29, 37, 48);

const ICON_SIZE: f32 = 104.0;
const GRID_CELL: f32 = 120.0;
const GRID_INSET: f32 = 24.0;
const ICON_LABEL: Color = Color::rgb(248, 249, 251);
const ICON_HOVER: Color = Color::rgba(255, 255, 255, 30);
const ICON_PRESSED: Color = Color::rgba(255, 255, 255, 46);

#[derive(Clone)]
pub struct DesktopState {
    positions: Signal<HashMap<usize, Point>>,
    drag_preview: Signal<Option<DragPreview>>,
    entries: Signal<Vec<DesktopEntry>>,
    /// The pointer's grab offset within the dragged icon, captured once at
    /// `with_drag_start`. Reconciliation reconstructs `desktop_icon`'s
    /// widget (and whatever it closes over) on every rebuild of the icon
    /// grid, so this can't live in a `Cell` local to that closure — it has
    /// to live on `DesktopState`, which reconciliation never touches, or a
    /// rebuild mid-drag would silently reset it to zero.
    grab: Rc<Cell<Point>>,
}

impl Default for DesktopState {
    fn default() -> Self {
        Self {
            positions: Signal::new(HashMap::new()),
            drag_preview: Signal::new(None),
            entries: Signal::new(Vec::new()),
            grab: Rc::new(Cell::new(Point::default())),
        }
    }
}

impl DesktopState {
    pub fn start_loading(&self, app: &AppHandle) {
        let entries = self.entries.clone();
        app.spawn_background(load_entries, move |next| entries.set(next));
    }
}

#[derive(Clone)]
enum DesktopTarget {
    Path(PathBuf),
    Launch {
        program: String,
        args: Vec<String>,
        terminal: bool,
    },
}

#[derive(Clone, Copy)]
enum EntryKind {
    Folder,
    File,
    Launcher,
}

#[derive(Clone)]
struct DesktopEntry {
    label: String,
    icon: Option<ImageData>,
    kind: EntryKind,
    target: DesktopTarget,
}

#[derive(Clone)]
struct DragPreview {
    position: Point,
    entry: DesktopEntry,
}

pub fn build(viewport: Size, state: DesktopState) -> BoxedWidget {
    Box::new(
        RawView::new(
            Style::new()
                .layout(LayoutStyle {
                    size: fixed(viewport.width, viewport.height),
                    ..Default::default()
                })
                .background(BACKGROUND),
        )
        .with_children(vec![
            wallpaper_layer(),
            widget_layer(),
            icon_layer(viewport, state),
        ]),
    )
}

pub fn build_drag_overlay(viewport: Size, state: DesktopState) -> BoxedWidget {
    let children = state
        .drag_preview
        .get()
        .map(|preview| {
            vec![Box::new(
                RawView::new(icon_style(preview.position)).child(icon_content(&preview.entry)),
            ) as BoxedWidget]
        })
        .unwrap_or_default();
    Box::new(
        RawView::new(LayoutStyle {
            size: fixed(viewport.width, viewport.height),
            ..Default::default()
        })
        .with_children(children),
    )
}

fn wallpaper_layer() -> BoxedWidget {
    configured_wallpaper()
        .map(|data| {
            Box::new(Image::new(data).layout(fill_layout()).fit(ImageFit::Cover)) as BoxedWidget
        })
        .unwrap_or_else(|| Box::new(RawView::new(fill_layout())))
}

fn widget_layer() -> BoxedWidget {
    Box::new(RawView::new(fill_layout()))
}

fn icon_layer(viewport: Size, state: DesktopState) -> BoxedWidget {
    let positions = state.positions.get();
    let entries = state.entries.get();
    let children = entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let position = positions
                .get(&index)
                .copied()
                .unwrap_or_else(|| initial_position(index));
            desktop_icon(viewport, state.clone(), index, position, entry)
        })
        .collect();
    Box::new(
        RawView::new(LayoutStyle {
            size: fixed(viewport.width, viewport.height),
            ..Default::default()
        })
        .with_children(children),
    )
}

fn desktop_icon(
    viewport: Size,
    state: DesktopState,
    index: usize,
    position: Point,
    entry: DesktopEntry,
) -> BoxedWidget {
    let target = entry.target.clone();
    let grab = state.grab.clone();
    let drag_start = grab.clone();
    let drag_state = state.clone();
    let drag_entry = entry.clone();
    let drop_state = state.clone();
    Box::new(
        RawButton::new(icon_style(position), move || open_target(target.clone()))
            .with_drag_start(move |local, _| {
                drag_start.set(local);
            })
            .with_drag(move |local, rect| {
                let next = free_icon_position(
                    Point {
                        x: rect.x + local.x - grab.get().x,
                        y: rect.y + local.y - grab.get().y,
                    },
                    viewport,
                );
                drag_state.drag_preview.set(Some(DragPreview {
                    position: next,
                    entry: drag_entry.clone(),
                }));
            })
            .with_drag_end(move || {
                let dropped = drop_state
                    .drag_preview
                    .peek()
                    .map_or(position, |preview| preview.position);
                let position = icon_position(dropped, viewport);
                drop_state.drag_preview.set(None);
                if drop_state.positions.peek().get(&index) != Some(&position) {
                    drop_state.positions.update(|positions| {
                        positions.insert(index, position);
                    });
                }
            })
            .child(icon_content(&entry)),
    )
}

fn icon_content(entry: &DesktopEntry) -> BoxedWidget {
    let icon = entry_icon(entry);
    Box::new(jsx! {
        <Flex direction={creamui_core::layout::FlexDirection::Column} size={(ICON_SIZE, ICON_SIZE)} padding={8.0} gap={8.0} align={Align::Center} justify={Justify::Center}>
            <Flex size={(58.0, 58.0)} align={Align::Center} justify={Justify::Center} background={entry_color(entry.kind)} border={(Color::rgba(255, 255, 255, 34), 1.0)} corner_radius={18.0}>
                {icon}
            </Flex>
            <RawText color={ICON_LABEL} font_size={12.0}>{short_name(&entry.label)}</RawText>
        </Flex>
    })
}

fn free_icon_position(position: Point, viewport: Size) -> Point {
    Point {
        x: position.x.clamp(
            GRID_INSET,
            (viewport.width - ICON_SIZE - GRID_INSET).max(GRID_INSET),
        ),
        y: position.y.clamp(
            GRID_INSET,
            (viewport.height - ICON_SIZE - DOCK_HEIGHT as f32 - GRID_INSET).max(GRID_INSET),
        ),
    }
}

fn entry_icon(entry: &DesktopEntry) -> BoxedWidget {
    if let Some(data) = entry.icon.clone() {
        return Box::new(
            Image::new(data)
                .layout(LayoutStyle {
                    size: fixed(34.0, 34.0),
                    ..Default::default()
                })
                .fit(ImageFit::Contain),
        );
    }
    pixel_icon(
        match entry.kind {
            EntryKind::Folder => "desktop-folder",
            EntryKind::File => "desktop-file",
            EntryKind::Launcher => "appgrid",
        },
        32.0,
    )
}

fn configured_wallpaper() -> Option<ImageData> {
    static WALLPAPER: OnceLock<Option<ImageData>> = OnceLock::new();
    WALLPAPER
        .get_or_init(|| {
            std::env::var_os("CREAMSHELL_WALLPAPER")
                .map(PathBuf::from)
                .filter(|path| path.is_file())
                .and_then(|path| ImageData::from_path(path).ok())
        })
        .clone()
}

fn load_entries() -> Vec<DesktopEntry> {
    let mut entries: Vec<_> = fs::read_dir(desktop_directory())
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| desktop_entry(entry.path()))
        .collect();
    entries.sort_by_key(|entry| entry.label.to_ascii_lowercase());
    entries
}

fn desktop_entry(path: PathBuf) -> Option<DesktopEntry> {
    let name = path.file_name()?.to_str()?;
    if name.starts_with('.') {
        return None;
    }
    if path.is_dir() {
        return Some(DesktopEntry {
            label: name.to_owned(),
            icon: None,
            kind: EntryKind::Folder,
            target: DesktopTarget::Path(path),
        });
    }
    if path
        .extension()
        .is_some_and(|extension| extension == "desktop")
    {
        return launcher_entry(&path);
    }
    Some(DesktopEntry {
        label: name.to_owned(),
        icon: None,
        kind: EntryKind::File,
        target: DesktopTarget::Path(path),
    })
}

fn launcher_entry(path: &Path) -> Option<DesktopEntry> {
    let contents = fs::read_to_string(path).ok()?;
    let mut fields = HashMap::new();
    let mut in_desktop_entry = false;
    for line in contents.lines().map(str::trim) {
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
        } else if in_desktop_entry {
            if let Some((key, value)) = line.split_once('=') {
                fields.insert(key, value);
            }
        }
    }
    if fields.get("Type") != Some(&"Application")
        || matches!(fields.get("Hidden"), Some(&"true"))
        || matches!(fields.get("NoDisplay"), Some(&"true"))
    {
        return None;
    }
    let args = parse_exec(fields.get("Exec")?);
    let (program, args) = args.split_first()?;
    let icon = fields
        .get("Icon")
        .and_then(|name| ImageData::from_path(name).ok());
    Some(DesktopEntry {
        label: fields
            .get("Name[en_US]")
            .or_else(|| fields.get("Name[en]"))
            .or_else(|| fields.get("Name"))?
            .to_string(),
        icon,
        kind: EntryKind::Launcher,
        target: DesktopTarget::Launch {
            program: program.clone(),
            args: args.to_vec(),
            terminal: fields.get("Terminal") == Some(&"true"),
        },
    })
}

fn parse_exec(value: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            } else {
                current.push(character);
            }
        } else if character.is_whitespace() && quote.is_none() {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args.into_iter()
        .filter(|argument| !argument.starts_with('%'))
        .collect()
}

fn desktop_directory() -> PathBuf {
    std::env::var_os("XDG_DESKTOP_DIR")
        .map(PathBuf::from)
        .or_else(|| desktop_directory_from_config())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Desktop")))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn desktop_directory_from_config() -> Option<PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    let contents = fs::read_to_string(config.join("user-dirs.dirs")).ok()?;
    let value = contents
        .lines()
        .find_map(|line| line.strip_prefix("XDG_DESKTOP_DIR="))?
        .trim()
        .trim_matches('"');
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    Some(PathBuf::from(
        value.replace("$HOME", &home.to_string_lossy()),
    ))
}

fn open_target(target: DesktopTarget) {
    std::thread::spawn(move || match target {
        DesktopTarget::Path(path) => open_path(path),
        DesktopTarget::Launch {
            program,
            args,
            terminal,
        } => launch(program, args, terminal),
    });
}

#[cfg(target_os = "windows")]
fn open_path(path: PathBuf) {
    let _ = crate::process::spawn_detached(Command::new("explorer.exe").arg(path));
}

#[cfg(not(target_os = "windows"))]
fn open_path(path: PathBuf) {
    let _ = crate::process::spawn_detached(Command::new("xdg-open").arg(path));
}

fn launch(program: String, args: Vec<String>, terminal: bool) {
    let mut command = if terminal {
        let terminal = std::env::var("TERMINAL").unwrap_or_else(|_| "x-terminal-emulator".into());
        let mut command = Command::new(terminal);
        command.arg("-e").arg(program).args(args);
        command
    } else {
        let mut command = Command::new(program);
        command.args(args);
        command
    };
    let _ = crate::process::spawn_detached(&mut command);
}

fn fill_layout() -> LayoutStyle {
    LayoutStyle {
        position: Position::Absolute,
        inset: LayoutRect {
            left: LengthPercentageAuto::Length(0.0),
            right: LengthPercentageAuto::Length(0.0),
            top: LengthPercentageAuto::Length(0.0),
            bottom: LengthPercentageAuto::Length(0.0),
        },
        size: LayoutSize {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
        },
        ..Default::default()
    }
}

fn icon_style(position: Point) -> Style {
    Style::new()
        .layout(LayoutStyle {
            position: Position::Absolute,
            inset: LayoutRect {
                left: LengthPercentageAuto::Length(position.x),
                right: LengthPercentageAuto::Auto,
                top: LengthPercentageAuto::Length(position.y),
                bottom: LengthPercentageAuto::Auto,
            },
            size: fixed(ICON_SIZE, ICON_SIZE),
            ..Default::default()
        })
        .corner_radius(18.0)
        .hover(StateStyle::new().background(ICON_HOVER))
        .pressed(StateStyle::new().background(ICON_PRESSED))
}

fn initial_position(index: usize) -> Point {
    Point {
        x: GRID_INSET + (index % 4) as f32 * GRID_CELL,
        y: GRID_INSET + (index / 4) as f32 * GRID_CELL,
    }
}

fn icon_position(position: Point, viewport: Size) -> Point {
    let position = free_icon_position(position, viewport);
    Point {
        x: (GRID_INSET + ((position.x - GRID_INSET) / GRID_CELL).round() * GRID_CELL)
            .clamp(GRID_INSET, grid_limit(viewport.width)),
        y: (GRID_INSET + ((position.y - GRID_INSET) / GRID_CELL).round() * GRID_CELL)
            .clamp(GRID_INSET, grid_limit(viewport.height - DOCK_HEIGHT as f32)),
    }
}

fn grid_limit(length: f32) -> f32 {
    GRID_INSET + ((length - ICON_SIZE - GRID_INSET * 2.0).max(0.0) / GRID_CELL).floor() * GRID_CELL
}

fn entry_color(kind: EntryKind) -> Color {
    match kind {
        EntryKind::Folder => Color::rgb(59, 126, 193),
        EntryKind::File => Color::rgb(108, 100, 166),
        EntryKind::Launcher => Color::rgb(55, 143, 156),
    }
}

fn short_name(name: &str) -> String {
    const MAX: usize = 16;
    let mut result: String = name.chars().take(MAX).collect();
    if name.chars().count() > MAX {
        result.push('…');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_drop_snaps_to_the_nearest_slot() {
        let viewport = Size {
            width: 1920.0,
            height: 1080.0,
        };
        assert_eq!(
            icon_position(
                Point {
                    x: GRID_INSET + GRID_CELL * 1.6,
                    y: GRID_INSET + GRID_CELL * 2.4,
                },
                viewport,
            ),
            Point {
                x: GRID_INSET + GRID_CELL * 2.0,
                y: GRID_INSET + GRID_CELL * 2.0,
            }
        );
    }

    #[test]
    fn icon_drop_keeps_the_last_slot_within_bounds() {
        let viewport = Size {
            width: 1920.0,
            height: 1080.0,
        };
        let position = icon_position(
            Point {
                x: viewport.width,
                y: viewport.height,
            },
            viewport,
        );
        assert_eq!(position.x, grid_limit(viewport.width));
        assert_eq!(position.y, grid_limit(viewport.height - DOCK_HEIGHT as f32));
    }
}
