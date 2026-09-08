use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use flamewm_api::applications::ApplicationLaunchOptions;
use flamewm_api::{OutputId, Point, Rect};
use flamewm_applications::DesktopEntry;
use flamewm_control_core::{ControlRequest, ControlResponse};
use flamewm_control_dbus::ControlClient;
use flamewm_dbus_reactor::BusKind;
use flamewm_desktop_core::file_actions::{
    CommandIntent, EntryMenuAction, LauncherOpen, create_new_folder, desktop_settings,
    entry_context_menu, launcher_open_kind, open_terminal, shortcut_destination, terminal_program,
};
use flamewm_desktop_core::layout::{
    Cell, DragItem, GridConfig, drag_cell_delta, group_drag_transaction, threshold_passed,
};
use flamewm_desktop_core::model::{
    DesktopDirResolver, DesktopItemKind, DesktopModel, StoredPosition,
};
use flamewm_desktop_core::persistence::LayoutStore;
use flamewm_desktop_core::presentation::{
    BlankDesktopAction, CONTEXT_MENU_WIDTH, blank_context_menu, clamp_menu_anchor,
    context_menu_rect, menu_height_for_rows,
};
use flamewm_desktop_core::selection::{SelectionModel, double_click_opens, rubber_visible};
use flamewm_desktop_core::sticky::{StickyNote, StickyNoteStore};
use flamewm_desktop_core::sticky_persistence;
use flamewm_desktop_core::trash::{Trash, TrashLocation, TrashScope};
use flamewm_integrations_linux::icons::IconResolver;
use flamewm_ui_x11::PointerButton;
use flamewm_ui_x11::{
    UiActionEvent, UiActionPhase, UiColor, UiControllerEvent, UiDocumentAccess, UiWindowConfig,
    UiWindowRole, decode_document, pointer_button_from_raw, root_geometry,
    run_with_controller_events_role,
};

mod projection;

const SLOT_COUNT: usize = 128;
const BLANK_ROW_IDS: [&str; 5] = [
    "menu-row-terminal",
    "menu-row-folder",
    "menu-row-note",
    "menu-row-desktop",
    "menu-row-settings",
];
const ENTRY_ROW_IDS: [&str; 3] = [
    "menu-row-empty-trash",
    "menu-row-shortcut",
    "menu-row-delete",
];

struct LauncherMeta {
    icon: String,
    valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuKind {
    Blank,
    Entry,
    Sticky,
}

struct MenuState {
    kind: MenuKind,
    anchor: Point,
    selected_id: Option<String>,
}

struct ClickRecord {
    id: String,
    x: f32,
    y: f32,
    at: Instant,
}

struct DesktopState {
    directory: PathBuf,
    model: DesktopModel,
    grid: GridConfig,
    icon_resolver: IconResolver,
    layout_path: PathBuf,
    sticky: StickyNoteStore,
    sticky_path: PathBuf,
    selection: SelectionModel,
    selection_start: Option<Point>,
    item_drag: Option<ItemDrag>,
    launchers: BTreeMap<PathBuf, LauncherMeta>,
    menu: Option<MenuState>,
    pending_delete: Option<PathBuf>,
    last_click: Option<ClickRecord>,
    watch: Receiver<SystemTime>,
    stamp: SystemTime,
}

#[derive(Clone)]
struct ItemDrag {
    anchor_index: usize,
    anchor_id: String,
    start_x: f32,
    start_y: f32,
    originals: Vec<OriginalItem>,
    moved: bool,
}

#[derive(Clone)]
struct OriginalItem {
    index: usize,
    id: String,
    path: PathBuf,
    output: OutputId,
    cell: Cell,
    pixel: Point,
}

fn main() -> Result<(), String> {
    let home = PathBuf::from(env::var_os("HOME").ok_or("HOME is not set")?);
    let user_dirs = home.join(".config/user-dirs.dirs");
    let user_dirs = fs::read_to_string(user_dirs).ok();
    let directory = DesktopDirResolver::resolve(&home, user_dirs.as_deref());
    fs::create_dir_all(&directory).map_err(|error| format!("create desktop directory: {error}"))?;

    let work_area = resolve_work_area()?;
    let grid = GridConfig::compute(work_area, 100);
    let mut model = DesktopModel::default();
    let output = OutputId::new("default");
    model
        .rescan(&directory, &output, grid)
        .map_err(|error| format!("{error:?}"))?;
    let layout_path = state_path("desktop-layout.state")?;
    if let Ok(text) = fs::read_to_string(&layout_path) {
        for (path, position) in LayoutStore::parse(&text).map_err(|error| format!("{error:?}"))? {
            model.set_position(path, position);
        }
        model
            .rescan(&directory, &output, grid)
            .map_err(|error| format!("{error:?}"))?;
    }
    let sticky_path = sticky_persistence::state_path().map_err(|error| format!("{error:?}"))?;
    let sticky = sticky_persistence::load(&sticky_path).map_err(|error| format!("{error:?}"))?;
    let stamp = directory_stamp(&directory);
    let watch = directory_watcher(directory.clone());
    let mut state = DesktopState {
        directory,
        model,
        grid,
        icon_resolver: IconResolver::from_environment(packaged_root(), 0),
        layout_path,
        sticky,
        sticky_path,
        selection: SelectionModel::default(),
        selection_start: None,
        item_drag: None,
        launchers: BTreeMap::new(),
        menu: None,
        pending_delete: None,
        last_click: None,
        watch,
        stamp,
    };
    refresh_launchers(&mut state);

    let mut document = decode_document(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/flamewm-desktop.rwr"
    )))?;
    projection::project_background(&mut document, None)?;
    sync_document(&mut document, &mut state)?;
    run_with_controller_events_role(
        document,
        UiWindowConfig {
            width: work_area.width.max(1) as u32,
            height: work_area.height.max(1) as u32,
            x: work_area.x,
            y: work_area.y,
            title: "FlameWM Desktop".to_owned(),
        },
        UiWindowRole::Desktop,
        move |event, document| {
            refresh_if_changed(document, &mut state)?;
            let UiControllerEvent::Action(action) = event;
            handle_event(action, document, &mut state)
        },
    )
}

