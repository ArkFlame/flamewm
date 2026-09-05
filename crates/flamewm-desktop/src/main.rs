use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, SystemTime};

use flamewm_api::{OutputId, Point, Rect};
use flamewm_desktop_core::layout::GridConfig;
use flamewm_desktop_core::model::{DesktopDirResolver, DesktopModel};
use flamewm_desktop_core::persistence::LayoutStore;
use flamewm_desktop_core::selection::SelectionModel;
use flamewm_desktop_core::sticky::{StickyNote, StickyNoteStore};
use flamewm_desktop_core::sticky_persistence;
use flamewm_render_core::{RuntimeDocument, decode};
use flamewm_render_x11::{ActionEvent, ActionPhase, X11Config, X11WindowRole, run_with_controller};

const SLOT_COUNT: usize = 128;

struct DesktopState {
    directory: PathBuf,
    model: DesktopModel,
    grid: GridConfig,
    sticky: StickyNoteStore,
    sticky_path: PathBuf,
    selection: SelectionModel,
    drag_start: Option<Point>,
    watch: Receiver<SystemTime>,
    stamp: SystemTime,
}

fn main() -> Result<(), String> {
    let home = PathBuf::from(env::var_os("HOME").ok_or("HOME is not set")?);
    let user_dirs = home.join(".config/user-dirs.dirs");
    let user_dirs = fs::read_to_string(user_dirs).ok();
    let directory = DesktopDirResolver::resolve(&home, user_dirs.as_deref());
    fs::create_dir_all(&directory).map_err(|error| format!("create desktop directory: {error}"))?;

    let work_area = Rect::new(0, 0, 1920, 1080);
    let grid = GridConfig::compute(work_area, 100);
    let mut model = DesktopModel::default();
    let output = OutputId::new("default");
    model
        .rescan(&directory, &output, grid)
        .map_err(|error| error.to_string())?;
    let layout_path = state_path("desktop-layout.state")?;
    if let Ok(text) = fs::read_to_string(&layout_path) {
        for (path, position) in LayoutStore::parse(&text).map_err(|error| error.to_string())? {
            model.set_position(path, position);
        }
        model
            .rescan(&directory, &output, grid)
            .map_err(|error| error.to_string())?;
    }
    let sticky_path = sticky_persistence::state_path().map_err(|error| error.to_string())?;
    let sticky = sticky_persistence::load(&sticky_path).map_err(|error| error.to_string())?;
    let stamp = directory_stamp(&directory);
    let watch = directory_watcher(directory.clone());
    let mut state = DesktopState {
        directory,
        model,
        grid,
        sticky,
        sticky_path,
        selection: SelectionModel::default(),
        drag_start: None,
        watch,
        stamp,
    };

    let compiled = decode(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/flamewm-desktop.rwr"
    )))?;
    let mut document = RuntimeDocument::new(compiled)?;
    sync_document(&mut document, &state)?;
    run_with_controller(
        document,
        X11Config {
            width: 1920,
            height: 1080,
            title: "FlameWM Desktop".to_owned(),
            role: X11WindowRole::Desktop,
        },
        move |event, document| {
            refresh_if_changed(document, &mut state)?;
            handle_event(event, document, &mut state)
        },
    )
}

fn state_path(name: &str) -> Result<PathBuf, String> {
    let base = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap()).join(".local/state"));
    Ok(base.join(name))
}

fn directory_stamp(path: &Path) -> SystemTime {
    fs::read_dir(path)
        .ok()
        .and_then(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| entry.metadata().ok())
                .filter_map(|meta| meta.modified().ok())
                .max()
        })
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

fn directory_watcher(path: PathBuf) -> Receiver<SystemTime> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut previous = directory_stamp(&path);
        loop {
            thread::sleep(Duration::from_millis(350));
            let current = directory_stamp(&path);
            if current != previous {
                previous = current;
                if sender.send(current).is_err() {
                    break;
                }
            }
        }
    });
    receiver
}

fn refresh_if_changed(
    document: &mut RuntimeDocument,
    state: &mut DesktopState,
) -> Result<(), String> {
    let Ok(stamp) = state.watch.try_recv() else {
        return Ok(());
    };
    if stamp == state.stamp {
        return Ok(());
    }
    state.stamp = stamp;
    state
        .model
        .rescan(&state.directory, &OutputId::new("default"), state.grid)
        .map_err(|error| error.to_string())?;
    sync_document(document, state)
}

