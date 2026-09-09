//! Async projection: labels/visibility immediate, icons cached-or-placeholder.
//! Never resolves icons on the caller thread. Schedules worker requests only.

use std::collections::HashMap;
use std::sync::OnceLock;

use flamewm_api::session::SessionCapabilities;
use flamewm_shell_core::StartModel;
use flamewm_ui_x11::{RuntimeImage, UiColor, UiDocumentAccess};

use crate::icon_loader::{IconLoader, IconTarget};
use crate::start::StartCategory;
use crate::taskbar::workspaces::{
    project as project_workspaces, slot_position, visible_page, PAGE_SIZE,
};
use crate::ShellSnapshot;

pub const TASK_SLOT_COUNT: usize = 8;
pub const START_APP_SLOT_COUNT: usize = 8;

fn icon_cache() -> &'static std::sync::Mutex<HashMap<(String, u32, u32), RuntimeImage>> {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<(String, u32, u32), RuntimeImage>>> =
        OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

fn redundant_hover_counter() -> &'static flamewm_profiler::CounterPoint {
    static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
    C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.start.redundant_hover"))
}

fn request_counter() -> &'static flamewm_profiler::CounterPoint {
    static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
    C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.icon.request"))
}

fn result_counter() -> &'static flamewm_profiler::CounterPoint {
    static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
    C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.icon.result"))
}

fn stale_counter() -> &'static flamewm_profiler::CounterPoint {
    static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
    C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.icon.stale_drop"))
}

fn queue_full_counter() -> &'static flamewm_profiler::CounterPoint {
    static C: OnceLock<flamewm_profiler::CounterPoint> = OnceLock::new();
    C.get_or_init(|| flamewm_profiler::CounterPoint::new("shell.icon.queue_full"))
}

/// Last projected start view; hover events that change nothing are counted.
#[derive(Debug, Default)]
pub struct StartViewNote {
    last_category: Option<StartCategory>,
    last_query: Option<String>,
}

impl StartViewNote {
    /// Returns true when the view semantically changed and needs reprojection.
    pub fn note_start_view(&mut self, category: StartCategory, query: &str) -> bool {
        if self.last_category == Some(category) && self.last_query.as_deref() == Some(query) {
            redundant_hover_counter().increment();
            return false;
        }
        self.last_category = Some(category);
        self.last_query = Some(query.to_owned());
        true
    }
}

fn cached_image(name: &str, w: u32, h: u32) -> Option<RuntimeImage> {
    icon_cache()
        .lock()
        .ok()?
        .get(&(name.to_owned(), w, h))
        .cloned()
}

pub fn note_icon_result(name: &str, w: u32, h: u32, image: RuntimeImage) {
    if let Ok(mut cache) = icon_cache().lock() {
        cache.insert((name.to_owned(), w, h), image);
    }
    result_counter().increment();
}

pub fn note_loader_stats(loader: &IconLoader) {
    if loader.queue_full_drops > 0 {
        queue_full_counter().increment_by(loader.queue_full_drops);
    }
    if loader.stale_drops > 0 {
        stale_counter().increment_by(loader.stale_drops);
    }
}

fn schedule(
    loader: &mut Option<&mut IconLoader>,
    target: IconTarget,
    name: &str,
    w: u32,
    h: u32,
) -> Option<RuntimeImage> {
    if let Some(hit) = cached_image(name, w, h) {
        return Some(hit);
    }
    if let Some(loader) = loader.as_deref_mut() {
        loader.request(target, name, w, h);
        request_counter().increment();
    }
    None
}