/// Root geometry comes from the actual X root through the X11 runtime owner.
/// Panel struts are subtracted when Control GetPanels answers; otherwise the
/// full root rect is used with exactly one degraded warning. No constant size.
fn resolve_work_area() -> Result<Rect, String> {
    let (width, height) = root_geometry()?;
    let root = Rect::new(0, 0, width as i32, height as i32);
    match panel_geometries() {
        Ok(panels) => {
            let mut area = root;
            for panel in panels {
                area = subtract_panel_strip(area, panel);
            }
            Ok(area)
        }
        Err(error) => {
            eprintln!("desktop: degraded work area: {error}; using full root geometry");
            Ok(root)
        }
    }
}

fn panel_geometries() -> Result<Vec<Rect>, String> {
    let client = ControlClient::connect(BusKind::Session).map_err(|error| format!("{error:?}"))?;
    match client.call(&ControlRequest::GetPanels) {
        Ok(ControlResponse::Panels(snapshot)) => Ok(snapshot
            .panels
            .into_iter()
            .filter(|panel| panel.visible)
            .map(|panel| panel.geometry)
            .collect()),
        Ok(response) => Err(format!("GetPanels returned {response:?}")),
        Err(error) => Err(error.message.clone()),
    }
}

fn subtract_panel_strip(mut area: Rect, panel: Rect) -> Rect {
    let Some(intersection) = area.intersection(panel) else {
        return area;
    };
    if intersection.width >= area.width - 1 {
        if intersection.y <= area.y + area.height / 2 {
            let bottom = intersection.y + intersection.height;
            let shift = bottom - area.y;
            if shift > 0 && shift < area.height {
                area.y = bottom;
                area.height -= shift;
            }
        } else {
            let height = intersection.y - area.y;
            if height > 0 {
                area.height = height;
            }
        }
        return area;
    }
    if intersection.height >= area.height - 1 {
        if intersection.x <= area.x + area.width / 2 {
            let right = intersection.x + intersection.width;
            let shift = right - area.x;
            if shift > 0 && shift < area.width {
                area.x = right;
                area.width -= shift;
            }
        } else {
            let width = intersection.x - area.x;
            if width > 0 {
                area.width = width;
            }
        }
    }
    area
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
    document: &mut impl UiDocumentAccess,
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
        .map_err(|error| format!("{error:?}"))?;
    refresh_launchers(state);
    sync_document(document, state)
}

/// Launcher metadata cache keyed by path, refreshed after every rescan.
/// The Icon= value wins; per-kind defaults apply otherwise. Parsing stays in
/// the applications owner; this cache only records the outcome.
fn refresh_launchers(state: &mut DesktopState) {
    state.launchers.clear();
    for item in state.model.items() {
        if item.kind != DesktopItemKind::DesktopLauncher {
            continue;
        }
        match DesktopEntry::from_file(&item.path) {
            Ok(Some(entry)) => {
                state.launchers.insert(
                    item.path.clone(),
                    LauncherMeta {
                        icon: entry.icon().to_owned(),
                        valid: true,
                    },
                );
            }
            Ok(None) | Err(_) => {
                state.launchers.insert(
                    item.path.clone(),
                    LauncherMeta {
                        icon: String::new(),
                        valid: false,
                    },
                );
            }
        }
    }
}

fn launcher_icon(state: &DesktopState, path: &Path) -> Option<String> {
    state
        .launchers
        .get(path)
        .filter(|meta| meta.valid && !meta.icon.trim().is_empty())
        .map(|meta| meta.icon.clone())
}