fn sync_document(document: &mut RuntimeDocument, state: &DesktopState) -> Result<(), String> {
    for index in 0..SLOT_COUNT {
        let item = state.model.items().nth(index);
        let id = format!("desktop-item-{index}");
        let label = format!("desktop-label-{index}");
        document.set_visible(&id, item.is_some())?;
        if let Some(item) = item {
            document.set_text(&label, &item.display_name)?;
            let point = state.grid.cell_to_pixel(item.cell);
            document.set_position_px(&id, point.x as f32, point.y as f32)?;
        }
    }
    document.set_visible(
        "sticky-note",
        state.sticky.enabled() && !state.sticky.notes().is_empty(),
    )?;
    if let Some(note) = state.sticky.notes().first() {
        document.set_text("sticky-text", &note.text)?;
    }
    Ok(())
}

fn handle_event(
    action: &ActionEvent,
    document: &mut RuntimeDocument,
    state: &mut DesktopState,
) -> Result<(), String> {
    if action.action == "desktop.surface" {
        return handle_surface(action, document, state);
    }
    if action.action == "sticky.move"
        && action.phase == ActionPhase::Release
        && action.button == 1
        && state.sticky.enabled()
        && state.sticky.notes().is_empty()
    {
        let note = StickyNote::new(
            "sticky-1",
            OutputId::new("default"),
            0,
            action.x as i32,
            action.y as i32,
        );
        state.sticky.create(note);
        sticky_persistence::persist(&state.sticky_path, &state.sticky)
            .map_err(|error| error.to_string())?;
        sync_document(document, state)?;
        return Ok(());
    }
    if let Some(raw) = action.action.strip_prefix("desktop.item.") {
        if action.phase == ActionPhase::Release && action.inside {
            if let Ok(index) = raw.parse::<usize>() {
                if let Some(item) = state.model.items().nth(index) {
                    Command::new("xdg-open")
                        .arg(&item.path)
                        .spawn()
                        .map_err(|error| error.to_string())?;
                }
            }
        }
    }
    if action.action == "sticky.move" && action.phase == ActionPhase::Release && action.button == 3
    {
        let note_id = state.sticky.notes().first().map(|note| note.id.clone());
        if let Some(note_id) = note_id {
            state.sticky.remove(&note_id);
            sticky_persistence::persist(&state.sticky_path, &state.sticky)
                .map_err(|error| error.to_string())?;
            sync_document(document, state)?;
        }
    }
    Ok(())
}

fn handle_surface(
    action: &ActionEvent,
    document: &mut RuntimeDocument,
    state: &mut DesktopState,
) -> Result<(), String> {
    if action.button == 3
        && action.phase == ActionPhase::Release
        && state.sticky.enabled()
        && state.sticky.notes().is_empty()
    {
        state.sticky.create(StickyNote::new(
            "sticky-1",
            OutputId::new("default"),
            0,
            action.x as i32,
            action.y as i32,
        ));
        sticky_persistence::persist(&state.sticky_path, &state.sticky)
            .map_err(|error| error.to_string())?;
        return sync_document(document, state);
    }
    if action.button != 1 {
        return Ok(());
    }
    match action.phase {
        ActionPhase::Press => {
            state.drag_start = Some(Point::new(action.x as i32, action.y as i32));
        }
        ActionPhase::Motion => {
            if let Some(start) = state.drag_start {
                let rect = Rect::new(
                    start.x.min(action.x as i32),
                    start.y.min(action.y as i32),
                    (start.x - action.x as i32).unsigned_abs() as i32,
                    (start.y - action.y as i32).unsigned_abs() as i32,
                );
                state.selection.set_rubber(rect);
                document.set_visible("desktop-selection", true)?;
                document.set_position_px("desktop-selection", rect.x as f32, rect.y as f32)?;
                document.set_size_px("desktop-selection", rect.width as f32, rect.height as f32)?;
            }
        }
        ActionPhase::Release => {
            if let Some(rect) = state.selection.rubber() {
                let item_rects = state
                    .model
                    .items()
                    .map(|item| {
                        let point = state.grid.cell_to_pixel(item.cell);
                        (
                            item.id.clone(),
                            Rect::new(
                                point.x,
                                point.y,
                                state.grid.cell_width,
                                state.grid.cell_height,
                            ),
                        )
                    })
                    .collect::<BTreeMap<_, _>>();
                state.selection.set_rubber(rect);
                let selected = state.selection.hit_test(&item_rects);
                state.selection.clear();
                for id in selected {
                    state.selection.select(id);
                }
            }
            state.drag_start = None;
            state.selection.clear_rubber();
            document.set_visible("desktop-selection", false)?;
        }
        ActionPhase::Hover => {}
    }
    Ok(())
}