/// Panel projection without any synchronous icon resolve: task labels,
/// visibility, backgrounds, workspaces, clock, and tray visibility project
/// immediately; icons apply only when cached, otherwise a request is queued.
pub fn project_panel_async(
    document: &mut impl UiDocumentAccess,
    snapshot: &ShellSnapshot,
    mut loader: Option<&mut IconLoader>,
) -> Result<(), String> {
    let states = flamewm_shell_core::task_visual_states(
        &snapshot.panels,
        &snapshot.windows,
        &snapshot.applications,
    );
    let status = crate::taskbar::status::project(&snapshot.system).views;
    let workspaces = project_workspaces(snapshot.workspaces.as_ref());
    for index in 0..TASK_SLOT_COUNT {
        let slot_id = format!("task-slot-{}", index + 1);
        let icon_id = format!("task-slot-{}-icon", index + 1);
        let label_id = format!("task-slot-{}-label", index + 1);
        let line_id = format!("task-slot-{}-line", index + 1);
        let state = states.get(index);
        if let Some(state) = state {
            let label = snapshot
                .applications
                .iter()
                .find(|application| application.id == state.app_id)
                .map(|application| application.name.as_str())
                .filter(|name| !name.is_empty())
                .unwrap_or(state.app_id.as_str());
            document.text(&label_id, label.to_owned())?;
            if let Some(image) = schedule(
                &mut loader.as_deref_mut(),
                IconTarget::TaskSlot(index as u8),
                &state.icon_name,
                25,
                25,
            ) {
                document.image_rgb8(&icon_id, image)?;
            }
            document.visible(&line_id, state.running)?;
            if state.focused {
                document.background(
                    &slot_id,
                    UiColor {
                        r: 239,
                        g: 64,
                        b: 72,
                        a: 56,
                    },
                )?;
                document.background(
                    &line_id,
                    UiColor {
                        r: 239,
                        g: 64,
                        b: 72,
                        a: 255,
                    },
                )?;
            } else if state.minimized {
                document.background(
                    &slot_id,
                    UiColor {
                        r: 36,
                        g: 39,
                        b: 42,
                        a: 255,
                    },
                )?;
                document.background(
                    &line_id,
                    UiColor {
                        r: 135,
                        g: 145,
                        b: 155,
                        a: 255,
                    },
                )?;
            } else {
                document.background_clear(&slot_id)?;
                document.background(
                    &line_id,
                    UiColor {
                        r: 135,
                        g: 145,
                        b: 155,
                        a: 255,
                    },
                )?;
            }
        } else {
            document.text(&label_id, String::new())?;
            document.visible(&line_id, false)?;
            document.background_clear(&slot_id)?;
        }
        document.visible(&slot_id, state.is_some())?;
    }
    for index in 0..PAGE_SIZE {
        document.visible(&format!("workspace-{}", index + 1), false)?;
    }
    let page: Vec<&crate::taskbar::workspaces::WorkspaceView> = visible_page(&workspaces);
    for workspace in page {
        let Some((row, column)) = slot_position(workspace.index, &workspaces) else {
            continue;
        };
        let slot = row * 2 + column;
        if slot >= PAGE_SIZE {
            continue;
        }
        let button_id = format!("workspace-{}", slot + 1);
        document.text(&button_id, workspace.label.clone())?;
        document.visible(&button_id, true)?;
        if workspace.active {
            document.background(
                &button_id,
                UiColor {
                    r: 239,
                    g: 64,
                    b: 72,
                    a: 56,
                },
            )?;
        } else {
            document.background(
                &button_id,
                UiColor {
                    r: 36,
                    g: 39,
                    b: 42,
                    a: 255,
                },
            )?;
        }
    }
    crate::projection::project_clock(document)?;
    document.visible(
        "tray-media",
        status.media.visible
            && matches!(
                snapshot.system.media.playback,
                flamewm_api::system::PlaybackState::Playing
                    | flamewm_api::system::PlaybackState::Paused
            ),
    )?;
    document.visible(
        "tray-play-icon",
        matches!(
            status.media.play_pause_icon,
            Some(flamewm_ui_core::IconRole::MediaPlay)
        ),
    )?;
    document.visible(
        "tray-pause-icon",
        matches!(
            status.media.play_pause_icon,
            Some(flamewm_ui_core::IconRole::MediaPause)
        ),
    )?;
    document.visible("tray-volume", status.audio.visible)?;
    document.visible("tray-volume-icon", !status.audio.muted)?;
    document.visible("tray-volume-muted-icon", status.audio.muted)?;
    document.visible("tray-network", status.network.visible)?;
    Ok(())
}