fn sync_document(
    document: &mut impl UiDocumentAccess,
    state: &mut DesktopState,
) -> Result<(), String> {
    for index in 0..SLOT_COUNT {
        let item = state.model.items().nth(index);
        if let Some(item) = item {
            let point = state.grid.cell_to_pixel(item.cell);
            let icon = launcher_icon(state, &item.path);
            projection::project_item(
                document,
                &mut state.icon_resolver,
                index,
                item.kind,
                &item.display_name,
                point,
                icon.as_deref(),
            )?;
        } else {
            projection::clear_item(document, index)?;
        }
    }
    sync_selection_visuals(document, state)?;
    sync_menu(document, state)?;
    document.visible(
        "sticky-note",
        state.sticky.enabled() && !state.sticky.notes().is_empty(),
    )?;
    if let Some(note) = state.sticky.notes().first() {
        document.text("sticky-text", note.text.clone())?;
    }
    Ok(())
}

fn handle_event(
    action: &UiActionEvent,
    document: &mut impl UiDocumentAccess,
    state: &mut DesktopState,
) -> Result<(), String> {
    if action.action == "keyboard.input" && action.text.as_deref() == Some("") {
        return cancel_transient(document, state);
    }
    if action.phase == UiActionPhase::Release
        && action.inside
        && pointer_button(action) == PointerButton::Primary
        && is_menu_action(&action.action)
    {
        return handle_menu_action(&action.action, document, state);
    }
    if matches!(
        action.action.as_str(),
        "desktop.menu.surface" | "desktop.entry-menu.surface" | "desktop.confirm.surface"
    ) {
        return Ok(());
    }
    if action.action == "desktop.surface" {
        return handle_surface(action, document, state);
    }
    if action.action == "sticky.move"
        && action.phase == UiActionPhase::Release
        && pointer_button(action) == PointerButton::Primary
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
            .map_err(|error| format!("{error:?}"))?;
        sync_document(document, state)?;
        return Ok(());
    }
    if let Some(raw) = action.action.strip_prefix("desktop.item.") {
        if let Ok(index) = raw.parse::<usize>() {
            return handle_item(action, document, state, index);
        }
    }
    if action.action == "sticky.menu.surface" {
        return Ok(());
    }
    if action.action == "sticky.move"
        && action.phase == UiActionPhase::Release
        && pointer_button(action) == PointerButton::Secondary
    {
        let anchor = pointer_point(action, state.grid.work_area);
        state.menu = Some(MenuState {
            kind: MenuKind::Sticky,
            anchor,
            selected_id: state.sticky.notes().first().map(|note| note.id.clone()),
        });
        state.pending_delete = None;
        hide_confirm(document)?;
        return sync_menu(document, state);
    }
    Ok(())
}

fn is_menu_action(action: &str) -> bool {
    matches!(
        action,
        "menu.terminal"
            | "menu.folder"
            | "menu.note"
            | "menu.desktop"
            | "menu.settings"
            | "entry.empty-trash"
            | "entry.shortcut"
            | "entry.delete"
            | "confirm.delete"
            | "confirm.cancel"
            | "sticky.menu.delete"
    )
}

fn cancel_transient(
    document: &mut impl UiDocumentAccess,
    state: &mut DesktopState,
) -> Result<(), String> {
    state.selection_start = None;
    state.selection.clear_rubber();
    state.item_drag = None;
    state.last_click = None;
    state.menu = None;
    state.pending_delete = None;
    document.visible("desktop-selection", false)?;
    hide_menus(document)?;
    hide_confirm(document)?;
    sync_selection_visuals(document, state)
}

