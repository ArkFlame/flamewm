use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;

use rustwebrender_core::{decode, Color, FlexDirection, RuntimeDocument};
use rustwebrender_x11::{run_with_controller, ActionEvent, ActionPhase, X11Config};

const VIEW_W: f32 = 1350.0;
const VIEW_H: f32 = 641.0;
const ICON_W: f32 = 82.0;
const ICON_H: f32 = 82.0;
const GRID_X: f32 = 92.0;
const GRID_Y: f32 = 91.0;
const GRID_ORIGIN: f32 = 12.0;

const SETTINGS_PAGES: [(&str, f32); 7] = [
    ("appearance", 64.0),
    ("desktop", 100.0),
    ("taskbar", 136.0),
    ("displays", 172.0),
    ("fonts", 208.0),
    ("hotkeys", 244.0),
    ("about", 280.0),
];

const START_GROUPS: [&str; 8] = [
    "development", "games", "graphics", "internet", "multimedia", "system", "utilities", "power",
];

const START_SEARCH_ITEMS: [(&str, &str, &str, &str); 10] = [
    ("start-search-firefox", "Firefox", "browser web internet", "start.web"),
    ("start-search-dolphin", "Dolphin", "files file manager home", "start.files"),
    ("start-search-konsole", "Konsole", "terminal shell console", "start.terminal"),
    ("start-search-code", "Visual Studio Code", "code editor development vscode", "start.code"),
    ("start-search-settings", "System Settings", "settings configuration system", "start.settings"),
    ("start-search-elisa", "Elisa", "music media player", "start.music"),
    ("start-search-kate", "Kate", "text editor development", "start.kate"),
    ("start-search-kwrite", "KWrite", "text editor", "start.kwrite"),
    ("start-search-ark", "Ark", "archive zip utility", "start.ark"),
    ("start-search-okular", "Okular", "document pdf viewer", "start.okular"),
];

const ALL_WINDOWS: [WindowKind; 7] = [
    WindowKind::Browser,
    WindowKind::Files,
    WindowKind::Terminal,
    WindowKind::Code,
    WindowKind::Settings,
    WindowKind::Music,
    WindowKind::Generic,
];

#[derive(Clone, Copy, Debug, Default)]
struct BoxState {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl BoxState {
    fn intersects(self, other: Self) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }
}

#[derive(Clone, Copy, Debug)]
struct WindowState {
    rect: BoxState,
    restore: Option<BoxState>,
    maximized: bool,
    snapped: bool,
    snap_target: Option<SnapTarget>,
    open: bool,
    minimized: bool,
    desktop: u8,
    z: i32,
}

#[derive(Clone, Copy, Debug)]
struct IconState {
    rect: BoxState,
    trashed: bool,
}

#[derive(Clone, Debug)]
enum DragState {
    Window {
        kind: WindowKind,
        offset_x: f32,
        offset_y: f32,
        floating_before_drag: BoxState,
    },
    Icons {
        anchor: IconKind,
        start_x: f32,
        start_y: f32,
        originals: Vec<(IconKind, BoxState)>,
    },
    Selection {
        start_x: f32,
        start_y: f32,
    },
    Sticky {
        start_x: f32,
        start_y: f32,
        original: BoxState,
    },
    TaskbarDock {
        start_x: f32,
        start_y: f32,
        candidate: Option<TaskbarPosition>,
        moved: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum WindowKind {
    Browser,
    Files,
    Terminal,
    Code,
    Settings,
    Music,
    Generic,
}

impl WindowKind {
    fn id(self) -> &'static str {
        match self {
            Self::Browser => "browser-window",
            Self::Files => "files-window",
            Self::Terminal => "terminal-window",
            Self::Code => "code-window",
            Self::Settings => "settings-window",
            Self::Music => "music-window",
            Self::Generic => "generic-window",
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::Browser => "browser",
            Self::Files => "files",
            Self::Terminal => "terminal",
            Self::Code => "code",
            Self::Settings => "settings",
            Self::Music => "music",
            Self::Generic => "generic",
        }
    }

    fn task_line(self) -> Option<&'static str> {
        match self {
            Self::Browser => Some("task-browser-line"),
            Self::Files => Some("task-files-line"),
            Self::Terminal => Some("task-terminal-line"),
            Self::Code => Some("task-code-line"),
            Self::Settings => Some("task-settings-line"),
            Self::Music => Some("task-music-line"),
            Self::Generic => Some("task-generic-line"),
        }
    }