/// Start submenu projection without synchronous resolve: labels and
/// visibility immediate; icons cached-or-placeholder with queued requests.
pub fn project_start_submenu_async(
    document: &mut impl UiDocumentAccess,
    model: &StartModel,
    category: StartCategory,
    query: &str,
    session: &SessionCapabilities,
    mut loader: Option<&mut IconLoader>,
) -> Result<(), String> {
    // Canonical indexed path: borrowed query view from the precomputed
    // index, then the single presentation-bucket filter. No clone, no
    // set_query-on-temporary. (Shell StartCategory is the presentation
    // seam; core filter_view takes the core category type.)
    let needle = query.trim();
    let base: Vec<&flamewm_api::applications::DesktopApplication> = if needle.is_empty() {
        model.results_view()
    } else {
        model.query_view(query)
    };
    let applications: Vec<&flamewm_api::applications::DesktopApplication> =
        if category == StartCategory::All {
            base
        } else if category == StartCategory::Power {
            Vec::new()
        } else {
            base.into_iter()
                .filter(|application| application_matches(application, category))
                .collect()
        };
    document.visible("start-applications", !applications.is_empty())?;
    let (first, last) = flamewm_shell_core::status::visible_window(applications.len(), 36, 320, 0);
    for index in 0..START_APP_SLOT_COUNT {
        let slot = index + 1;
        let button_id = format!("start-app-slot-{slot}");
        let label_id = format!("start-app-slot-{slot}-label");
        let icon_id = format!("start-app-slot-{slot}-icon");
        if let Some(application) = applications
            .iter()
            .enumerate()
            .filter(|(position, _)| *position >= first && *position < last)
            .map(|(_, application)| application)
            .nth(index)
        {
            document.text(&label_id, application.name.clone())?;
            document.visible(&button_id, true)?;
            document.visible(&icon_id, true)?;
            if let Some(image) = schedule(
                &mut loader.as_deref_mut(),
                IconTarget::StartSlot(index as u8),
                &application.icon_name,
                20,
                20,
            ) {
                document.image_rgb8(&icon_id, image)?;
            }
        } else {
            document.text(&label_id, String::new())?;
            document.visible(&button_id, false)?;
            document.visible(&icon_id, false)?;
        }
    }
    let show_session = query.trim().is_empty()
        && category == StartCategory::Power
        && (session.lock
            || session.logout
            || session.suspend
            || session.reboot
            || session.shutdown);
    document.visible("start-group-power", show_session)?;
    document.visible("start-session-lock", show_session && session.lock)?;
    document.visible("start-session-logout", show_session && session.logout)?;
    document.visible("start-session-suspend", show_session && session.suspend)?;
    document.visible("start-session-reboot", show_session && session.reboot)?;
    document.visible("start-session-shutdown", show_session && session.shutdown)?;
    Ok(())
}