fn handle_surface(
    action: &UiActionEvent,
    document: &mut impl UiDocumentAccess,
    state: &mut DesktopState,
) -> Result<(), String> {
    let button = pointer_button(action);
    // Right-click never starts a selection drag: the blank desktop menu only
    // opens on Secondary release at the pointer.
    if button == PointerButton::Secondary {
        if action.phase != UiActionPhase::Release {
            return Ok(());
        }
        let anchor = pointer_point(action, state.grid.work_area);
        state.menu = Some(MenuState {
            kind: MenuKind::Blank,
            anchor,
            selected_id: None,
        });
        state.pending_delete = None;
        hide_confirm(document)?;
        state.selection_start = None;
        state.selection.clear_rubber();
        document.visible("desktop-selection", false)?;
        state.selection.clear();
        sync_selection_visuals(document, state)?;
        return sync_menu(document, state);
    }
    if button != PointerButton::Primary {
        return Ok(());
    }
    match action.phase {
        UiActionPhase::Press => {
            hide_menus(document)?;
            hide_confirm(document)?;
            state.pending_delete = None;
            state.last_click = None;
            let start = pointer_point(action, state.grid.work_area);
            state.item_drag = None;
            state.selection_start = Some(start);
            state.selection.clear();
            state.selection.set_rubber(normalized_selection_rect(
                start,
                start,
                state.grid.work_area,
            ));
            document.visible("desktop-selection", false)?;
            sync_selection_visuals(document, state)?;
        }
        UiActionPhase::Motion => {
            if let Some(start) = state.selection_start {
                let point = pointer_point(action, state.grid.work_area);
                let rect = normalized_selection_rect(start, point, state.grid.work_area);
                state.selection.set_rubber(rect);
                update_selection_from_rubber(state);
                if rubber_visible(
                    point.x as f32 - start.x as f32,
                    point.y as f32 - start.y as f32,
                ) {
                    set_selection_overlay(document, rect)?;
                } else {
                    document.visible("desktop-selection", false)?;
                }
                sync_selection_visuals(document, state)?;
            }
        }
        UiActionPhase::Release => {
            if let Some(start) = state.selection_start {
                let point = pointer_point(action, state.grid.work_area);
                let rect = normalized_selection_rect(start, point, state.grid.work_area);
                state.selection.set_rubber(rect);
                update_selection_from_rubber(state);
            }
            state.selection_start = None;
            state.selection.clear_rubber();
            document.visible("desktop-selection", false)?;
            sync_selection_visuals(document, state)?;
        }
        UiActionPhase::Hover => {}
    }
    Ok(())
}

fn handle_item(
    action: &UiActionEvent,
    document: &mut impl UiDocumentAccess,
    state: &mut DesktopState,
    index: usize,
) -> Result<(), String> {
    let button = pointer_button(action);
    // Entry Secondary only opens the entry menu on release after selecting
    // the target; it never enters the open/double-click or drag paths.
    if button == PointerButton::Secondary {
        if action.phase != UiActionPhase::Release {
            return Ok(());
        }
        let Some(item) = state.model.items().nth(index) else {
            return Ok(());
        };
        let item_id = item.id.clone();
        state.selection.select_exclusive(item_id.clone());
        state.item_drag = None;
        state.selection_start = None;
        state.selection.clear_rubber();
        document.visible("desktop-selection", false)?;
        state.last_click = None;
        state.menu = Some(MenuState {
            kind: MenuKind::Entry,
            anchor: pointer_point(action, state.grid.work_area),
            selected_id: Some(item_id),
        });
        state.pending_delete = None;
        hide_confirm(document)?;
        sync_selection_visuals(document, state)?;
        return sync_menu(document, state);
    }
    if button != PointerButton::Primary {
        return Ok(());
    }
    match action.phase {
        UiActionPhase::Press => {
            hide_menus(document)?;
            hide_confirm(document)?;
            state.pending_delete = None;
            let Some(item) = state.model.items().nth(index) else {
                return Ok(());
            };
            let item_id = item.id.clone();
            if !state.selection.selected().contains(&item_id) {
                state.selection.clear();
                state.selection.select(item_id.clone());
            }
            if double_click_pressed(state, &item_id, action.x, action.y) {
                state.last_click = None;
                state.item_drag = None;
                sync_selection_visuals(document, state)?;
                return open_item(state, index);
            }
            state.last_click = Some(ClickRecord {
                id: item_id.clone(),
                x: action.x,
                y: action.y,
                at: Instant::now(),
            });
            let originals = state
                .model
                .items()
                .enumerate()
                .filter(|(_, candidate)| state.selection.selected().contains(&candidate.id))
                .map(|(candidate_index, candidate)| OriginalItem {
                    index: candidate_index,
                    id: candidate.id.clone(),
                    path: candidate.path.clone(),
                    output: candidate.output.clone(),
                    cell: candidate.cell,
                    pixel: state.grid.cell_to_pixel(candidate.cell),
                })
                .collect();
            state.item_drag = Some(ItemDrag {
                anchor_index: index,
                anchor_id: item_id,
                start_x: action.x,
                start_y: action.y,
                originals,
                moved: false,
            });
            state.selection_start = None;
            sync_selection_visuals(document, state)?;
        }
        UiActionPhase::Motion => {
            let Some(item) = state.model.items().nth(index) else {
                return Ok(());
            };
            let item_id = item.id.clone();
            let Some(drag) = state.item_drag.as_mut() else {
                return Ok(());
            };
            if drag.anchor_index != index || drag.anchor_id != item_id {
                return Ok(());
            }
            if !drag.moved && !threshold_passed(action.x - drag.start_x, action.y - drag.start_y) {
                return Ok(());
            }
            drag.moved = true;
            state.last_click = None;
            let snapshot = drag.clone();
            apply_drag_visual(document, state, &snapshot, action.x, action.y)?;
        }
        UiActionPhase::Release => {
            let Some(drag) = state.item_drag.take() else {
                return Ok(());
            };
            if drag.anchor_index != index {
                state.item_drag = Some(drag);
                return Ok(());
            }
            let moved =
                drag.moved || threshold_passed(action.x - drag.start_x, action.y - drag.start_y);
            if moved {
                if !drag.moved {
                    let snapshot = drag.clone();
                    apply_drag_visual(document, state, &snapshot, action.x, action.y)?;
                }
                commit_item_drag(document, state, &drag, action.x, action.y)?;
            }
        }
        UiActionPhase::Hover => {}
    }
    Ok(())
}