    fn task_id(self) -> Option<&'static str> {
        match self {
            Self::Browser => Some("task-browser"),
            Self::Files => Some("task-files"),
            Self::Terminal => Some("task-terminal"),
            Self::Code => Some("task-code"),
            Self::Settings => Some("task-settings"),
            Self::Music => Some("task-music"),
            Self::Generic => Some("task-generic"),
        }
    }

    fn body_id(self) -> &'static str {
        match self {
            Self::Browser => "browser-body",
            Self::Files => "files-body",
            Self::Terminal => "terminal-body",
            Self::Code => "code-body",
            Self::Settings => "settings-body",
            Self::Music => "music-body",
            Self::Generic => "generic-body",
        }
    }

    fn menu_icon_id(self) -> Option<&'static str> {
        match self {
            Self::Browser => Some("task-menu-icon-browser"),
            Self::Files => Some("task-menu-icon-files"),
            Self::Terminal => Some("task-menu-icon-terminal"),
            Self::Code => Some("task-menu-icon-code"),
            Self::Settings => Some("task-menu-icon-settings"),
            Self::Music => Some("task-menu-icon-music"),
            Self::Generic => Some("task-menu-icon-generic"),
        }
    }

    fn default_rect(self) -> BoxState {
        match self {
            Self::Browser => BoxState { x: 182.0, y: 84.0, width: 790.0, height: 470.0 },
            Self::Files => BoxState { x: 145.0, y: 92.0, width: 760.0, height: 450.0 },
            Self::Terminal => BoxState { x: 235.0, y: 150.0, width: 650.0, height: 390.0 },
            Self::Code => BoxState { x: 205.0, y: 86.0, width: 780.0, height: 460.0 },
            Self::Settings => BoxState { x: 93.0, y: 65.0, width: 720.0, height: 480.0 },
            Self::Music => BoxState { x: 250.0, y: 105.0, width: 690.0, height: 420.0 },
            Self::Generic => BoxState { x: 290.0, y: 120.0, width: 610.0, height: 360.0 },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopupKind {
    Media,
    Volume,
    Network,
    Clock,
}

impl PopupKind {
    fn id(self) -> &'static str {
        match self {
            Self::Media => "media-popup",
            Self::Volume => "audio-popup",
            Self::Network => "wifi-popup",
            Self::Clock => "clock-popup",
        }
    }

    fn size(self) -> (f32, f32) {
        match self {
            Self::Media => (310.0, 170.0),
            Self::Volume => (310.0, 125.0),
            Self::Network => (310.0, 205.0),
            Self::Clock => (286.0, 250.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum IconKind {
    Home,
    Downloads,
    Projects,
    Browser,
    Trash,
    NewFolder,
}

impl IconKind {
    fn id(self) -> &'static str {
        match self {
            Self::Home => "icon-home",
            Self::Downloads => "icon-downloads",
            Self::Projects => "icon-projects",
            Self::Browser => "icon-browser",
            Self::Trash => "icon-trash",
            Self::NewFolder => "icon-new-folder",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum SnapTarget {
    Maximize,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskbarPosition {
    Bottom,
    Top,
    Left,
    Right,
}

impl TaskbarPosition {
    fn next(self) -> Self {
        match self {
            Self::Bottom => Self::Top,
            Self::Top => Self::Left,
            Self::Left => Self::Right,
            Self::Right => Self::Bottom,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Bottom => "Bottom",
            Self::Top => "Top",
            Self::Left => "Left",
            Self::Right => "Right",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayId {
    Edp,
    Hdmi,
}

#[derive(Clone, Copy, Debug)]
struct DisplayState {
    resolution_index: usize,
    scale_index: usize,
}

struct DemoState {
    start_open: bool,
    start_submenu_open: bool,
    start_search: String,
    start_search_focused: bool,
    active_start_group: Option<String>,
    windows: HashMap<WindowKind, WindowState>,
    focused_window: Option<WindowKind>,
    z_counter: i32,
    icons: HashMap<IconKind, IconState>,
    selected_icons: HashSet<IconKind>,
    new_folder_visible: bool,
    drag: Option<DragState>,
    popup: Option<PopupKind>,
    volume: u8,
    muted: bool,
    media_playing: bool,
    accent: Color,
    selection_opacity: u8,
    snap_opacity: u8,
    show_watermark: bool,
    sticky_enabled: bool,
    sticky_visible: bool,
    sticky_rect: BoxState,
    taskbar_position: TaskbarPosition,
    taskbar_height: f32,
    taskbar_opacity: u8,
    taskbar_color_index: usize,
    current_desktop: u8,
    desktops: u8,
    selected_display: DisplayId,
    displays: [DisplayState; 2],
    font_family_index: usize,
    font_bold: bool,
    font_size_offset: i8,
    icon_theme_index: usize,
    hotkey_variant: [usize; 5],
    last_icon_click: Option<(IconKind, u64)>,
    pinned_tasks: HashSet<WindowKind>,
    task_context: Option<WindowKind>,
    entry_context: Option<IconKind>,
    sticky_bg_index: usize,
    sticky_fg_index: usize,
    sticky_font_size: f32,
    sticky_text: String,
    sticky_editing: bool,
}

fn main() {
    if let Err(error) = execute() {
        eprintln!("rwr-flamewm-demo: {error}");
        std::process::exit(1);
    }
}

fn execute() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or("usage: rwr-flamewm-demo file.rwr")?;
    if args.next().is_some() {
        return Err("usage: rwr-flamewm-demo file.rwr".to_string());
    }

    let bytes = fs::read(&path).map_err(|error| format!("failed to read {path}: {error}"))?;
    let compiled = decode(&bytes)?;
    let mut runtime = RuntimeDocument::new(compiled)?;
    let mut state = DemoState::new();
    initialize_demo(&mut runtime, &mut state)?;

    let config = X11Config {
        width: VIEW_W as u32,
        height: VIEW_H as u32,
        title: "RustWebRender 0.0.9 - FlameWM Virtual Prototype".to_string(),
    };

    run_with_controller(runtime, config, move |event, document| {
        handle_action(event, document, &mut state)
    })
}

impl DemoState {
    fn new() -> Self {
        let mut windows = HashMap::new();
        let mut z = 100;
        for kind in ALL_WINDOWS {
            z += 1;
            windows.insert(kind, WindowState {
                rect: kind.default_rect(),
                restore: None,
                maximized: false,
                snapped: false,
                snap_target: None,
                open: kind == WindowKind::Settings,
                minimized: false,
                desktop: 1,
                z,
            });
        }

        let mut icons = HashMap::new();
        icons.insert(IconKind::Home, icon_at(0, 0));
        icons.insert(IconKind::Downloads, icon_at(0, 1));
        icons.insert(IconKind::Projects, icon_at(0, 2));
        icons.insert(IconKind::Browser, icon_at(1, 0));
        icons.insert(IconKind::Trash, icon_at(0, 4));
        icons.insert(IconKind::NewFolder, icon_at(1, 1));

        let pinned_tasks = [WindowKind::Browser, WindowKind::Files, WindowKind::Terminal, WindowKind::Code]
            .into_iter()
            .collect();

        Self {
            start_open: false,
            start_submenu_open: false,
            start_search: String::new(),
            start_search_focused: false,
            active_start_group: None,
            windows,
            focused_window: Some(WindowKind::Settings),
            z_counter: z,
            icons,
            selected_icons: HashSet::new(),
            new_folder_visible: false,
            drag: None,
            popup: None,
            volume: 72,
            muted: false,
            media_playing: true,
            accent: Color::rgb(239, 64, 72),
            selection_opacity: 20,
            snap_opacity: 20,
            show_watermark: true,
            sticky_enabled: true,
            sticky_visible: false,
            sticky_rect: BoxState { x: 220.0, y: 180.0, width: 190.0, height: 190.0 },
            taskbar_position: TaskbarPosition::Bottom,
            taskbar_height: 44.0,
            taskbar_opacity: 99,
            taskbar_color_index: 0,
            current_desktop: 1,
            desktops: 4,
            selected_display: DisplayId::Edp,
            displays: [
                DisplayState { resolution_index: 0, scale_index: 0 },
                DisplayState { resolution_index: 0, scale_index: 0 },
            ],
            font_family_index: 0,
            font_bold: false,
            font_size_offset: 0,
            icon_theme_index: 0,
            hotkey_variant: [0, 0, 0, 0, 0],
            last_icon_click: None,
            pinned_tasks,
            task_context: None,
            entry_context: None,
            sticky_bg_index: 0,
            sticky_fg_index: 0,
            sticky_font_size: 15.0,
            sticky_text: "Type your idea here in the real FlameWM implementation.".to_string(),
            sticky_editing: false,
        }
    }
}

fn icon_at(col: i32, row: i32) -> IconState {
    IconState {
        rect: BoxState {
            x: GRID_ORIGIN + col as f32 * GRID_X,
            y: GRID_ORIGIN + row as f32 * GRID_Y,
            width: ICON_W,
            height: ICON_H,
        },
        trashed: false,
    }
}

fn initialize_demo(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    for id in [
        "start-menu", "start-submenu", "desktop-menu", "desktop-selection", "snap-preview",
        "media-popup", "audio-popup", "wifi-popup", "clock-popup", "tray-pause-icon",
        "tray-volume-muted-icon", "popup-muted-icon", "media-play-icon", "icon-new-folder",
        "sticky-note", "toast", "desktop-entry-menu", "task-menu", "sticky-menu",
        "sticky-menu-settings", "task-menu-pin", "task-menu-unpin", "task-menu-open",
        "task-menu-separator", "task-menu-maximize", "task-menu-minimize", "task-menu-close",
        "entry-menu-empty-trash", "start-search-results", "start-search-empty",
        "taskbar-dock-preview",
    ] {
        document.set_visible(id, false)?;
    }
    for group in START_GROUPS {
        document.set_visible(&format!("start-group-{group}"), false)?;
    }
    for id in [
        "task-menu-icon-browser", "task-menu-icon-files", "task-menu-icon-terminal",
        "task-menu-icon-code", "task-menu-icon-settings", "task-menu-icon-music", "task-menu-icon-generic",
    ] {
        document.set_visible(id, false)?;
    }
    for kind in ALL_WINDOWS {
        let restore = format!("{}-restore-icon", kind.prefix());
        let max = format!("{}-max-icon", kind.prefix());
        if document.node_by_id(&restore).is_some() {
            document.set_visible(&restore, false)?;
        }
        if document.node_by_id(&max).is_some() {
            document.set_visible(&max, true)?;
        }
        apply_window(document, state, kind)?;
    }
    for kind in all_icons() {
        apply_icon_rect(document, state, kind)?;
    }

    document.set_z_index("sticky-note", 80)?;
    document.set_z_index("desktop-selection", 50)?;
    document.set_z_index("snap-preview", 950)?;
    document.set_z_index("taskbar-dock-preview", 1250)?;
    document.set_z_index("start-menu", 1200)?;
    document.set_z_index("start-submenu", 1201)?;
    document.set_z_index("desktop-menu", 1210)?;
    document.set_z_index("desktop-entry-menu", 1215)?;
    document.set_z_index("task-menu", 1320)?;
    document.set_z_index("sticky-menu", 1230)?;
    document.set_z_index("media-popup", 1220)?;
    document.set_z_index("audio-popup", 1220)?;
    document.set_z_index("wifi-popup", 1220)?;
    document.set_z_index("clock-popup", 1220)?;
    document.set_z_index("taskbar", 1300)?;
    document.set_z_index("toast", 1400)?;

    show_settings_page(document, "about")?;
    apply_accent(document, state, state.accent)?;
    apply_font_settings(document, state)?;
    apply_taskbar(document, state)?;
    sync_display_settings(document, state)?;
    sync_settings_controls(document, state)?;
    sync_volume(document, state)?;
    sync_media_icons(document, state)?;
    apply_sticky_style(document, state)?;
    sync_sticky_text(document, state)?;
    sync_start_search(document, state)?;
    sync_window_visibility(document, state)?;
    sync_taskbar_activity(document, state)?;
    sync_workspace_pager(document, state)?;
    Ok(())
}

fn handle_action(event: &ActionEvent, document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    match event.action.as_str() {
        "desktop.surface" => handle_desktop_surface(event, document, state),
        "taskbar.surface" => handle_taskbar_drag(event, document, state),
        "sticky.note.move" | "sticky.note.editor" => handle_sticky_interaction(event, document, state),
        action if action.starts_with("start.category.")
            && event.inside
            && matches!(event.phase, ActionPhase::Hover | ActionPhase::Release) => {
            let category = action.trim_start_matches("start.category.");
            show_start_group(document, state, category)
        }
        action if action.starts_with("window.") && action.ends_with(".move") => {
            if let Some(kind) = window_from_action(action) {
                handle_window_drag(event, document, state, kind)
            } else {
                Ok(())
            }
        }
        action if action.starts_with("desktop.icon.") => handle_icon_drag(event, document, state, action),
        action if event.button == 3 && event.phase == ActionPhase::Release && task_kind_from_action(action).is_some() => {
            open_task_context_menu(document, state, task_kind_from_action(action).expect("guarded task kind"), event.x, event.y)
        }
        _ if event.phase != ActionPhase::Release || !event.inside => Ok(()),

        "start.toggle" => toggle_start(document, state),
        "start.search.focus" => focus_start_search(document, state),
        "keyboard.input" => handle_keyboard_input(event, document, state),
        "keyboard.alt-tab" => cycle_windows(document, state),
        action if action.starts_with("keyboard.start.") => {
            if hotkey_action_enabled(state, 0, action) { toggle_start(document, state) } else { Ok(()) }
        }
        action if action.starts_with("keyboard.desktop.left.") => {
            if hotkey_action_enabled(state, 1, action) { switch_desktop_direction(document, state, -1) } else { Ok(()) }
        }
        action if action.starts_with("keyboard.desktop.right.") => {
            if hotkey_action_enabled(state, 2, action) { switch_desktop_direction(document, state, 1) } else { Ok(()) }
        }
        action if action.starts_with("keyboard.desktop.up.") => {
            if hotkey_action_enabled(state, 3, action) { switch_desktop_grid(document, state, -2) } else { Ok(()) }
        }
        action if action.starts_with("keyboard.desktop.down.") => {
            if hotkey_action_enabled(state, 4, action) { switch_desktop_grid(document, state, 2) } else { Ok(()) }
        }

        action if action.starts_with("workspace.") => {
            let index = action.trim_start_matches("workspace.").parse::<u8>().unwrap_or(1);
            switch_desktop(document, state, index)
        }
        "task.browser" => activate_task(document, state, WindowKind::Browser),
        "task.files" => activate_task(document, state, WindowKind::Files),
        "task.terminal" => activate_task(document, state, WindowKind::Terminal),
        "task.code" => activate_task(document, state, WindowKind::Code),
        "task.settings" => activate_task(document, state, WindowKind::Settings),
        "task.music" => activate_task(document, state, WindowKind::Music),
        "task.generic" => activate_task(document, state, WindowKind::Generic),
        "task.menu.open" => task_menu_open(document, state),
        "task.menu.pin" => task_menu_pin(document, state),
        "task.menu.unpin" => task_menu_unpin(document, state),
        "task.menu.maximize" => task_menu_maximize(document, state),
        "task.menu.minimize" => task_menu_minimize(document, state),
        "task.menu.close" => task_menu_close(document, state),

        "start.web" => open_window(document, state, WindowKind::Browser),
        "start.files" => open_files_window(document, state, "Home", "/home/juan/"),
        "start.terminal" | "menu.terminal" => open_window(document, state, WindowKind::Terminal),
        "start.settings" => open_window(document, state, WindowKind::Settings),
        "start.code" => open_window(document, state, WindowKind::Code),
        "start.music" => open_window(document, state, WindowKind::Music),
        "start.game" => open_generic(document, state, "BlockCraft", "A fake game window inside the FlameWM web-prototype replica."),
        "start.kate" => open_generic(document, state, "Kate", "Text editor activity simulated entirely inside RustWebRender."),
        "start.kwrite" => open_generic(document, state, "KWrite", "Lightweight editor activity simulated inside the virtual desktop."),
        "start.kcalc" => open_generic(document, state, "KCalc", "Calculator activity from the Utilities category."),
        "start.ark" => open_generic(document, state, "Ark", "Archive manager activity from the Utilities category."),
        "start.okular" => open_generic(document, state, "Okular", "Document viewer activity from the Utilities category."),
        "start.spectacle" => open_generic(document, state, "Spectacle", "Screenshot utility activity from the Graphics category."),
        "start.gwenview" => open_generic(document, state, "Gwenview", "Image viewer activity from the Graphics category."),
        "start.kdenlive" => open_generic(document, state, "Kdenlive", "Video editor activity from the Multimedia category."),
        "start.graphics" => open_generic(document, state, "Graphics", "Graphics application showcase."),

        action if action.ends_with(".minimize") => window_control(document, state, action, "minimize"),
        action if action.ends_with(".maximize") => window_control(document, state, action, "maximize"),
        action if action.ends_with(".close") => window_control(document, state, action, "close"),
        action if action.starts_with("window.") && action.ends_with(".focus") => {
            if let Some(kind) = window_from_action(action) {
                focus_window(document, state, kind)
            } else {
                Ok(())
            }
        }

        action if action.starts_with("settings.page.") => {
            let page = action.trim_start_matches("settings.page.");
            show_settings_page(document, page)
        }
        "settings.icon-theme.next" => cycle_icon_theme(document, state),
        "settings.wallpaper.reset" => show_toast(document, "FlameWM default wallpaper restored in this virtual desktop"),
        "settings.watermark.toggle" => {
            state.show_watermark = !state.show_watermark;
            document.set_visible("watermark", state.show_watermark)?;
            sync_settings_controls(document, state)
        }
        "settings.sticky.toggle" => {
            state.sticky_enabled = !state.sticky_enabled;
            if !state.sticky_enabled {
                state.sticky_visible = false;
                document.set_visible("sticky-note", false)?;
            }
            sync_settings_controls(document, state)
        }
        "settings.selection-opacity.down" => adjust_overlay_opacity(document, state, true, -5),
        "settings.selection-opacity.up" => adjust_overlay_opacity(document, state, true, 5),
        "settings.snap-opacity.down" => adjust_overlay_opacity(document, state, false, -5),
        "settings.snap-opacity.up" => adjust_overlay_opacity(document, state, false, 5),
        "settings.taskbar.color" => {
            state.taskbar_color_index = (state.taskbar_color_index + 1) % taskbar_colors().len();
            apply_taskbar(document, state)?;
            sync_settings_controls(document, state)
        }
        "settings.taskbar.position" => {
            state.taskbar_position = state.taskbar_position.next();
            apply_virtual_viewport(document, state)?;
            sync_settings_controls(document, state)
        }
        "settings.taskbar.size" => {
            state.taskbar_height = match state.taskbar_height.round() as i32 {
                34 => 44.0,
                44 => 56.0,
                _ => 34.0,
            };
            apply_virtual_viewport(document, state)?;
            sync_settings_controls(document, state)
        }
        "settings.taskbar-opacity.down" => {
            state.taskbar_opacity = state.taskbar_opacity.saturating_add(10).min(100);
            apply_taskbar(document, state)?;
            sync_settings_controls(document, state)
        }
        "settings.taskbar-opacity.up" => {
            state.taskbar_opacity = state.taskbar_opacity.saturating_sub(10);
            apply_taskbar(document, state)?;
            sync_settings_controls(document, state)
        }
        "settings.display.edp" => {
            state.selected_display = DisplayId::Edp;
            apply_virtual_viewport(document, state)?;
            sync_display_settings(document, state)
        }
        "settings.display.hdmi" => {
            state.selected_display = DisplayId::Hdmi;
            apply_virtual_viewport(document, state)?;
            sync_display_settings(document, state)
        }
        "settings.display.resolution" => {
            let slot = display_slot(state.selected_display);
            let count = display_resolutions(state.selected_display).len();
            state.displays[slot].resolution_index = (state.displays[slot].resolution_index + 1) % count;
            sync_display_settings(document, state)
        }
        "settings.display.scale" => {
            let slot = display_slot(state.selected_display);
            state.displays[slot].scale_index = (state.displays[slot].scale_index + 1) % 5;
            apply_virtual_viewport(document, state)?;
            sync_display_settings(document, state)
        }
        "settings.font.family" => {
            state.font_family_index = (state.font_family_index + 1) % font_families().len();
            apply_font_settings(document, state)?;
            sync_settings_controls(document, state)
        }
        "settings.font.bold" => {
            state.font_bold = !state.font_bold;
            apply_font_settings(document, state)?;
            sync_settings_controls(document, state)
        }
        "settings.font-size.down" => {
            state.font_size_offset = (state.font_size_offset - 1).max(-3);
            apply_font_settings(document, state)?;
            sync_settings_controls(document, state)
        }
        "settings.font-size.up" => {
            state.font_size_offset = (state.font_size_offset + 1).min(6);
            apply_font_settings(document, state)?;
            sync_settings_controls(document, state)
        }
        "settings.hotkey.start" => cycle_hotkey(document, state, 0),
        "settings.hotkey.left" => cycle_hotkey(document, state, 1),
        "settings.hotkey.right" => cycle_hotkey(document, state, 2),
        "settings.hotkey.up" => cycle_hotkey(document, state, 3),
        "settings.hotkey.down" => cycle_hotkey(document, state, 4),

        "menu.folder" => create_demo_folder(document, state),
        "menu.desktop-settings" => {
            open_window(document, state, WindowKind::Settings)?;
            show_settings_page(document, "desktop")
        }
        "menu.note" => {
            document.set_visible("desktop-menu", false)?;
            if state.sticky_enabled {
                state.sticky_visible = true;
                apply_sticky(document, state)?;
                sync_sticky_text(document, state)?;
                document.set_visible("sticky-note", true)
            } else {
                show_toast(document, "Sticky notes are disabled in Desktop settings")
            }
        }
        "entry.shortcut" => desktop_entry_shortcut(document, state),
        "entry.delete" => desktop_entry_delete(document, state),
        "entry.empty-trash" => empty_virtual_trash(document, state),
        "sticky.menu.settings" => show_sticky_settings_menu(document),
        "sticky.menu.delete" => delete_sticky(document, state),
        "sticky.bg.next" => cycle_sticky_background(document, state),
        "sticky.fg.next" => cycle_sticky_foreground(document, state),
        "sticky.size.next" => cycle_sticky_size(document, state),

        "accent.red" => set_accent(document, state, Color::rgb(239, 64, 72)),
        "accent.blue" => set_accent(document, state, Color::rgb(61, 174, 233)),
        "accent.purple" => set_accent(document, state, Color::rgb(155, 89, 182)),
        "accent.teal" => set_accent(document, state, Color::rgb(26, 188, 156)),
        "accent.green" => set_accent(document, state, Color::rgb(84, 213, 107)),
        "accent.amber" => set_accent(document, state, Color::rgb(243, 156, 18)),

        "about.donate" => show_toast(document, "Prototype link: PayPal would open outside the real desktop"),
        "about.source" => show_toast(document, "Prototype link: GitHub would open outside the real desktop"),
        "about.website" => show_toast(document, "Prototype website: https://wm.arkflame.com"),
        "toast.dismiss" => document.set_visible("toast", false),

        "media.toggle" => toggle_popup(document, state, PopupKind::Media),
        "volume.open" => toggle_popup(document, state, PopupKind::Volume),
        "network.open" => toggle_popup(document, state, PopupKind::Network),
        "clock.open" => toggle_popup(document, state, PopupKind::Clock),
        "media.playpause" => {
            state.media_playing = !state.media_playing;
            sync_media_icons(document, state)
        }
        "media.previous" => show_toast(document, "Previous track — virtual media state"),
        "media.next" => show_toast(document, "Next track — virtual media state"),
        "volume.down" => {
            state.volume = state.volume.saturating_sub(8);
            state.muted = state.volume == 0;
            sync_volume(document, state)
        }
        "volume.up" => {
            state.volume = state.volume.saturating_add(8).min(100);
            state.muted = false;
            sync_volume(document, state)
        }
        "volume.mute" => {
            state.muted = !state.muted;
            sync_volume(document, state)
        }
        action if action.starts_with("network.connect.") => {
            let name = action.trim_start_matches("network.connect.");
            document.set_text("wifi-connected-name", match name {
                "arknet" => "ArkNet 5G",
                "studio" => "Studio",
                "guest" => "Guest Network",
                _ => "ArkNet 5G",
            })?;
            Ok(())
        }
        action if action.starts_with("session.") => {
            show_toast(document, &format!("{} simulated inside the virtual desktop", action.trim_start_matches("session.")))?;
            close_popovers(document, state)
        }

        "settings.nav.surface" | "start.menu.surface" | "desktop.menu.surface" |
        "desktop.entry.menu.surface" | "task.menu.surface" | "sticky.menu.surface" | "popup.surface" => Ok(()),
        other => {
            println!("RWR_DEMO_ACTION {other} button={} x={} y={}", event.button, event.x, event.y);
            Ok(())
        }
    }
}

fn handle_desktop_surface(event: &ActionEvent, document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    if event.button == 3 && event.phase == ActionPhase::Release {
        let area = work_area(state);
        let x = event.x.clamp(area.x, (area.x + area.width - 215.0).max(area.x));
        let y = event.y.clamp(area.y, (area.y + area.height - 145.0).max(area.y));
        close_popovers(document, state)?;
        document.set_position_px("desktop-menu", x, y)?;
        document.set_visible("desktop-menu", true)?;
        return Ok(());
    }
    if event.button != 1 {
        return Ok(());
    }
    let area = work_area(state);
    match event.phase {
        ActionPhase::Press => {
            state.sticky_editing = false;
            close_popovers(document, state)?;
            state.selected_icons.clear();
            sync_icon_selection(document, state)?;
            let start_x = event.x.clamp(area.x, area.x + area.width);
            let start_y = event.y.clamp(area.y, area.y + area.height);
            state.drag = Some(DragState::Selection { start_x, start_y });
            document.set_position_px("desktop-selection", start_x, start_y)?;
            document.set_size_px("desktop-selection", 1.0, 1.0)?;
            document.set_visible("desktop-selection", true)?;
        }
        ActionPhase::Motion => {
            if let Some(DragState::Selection { start_x, start_y }) = state.drag.as_ref() {
                let x = event.x.clamp(area.x, area.x + area.width);
                let y = event.y.clamp(area.y, area.y + area.height);
                let select = BoxState {
                    x: start_x.min(x),
                    y: start_y.min(y),
                    width: (x - start_x).abs().max(1.0),
                    height: (y - start_y).abs().max(1.0),
                };
                document.set_position_px("desktop-selection", select.x, select.y)?;
                document.set_size_px("desktop-selection", select.width, select.height)?;
                state.selected_icons.clear();
                for kind in all_icons() {
                    if !icon_is_visible(state, kind) {
                        continue;
                    }
                    if let Some(icon) = state.icons.get(&kind) {
                        if select.width > 3.0 && select.height > 3.0 && select.intersects(icon.rect) {
                            state.selected_icons.insert(kind);
                        }
                    }
                }
                sync_icon_selection(document, state)?;
            }
        }
        ActionPhase::Hover => {}
        ActionPhase::Release => {
            state.drag = None;
            document.set_visible("desktop-selection", false)?;
        }
    }
    Ok(())
}

fn handle_icon_drag(event: &ActionEvent, document: &mut RuntimeDocument, state: &mut DemoState, action: &str) -> Result<(), String> {
    let Some(kind) = icon_from_action(action) else { return Ok(()); };
    if !icon_is_visible(state, kind) {
        return Ok(());
    }
    if event.button == 3 && event.phase == ActionPhase::Release {
        if !state.selected_icons.contains(&kind) {
            state.selected_icons.clear();
            state.selected_icons.insert(kind);
            sync_icon_selection(document, state)?;
        }
        return open_desktop_entry_menu(document, state, kind);
    }
    if event.button != 1 {
        return Ok(());
    }
    match event.phase {
        ActionPhase::Press => {
            if !state.selected_icons.contains(&kind) {
                state.selected_icons.clear();
                state.selected_icons.insert(kind);
                sync_icon_selection(document, state)?;
            }
            let originals: Vec<(IconKind, BoxState)> = state.selected_icons.iter().copied()
                .filter(|selected| icon_is_visible(state, *selected))
                .filter_map(|selected| state.icons.get(&selected).map(|entry| (selected, entry.rect)))
                .collect();
            state.drag = Some(DragState::Icons {
                anchor: kind,
                start_x: event.x,
                start_y: event.y,
                originals,
            });
            close_popovers(document, state)?;
        }
        ActionPhase::Motion => {
            let Some(DragState::Icons { anchor, start_x, start_y, originals }) = state.drag.as_ref() else { return Ok(()); };
            if *anchor != kind {
                return Ok(());
            }
            let area = work_area(state);
            let mut dx = event.x - *start_x;
            let mut dy = event.y - *start_y;
            let min_x = originals.iter().map(|(_, r)| r.x).fold(f32::INFINITY, f32::min);
            let min_y = originals.iter().map(|(_, r)| r.y).fold(f32::INFINITY, f32::min);
            let max_x = originals.iter().map(|(_, r)| r.x + r.width).fold(f32::NEG_INFINITY, f32::max);
            let max_y = originals.iter().map(|(_, r)| r.y + r.height).fold(f32::NEG_INFINITY, f32::max);
            dx = dx.clamp(area.x - min_x, area.x + area.width - max_x);
            dy = dy.clamp(area.y - min_y, area.y + area.height - max_y);
            let updates: Vec<(IconKind, BoxState)> = originals.iter().map(|(selected, original)| {
                (*selected, BoxState { x: original.x + dx, y: original.y + dy, ..*original })
            }).collect();
            for (selected, rect) in updates {
                if let Some(icon) = state.icons.get_mut(&selected) {
                    icon.rect = rect;
                }
                apply_icon_rect(document, state, selected)?;
            }
        }
        ActionPhase::Hover => {}
        ActionPhase::Release => {
            let Some(DragState::Icons { anchor, start_x, start_y, originals }) = state.drag.take() else { return Ok(()); };
            if anchor != kind {
                return Ok(());
            }
            let moved = (event.x - start_x).abs() + (event.y - start_y).abs() >= 5.0;
            if moved {
                state.last_icon_click = None;
                let trash_hit = state.icons.get(&IconKind::Trash)
                    .map(|trash| !trash.trashed && event.x >= trash.rect.x && event.x <= trash.rect.x + trash.rect.width && event.y >= trash.rect.y && event.y <= trash.rect.y + trash.rect.height)
                    .unwrap_or(false);
                if trash_hit && originals.iter().any(|(selected, _)| *selected != IconKind::Trash) {
                    trash_icon_group(document, state, &originals)?;
                } else {
                    snap_icon_group(document, state, &originals, event.x - start_x, event.y - start_y)?;
                }
            } else {
                let double_click = state.last_icon_click
                    .map(|(last_kind, last_time)| last_kind == kind && event.time_ms.saturating_sub(last_time) <= 350)
                    .unwrap_or(false);
                if double_click {
                    state.last_icon_click = None;
                    open_desktop_icon(document, state, kind)?;
                } else {
                    state.last_icon_click = Some((kind, event.time_ms));
                }
            }
        }
    }
    Ok(())
}

fn trash_icon_group(document: &mut RuntimeDocument, state: &mut DemoState, originals: &[(IconKind, BoxState)]) -> Result<(), String> {
    let mut count = 0usize;
    for (kind, _) in originals {
        if *kind == IconKind::Trash {
            continue;
        }
        if let Some(icon) = state.icons.get_mut(kind) {
            icon.trashed = true;
            count += 1;
        }
        state.selected_icons.remove(kind);
        document.set_visible(kind.id(), false)?;
    }
    sync_icon_selection(document, state)?;
    if count > 0 {
        show_toast(document, &format!("Moved {count} desktop item{} to the virtual Trash", if count == 1 { "" } else { "s" }))?;
    }
    Ok(())
}

fn snap_icon_group(
    document: &mut RuntimeDocument,
    state: &mut DemoState,
    originals: &[(IconKind, BoxState)],
    dx: f32,
    dy: f32,
) -> Result<(), String> {
    let selected: HashSet<IconKind> = originals.iter().map(|(kind, _)| *kind).collect();
    let proposed_dc = (dx / GRID_X).round() as i32;
    let proposed_dr = (dy / GRID_Y).round() as i32;
    let candidate = find_group_delta(state, originals, &selected, proposed_dc, proposed_dr).unwrap_or((0, 0));
    for (kind, original) in originals {
        let rect = BoxState {
            x: original.x + candidate.0 as f32 * GRID_X,
            y: original.y + candidate.1 as f32 * GRID_Y,
            ..*original
        };
        if let Some(icon) = state.icons.get_mut(kind) {
            icon.rect = rect;
        }
        apply_icon_rect(document, state, *kind)?;
    }
    Ok(())
}

fn find_group_delta(
    state: &DemoState,
    originals: &[(IconKind, BoxState)],
    selected: &HashSet<IconKind>,
    dc: i32,
    dr: i32,
) -> Option<(i32, i32)> {
    let area = work_area(state);
    let occupied: Vec<BoxState> = state.icons.iter()
        .filter(|(kind, icon)| !selected.contains(kind) && !icon.trashed && (**kind != IconKind::NewFolder || state.new_folder_visible))
        .map(|(_, icon)| icon.rect)
        .collect();
    let valid = |candidate_dc: i32, candidate_dr: i32| -> bool {
        originals.iter().all(|(_, original)| {
            let target = BoxState {
                x: original.x + candidate_dc as f32 * GRID_X,
                y: original.y + candidate_dr as f32 * GRID_Y,
                ..*original
            };
            target.x >= area.x
                && target.y >= area.y
                && target.x + target.width <= area.x + area.width
                && target.y + target.height <= area.y + area.height
                && !occupied.iter().any(|other| target.intersects(*other))
        })
    };
    if valid(dc, dr) {
        return Some((dc, dr));
    }
    for radius in 1_i32..=8_i32 {
        for ox in -radius..=radius {
            for oy in -radius..=radius {
                if ox.abs() != radius && oy.abs() != radius {
                    continue;
                }
                let candidate = (dc + ox, dr + oy);
                if valid(candidate.0, candidate.1) {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn handle_sticky_interaction(event: &ActionEvent, document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    if !state.sticky_visible {
        return Ok(());
    }
    if event.button == 3 && event.phase == ActionPhase::Release {
        state.sticky_editing = false;
        return open_sticky_menu(document, state, event.x, event.y);
    }
    if event.action == "sticky.note.editor" {
        if event.button == 1 && matches!(event.phase, ActionPhase::Press | ActionPhase::Release) {
            state.sticky_editing = true;
            state.drag = None;
            document.set_visible("desktop-menu", false)?;
            close_context_menus(document, state)?;
            close_tray_popups(document, state)?;
            close_start(document, state)?;
        }
        return Ok(());
    }
    if event.button != 1 {
        return Ok(());
    }
    match event.phase {
        ActionPhase::Press => {
            state.sticky_editing = false;
            state.drag = Some(DragState::Sticky {
                start_x: event.x,
                start_y: event.y,
                original: state.sticky_rect,
            });
        }
        ActionPhase::Motion => {
            if let Some(DragState::Sticky { start_x, start_y, original }) = state.drag.as_ref() {
                let area = work_area(state);
                state.sticky_rect.x = (original.x + event.x - *start_x)
                    .clamp(area.x, (area.x + area.width - original.width).max(area.x));
                state.sticky_rect.y = (original.y + event.y - *start_y)
                    .clamp(area.y, (area.y + area.height - original.height).max(area.y));
                apply_sticky(document, state)?;
            }
        }
        ActionPhase::Hover => {}
        ActionPhase::Release => state.drag = None,
    }
    Ok(())
}

fn handle_window_drag(event: &ActionEvent, document: &mut RuntimeDocument, state: &mut DemoState, kind: WindowKind) -> Result<(), String> {
    if event.button != 1 {
        return Ok(());
    }
    match event.phase {
        ActionPhase::Press => {
            focus_window(document, state, kind)?;
            let mut window = window_state(state, kind)?;
            if window.maximized || window.snapped {
                if let Some(restore) = window.restore {
                    let old = window.rect;
                    window.rect = restore;
                    window.maximized = false;
                    window.snapped = false;
                    window.snap_target = None;
                    let ratio = ((event.x - old.x) / old.width.max(1.0)).clamp(0.08, 0.92);
                    let offset_x = (restore.width * ratio).clamp(35.0, restore.width - 35.0);
                    let offset_y = (event.y - old.y).clamp(8.0, 28.0);
                    set_window_state(state, kind, window);
                    apply_window(document, state, kind)?;
                    set_maximize_icons(document, kind, false)?;
                    state.drag = Some(DragState::Window { kind, offset_x, offset_y, floating_before_drag: restore });
                    return Ok(());
                }
            }
            state.drag = Some(DragState::Window {
                kind,
                offset_x: event.x - window.rect.x,
                offset_y: event.y - window.rect.y,
                floating_before_drag: window.rect,
            });
            close_popovers(document, state)?;
        }
        ActionPhase::Motion => {
            let Some(DragState::Window { kind: active, offset_x, offset_y, .. }) = state.drag.as_ref() else { return Ok(()); };
            if *active != kind {
                return Ok(());
            }
            let area = work_area(state);
            let mut window = window_state(state, kind)?;
            window.rect.x = (event.x - *offset_x).clamp(area.x - window.rect.width + 80.0, area.x + area.width - 80.0);
            window.rect.y = (event.y - *offset_y).clamp(area.y, area.y + area.height - 31.0);
            window.maximized = false;
            window.snapped = false;
            window.snap_target = None;
            set_window_state(state, kind, window);
            apply_window(document, state, kind)?;
            if let Some(target) = snap_target(state, event.x, event.y) {
                let preview = snap_geometry(state, target);
                document.set_position_px("snap-preview", preview.x, preview.y)?;
                document.set_size_px("snap-preview", preview.width, preview.height)?;
                document.set_visible("snap-preview", true)?;
            } else {
                document.set_visible("snap-preview", false)?;
            }
        }
        ActionPhase::Hover => {}
        ActionPhase::Release => {
            let Some(DragState::Window { kind: active, floating_before_drag, .. }) = state.drag.take() else { return Ok(()); };
            if active != kind {
                return Ok(());
            }
            if let Some(target) = snap_target(state, event.x, event.y) {
                let mut window = window_state(state, kind)?;
                window.restore = Some(floating_before_drag);
                window.rect = snap_geometry(state, target);
                window.maximized = matches!(target, SnapTarget::Maximize);
                window.snapped = !window.maximized;
                window.snap_target = if window.maximized { None } else { Some(target) };
                set_window_state(state, kind, window);
                apply_window(document, state, kind)?;
                set_maximize_icons(document, kind, true)?;
            } else {
                let mut window = window_state(state, kind)?;
                window.restore = None;
                window.snap_target = None;
                set_window_state(state, kind, window);
            }
            document.set_visible("snap-preview", false)?;
        }
    }
    Ok(())
}

fn window_control(document: &mut RuntimeDocument, state: &mut DemoState, action: &str, control: &str) -> Result<(), String> {
    let Some(kind) = window_from_action(action) else { return Ok(()); };
    match control {
        "minimize" => minimize_window(document, state, kind),
        "maximize" => toggle_maximize(document, state, kind),
        "close" => close_window(document, state, kind),
        _ => Ok(()),
    }
}

fn open_window(document: &mut RuntimeDocument, state: &mut DemoState, kind: WindowKind) -> Result<(), String> {
    let mut window = window_state(state, kind)?;
    if !window.open {
        window.rect = kind.default_rect();
        window.restore = None;
        window.maximized = false;
        window.snapped = false;
        window.snap_target = None;
        window.desktop = state.current_desktop;
    }
    window.open = true;
    window.minimized = false;
    set_window_state(state, kind, window);
    close_popovers(document, state)?;
    focus_window(document, state, kind)
}

fn open_files_window(document: &mut RuntimeDocument, state: &mut DemoState, label: &str, path: &str) -> Result<(), String> {
    document.set_text("file-location", path)?;
    show_toast(document, &format!("{label} opened in the virtual Dolphin window"))?;
    open_window(document, state, WindowKind::Files)
}

fn open_generic(document: &mut RuntimeDocument, state: &mut DemoState, title: &str, description: &str) -> Result<(), String> {
    document.set_text("generic-window-title", title)?;
    document.set_text("generic-heading", title)?;
    document.set_text("generic-description", description)?;
    open_window(document, state, WindowKind::Generic)
}

fn activate_task(document: &mut RuntimeDocument, state: &mut DemoState, kind: WindowKind) -> Result<(), String> {
    let window = window_state(state, kind)?;
    if !window.open || window.desktop != state.current_desktop {
        return open_window(document, state, kind);
    }
    if window.minimized {
        let mut restored = window;
        restored.minimized = false;
        set_window_state(state, kind, restored);
        return focus_window(document, state, kind);
    }
    if state.focused_window == Some(kind) {
        minimize_window(document, state, kind)
    } else {
        focus_window(document, state, kind)
    }
}

fn focus_window(document: &mut RuntimeDocument, state: &mut DemoState, kind: WindowKind) -> Result<(), String> {
    let mut window = window_state(state, kind)?;
    if !window.open {
        return Ok(());
    }
    window.minimized = false;
    state.z_counter += 1;
    window.z = state.z_counter;
    set_window_state(state, kind, window);
    state.focused_window = Some(kind);
    document.set_z_index(kind.id(), window.z)?;
    sync_window_visibility(document, state)?;
    sync_taskbar_activity(document, state)
}

fn minimize_window(document: &mut RuntimeDocument, state: &mut DemoState, kind: WindowKind) -> Result<(), String> {
    let mut window = window_state(state, kind)?;
    window.minimized = true;
    set_window_state(state, kind, window);
    if state.focused_window == Some(kind) {
        state.focused_window = None;
    }
    sync_window_visibility(document, state)?;
    focus_top_window(document, state)?;
    sync_taskbar_activity(document, state)
}

fn close_window(document: &mut RuntimeDocument, state: &mut DemoState, kind: WindowKind) -> Result<(), String> {
    let mut window = window_state(state, kind)?;
    window.open = false;
    window.minimized = false;
    window.restore = None;
    window.maximized = false;
    window.snapped = false;
    window.snap_target = None;
    set_window_state(state, kind, window);
    if state.focused_window == Some(kind) {
        state.focused_window = None;
    }
    sync_window_visibility(document, state)?;
    focus_top_window(document, state)?;
    sync_taskbar_activity(document, state)
}

fn focus_top_window(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let top = ALL_WINDOWS.iter().copied()
        .filter_map(|kind| state.windows.get(&kind).copied().map(|window| (kind, window)))
        .filter(|(_, window)| window.open && !window.minimized && window.desktop == state.current_desktop)
        .max_by_key(|(_, window)| window.z)
        .map(|(kind, _)| kind);
    if let Some(kind) = top {
        focus_window(document, state, kind)?;
    }
    Ok(())
}

fn cycle_windows(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let mut windows: Vec<(WindowKind, WindowState)> = ALL_WINDOWS.iter().copied()
        .filter_map(|kind| state.windows.get(&kind).copied().map(|window| (kind, window)))
        .filter(|(_, window)| window.open && window.desktop == state.current_desktop)
        .collect();
    windows.sort_by_key(|(_, window)| -window.z);
    if windows.is_empty() {
        return Ok(());
    }
    let index = windows.iter().position(|(kind, _)| Some(*kind) == state.focused_window).unwrap_or(0);
    let next = windows[(index + 1) % windows.len()].0;
    focus_window(document, state, next)
}

fn toggle_maximize(document: &mut RuntimeDocument, state: &mut DemoState, kind: WindowKind) -> Result<(), String> {
    let mut window = window_state(state, kind)?;
    if window.maximized || window.snapped {
        if let Some(restore) = window.restore {
            window.rect = restore;
        }
        window.restore = None;
        window.maximized = false;
        window.snapped = false;
        window.snap_target = None;
        set_maximize_icons(document, kind, false)?;
    } else {
        window.restore = Some(window.rect);
        window.rect = work_area(state);
        window.maximized = true;
        window.snapped = false;
        window.snap_target = None;
        set_maximize_icons(document, kind, true)?;
    }
    set_window_state(state, kind, window);
    apply_window(document, state, kind)?;
    focus_window(document, state, kind)
}

fn set_maximize_icons(document: &mut RuntimeDocument, kind: WindowKind, restore: bool) -> Result<(), String> {
    let max_id = format!("{}-max-icon", kind.prefix());
    let restore_id = format!("{}-restore-icon", kind.prefix());
    if document.node_by_id(&max_id).is_some() {
        document.set_visible(&max_id, !restore)?;
    }
    if document.node_by_id(&restore_id).is_some() {
        document.set_visible(&restore_id, restore)?;
    }
    Ok(())
}

fn window_state(state: &DemoState, kind: WindowKind) -> Result<WindowState, String> {
    state.windows.get(&kind).copied().ok_or_else(|| format!("missing window state for {kind:?}"))
}

fn set_window_state(state: &mut DemoState, kind: WindowKind, window: WindowState) {
    state.windows.insert(kind, window);
}

fn apply_window(document: &mut RuntimeDocument, state: &DemoState, kind: WindowKind) -> Result<(), String> {
    let window = window_state(state, kind)?;
    document.set_position_px(kind.id(), window.rect.x, window.rect.y)?;
    document.set_size_px(kind.id(), window.rect.width, window.rect.height)?;
    // The web prototype treats the content area as flex:1. Keep the native
    // compiled tree in the same contract so maximize/snap changes resize both
    // the outer frame and the actual application surface down to the panel.
    let body_height = (window.rect.height - 31.0).max(1.0);
    if document.node_by_id(kind.body_id()).is_some() {
        document.set_size_px(kind.body_id(), window.rect.width, body_height)?;
    }
    document.set_z_index(kind.id(), window.z)?;
    Ok(())
}

fn sync_window_visibility(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    for kind in ALL_WINDOWS {
        let window = window_state(state, kind)?;
        document.set_visible(kind.id(), window.open && !window.minimized && window.desktop == state.current_desktop)?;
    }
    Ok(())
}

fn sync_taskbar_activity(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    let vertical = matches!(state.taskbar_position, TaskbarPosition::Left | TaskbarPosition::Right);
    let h = state.taskbar_height;
    for kind in ALL_WINDOWS {
        let Some(task_id) = kind.task_id() else { continue; };
        let Some(line_id) = kind.task_line() else { continue; };
        let window = window_state(state, kind)?;
        let running = window.open && window.desktop == state.current_desktop;
        let pinned = state.pinned_tasks.contains(&kind);
        document.set_visible(task_id, pinned || running)?;
        document.set_visible(line_id, running)?;
        if running {
            let focused = state.focused_window == Some(kind) && !window.minimized;
            document.set_background_color(line_id, if focused { state.accent } else { Color::rgb(135, 145, 155) })?;
            if vertical {
                document.set_size_px(line_id, if focused { 3.0 } else { 2.0 }, if focused { 32.0 } else { 24.0 })?;
                document.set_position_px(line_id, if focused { h - 3.0 } else { h - 2.0 }, if focused { 5.0 } else { 9.0 })?;
            } else {
                document.set_size_px(line_id, if focused { 32.0 } else { 24.0 }, if focused { 3.0 } else { 2.0 })?;
                document.set_position_px(line_id, if focused { 5.0 } else { 9.0 }, if focused { h - 5.0 } else { h - 4.0 })?;
            }
        }
    }
    Ok(())
}

fn current_ui_scale(state: &DemoState) -> f32 {
    let slot = display_slot(state.selected_display);
    display_scales()[state.displays[slot].scale_index] as f32 / 100.0
}

fn viewport_size(state: &DemoState) -> (f32, f32) {
    let scale = current_ui_scale(state).max(0.5);
    (VIEW_W / scale, VIEW_H / scale)
}

fn apply_virtual_viewport(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    document.set_ui_scale(current_ui_scale(state))?;
    apply_taskbar(document, state)?;
    clamp_windows_to_work_area(document, state)?;
    reflow_icons_for_viewport(document, state)?;
    position_panel_surfaces(document, state)?;
    sync_taskbar_activity(document, state)?;
    Ok(())
}

fn reflow_icons_for_viewport(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let area = work_area(state);
    let rows = (((area.height - GRID_ORIGIN - ICON_H).max(0.0) / GRID_Y).floor() as usize + 1).max(1);
    let mut visible_index = 0usize;
    for kind in all_icons() {
        if kind == IconKind::NewFolder && !state.new_folder_visible { continue; }
        if state.icons.get(&kind).map(|icon| icon.trashed).unwrap_or(false) { continue; }
        let col = visible_index / rows;
        let row = visible_index % rows;
        let x = (area.x + GRID_ORIGIN + col as f32 * GRID_X).clamp(area.x, (area.x + area.width - ICON_W).max(area.x));
        let y = (area.y + GRID_ORIGIN + row as f32 * GRID_Y).clamp(area.y, (area.y + area.height - ICON_H).max(area.y));
        if let Some(icon) = state.icons.get_mut(&kind) { icon.rect.x = x; icon.rect.y = y; }
        apply_icon_rect(document, state, kind)?;
        visible_index += 1;
    }
    Ok(())
}

fn nearest_taskbar_edge(state: &DemoState, x: f32, y: f32) -> Option<TaskbarPosition> {
    let (view_w, view_h) = viewport_size(state);
    let threshold = 82.0;
    let candidates = [
        (y.max(0.0), TaskbarPosition::Top),
        ((view_h - y).max(0.0), TaskbarPosition::Bottom),
        (x.max(0.0), TaskbarPosition::Left),
        ((view_w - x).max(0.0), TaskbarPosition::Right),
    ];
    candidates.into_iter().min_by(|a, b| a.0.total_cmp(&b.0)).and_then(|(distance, edge)| if distance <= threshold { Some(edge) } else { None })
}

fn show_taskbar_dock_preview(document: &mut RuntimeDocument, state: &DemoState, edge: Option<TaskbarPosition>) -> Result<(), String> {
    let Some(edge) = edge else { return document.set_visible("taskbar-dock-preview", false); };
    let (view_w, view_h) = viewport_size(state);
    let h = state.taskbar_height;
    let rect = match edge {
        TaskbarPosition::Bottom => BoxState { x: 0.0, y: (view_h - h).max(0.0), width: view_w, height: h },
        TaskbarPosition::Top => BoxState { x: 0.0, y: 0.0, width: view_w, height: h },
        TaskbarPosition::Left => BoxState { x: 0.0, y: 0.0, width: h, height: view_h },
        TaskbarPosition::Right => BoxState { x: (view_w - h).max(0.0), y: 0.0, width: h, height: view_h },
    };
    document.set_position_px("taskbar-dock-preview", rect.x, rect.y)?;
    document.set_size_px("taskbar-dock-preview", rect.width, rect.height)?;
    document.set_visible("taskbar-dock-preview", true)
}

fn handle_taskbar_drag(event: &ActionEvent, document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    if event.button != 1 { return Ok(()); }
    match event.phase {
        ActionPhase::Press => {
            close_popovers(document, state)?;
            state.drag = Some(DragState::TaskbarDock { start_x: event.x, start_y: event.y, candidate: None, moved: false });
            document.set_visible("taskbar-dock-preview", false)?;
        }
        ActionPhase::Motion => {
            let (start_x, start_y) = match state.drag.as_ref() {
                Some(DragState::TaskbarDock { start_x, start_y, .. }) => (*start_x, *start_y),
                _ => return Ok(()),
            };
            let moved = (event.x - start_x).hypot(event.y - start_y) >= 6.0;
            let candidate = if moved { nearest_taskbar_edge(state, event.x, event.y) } else { None };
            if let Some(DragState::TaskbarDock { candidate: slot, moved: did_move, .. }) = state.drag.as_mut() {
                *slot = candidate;
                *did_move = moved;
            }
            show_taskbar_dock_preview(document, state, candidate)?;
        }
        ActionPhase::Hover => {}
        ActionPhase::Release => {
            let drag = state.drag.take();
            document.set_visible("taskbar-dock-preview", false)?;
            if let Some(DragState::TaskbarDock { candidate: Some(edge), moved: true, .. }) = drag {
                state.taskbar_position = edge;
                apply_virtual_viewport(document, state)?;
                sync_settings_controls(document, state)?;
            }
        }
    }
    Ok(())
}

fn work_area(state: &DemoState) -> BoxState {
    let (view_w, view_h) = viewport_size(state);
    let h = state.taskbar_height.min(view_w.min(view_h).max(1.0));
    match state.taskbar_position {
        TaskbarPosition::Bottom => BoxState { x: 0.0, y: 0.0, width: view_w, height: (view_h - h).max(1.0) },
        TaskbarPosition::Top => BoxState { x: 0.0, y: h, width: view_w, height: (view_h - h).max(1.0) },
        TaskbarPosition::Left => BoxState { x: h, y: 0.0, width: (view_w - h).max(1.0), height: view_h },
        TaskbarPosition::Right => BoxState { x: 0.0, y: 0.0, width: (view_w - h).max(1.0), height: view_h },
    }
}

fn snap_target(state: &DemoState, x: f32, y: f32) -> Option<SnapTarget> {
    let area = work_area(state);
    let corner = 42.0;
    let edge = 18.0;
    let left = area.x;
    let right = area.x + area.width;
    let top = area.y;
    let bottom = area.y + area.height;
    if y <= top + corner && x <= left + corner { return Some(SnapTarget::TopLeft); }
    if y <= top + corner && x >= right - corner { return Some(SnapTarget::TopRight); }
    if y >= bottom - corner && x <= left + corner { return Some(SnapTarget::BottomLeft); }
    if y >= bottom - corner && x >= right - corner { return Some(SnapTarget::BottomRight); }
    if y <= top + edge { return Some(SnapTarget::Maximize); }
    if x <= left + edge { return Some(SnapTarget::Left); }
    if x >= right - edge { return Some(SnapTarget::Right); }
    None
}

fn snap_geometry(state: &DemoState, target: SnapTarget) -> BoxState {
    let area = work_area(state);
    let half_w = area.width / 2.0;
    let half_h = area.height / 2.0;
    match target {
        SnapTarget::Maximize => area,
        SnapTarget::Left => BoxState { x: area.x, y: area.y, width: half_w, height: area.height },
        SnapTarget::Right => BoxState { x: area.x + half_w, y: area.y, width: area.width - half_w, height: area.height },
        SnapTarget::TopLeft => BoxState { x: area.x, y: area.y, width: half_w, height: half_h },
        SnapTarget::TopRight => BoxState { x: area.x + half_w, y: area.y, width: area.width - half_w, height: half_h },
        SnapTarget::BottomLeft => BoxState { x: area.x, y: area.y + half_h, width: half_w, height: area.height - half_h },
        SnapTarget::BottomRight => BoxState { x: area.x + half_w, y: area.y + half_h, width: area.width - half_w, height: area.height - half_h },
    }
}

fn switch_desktop(document: &mut RuntimeDocument, state: &mut DemoState, index: u8) -> Result<(), String> {
    if index == 0 || index > state.desktops || index == state.current_desktop {
        return Ok(());
    }
    state.current_desktop = index;
    state.focused_window = None;
    close_popovers(document, state)?;
    sync_window_visibility(document, state)?;
    sync_taskbar_activity(document, state)?;
    sync_workspace_pager(document, state)?;
    focus_top_window(document, state)
}

fn workspace_grid_position(index: u8, desktops: u8) -> (u8, u8, u8) {
    let columns = ((desktops as u16 + 1) / 2).max(1) as u8;
    if index <= columns {
        (1, index, columns)
    } else {
        (2, index - columns, columns)
    }
}

fn switch_desktop_direction(document: &mut RuntimeDocument, state: &mut DemoState, delta: i8) -> Result<(), String> {
    let (row, col, columns) = workspace_grid_position(state.current_desktop, state.desktops);
    let target = match delta {
        -1 if col > 1 => Some(state.current_desktop - 1),
        1 if col < columns => {
            let candidate = state.current_desktop + 1;
            (candidate <= state.desktops && workspace_grid_position(candidate, state.desktops).0 == row).then_some(candidate)
        }
        _ => None,
    };
    if let Some(next) = target {
        switch_desktop(document, state, next)?;
    }
    Ok(())
}

fn switch_desktop_grid(document: &mut RuntimeDocument, state: &mut DemoState, delta: i8) -> Result<(), String> {
    let (row, _col, columns) = workspace_grid_position(state.current_desktop, state.desktops);
    let target = match delta {
        value if value < 0 && row == 2 => state.current_desktop.checked_sub(columns),
        value if value > 0 && row == 1 => {
            let candidate = state.current_desktop.saturating_add(columns);
            (candidate <= state.desktops).then_some(candidate)
        }
        _ => None,
    };
    if let Some(next) = target {
        switch_desktop(document, state, next)?;
    }
    Ok(())
}

fn sync_workspace_pager(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    for index in 1..=4u8 {
        let id = format!("workspace-{index}");
        let visible = index <= state.desktops;
        document.set_visible(&id, visible)?;
        if visible {
            let active = index == state.current_desktop;
            document.set_background_color(&id, if active { Color { a: 87, ..state.accent } } else { Color::rgb(36, 39, 42) })?;
            document.set_border_color(&id, if active { state.accent } else { Color::rgb(81, 86, 91) })?;
        }
    }
    Ok(())
}

fn apply_taskbar(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    let (view_w, view_h) = viewport_size(state);
    let h = state.taskbar_height;
    let base = taskbar_colors()[state.taskbar_color_index].1;
    let panel_color = Color { a: ((state.taskbar_opacity as u16 * 255) / 100) as u8, ..base };
    document.set_background_color("taskbar", panel_color)?;
    let vertical = matches!(state.taskbar_position, TaskbarPosition::Left | TaskbarPosition::Right);
    match state.taskbar_position {
        TaskbarPosition::Bottom => {
            document.set_position_px("taskbar", 0.0, (view_h - h).max(0.0))?;
            document.set_size_px("taskbar", view_w, h)?;
            document.set_flex_direction("taskbar", FlexDirection::Row)?;
            document.set_position_px("taskbar-line", 0.0, 0.0)?;
            document.set_size_px("taskbar-line", view_w, 1.0)?;
        }
        TaskbarPosition::Top => {
            document.set_position_px("taskbar", 0.0, 0.0)?;
            document.set_size_px("taskbar", view_w, h)?;
            document.set_flex_direction("taskbar", FlexDirection::Row)?;
            document.set_position_px("taskbar-line", 0.0, h - 1.0)?;
            document.set_size_px("taskbar-line", view_w, 1.0)?;
        }
        TaskbarPosition::Left => {
            document.set_position_px("taskbar", 0.0, 0.0)?;
            document.set_size_px("taskbar", h, view_h)?;
            document.set_flex_direction("taskbar", FlexDirection::Column)?;
            document.set_position_px("taskbar-line", h - 1.0, 0.0)?;
            document.set_size_px("taskbar-line", 1.0, view_h)?;
        }
        TaskbarPosition::Right => {
            document.set_position_px("taskbar", (view_w - h).max(0.0), 0.0)?;
            document.set_size_px("taskbar", h, view_h)?;
            document.set_flex_direction("taskbar", FlexDirection::Column)?;
            document.set_position_px("taskbar-line", 0.0, 0.0)?;
            document.set_size_px("taskbar-line", 1.0, view_h)?;
        }
    }
    let task_ids = ["task-start", "task-browser", "task-files", "task-terminal", "task-code", "task-settings", "task-music", "task-generic"];
    let tray_ids = ["tray-media", "tray-volume", "tray-network"];
    if vertical {
        for id in task_ids { document.set_size_px(id, h, 42.0)?; }
        document.set_size_px("taskbar-spacer", h, 6.0)?;
        document.set_flex_direction("workspace-pager", FlexDirection::Column)?;
        document.set_size_px("workspace-pager", h, 51.0)?;
        for id in tray_ids { document.set_size_px(id, h, 30.0)?; }
        document.set_size_px("clock-button", h, 74.0)?;
    } else {
        for id in task_ids { document.set_size_px(id, 42.0, h)?; }
        document.set_size_px("taskbar-spacer", 6.0, h)?;
        document.set_flex_direction("workspace-pager", FlexDirection::Row)?;
        document.set_size_px("workspace-pager", 51.0, h)?;
        for id in tray_ids { document.set_size_px(id, 30.0, h)?; }
        document.set_size_px("clock-button", 74.0, h)?;
    }
    position_panel_surfaces(document, state)?;
    Ok(())
}

fn position_panel_surfaces(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    let (view_w, view_h) = viewport_size(state);
    let h = state.taskbar_height;
    let start_w = 292.0;
    let start_h = 352.0;
    let submenu_w = 260.0;
    let (start_x, start_y, submenu_x, submenu_y) = match state.taskbar_position {
        TaskbarPosition::Bottom => (4.0, view_h - h - start_h, 300.0, view_h - h - start_h),
        TaskbarPosition::Top => (4.0, h + 3.0, 300.0, h + 3.0),
        TaskbarPosition::Left => (h + 3.0, 4.0, h + start_w + 7.0, 4.0),
        TaskbarPosition::Right => (view_w - h - start_w - 4.0, 4.0, view_w - h - start_w - submenu_w - 8.0, 4.0),
    };
    document.set_position_px("start-menu", start_x.clamp(0.0, (view_w - start_w).max(0.0)), start_y.clamp(0.0, (view_h - start_h).max(0.0)))?;
    document.set_position_px("start-submenu", submenu_x.clamp(0.0, (view_w - submenu_w).max(0.0)), submenu_y.clamp(0.0, (view_h - 46.0).max(0.0)))?;

    for popup in [PopupKind::Media, PopupKind::Volume, PopupKind::Network, PopupKind::Clock] {
        let (w, ph) = popup.size();
        let pw = w.min((view_w - 8.0).max(80.0));
        let pheight = ph.min((view_h - 8.0).max(80.0));
        document.set_size_px(popup.id(), pw, pheight)?;
        let (x, y) = match state.taskbar_position {
            TaskbarPosition::Bottom => (view_w - pw - 8.0, view_h - h - pheight - 5.0),
            TaskbarPosition::Top => (view_w - pw - 8.0, h + 5.0),
            TaskbarPosition::Left => (h + 5.0, (view_h - pheight - 8.0).max(5.0)),
            TaskbarPosition::Right => (view_w - h - pw - 5.0, (view_h - pheight - 8.0).max(5.0)),
        };
        document.set_position_px(popup.id(), x.max(0.0), y.max(0.0))?;
    }
    Ok(())
}

fn clamp_windows_to_work_area(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let area = work_area(state);
    for kind in ALL_WINDOWS {
        let mut window = window_state(state, kind)?;
        if window.maximized {
            window.rect = area;
        } else if window.snapped {
            if let Some(target) = window.snap_target {
                window.rect = snap_geometry(state, target);
            } else {
                window.snapped = false;
                window.rect.width = window.rect.width.min(area.width.max(120.0));
                window.rect.height = window.rect.height.min(area.height.max(80.0));
            }
        } else {
            window.rect.width = window.rect.width.min(area.width.max(120.0));
            window.rect.height = window.rect.height.min(area.height.max(80.0));
            window.rect.x = window.rect.x.clamp(area.x - window.rect.width + 80.0, (area.x + area.width - 80.0).max(area.x - window.rect.width + 80.0));
            window.rect.y = window.rect.y.clamp(area.y, (area.y + area.height - 31.0).max(area.y));
        }
        set_window_state(state, kind, window);
        apply_window(document, state, kind)?;
    }
    let sticky_w = state.sticky_rect.width.min(area.width.max(80.0));
    let sticky_h = state.sticky_rect.height.min(area.height.max(80.0));
    state.sticky_rect.width = sticky_w;
    state.sticky_rect.height = sticky_h;
    state.sticky_rect.x = state.sticky_rect.x.clamp(area.x, (area.x + area.width - sticky_w).max(area.x));
    state.sticky_rect.y = state.sticky_rect.y.clamp(area.y, (area.y + area.height - sticky_h).max(area.y));
    apply_sticky(document, state)?;
    Ok(())
}

fn apply_icon_rect(document: &mut RuntimeDocument, state: &DemoState, kind: IconKind) -> Result<(), String> {
    let rect = state.icons.get(&kind).ok_or("missing icon state")?.rect;
    document.set_position_px(kind.id(), rect.x, rect.y)
}

fn all_icons() -> [IconKind; 6] {
    [IconKind::Home, IconKind::Downloads, IconKind::Projects, IconKind::Browser, IconKind::Trash, IconKind::NewFolder]
}

fn icon_is_visible(state: &DemoState, kind: IconKind) -> bool {
    if kind == IconKind::NewFolder && !state.new_folder_visible {
        return false;
    }
    state.icons.get(&kind).map(|icon| !icon.trashed).unwrap_or(false)
}

fn icon_from_action(action: &str) -> Option<IconKind> {
    match action {
        "desktop.icon.home" => Some(IconKind::Home),
        "desktop.icon.downloads" => Some(IconKind::Downloads),
        "desktop.icon.projects" => Some(IconKind::Projects),
        "desktop.icon.browser" => Some(IconKind::Browser),
        "desktop.icon.trash" => Some(IconKind::Trash),
        "desktop.icon.new-folder" => Some(IconKind::NewFolder),
        _ => None,
    }
}


fn task_kind_from_action(action: &str) -> Option<WindowKind> {
    match action {
        "task.browser" => Some(WindowKind::Browser),
        "task.files" => Some(WindowKind::Files),
        "task.terminal" => Some(WindowKind::Terminal),
        "task.code" => Some(WindowKind::Code),
        "task.settings" => Some(WindowKind::Settings),
        "task.music" => Some(WindowKind::Music),
        "task.generic" => Some(WindowKind::Generic),
        _ => None,
    }
}

fn open_task_context_menu(
    document: &mut RuntimeDocument,
    state: &mut DemoState,
    kind: WindowKind,
    pointer_x: f32,
    pointer_y: f32,
) -> Result<(), String> {
    let Some(_task_id) = kind.task_id() else {
        return Ok(());
    };
    close_popovers(document, state)?;
    state.task_context = Some(kind);

    let window = window_state(state, kind)?;
    let running = window.open && window.desktop == state.current_desktop;
    let pinned = state.pinned_tasks.contains(&kind);

    document.set_visible("task-menu-open", pinned && !running)?;
    document.set_visible("task-menu-pin", running && !pinned)?;
    document.set_visible("task-menu-unpin", pinned)?;
    document.set_visible("task-menu-separator", running)?;
    document.set_visible("task-menu-maximize", running && !(window.maximized && !window.minimized))?;
    document.set_visible("task-menu-minimize", running && window.maximized && !window.minimized)?;
    document.set_visible("task-menu-close", running)?;

    for candidate in [
        WindowKind::Browser,
        WindowKind::Files,
        WindowKind::Terminal,
        WindowKind::Code,
        WindowKind::Settings,
        WindowKind::Music,
        WindowKind::Generic,
    ] {
        if let Some(icon_id) = candidate.menu_icon_id() {
            document.set_visible(icon_id, candidate == kind && pinned && !running)?;
        }
    }

    let menu_w = 205.0;
    let menu_h = if running { 114.0 } else { 72.0 };
    let (view_w, view_h) = viewport_size(state);
    let (x, y) = match state.taskbar_position {
        TaskbarPosition::Bottom => (
            (pointer_x - menu_w / 2.0).clamp(4.0, view_w - menu_w - 4.0),
            (view_h - state.taskbar_height - menu_h - 4.0).max(4.0),
        ),
        TaskbarPosition::Top => (
            (pointer_x - menu_w / 2.0).clamp(4.0, view_w - menu_w - 4.0),
            state.taskbar_height + 4.0,
        ),
        TaskbarPosition::Left => (
            state.taskbar_height + 4.0,
            (pointer_y - menu_h / 2.0).clamp(4.0, view_h - menu_h - 4.0),
        ),
        TaskbarPosition::Right => (
            (view_w - state.taskbar_height - menu_w - 4.0).max(4.0),
            (pointer_y - menu_h / 2.0).clamp(4.0, view_h - menu_h - 4.0),
        ),
    };
    document.set_position_px("task-menu", x, y)?;
    document.set_visible("task-menu", true)
}

fn task_context_kind(state: &DemoState) -> Result<WindowKind, String> {
    state.task_context.ok_or_else(|| "task context menu has no task target".to_string())
}

fn task_menu_open(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let kind = task_context_kind(state)?;
    close_popovers(document, state)?;
    open_window(document, state, kind)
}

fn task_menu_pin(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let kind = task_context_kind(state)?;
    state.pinned_tasks.insert(kind);
    close_popovers(document, state)?;
    sync_taskbar_activity(document, state)?;
    show_toast(document, &format!("{} pinned to the virtual taskbar", task_label(kind)))
}

fn task_menu_unpin(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let kind = task_context_kind(state)?;
    state.pinned_tasks.remove(&kind);
    close_popovers(document, state)?;
    sync_taskbar_activity(document, state)?;
    show_toast(document, &format!("{} unpinned from the virtual taskbar", task_label(kind)))
}

fn task_menu_maximize(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let kind = task_context_kind(state)?;
    close_popovers(document, state)?;
    let window = window_state(state, kind)?;
    if window.open && (!window.maximized || window.minimized) {
        if window.minimized {
            let mut restored = window;
            restored.minimized = false;
            set_window_state(state, kind, restored);
        }
        let current = window_state(state, kind)?;
        if !current.maximized {
            toggle_maximize(document, state, kind)?;
        } else {
            focus_window(document, state, kind)?;
        }
    }
    Ok(())
}

fn task_menu_minimize(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let kind = task_context_kind(state)?;
    close_popovers(document, state)?;
    minimize_window(document, state, kind)
}

fn task_menu_close(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let kind = task_context_kind(state)?;
    close_popovers(document, state)?;
    close_window(document, state, kind)
}

fn task_label(kind: WindowKind) -> &'static str {
    match kind {
        WindowKind::Browser => "Firefox",
        WindowKind::Files => "Dolphin",
        WindowKind::Terminal => "Konsole",
        WindowKind::Code => "Code",
        WindowKind::Settings => "System Settings",
        WindowKind::Music => "Elisa",
        WindowKind::Generic => "Application",
    }
}

fn icon_label(kind: IconKind) -> &'static str {
    match kind {
        IconKind::Home => "Home",
        IconKind::Downloads => "Downloads",
        IconKind::Projects => "Projects",
        IconKind::Browser => "Firefox",
        IconKind::Trash => "Trash",
        IconKind::NewFolder => "New Folder",
    }
}

fn open_desktop_entry_menu(
    document: &mut RuntimeDocument,
    state: &mut DemoState,
    kind: IconKind,
) -> Result<(), String> {
    close_popovers(document, state)?;
    state.entry_context = Some(kind);

    let trash = kind == IconKind::Trash;
    document.set_visible("entry-menu-shortcut", !trash)?;
    document.set_visible("entry-menu-separator", !trash)?;
    document.set_visible("entry-menu-delete", !trash)?;
    document.set_visible("entry-menu-empty-trash", trash)?;

    let rect = state.icons.get(&kind).ok_or("missing desktop icon state")?.rect;
    let area = work_area(state);
    let menu_w = 205.0;
    let menu_h = if trash { 41.0 } else { 92.0 };
    let preferred_x = rect.x + rect.width + 4.0;
    let x = if preferred_x + menu_w <= area.x + area.width {
        preferred_x
    } else {
        (rect.x - menu_w - 4.0).max(area.x)
    };
    let y = rect.y.clamp(area.y, (area.y + area.height - menu_h).max(area.y));
    document.set_position_px("desktop-entry-menu", x, y)?;
    document.set_visible("desktop-entry-menu", true)
}

fn entry_context_kind(state: &DemoState) -> Result<IconKind, String> {
    state.entry_context.ok_or_else(|| "desktop entry context menu has no target".to_string())
}

fn desktop_entry_shortcut(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let kind = entry_context_kind(state)?;
    close_popovers(document, state)?;
    show_toast(document, &format!("Created virtual shortcut for {}", icon_label(kind)))
}

fn desktop_entry_delete(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let kind = entry_context_kind(state)?;
    if kind == IconKind::Trash {
        return Ok(());
    }
    if let Some(icon) = state.icons.get_mut(&kind) {
        icon.trashed = true;
    }
    if kind == IconKind::NewFolder {
        state.new_folder_visible = false;
    }
    state.selected_icons.remove(&kind);
    document.set_visible(kind.id(), false)?;
    sync_icon_selection(document, state)?;
    close_popovers(document, state)?;
    show_toast(document, &format!("{} moved to virtual Trash", icon_label(kind)))
}

fn empty_virtual_trash(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let count = all_icons()
        .into_iter()
        .filter(|kind| *kind != IconKind::Trash)
        .filter(|kind| state.icons.get(kind).map(|icon| icon.trashed).unwrap_or(false))
        .count();
    close_popovers(document, state)?;
    show_toast(document, &format!("Virtual Trash emptied ({count} item{})", if count == 1 { "" } else { "s" }))
}

fn open_sticky_menu(
    document: &mut RuntimeDocument,
    state: &mut DemoState,
    x: f32,
    y: f32,
) -> Result<(), String> {
    close_popovers(document, state)?;
    document.set_visible("sticky-menu-main", true)?;
    document.set_visible("sticky-menu-settings", false)?;
    let area = work_area(state);
    let menu_w = 205.0;
    let menu_h = 92.0;
    let x = x.clamp(area.x, (area.x + area.width - menu_w).max(area.x));
    let y = y.clamp(area.y, (area.y + area.height - menu_h).max(area.y));
    document.set_position_px("sticky-menu", x, y)?;
    document.set_visible("sticky-menu", true)
}

fn show_sticky_settings_menu(document: &mut RuntimeDocument) -> Result<(), String> {
    document.set_visible("sticky-menu-main", false)?;
    document.set_visible("sticky-menu-settings", true)
}

fn delete_sticky(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    state.sticky_visible = false;
    document.set_visible("sticky-note", false)?;
    close_popovers(document, state)?;
    show_toast(document, "Sticky note deleted from this virtual desktop")
}

fn sticky_backgrounds() -> [(&'static str, Color); 5] {
    [
        ("Yellow", Color::rgb(255, 229, 107)),
        ("Rose", Color::rgb(255, 198, 207)),
        ("Mint", Color::rgb(186, 236, 205)),
        ("Sky", Color::rgb(188, 220, 255)),
        ("Lavender", Color::rgb(218, 203, 255)),
    ]
}

fn sticky_foregrounds() -> [(&'static str, Color); 3] {
    [
        ("Dark", Color::rgb(23, 23, 23)),
        ("Graphite", Color::rgb(52, 55, 59)),
        ("Flame", Color::rgb(141, 28, 34)),
    ]
}

fn sticky_sizes() -> [f32; 5] {
    [11.0, 13.0, 15.0, 18.0, 22.0]
}

fn apply_sticky_style(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    let backgrounds = sticky_backgrounds();
    let foregrounds = sticky_foregrounds();
    let (bg_name, bg) = backgrounds[state.sticky_bg_index % backgrounds.len()];
    let (fg_name, fg) = foregrounds[state.sticky_fg_index % foregrounds.len()];
    document.set_color_variable("--sticky-bg", bg)?;
    document.set_color_variable("--sticky-fg", fg)?;
    document.set_text("sticky-bg-value", bg_name)?;
    document.set_text("sticky-fg-value", fg_name)?;
    document.set_text("sticky-size-value", format!("{}px", state.sticky_font_size.round() as i32))?;
    for index in 1..=8 {
        document.set_font_size_px(&format!("sticky-line-{index}"), state.sticky_font_size)?;
    }
    Ok(())
}

fn wrap_sticky_text(text: &str, width_px: f32, font_size: f32, max_lines: usize) -> Vec<String> {
    let max_chars = (width_px / (font_size * 0.56)).floor().max(8.0) as usize;
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if lines.len() >= max_lines { break; }
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let extra = if current.is_empty() { word.chars().count() } else { word.chars().count() + 1 };
            if !current.is_empty() && current.chars().count() + extra > max_chars {
                lines.push(current);
                current = String::new();
                if lines.len() >= max_lines { break; }
            }
            if !current.is_empty() { current.push(' '); }
            current.push_str(word);
        }
        if !current.is_empty() && lines.len() < max_lines { lines.push(current); }
    }
    lines.truncate(max_lines);
    lines
}

fn sync_sticky_text(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    let width = (state.sticky_rect.width - 24.0).max(60.0);
    let lines = wrap_sticky_text(&state.sticky_text, width, state.sticky_font_size, 8);
    for index in 0..8 {
        document.set_text(
            &format!("sticky-line-{}", index + 1),
            lines.get(index).cloned().unwrap_or_default(),
        )?;
    }
    Ok(())
}

fn cycle_sticky_background(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    state.sticky_bg_index = (state.sticky_bg_index + 1) % sticky_backgrounds().len();
    apply_sticky_style(document, state)
}

fn cycle_sticky_foreground(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    state.sticky_fg_index = (state.sticky_fg_index + 1) % sticky_foregrounds().len();
    apply_sticky_style(document, state)
}

fn cycle_sticky_size(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let sizes = sticky_sizes();
    let current = sizes.iter().position(|size| (*size - state.sticky_font_size).abs() < f32::EPSILON).unwrap_or(1);
    state.sticky_font_size = sizes[(current + 1) % sizes.len()];
    apply_sticky_style(document, state)?;
    sync_sticky_text(document, state)
}

fn open_desktop_icon(document: &mut RuntimeDocument, state: &mut DemoState, kind: IconKind) -> Result<(), String> {
    match kind {
        IconKind::Browser => open_window(document, state, WindowKind::Browser),
        IconKind::Home => open_files_window(document, state, "Home", "/home/juan/"),
        IconKind::Downloads => open_files_window(document, state, "Downloads", "/home/juan/Downloads/"),
        IconKind::Projects => open_files_window(document, state, "Projects", "/home/juan/Projects/"),
        IconKind::Trash => open_files_window(document, state, "Trash", "trash:/"),
        IconKind::NewFolder => open_files_window(document, state, "New Folder", "/home/juan/Desktop/New Folder/"),
    }
}

fn sync_icon_selection(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    for kind in all_icons() {
        // Selection is painted on the exact same 82x82 desktop-entry box as
        // hover. Runtime overrides intentionally win over :hover so a selected
        // entry does not stack 15% hover alpha on top of its 28% selected fill.
        if state.selected_icons.contains(&kind) && icon_is_visible(state, kind) {
            document.set_background_color(kind.id(), Color { a: alpha_percent(28), ..state.accent })?;
            document.set_border_color(kind.id(), Color { a: alpha_percent(65), ..state.accent })?;
        } else {
            document.clear_background_color(kind.id())?;
            document.clear_border_color(kind.id())?;
        }
    }
    Ok(())
}

fn create_demo_folder(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    state.new_folder_visible = true;
    if let Some(icon) = state.icons.get_mut(&IconKind::NewFolder) {
        icon.trashed = false;
    }
    document.set_visible("icon-new-folder", true)?;
    state.selected_icons.remove(&IconKind::NewFolder);
    sync_icon_selection(document, state)?;
    document.set_visible("desktop-menu", false)?;
    show_toast(document, "Created New Folder inside the virtual desktop")
}

fn show_settings_page(document: &mut RuntimeDocument, selected: &str) -> Result<(), String> {
    let mut selection_top = None;
    for (page, top) in SETTINGS_PAGES {
        document.set_visible(&format!("page-{page}"), page == selected)?;
        if page == selected {
            selection_top = Some(top);
        }
    }
    let top = selection_top.ok_or_else(|| format!("unknown settings page '{selected}'"))?;
    document.set_position_px("nav-selection", 8.0, top)?;
    Ok(())
}

fn sync_settings_controls(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    document.set_text("selection-opacity-value", format!("{}%", state.selection_opacity))?;
    document.set_text("snap-opacity-value", format!("{}%", state.snap_opacity))?;
    document.set_text("taskbar-color-value", taskbar_colors()[state.taskbar_color_index].0)?;
    document.set_text("taskbar-position-value", state.taskbar_position.label())?;
    document.set_text("taskbar-size-value", match state.taskbar_height.round() as i32 {
        34 => "Compact",
        56 => "Large",
        _ => "Default",
    })?;
    document.set_text("taskbar-opacity-value", format!("{}%", 100u8.saturating_sub(state.taskbar_opacity)))?;
    document.set_text("font-family-value", font_families()[state.font_family_index])?;
    document.set_text("font-size-value", if state.font_size_offset == 0 {
        "Default".to_string()
    } else {
        format!("{:+}px", state.font_size_offset)
    })?;
    document.set_text("icon-theme-value", icon_themes()[state.icon_theme_index])?;
    document.set_text("sticky-status", if state.sticky_enabled { "Enabled" } else { "Disabled" })?;
    for (track, knob, enabled) in [
        ("watermark-toggle", "watermark-toggle-knob", state.show_watermark),
        ("sticky-toggle", "sticky-toggle-knob", state.sticky_enabled),
        ("font-bold-toggle", "font-bold-toggle-knob", state.font_bold),
    ] {
        document.set_background_color(track, if enabled { state.accent } else { Color::rgb(85, 90, 95) })?;
        document.set_position_px(knob, if enabled { 20.0 } else { 2.0 }, 2.0)?;
    }
    for (id, index) in [("hotkey-start", 0usize), ("hotkey-left", 1), ("hotkey-right", 2), ("hotkey-up", 3), ("hotkey-down", 4)] {
        document.set_text(id, hotkey_values(index)[state.hotkey_variant[index]])?;
    }
    Ok(())
}

fn set_accent(document: &mut RuntimeDocument, state: &mut DemoState, color: Color) -> Result<(), String> {
    state.accent = color;
    apply_accent(document, state, color)?;
    sync_workspace_pager(document, state)?;
    sync_taskbar_activity(document, state)?;
    sync_icon_selection(document, state)?;
    sync_display_settings(document, state)?;
    sync_settings_controls(document, state)
}

fn apply_accent(document: &mut RuntimeDocument, state: &DemoState, color: Color) -> Result<(), String> {
    document.set_color_variable("--accent", color)?;
    document.set_color_variable("--accent-soft", Color { a: 56, ..color })?;
    document.set_color_variable("--accent-strong", Color { a: 221, ..color })?;
    document.set_color_variable("--desktop-hover", Color { a: alpha_percent(15), ..color })?;
    document.set_color_variable("--desktop-hover-border", Color { a: alpha_percent(30), ..color })?;
    document.set_color_variable("--desktop-selected", Color { a: alpha_percent(28), ..color })?;
    document.set_color_variable("--desktop-selected-border", Color { a: alpha_percent(65), ..color })?;
    document.set_color_variable("--selection-fill", Color { a: alpha_percent(state.selection_opacity), ..color })?;
    document.set_color_variable("--snap-fill", Color { a: alpha_percent(state.snap_opacity), ..color })?;
    Ok(())
}

fn alpha_percent(value: u8) -> u8 {
    ((u16::from(value.min(100)) * 255) / 100) as u8
}

fn adjust_overlay_opacity(document: &mut RuntimeDocument, state: &mut DemoState, selection: bool, delta: i8) -> Result<(), String> {
    if selection {
        state.selection_opacity = (state.selection_opacity as i16 + delta as i16).clamp(0, 60) as u8;
    } else {
        state.snap_opacity = (state.snap_opacity as i16 + delta as i16).clamp(0, 60) as u8;
    }
    apply_accent(document, state, state.accent)?;
    sync_settings_controls(document, state)
}

fn font_families() -> [&'static str; 4] {
    ["IBM Plex Sans", "Noto Sans", "DejaVu Sans", "sans-serif"]
}

fn apply_font_settings(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    document.set_ui_font(
        font_families()[state.font_family_index],
        state.font_size_offset as f32,
        state.font_bold,
    )
}

fn icon_themes() -> [&'static str; 4] {
    ["FlameWM Breeze (Built-in)", "Breeze Dark", "Breeze", "hicolor"]
}

fn cycle_icon_theme(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    state.icon_theme_index = (state.icon_theme_index + 1) % icon_themes().len();
    sync_settings_controls(document, state)?;
    show_toast(document, &format!("Icon theme: {} (virtual preview)", icon_themes()[state.icon_theme_index]))
}

fn hotkey_values(index: usize) -> &'static [&'static str] {
    match index {
        0 => &["Super", "Ctrl + Space", "Not assigned"],
        1 => &["Ctrl + Super + Left", "Alt + Left", "Not assigned"],
        2 => &["Ctrl + Super + Right", "Alt + Right", "Not assigned"],
        3 => &["Ctrl + Super + Up", "Alt + Up", "Not assigned"],
        _ => &["Ctrl + Super + Down", "Alt + Down", "Not assigned"],
    }
}

fn taskbar_colors() -> [(&'static str, Color); 4] {
    [
        ("Pitch black", Color::rgb(0, 0, 0)),
        ("Graphite", Color::rgb(24, 26, 28)),
        ("Charcoal", Color::rgb(35, 38, 41)),
        ("Flame dark", Color::rgb(28, 20, 21)),
    ]
}

fn cycle_hotkey(document: &mut RuntimeDocument, state: &mut DemoState, index: usize) -> Result<(), String> {
    state.hotkey_variant[index] = (state.hotkey_variant[index] + 1) % hotkey_values(index).len();
    sync_settings_controls(document, state)
}

fn hotkey_action_enabled(state: &DemoState, index: usize, action: &str) -> bool {
    let variant = action.rsplit('.').next().and_then(|value| value.parse::<usize>().ok());
    variant == Some(state.hotkey_variant[index]) && state.hotkey_variant[index] < 2
}

fn display_slot(id: DisplayId) -> usize {
    match id { DisplayId::Edp => 0, DisplayId::Hdmi => 1 }
}

fn display_resolutions(id: DisplayId) -> &'static [&'static str] {
    match id {
        DisplayId::Edp => &["1350 × 641", "1920 × 1080", "1600 × 900", "1366 × 768"],
        DisplayId::Hdmi => &["1920 × 1080", "2560 × 1440", "1600 × 900", "1280 × 720"],
    }
}

fn display_scales() -> [u16; 5] {
    [100, 125, 150, 175, 200]
}

fn sync_display_settings(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    let accent = state.accent;
    let inactive = Color::rgb(89, 96, 104);
    document.set_border_color("monitor-edp", if state.selected_display == DisplayId::Edp { accent } else { inactive })?;
    document.set_border_color("monitor-hdmi", if state.selected_display == DisplayId::Hdmi { accent } else { inactive })?;
    for id in [DisplayId::Edp, DisplayId::Hdmi] {
        let slot = display_slot(id);
        let resolution = display_resolutions(id)[state.displays[slot].resolution_index];
        let scale = display_scales()[state.displays[slot].scale_index];
        document.set_text(match id { DisplayId::Edp => "monitor-edp-summary", DisplayId::Hdmi => "monitor-hdmi-summary" }, format!("{resolution} · {scale}%"))?;
    }
    let slot = display_slot(state.selected_display);
    let resolution = display_resolutions(state.selected_display)[state.displays[slot].resolution_index];
    let scale = display_scales()[state.displays[slot].scale_index];
    match state.selected_display {
        DisplayId::Edp => {
            document.set_text("display-selected-title", "Selected: eDP-1")?;
            document.set_text("display-selected-note", "Built-in display")?;
        }
        DisplayId::Hdmi => {
            document.set_text("display-selected-title", "Selected: HDMI-1")?;
            document.set_text("display-selected-note", "External display")?;
        }
    }
    document.set_text("display-resolution-value", resolution)?;
    document.set_text("display-scale-value", format!("{scale}%"))?;
    document.set_text("display-selected-note", match state.selected_display {
        DisplayId::Edp => format!("Built-in display · virtual desktop {}×{} logical px", (VIEW_W / current_ui_scale(state)).round() as i32, (VIEW_H / current_ui_scale(state)).round() as i32),
        DisplayId::Hdmi => format!("External display · virtual desktop {}×{} logical px", (VIEW_W / current_ui_scale(state)).round() as i32, (VIEW_H / current_ui_scale(state)).round() as i32),
    })?;
    Ok(())
}

fn apply_sticky(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    document.set_position_px("sticky-note", state.sticky_rect.x, state.sticky_rect.y)?;
    document.set_size_px("sticky-note", state.sticky_rect.width, state.sticky_rect.height)
}

fn toggle_start(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let will_open = !state.start_open;
    document.set_visible("desktop-menu", false)?;
    close_context_menus(document, state)?;
    close_tray_popups(document, state)?;
    state.start_open = will_open;
    document.set_visible("start-menu", will_open)?;
    if will_open {
        state.start_search.clear();
        state.start_search_focused = true;
        hide_start_submenu(document, state)?;
        sync_start_search(document, state)?;
    } else {
        state.start_search_focused = false;
        state.start_search.clear();
        hide_start_submenu(document, state)?;
        sync_start_search(document, state)?;
    }
    Ok(())
}

fn show_start_group(document: &mut RuntimeDocument, state: &mut DemoState, selected: &str) -> Result<(), String> {
    if !START_GROUPS.contains(&selected) {
        return Err(format!("unknown start category '{selected}'"));
    }
    state.start_search.clear();
    state.start_search_focused = false;
    state.active_start_group = Some(selected.to_string());
    document.set_visible("start-normal-categories", true)?;
    document.set_visible("start-search-results", false)?;
    document.set_visible("start-search-empty", false)?;
    for group in START_GROUPS {
        document.set_visible(&format!("start-group-{group}"), group == selected)?;
        if group == selected {
            document.set_background_color(&format!("start-category-{group}"), Color { a: 56, ..state.accent })?;
        } else {
            document.clear_background_color(&format!("start-category-{group}"))?;
        }
    }
    let rows = match selected {
        "development" | "system" | "utilities" => 3.0,
        "games" | "internet" => 1.0,
        "graphics" | "multimedia" => 2.0,
        "power" => 4.0,
        _ => 1.0,
    };
    document.set_size_px("start-submenu", 260.0, rows * 36.0 + 10.0)?;
    document.set_visible("start-submenu", true)?;
    state.start_submenu_open = true;
    sync_start_search(document, state)?;
    Ok(())
}

fn hide_start_submenu(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    document.set_visible("start-submenu", false)?;
    for group in START_GROUPS {
        document.set_visible(&format!("start-group-{group}"), false)?;
        document.clear_background_color(&format!("start-category-{group}"))?;
    }
    state.start_submenu_open = false;
    state.active_start_group = None;
    Ok(())
}

fn close_start(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    document.set_visible("start-menu", false)?;
    state.start_open = false;
    state.start_search_focused = false;
    state.start_search.clear();
    hide_start_submenu(document, state)?;
    sync_start_search(document, state)
}

fn focus_start_search(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    state.start_search_focused = true;
    hide_start_submenu(document, state)?;
    sync_start_search(document, state)
}

fn sync_start_search(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    let query = state.start_search.trim().to_ascii_lowercase();
    document.set_text("start-search-text", if state.start_search.is_empty() { "Search…".to_string() } else { state.start_search.clone() })?;
    document.set_border_color("start-search-field", if state.start_search_focused { state.accent } else { Color::rgb(55, 58, 61) })?;
    let searching = !query.is_empty();
    document.set_visible("start-normal-categories", !searching)?;
    document.set_visible("start-search-results", searching)?;
    let mut matches = 0usize;
    for (id, name, keywords, _) in START_SEARCH_ITEMS {
        let visible = searching && (name.to_ascii_lowercase().contains(&query) || keywords.contains(&query));
        document.set_visible(id, visible)?;
        if visible { matches += 1; }
    }
    document.set_visible("start-search-empty", searching && matches == 0)?;
    Ok(())
}

fn first_search_action(state: &DemoState) -> Option<&'static str> {
    let query = state.start_search.trim().to_ascii_lowercase();
    if query.is_empty() { return None; }
    START_SEARCH_ITEMS.into_iter()
        .find(|(_, name, keywords, _)| name.to_ascii_lowercase().contains(&query) || keywords.contains(&query))
        .map(|(_, _, _, action)| action)
}

fn activate_start_action(document: &mut RuntimeDocument, state: &mut DemoState, action: &str) -> Result<(), String> {
    close_start(document, state)?;
    match action {
        "start.web" => open_window(document, state, WindowKind::Browser),
        "start.files" => open_files_window(document, state, "Home", "/home/juan/"),
        "start.terminal" => open_window(document, state, WindowKind::Terminal),
        "start.code" => open_window(document, state, WindowKind::Code),
        "start.settings" => open_window(document, state, WindowKind::Settings),
        "start.music" => open_window(document, state, WindowKind::Music),
        "start.kate" => open_generic(document, state, "Kate", "Text editor activity simulated entirely inside RustWebRender."),
        "start.kwrite" => open_generic(document, state, "KWrite", "Lightweight editor activity simulated inside the virtual desktop."),
        "start.ark" => open_generic(document, state, "Ark", "Archive manager activity from the Utilities category."),
        "start.okular" => open_generic(document, state, "Okular", "Document viewer activity from the Utilities category."),
        _ => Ok(()),
    }
}

fn handle_keyboard_input(event: &ActionEvent, document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    let Some(text) = event.text.as_deref() else { return Ok(()); };
    if state.start_open && state.start_search_focused {
        match text {
            "\u{1b}" => {
                if state.start_search.is_empty() { return close_start(document, state); }
                state.start_search.clear();
            }
            "\u{8}" => { state.start_search.pop(); }
            "\n" => {
                if let Some(action) = first_search_action(state) {
                    return activate_start_action(document, state, action);
                }
            }
            _ => {
                for ch in text.chars() {
                    if !ch.is_control() && state.start_search.chars().count() < 48 { state.start_search.push(ch); }
                }
            }
        }
        if !state.start_search.is_empty() { hide_start_submenu(document, state)?; }
        return sync_start_search(document, state);
    }
    if state.sticky_visible && state.sticky_editing {
        match text {
            "\u{1b}" => state.sticky_editing = false,
            "\u{8}" => { state.sticky_text.pop(); }
            "\n" => { if state.sticky_text.chars().count() < 500 { state.sticky_text.push('\n'); } }
            _ => {
                for ch in text.chars() {
                    if !ch.is_control() && state.sticky_text.chars().count() < 500 { state.sticky_text.push(ch); }
                }
            }
        }
        return sync_sticky_text(document, state);
    }
    Ok(())
}

fn close_tray_popups(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    for popup in [PopupKind::Media, PopupKind::Volume, PopupKind::Network, PopupKind::Clock] {
        document.set_visible(popup.id(), false)?;
    }
    state.popup = None;
    Ok(())
}

fn close_context_menus(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    document.set_visible("desktop-entry-menu", false)?;
    document.set_visible("task-menu", false)?;
    document.set_visible("sticky-menu", false)?;
    document.set_visible("sticky-menu-main", true)?;
    document.set_visible("sticky-menu-settings", false)?;
    state.task_context = None;
    state.entry_context = None;
    Ok(())
}

fn close_popovers(document: &mut RuntimeDocument, state: &mut DemoState) -> Result<(), String> {
    document.set_visible("desktop-menu", false)?;
    close_context_menus(document, state)?;
    close_tray_popups(document, state)?;
    close_start(document, state)
}

fn toggle_popup(document: &mut RuntimeDocument, state: &mut DemoState, target: PopupKind) -> Result<(), String> {
    let will_open = state.popup != Some(target);
    document.set_visible("desktop-menu", false)?;
    close_context_menus(document, state)?;
    close_start(document, state)?;
    close_tray_popups(document, state)?;
    if will_open {
        document.set_visible(target.id(), true)?;
        state.popup = Some(target);
    }
    Ok(())
}

fn sync_volume(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    let muted = state.muted || state.volume == 0;
    document.set_text("volume-value", format!("{}%", if muted { 0 } else { state.volume }))?;
    document.set_visible("tray-volume-icon", !muted)?;
    document.set_visible("tray-volume-muted-icon", muted)?;
    document.set_visible("popup-volume-icon", !muted)?;
    document.set_visible("popup-muted-icon", muted)?;
    Ok(())
}

fn sync_media_icons(document: &mut RuntimeDocument, state: &DemoState) -> Result<(), String> {
    document.set_visible("tray-play-icon", state.media_playing)?;
    document.set_visible("tray-pause-icon", !state.media_playing)?;
    document.set_visible("media-play-icon", !state.media_playing)?;
    document.set_visible("media-pause-icon", state.media_playing)?;
    Ok(())
}

fn show_toast(document: &mut RuntimeDocument, text: &str) -> Result<(), String> {
    document.set_text("toast", text)?;
    document.set_visible("toast", true)
}

fn window_from_action(action: &str) -> Option<WindowKind> {
    if let Some(rest) = action.strip_prefix("window.") {
        return window_from_prefix(rest.split('.').next().unwrap_or(""));
    }
    window_from_prefix(action.split('.').next().unwrap_or(""))
}

fn window_from_prefix(prefix: &str) -> Option<WindowKind> {
    match prefix {
        "browser" => Some(WindowKind::Browser),
        "files" => Some(WindowKind::Files),
        "terminal" => Some(WindowKind::Terminal),
        "code" => Some(WindowKind::Code),
        "settings" => Some(WindowKind::Settings),
        "music" => Some(WindowKind::Music),
        "generic" => Some(WindowKind::Generic),
        _ => None,
    }
}