fn application_matches(
    application: &flamewm_api::applications::DesktopApplication,
    category: StartCategory,
) -> bool {
    if category == StartCategory::All {
        return true;
    }
    let low = application
        .categories
        .iter()
        .find_map(|value| match value.to_ascii_lowercase().as_str() {
            "accessories" | "utility" => Some("utilities"),
            "development" => Some("development"),
            "education" => Some("utilities"),
            "game" | "games" => Some("games"),
            "graphics" => Some("graphics"),
            "network" | "internet" => Some("internet"),
            "audiovideo" | "audio" | "video" | "multimedia" => Some("multimedia"),
            "office" => Some("utilities"),
            "science" => Some("utilities"),
            "settings" | "system" => Some("system"),
            _ => None,
        })
        .unwrap_or("utilities");
    match category {
        StartCategory::Development => low == "development",
        StartCategory::Games => low == "games",
        StartCategory::Graphics => low == "graphics",
        StartCategory::Internet => low == "internet",
        StartCategory::Multimedia => low == "multimedia",
        StartCategory::System => low == "system",
        StartCategory::Utilities => low == "utilities",
        StartCategory::Power | StartCategory::All => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[derive(Default)]
    struct FakeDoc {
        texts: HashMap<String, String>,
        images: Vec<String>,
        visible: HashMap<String, bool>,
    }

    impl UiDocumentAccess for FakeDoc {
        fn text(&mut self, id: &str, text: String) -> Result<(), String> {
            self.texts.insert(id.to_owned(), text);
            Ok(())
        }
        fn image_rgb8(&mut self, id: &str, _image: RuntimeImage) -> Result<(), String> {
            self.images.push(id.to_owned());
            Ok(())
        }
        fn image_rgba8(&mut self, id: &str, _image: RuntimeImage) -> Result<(), String> {
            self.images.push(id.to_owned());
            Ok(())
        }
        fn visible(&mut self, id: &str, v: bool) -> Result<(), String> {
            self.visible.insert(id.to_owned(), v);
            Ok(())
        }
        fn toggle(&mut self, _: &str, _: &str, _: bool) -> Result<(), String> {
            Ok(())
        }
        fn position(&mut self, _: &str, _: f32, _: f32) -> Result<(), String> {
            Ok(())
        }
        fn size(&mut self, _: &str, _: f32, _: f32) -> Result<(), String> {
            Ok(())
        }
        fn background(&mut self, _: &str, _: UiColor) -> Result<(), String> {
            Ok(())
        }
        fn border(&mut self, _: &str, _: UiColor) -> Result<(), String> {
            Ok(())
        }
        fn background_clear(&mut self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn foreground(&mut self, _: &str, _: UiColor) -> Result<(), String> {
            Ok(())
        }
        fn foreground_clear(&mut self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn border_clear(&mut self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn layer(&mut self, _: &str, _: flamewm_ui_core::style::UiLayer) -> Result<(), String> {
            Ok(())
        }
        fn overflow(
            &mut self,
            _: &str,
            _: flamewm_ui_x11::Overflow,
            _: flamewm_ui_x11::Overflow,
        ) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn async_panel_projects_labels_without_sync_resolve() {
        let snapshot = ShellSnapshot::default();
        let mut doc = FakeDoc::default();
        project_panel_async(&mut doc, &snapshot, None).expect("projects");
        assert!(doc.images.is_empty(), "no icons without cache/loader");
        for index in 0..8 {
            let id = format!("task-slot-{}", index + 1);
            assert!(doc.visible.contains_key(&id), "slot {id} projected");
        }
    }

    #[test]
    fn async_start_submenu_hides_empty_slots() {
        let model = StartModel::new(Vec::new());
        let session = SessionCapabilities::default();
        let mut doc = FakeDoc::default();
        project_start_submenu_async(&mut doc, &model, StartCategory::All, "", &session, None)
            .expect("projects");
        assert_eq!(doc.visible.get("start-app-slot-1"), Some(&false));
    }

    #[test]
    fn start_view_note_gates_redundant_hover() {
        let mut note = StartViewNote::default();
        assert!(note.note_start_view(StartCategory::All, ""));
        assert!(!note.note_start_view(StartCategory::All, ""));
        assert!(note.note_start_view(StartCategory::Games, ""));
    }

    #[test]
    fn no_sync_resolve_calls_in_async_path() {
        let src = include_str!("async_projection.rs");
        // Import of the loader handle is fine; synchronous resolve calls are not.
        let probe = ["prepare", "_", "name", "("].concat();
        let env = ["from", "_", "environment"].concat();
        assert!(!src.contains(&probe), "async path must not resolve sync");
        assert!(
            !src.contains(&env),
            "async path must not construct resolver"
        );
    }

    #[test]
    fn popup_roles_are_distinct() {
        let roles = [
            flamewm_ui_x11::SurfaceRole::PopupMenu,
            flamewm_ui_x11::SurfaceRole::DropdownMenu,
        ];
        assert_ne!(roles[0], roles[1]);
        let _ = flamewm_ui_x11::SurfaceRole::Dock;
    }

    #[test]
    fn eight_slots_project_distinct_ids() {
        let mut ids = std::collections::HashSet::new();
        for index in 0..START_APP_SLOT_COUNT {
            ids.insert(format!("start-app-slot-{}", index + 1));
        }
        assert_eq!(ids.len(), 8);
    }
}