/// Single click selects only. A second press on the same item within
/// 500ms and 4px opens; any drag cancels the chain. No single-click timer.
fn double_click_pressed(state: &DesktopState, id: &str, x: f32, y: f32) -> bool {
    let Some(previous) = state.last_click.as_ref() else {
        return false;
    };
    let delta_ms = previous.at.elapsed().as_millis().min(u64::MAX as u128) as u64;
    double_click_opens(&previous.id, previous.x, previous.y, 0, id, x, y, delta_ms)
}

fn open_item(state: &DesktopState, index: usize) -> Result<(), String> {
    let Some(item) = state.model.items().nth(index) else {
        return Ok(());
    };
    match launcher_open_kind(item.kind == DesktopItemKind::DesktopLauncher) {
        LauncherOpen::DesktopEntry => {
            let launchable = state
                .launchers
                .get(&item.path)
                .is_some_and(|meta| meta.valid);
            if !launchable {
                return Err(format!(
                    "desktop entry is not launchable: {}",
                    item.path.display()
                ));
            }
            let entry = DesktopEntry::from_file(&item.path)
                .map_err(|error| format!("{error:?}"))?
                .ok_or_else(|| {
                    format!("desktop entry is not launchable: {}", item.path.display())
                })?;
            entry
                .launch(&ApplicationLaunchOptions::default())
                .map(|_| ())
                .map_err(|error| format!("{error:?}"))?;
            Ok(())
        }
        LauncherOpen::RegularFile => {
            Command::new("xdg-open")
                .arg(&item.path)
                .spawn()
                .map_err(|error| error.to_string())?;
            Ok(())
        }
    }
}

fn spawn_intent(intent: &CommandIntent) -> Result<(), String> {
    let mut command = Command::new(&intent.program);
    command.args(&intent.argv);
    if let Some(cwd) = &intent.cwd {
        command.current_dir(cwd);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("spawn {}: {error}", intent.program))?;
    Ok(())
}

fn handle_menu_action(
    action: &str,
    document: &mut impl UiDocumentAccess,
    state: &mut DesktopState,
) -> Result<(), String> {
    match action {
        "menu.terminal" => {
            let intent = open_terminal(&terminal_program(), &state.directory);
            spawn_intent(&intent)?;
            state.menu = None;
            sync_menu(document, state)?;
        }
        "menu.folder" => {
            let path = create_new_folder(&state.directory).map_err(|error| format!("{error:?}"))?;
            state
                .model
                .rescan(&state.directory, &OutputId::new("default"), state.grid)
                .map_err(|error| format!("{error:?}"))?;
            refresh_launchers(state);
            let id = path.to_string_lossy().into_owned();
            state.selection.clear();
            state.selection.select(id);
            state.menu = None;
            sync_document(document, state)?;
        }
        "menu.note" => {
            let anchor = state.menu.as_ref().map(|menu| menu.anchor);
            let anchor = anchor.unwrap_or(state.grid.work_area.origin());
            if state.sticky.enabled() {
                let id = format!("sticky-{}", state.sticky.notes().len() + 1);
                state.sticky.create(StickyNote::new(
                    id,
                    OutputId::new("default"),
                    0,
                    anchor.x,
                    anchor.y,
                ));
                sticky_persistence::persist(&state.sticky_path, &state.sticky)
                    .map_err(|error| format!("{error:?}"))?;
            }
            state.menu = None;
            sync_document(document, state)?;
        }
        "menu.desktop" => {
            insert_workspace_after()?;
            state.menu = None;
            sync_menu(document, state)?;
        }
        "menu.settings" => {
            spawn_intent(&desktop_settings())?;
            state.menu = None;
            sync_menu(document, state)?;
        }
        "entry.shortcut" => {
            create_shortcut(state)?;
            state.menu = None;
            sync_document(document, state)?;
        }
        "entry.delete" => {
            let selected = state
                .menu
                .as_ref()
                .and_then(|menu| menu.selected_id.clone());
            let Some(id) = selected else {
                state.menu = None;
                return sync_menu(document, state);
            };
            let path = state
                .model
                .items()
                .find(|item| item.id == id)
                .map(|item| item.path.clone());
            let Some(path) = path else {
                state.menu = None;
                return sync_menu(document, state);
            };
            state.menu = None;
            hide_menus(document)?;
            state.pending_delete = Some(path);
            show_confirm(document, state)?;
        }
        "confirm.delete" => {
            if let Some(path) = state.pending_delete.take() {
                delete_permanent(&path)?;
                state
                    .model
                    .rescan(&state.directory, &OutputId::new("default"), state.grid)
                    .map_err(|error| format!("{error:?}"))?;
                refresh_launchers(state);
            }
            hide_confirm(document)?;
            sync_document(document, state)?;
        }
        "confirm.cancel" => {
            state.pending_delete = None;
            hide_confirm(document)?;
        }
        "entry.empty-trash" => {
            empty_home_trash()?;
            state.menu = None;
            sync_menu(document, state)?;
        }
        "sticky.menu.delete" => {
            let note_id = state
                .menu
                .as_ref()
                .and_then(|menu| menu.selected_id.clone())
                .or_else(|| state.sticky.notes().first().map(|note| note.id.clone()));
            if let Some(note_id) = note_id {
                state.sticky.remove(&note_id);
                sticky_persistence::persist(&state.sticky_path, &state.sticky)
                    .map_err(|error| format!("{error:?}"))?;
            }
            state.menu = None;
            sync_document(document, state)?;
        }
        _ => {}
    }
    Ok(())
}

fn control_client() -> Result<ControlClient, String> {
    ControlClient::connect(BusKind::Session).map_err(|error| format!("{error:?}"))
}

fn insert_workspace_after() -> Result<(), String> {
    let client = control_client()?;
    let (index, revision) = match client.call(&ControlRequest::GetWorkspaces) {
        Ok(ControlResponse::Workspaces(snapshot)) => {
            (snapshot.count.saturating_sub(1), snapshot.revision)
        }
        Ok(response) => return Err(format!("GetWorkspaces returned {response:?}")),
        Err(error) => return Err(error.message.clone()),
    };
    match client.call(&ControlRequest::InsertWorkspaceAfter {
        index,
        expected_revision: revision,
    }) {
        Ok(ControlResponse::Unit) => Ok(()),
        Ok(response) => Err(format!("InsertWorkspaceAfter returned {response:?}")),
        Err(error) => Err(error.message.clone()),
    }
}

/// Collision-safe shortcut: `.desktop` sources are copied with a ` link`
/// suffix, other sources become symlinks. Pure path math plus direct fs
/// calls; no shell strings.
fn create_shortcut(state: &mut DesktopState) -> Result<(), String> {
    let selected = state
        .menu
        .as_ref()
        .and_then(|menu| menu.selected_id.clone());
    let Some(id) = selected else {
        return Ok(());
    };
    let source = state
        .model
        .items()
        .find(|item| item.id == id)
        .map(|item| item.path.clone());
    let Some(source) = source else {
        return Ok(());
    };
    let file_name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| "shortcut source has no file name".to_owned())?;
    let destination = shortcut_destination(&state.directory, &file_name);
    if source.extension().and_then(|value| value.to_str()) == Some("desktop") {
        fs::copy(&source, &destination)
            .map(|_| ())
            .map_err(|error| format!("copy {}: {error}", destination.display()))?;
    } else {
        std::os::unix::fs::symlink(&source, &destination)
            .map_err(|error| format!("symlink {}: {error}", destination.display()))?;
    }
    state
        .model
        .rescan(&state.directory, &OutputId::new("default"), state.grid)
        .map_err(|error| format!("{error:?}"))?;
    refresh_launchers(state);
    state.selection.clear();
    state
        .selection
        .select(destination.to_string_lossy().into_owned());
    Ok(())
}

/// Permanent delete only runs after the confirmation step armed it.
fn delete_permanent(path: &Path) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("stat {}: {error}", path.display()))?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path).map_err(|error| format!("delete {}: {error}", path.display()))?;
    } else {
        fs::remove_file(path).map_err(|error| format!("delete {}: {error}", path.display()))?;
    }
    Ok(())
}

fn empty_home_trash() -> Result<(), String> {
    let home = PathBuf::from(env::var_os("HOME").ok_or("HOME is not set")?);
    let data_home = env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    let location = TrashLocation {
        scope: TrashScope::Home,
        top_directory: None,
        base: data_home.join("Trash"),
    };
    Trash::empty(&location)
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))?;
    Ok(())
}

fn blank_row_id(action: BlankDesktopAction) -> &'static str {
    match action {
        BlankDesktopAction::OpenTerminal => "menu-row-terminal",
        BlankDesktopAction::CreateNewFolder => "menu-row-folder",
        BlankDesktopAction::NewStickyNote => "menu-row-note",
        BlankDesktopAction::AddVirtualDesktop => "menu-row-desktop",
        BlankDesktopAction::DesktopAndWallpaper => "menu-row-settings",
    }
}

fn entry_row_id(action: EntryMenuAction) -> &'static str {
    match action {
        EntryMenuAction::EmptyTrash => "menu-row-empty-trash",
        EntryMenuAction::CreateShortcut => "menu-row-shortcut",
        EntryMenuAction::Delete => "menu-row-delete",
    }
}

fn hide_menus(document: &mut impl UiDocumentAccess) -> Result<(), String> {
    document.visible("desktop-menu", false)?;
    document.visible("desktop-entry-menu", false)?;
    document.visible("sticky-menu", false)?;
    Ok(())
}

fn hide_confirm(document: &mut impl UiDocumentAccess) -> Result<(), String> {
    document.visible("desktop-confirm", false)
}

fn show_confirm(document: &mut impl UiDocumentAccess, state: &DesktopState) -> Result<(), String> {
    let size = (CONTEXT_MENU_WIDTH, menu_height_for_rows(2));
    let center = Point::new(
        state.grid.work_area.x + (state.grid.work_area.width - size.0) / 2,
        state.grid.work_area.y + (state.grid.work_area.height - size.1) / 2,
    );
    let placed = clamp_menu_anchor(
        state.grid.work_area,
        Rect::new(center.x, center.y, size.0, size.1),
        size,
    );
    document.position("desktop-confirm", placed.x as f32, placed.y as f32)?;
    document.size("desktop-confirm", placed.width as f32, placed.height as f32)?;
    document.visible("desktop-confirm", true)
}

fn sync_menu(document: &mut impl UiDocumentAccess, state: &DesktopState) -> Result<(), String> {
    hide_menus(document)?;
    let Some(menu) = state.menu.as_ref() else {
        return Ok(());
    };
    match menu.kind {
        MenuKind::Blank => {
            let rows = blank_context_menu(state.sticky.enabled());
            let visible: BTreeSet<&str> = rows.iter().map(|action| blank_row_id(*action)).collect();
            for id in BLANK_ROW_IDS {
                document.visible(id, visible.contains(id))?;
            }
            place_menu(document, state, "desktop-menu", menu.anchor, rows.len())?;
        }
        MenuKind::Entry => {
            let selected_kind = menu
                .selected_id
                .as_ref()
                .and_then(|id| state.model.items().find(|item| &item.id == id))
                .map(|item| item.kind);
            let is_trash = selected_kind == Some(DesktopItemKind::TrashPseudo);
            let rows = entry_context_menu(is_trash);
            let visible: BTreeSet<&str> = rows.iter().map(|action| entry_row_id(*action)).collect();
            for id in ENTRY_ROW_IDS {
                document.visible(id, visible.contains(id))?;
            }
            place_menu(
                document,
                state,
                "desktop-entry-menu",
                menu.anchor,
                rows.len(),
            )?;
        }
        MenuKind::Sticky => {
            place_menu(document, state, "sticky-menu", menu.anchor, 1)?;
        }
    }
    Ok(())
}

fn place_menu(
    document: &mut impl UiDocumentAccess,
    state: &DesktopState,
    id: &str,
    anchor: Point,
    rows: usize,
) -> Result<(), String> {
    // Canonical popover measurement: pointer-anchored rect through
    // PopoverGeometry, clamped to the work area. Position/size land first so
    // the first presented frame already shows the menu above the contents.
    let placed = context_menu_rect(state.grid.work_area, anchor, rows);
    document.position(id, placed.x as f32, placed.y as f32)?;
    document.size(id, placed.width as f32, placed.height as f32)?;
    document.visible(id, true)
}

fn apply_drag_visual(
    document: &mut impl UiDocumentAccess,
    state: &DesktopState,
    drag: &ItemDrag,
    x: f32,
    y: f32,
) -> Result<(), String> {
    let (mut min_x, mut min_y) = (f32::INFINITY, f32::INFINITY);
    let (mut max_x, mut max_y) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for original in &drag.originals {
        min_x = min_x.min(original.pixel.x as f32);
        min_y = min_y.min(original.pixel.y as f32);
        max_x = max_x.max(original.pixel.x as f32 + projection::DESKTOP_TILE_WIDTH);
        max_y = max_y.max(original.pixel.y as f32 + projection::DESKTOP_TILE_HEIGHT);
    }
    let work_area = state.grid.work_area;
    let dx = clamp_delta(
        x - drag.start_x,
        work_area.x as f32 - min_x,
        work_area.right() as f32 - max_x,
    );
    let dy = clamp_delta(
        y - drag.start_y,
        work_area.y as f32 - min_y,
        work_area.bottom() as f32 - max_y,
    );
    for original in &drag.originals {
        let point = original.pixel;
        document.position(
            &format!("desktop-item-{}", original.index),
            point.x as f32 + dx,
            point.y as f32 + dy,
        )?;
    }
    Ok(())
}

fn commit_item_drag(
    document: &mut impl UiDocumentAccess,
    state: &mut DesktopState,
    drag: &ItemDrag,
    x: f32,
    y: f32,
) -> Result<(), String> {
    let selected = drag
        .originals
        .iter()
        .map(|item| DragItem {
            id: item.id.clone(),
            cell: item.cell,
        })
        .collect::<Vec<_>>();
    let selected_ids = drag
        .originals
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
    let occupied = state
        .model
        .items()
        .filter(|item| !selected_ids.contains(item.id.as_str()))
        .map(|item| item.cell)
        .collect::<BTreeSet<_>>();
    let delta = drag_cell_delta(
        x - drag.start_x,
        y - drag.start_y,
        state.grid.cell_width,
        state.grid.cell_height,
    );
    let updates = match group_drag_transaction(&selected, &occupied, delta, state.grid) {
        Ok(updates) => updates,
        Err(_) => {
            sync_document(document, state)?;
            return Ok(());
        }
    };

    for original in &drag.originals {
        if let Some(cell) = updates.get(&original.id).copied() {
            state.model.set_position(
                original.path.clone(),
                StoredPosition {
                    output: original.output.clone(),
                    cell,
                },
            );
        }
    }
    if let Err(error) = state
        .model
        .rescan(&state.directory, &OutputId::new("default"), state.grid)
    {
        restore_original_positions(state, drag)?;
        sync_document(document, state)?;
        return Err(format!("{error:?}"));
    }
    refresh_launchers(state);
    if let Err(error) = LayoutStore::persist(&state.layout_path, state.model.positions()) {
        let restore_error = restore_original_positions(state, drag).err();
        sync_document(document, state)?;
        return Err(match restore_error {
            Some(restore_error) => format!("{error:?}; restore failed: {restore_error}"),
            None => format!("{error:?}"),
        });
    }
    sync_document(document, state)
}

fn restore_original_positions(state: &mut DesktopState, drag: &ItemDrag) -> Result<(), String> {
    for original in &drag.originals {
        state.model.set_position(
            original.path.clone(),
            StoredPosition {
                output: original.output.clone(),
                cell: original.cell,
            },
        );
    }
    state
        .model
        .rescan(&state.directory, &OutputId::new("default"), state.grid)
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

fn update_selection_from_rubber(state: &mut DesktopState) {
    let Some(_) = state.selection.rubber() else {
        return;
    };
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
    let selected = state.selection.hit_test(&item_rects);
    state.selection.clear();
    for id in selected {
        state.selection.select(id);
    }
}

fn sync_selection_visuals(
    document: &mut impl UiDocumentAccess,
    state: &DesktopState,
) -> Result<(), String> {
    for (index, item) in state.model.items().enumerate() {
        let id = format!("desktop-item-{index}");
        if state.selection.selected().contains(&item.id) {
            document.background(
                &id,
                UiColor {
                    r: 239,
                    g: 64,
                    b: 72,
                    a: 52,
                },
            )?;
            document.border(
                &id,
                UiColor {
                    r: 239,
                    g: 64,
                    b: 72,
                    a: 221,
                },
            )?;
        } else {
            document.background_clear(&id)?;
            document.border_clear(&id)?;
        }
    }
    Ok(())
}

fn set_selection_overlay(document: &mut impl UiDocumentAccess, rect: Rect) -> Result<(), String> {
    document.visible("desktop-selection", true)?;
    document.position("desktop-selection", rect.x as f32, rect.y as f32)?;
    document.size("desktop-selection", rect.width as f32, rect.height as f32)
}

/// Raw X button translated once at the event boundary (ui-x11 helper).
fn pointer_button(action: &UiActionEvent) -> PointerButton {
    pointer_button_from_raw(action.button)
}

fn pointer_point(action: &UiActionEvent, bounds: Rect) -> Point {
    let max_x = bounds.right().saturating_sub(1);
    let max_y = bounds.bottom().saturating_sub(1);
    Point::new(
        (action.x.round() as i32).clamp(bounds.x, max_x),
        (action.y.round() as i32).clamp(bounds.y, max_y),
    )
}

fn normalized_selection_rect(start: Point, end: Point, bounds: Rect) -> Rect {
    let start_x = start.x.clamp(bounds.x, bounds.right().saturating_sub(1));
    let start_y = start.y.clamp(bounds.y, bounds.bottom().saturating_sub(1));
    let end_x = end.x.clamp(bounds.x, bounds.right().saturating_sub(1));
    let end_y = end.y.clamp(bounds.y, bounds.bottom().saturating_sub(1));
    Rect::new(
        start_x.min(end_x),
        start_y.min(end_y),
        (start_x - end_x).unsigned_abs().max(1) as i32,
        (start_y - end_y).unsigned_abs().max(1) as i32,
    )
}

fn clamp_delta(value: f32, minimum: f32, maximum: f32) -> f32 {
    if minimum > maximum {
        0.0
    } else {
        value.clamp(minimum, maximum)
    }
}

fn packaged_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("flamewm workspace root")
}
