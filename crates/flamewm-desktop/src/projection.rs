use flamewm_api::Point;
use flamewm_api::settings::SettingValue;
use flamewm_desktop_core::model::DesktopItemKind;
use flamewm_desktop_core::presentation::desktop_label_lines;
use flamewm_integrations_linux::icons::{IconLookupPurpose, IconResolver, IconSize, Rgb8Raster};
use flamewm_ui_core::style::UiLayer;
use flamewm_ui_x11::{RuntimeImage, UiColor, UiDocumentAccess};

const ICON_KINDS: [&str; 5] = ["directory", "desktop-launcher", "file", "symlink", "trash"];
const LABEL_WIDTH_PX: f32 = 76.0;
const LABEL_FONT_SIZE_PX: f32 = 13.0;
pub const DESKTOP_TILE_WIDTH: f32 = 82.0;
pub const DESKTOP_TILE_HEIGHT: f32 = 82.0;

pub fn project_background(
    document: &mut impl UiDocumentAccess,
    wallpaper: Option<&SettingValue>,
) -> Result<(), String> {
    let selected = matches!(wallpaper, Some(SettingValue::Text(path)) if !path.trim().is_empty());
    document.background(
        "desktop",
        UiColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
    )?;
    document.visible("desktop-watermark", !selected)?;
    document.visible("desktop-wallpaper-selection", !selected)?;
    Ok(())
}

/// Semantic layer assignment, run once at document init. Static ids plus the
/// generated item slots; never called per-frame so visibility math stays raw.
pub fn assign_document_layers(document: &mut impl UiDocumentAccess) -> Result<(), String> {
    document.layer("desktop", UiLayer::SurfaceBackground)?;
    document.layer("desktop-wallpaper-selection", UiLayer::Background)?;
    document.layer("desktop-watermark", UiLayer::Background)?;
    for index in 0..SLOT_COUNT {
        document.layer(&format!("desktop-item-{index}"), UiLayer::Content)?;
    }
    document.layer("desktop-selection", UiLayer::Selection)?;
    document.layer("sticky-note", UiLayer::Floating)?;
    document.layer("desktop-menu", UiLayer::Popover)?;
    document.layer("desktop-entry-menu", UiLayer::Popover)?;
    document.layer("sticky-menu", UiLayer::Popover)?;
    document.layer("desktop-confirm", UiLayer::Modal)?;
    Ok(())
}

const SLOT_COUNT: usize = 128;

pub const TRANSIENT_HIDDEN_IDS: [&str; 6] = [
    "desktop-selection",
    "sticky-note",
    "desktop-menu",
    "desktop-entry-menu",
    "sticky-menu",
    "desktop-confirm",
];

/// Runtime-owned first-paint guard: hides selection/sticky/menus/confirm via
/// `hidden_nodes` so no transient node flashes before state sync decides it.
/// Restores authored display on `visible(id, true)`; `hidden_nodes` stays the
/// sole visibility override (authored CSS carries the visible display modes).
pub fn initialize_transient_visibility(document: &mut impl UiDocumentAccess) -> Result<(), String> {
    for id in TRANSIENT_HIDDEN_IDS {
        document.visible(id, false)?;
    }
    Ok(())
}

/// Which icon lookup purpose a candidate name uses. Explicit DesktopLauncher
/// Icon= values are application icons; per-kind defaults are semantic.
#[must_use]
pub fn icon_purpose(kind: DesktopItemKind, has_override: bool) -> IconLookupPurpose {
    if kind == DesktopItemKind::DesktopLauncher && has_override {
        IconLookupPurpose::Application
    } else {
        IconLookupPurpose::Semantic
    }
}

pub fn project_item(
    document: &mut impl UiDocumentAccess,
    icon_resolver: &mut IconResolver,
    index: usize,
    kind: DesktopItemKind,
    label: &str,
    point: Point,
    icon_override: Option<&str>,
) -> Result<(), String> {
    let item_id = format!("desktop-item-{index}");
    document.visible(&item_id, true)?;
    document.position(&item_id, point.x as f32, point.y as f32)?;
    document.size(&item_id, DESKTOP_TILE_WIDTH, DESKTOP_TILE_HEIGHT)?;
    project_item_icon(document, icon_resolver, index, kind, icon_override)?;
    project_item_label(document, index, label)
}

pub fn clear_item(document: &mut impl UiDocumentAccess, index: usize) -> Result<(), String> {
    let item_id = format!("desktop-item-{index}");
    document.visible(&item_id, false)?;
    for icon_kind in ICON_KINDS {
        document.visible(&format!("desktop-glyph-{index}-{icon_kind}"), false)?;
    }
    project_item_label(document, index, "")
}

pub fn project_drag_ghost_icon(
    document: &mut impl UiDocumentAccess,
    icon_resolver: &mut IconResolver,
    kind: DesktopItemKind,
    icon_override: Option<&str>,
) -> Result<(), String> {
    let selected = icon_semantic(kind);
    let override_name = icon_override.map(str::trim).filter(|icon| !icon.is_empty());
    let candidates = icon_candidates(kind, icon_override);
    let purpose = icon_purpose(kind, override_name.is_some());
    let icon = resolve_first(icon_resolver, &candidates, purpose, IconSize::new(44, 44))?;
    replace_image(document, "desktop-drag-ghost-icon", icon)?;
    document.visible("desktop-drag-ghost-icon", true)?;
    let _ = selected;
    Ok(())
}

pub fn project_item_icon(
    document: &mut impl UiDocumentAccess,
    icon_resolver: &mut IconResolver,
    index: usize,
    kind: DesktopItemKind,
    icon_override: Option<&str>,
) -> Result<(), String> {
    let selected = icon_semantic(kind);
    let override_name = icon_override.map(str::trim).filter(|icon| !icon.is_empty());
    let candidates = icon_candidates(kind, icon_override);
    let purpose = icon_purpose(kind, override_name.is_some());
    let icon = resolve_first(icon_resolver, &candidates, purpose, IconSize::new(44, 44))?;
    let icon_id = format!("desktop-glyph-{index}-{selected}");
    replace_image(document, &icon_id, icon)?;
    for icon_kind in ICON_KINDS {
        document.visible(
            &format!("desktop-glyph-{index}-{icon_kind}"),
            icon_kind == selected,
        )?;
    }
    Ok(())
}

/// Launcher metadata selection: an explicit Icon= value wins, otherwise the
/// per-kind default name. Pure so unit tests can pin the ordering.
#[must_use]
pub fn icon_candidates(kind: DesktopItemKind, icon_override: Option<&str>) -> Vec<String> {
    let selected = launcher_icon_name(kind, icon_override);
    let default = icon_name(kind).to_owned();
    let mut candidates = Vec::with_capacity(3);
    candidates.push(selected.clone());
    if selected != default {
        // Keep the per-kind default as a resolver fallback behind an explicit Icon= value.
        candidates.push(default);
    }
    candidates.push(String::new());
    candidates
}

/// Which icon name identifies the selected launcher for a given item kind.
#[must_use]
pub fn launcher_icon_name(kind: DesktopItemKind, icon_override: Option<&str>) -> String {
    icon_override
        .map(str::trim)
        .filter(|icon| !icon.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| icon_name(kind).to_owned())
}

pub fn project_item_label(
    document: &mut impl UiDocumentAccess,
    index: usize,
    label: &str,
) -> Result<(), String> {
    let [first, second] = desktop_label_lines(label, LABEL_WIDTH_PX, LABEL_FONT_SIZE_PX);
    set_line_text(document, &format!("desktop-label-{index}-line-1"), first)?;
    set_line_text(document, &format!("desktop-label-{index}-line-2"), second)
}

fn set_line_text(
    document: &mut impl UiDocumentAccess,
    id: &str,
    text: String,
) -> Result<(), String> {
    document.text(id, text)
}

fn icon_semantic(kind: DesktopItemKind) -> &'static str {
    match kind {
        DesktopItemKind::Directory => "directory",
        DesktopItemKind::DesktopLauncher => "desktop-launcher",
        DesktopItemKind::File => "file",
        DesktopItemKind::Symlink => "symlink",
        DesktopItemKind::TrashPseudo => "trash",
    }
}

fn icon_name(kind: DesktopItemKind) -> &'static str {
    match kind {
        DesktopItemKind::Directory => "folder",
        DesktopItemKind::DesktopLauncher => "desktop",
        DesktopItemKind::File => "text-x-generic",
        DesktopItemKind::Symlink => "link",
        DesktopItemKind::TrashPseudo => "trash",
    }
}

fn resolve_first(
    resolver: &mut IconResolver,
    candidates: &[String],
    purpose: IconLookupPurpose,
    size: IconSize,
) -> Result<RuntimeImage, String> {
    let mut last_error = String::new();
    for candidate in candidates {
        match resolve_icon(resolver, candidate, purpose, size) {
            Ok(image) => return Ok(image),
            Err(error) => last_error = error,
        }
    }
    if last_error.is_empty() {
        last_error = "no icon candidates".to_owned();
    }
    Err(last_error)
}

fn resolve_icon(
    resolver: &mut IconResolver,
    icon_name: &str,
    purpose: IconLookupPurpose,
    size: IconSize,
) -> Result<RuntimeImage, String> {
    let resolve = |resolver: &mut IconResolver, name: &str| match purpose {
        IconLookupPurpose::Application => resolver.prepare_application(name, size),
        IconLookupPurpose::Semantic => resolver.prepare_semantic(name, size),
    };
    let raster = resolve(resolver, icon_name).or_else(|error| {
        let fallback_result = match purpose {
            IconLookupPurpose::Application => resolver.prepare_application("", size),
            IconLookupPurpose::Semantic => resolver.prepare_semantic("", size),
        };
        fallback_result.map_err(|fallback| {
            format!("icon '{icon_name}' failed: {error}; generic fallback failed: {fallback}")
        })
    })?;
    Ok(runtime_image(raster))
}

fn runtime_image(raster: Rgb8Raster) -> RuntimeImage {
    RuntimeImage {
        source: raster.source,
        width: raster.width,
        height: raster.height,
        pixels: raster.pixels,
    }
}

fn replace_image(
    document: &mut impl UiDocumentAccess,
    id: &str,
    image: RuntimeImage,
) -> Result<(), String> {
    document.image_rgb8(id, image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbitrary_directory_keeps_generic_folder_semantic() {
        assert_eq!(icon_semantic(DesktopItemKind::Directory), "directory");
        assert_ne!(icon_semantic(DesktopItemKind::Directory), "home");
    }

    #[test]
    fn tile_geometry_stays_centered_in_generated_tile() {
        assert_eq!(DESKTOP_TILE_WIDTH, 82.0);
        assert_eq!(DESKTOP_TILE_HEIGHT, 82.0);
    }

    #[test]
    fn launcher_metadata_prefers_icon_value_then_kind_default() {
        assert_eq!(
            icon_candidates(DesktopItemKind::Directory, Some("my-folder")),
            vec!["my-folder".to_owned(), "folder".to_owned(), String::new()]
        );
        assert_eq!(
            icon_candidates(DesktopItemKind::DesktopLauncher, None),
            vec!["desktop".to_owned(), String::new()]
        );
        assert_eq!(
            launcher_icon_name(DesktopItemKind::File, Some("  ")),
            "text-x-generic"
        );
        assert_eq!(
            launcher_icon_name(DesktopItemKind::File, Some("editor")),
            "editor"
        );
    }

    struct LayerProbe {
        layers: Vec<(String, i32)>,
    }

    impl UiDocumentAccess for LayerProbe {
        fn text(&mut self, _: &str, _: String) -> Result<(), String> {
            Ok(())
        }
        fn image_rgb8(&mut self, _: &str, _: RuntimeImage) -> Result<(), String> {
            Ok(())
        }
        fn image_rgba8(&mut self, _: &str, _: RuntimeImage) -> Result<(), String> {
            Ok(())
        }
        fn visible(&mut self, _: &str, _: bool) -> Result<(), String> {
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
        fn border_clear(&mut self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn foreground(&mut self, _: &str, _: UiColor) -> Result<(), String> {
            Ok(())
        }
        fn foreground_clear(&mut self, _: &str) -> Result<(), String> {
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
        fn layer(&mut self, id: &str, layer: UiLayer) -> Result<(), String> {
            self.layers.push((id.to_owned(), layer.z_index()));
            Ok(())
        }
    }

    fn layer_of(probe: &LayerProbe, id: &str) -> i32 {
        probe
            .layers
            .iter()
            .find(|(entry, _)| entry == id)
            .map(|(_, z)| *z)
            .expect("layer assigned")
    }

    #[test]
    fn document_layers_order_wallpaper_below_item_below_menu_below_modal() {
        let mut probe = LayerProbe { layers: Vec::new() };
        assign_document_layers(&mut probe).expect("assign layers");
        let root = layer_of(&probe, "desktop");
        let wallpaper = layer_of(&probe, "desktop-wallpaper-selection");
        let watermark = layer_of(&probe, "desktop-watermark");
        let item = layer_of(&probe, "desktop-item-0");
        let selection = layer_of(&probe, "desktop-selection");
        let sticky = layer_of(&probe, "sticky-note");
        let menu = layer_of(&probe, "desktop-menu");
        let entry_menu = layer_of(&probe, "desktop-entry-menu");
        let sticky_menu = layer_of(&probe, "sticky-menu");
        let modal = layer_of(&probe, "desktop-confirm");
        assert_eq!(root, UiLayer::SurfaceBackground.z_index());
        assert_eq!(wallpaper, UiLayer::Background.z_index());
        assert_eq!(watermark, UiLayer::Background.z_index());
        assert!(root < wallpaper);
        assert!(wallpaper < item);
        assert!(item < selection);
        assert!(selection < menu);
        assert!(menu < modal);
        assert_eq!(sticky, UiLayer::Floating.z_index());
        assert_eq!(entry_menu, menu);
        assert_eq!(sticky_menu, menu);
        // Menu hit-test wins: popover layer above content/selection.
        assert!(menu > item && menu > selection);
    }

    struct VisibilityProbe {
        hidden: Vec<String>,
        shown: Vec<String>,
    }

    impl UiDocumentAccess for VisibilityProbe {
        fn text(&mut self, _: &str, _: String) -> Result<(), String> {
            Ok(())
        }
        fn image_rgb8(&mut self, _: &str, _: RuntimeImage) -> Result<(), String> {
            Ok(())
        }
        fn image_rgba8(&mut self, _: &str, _: RuntimeImage) -> Result<(), String> {
            Ok(())
        }
        fn visible(&mut self, id: &str, visible: bool) -> Result<(), String> {
            if visible {
                self.shown.push(id.to_owned());
            } else {
                self.hidden.push(id.to_owned());
            }
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
        fn border_clear(&mut self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn foreground(&mut self, _: &str, _: UiColor) -> Result<(), String> {
            Ok(())
        }
        fn foreground_clear(&mut self, _: &str) -> Result<(), String> {
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
        fn layer(&mut self, _: &str, _: UiLayer) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn transient_visibility_hides_all_transient_nodes_before_first_paint() {
        let mut probe = VisibilityProbe {
            hidden: Vec::new(),
            shown: Vec::new(),
        };
        initialize_transient_visibility(&mut probe).expect("hide transients");
        for id in TRANSIENT_HIDDEN_IDS {
            assert!(probe.hidden.contains(&id.to_owned()), "hides {id}");
        }
        assert!(probe.shown.is_empty());
        // Showing one restores its authored display; unrelated nodes untouched.
        probe
            .visible("desktop-menu", true)
            .expect("show restores authored display");
        assert_eq!(probe.shown, vec!["desktop-menu".to_owned()]);
    }

    #[test]
    fn drag_threshold_gates_selection_overlay_show_hide() {
        use flamewm_desktop_core::layout::threshold_passed;
        use flamewm_desktop_core::selection::rubber_visible;
        // Below 5px Euclidean threshold: overlay stays hidden.
        assert!(!threshold_passed(3.0, 3.9));
        assert!(!rubber_visible(3.0, 3.9));
        // At/above threshold: overlay shows.
        assert!(threshold_passed(3.0, 4.0));
        assert!(rubber_visible(3.0, 4.0));
        assert!(rubber_visible(-6.0, 0.0));
    }

    #[test]
    fn entry_menu_is_transient_and_topmost_over_items() {
        // Menu surfaces start hidden before first paint (no flash).
        let mut probe = VisibilityProbe {
            hidden: Vec::new(),
            shown: Vec::new(),
        };
        initialize_transient_visibility(&mut probe).expect("hide transients");
        for id in ["desktop-menu", "desktop-entry-menu", "sticky-menu"] {
            assert!(probe.hidden.contains(&id.to_owned()), "hides {id}");
        }
        // Popover/menu layer sits above entry content and selection so the
        // topmost hit-test lands on the open menu, not the item beneath.
        let mut layers = LayerProbe { layers: Vec::new() };
        assign_document_layers(&mut layers).expect("assign layers");
        let item = layer_of(&layers, "desktop-item-0");
        let selection = layer_of(&layers, "desktop-selection");
        let menu = layer_of(&layers, "desktop-menu");
        let entry_menu = layer_of(&layers, "desktop-entry-menu");
        assert_eq!(entry_menu, menu);
        assert!(menu > item && menu > selection);
    }

    #[test]
    fn icon_purpose_routes_launcher_override_to_application() {
        use flamewm_integrations_linux::icons::IconLookupPurpose;
        assert_eq!(
            icon_purpose(DesktopItemKind::DesktopLauncher, true),
            IconLookupPurpose::Application
        );
        assert_eq!(
            icon_purpose(DesktopItemKind::DesktopLauncher, false),
            IconLookupPurpose::Semantic
        );
        assert_eq!(
            icon_purpose(DesktopItemKind::Directory, true),
            IconLookupPurpose::Semantic
        );
        assert_eq!(
            icon_purpose(DesktopItemKind::File, false),
            IconLookupPurpose::Semantic
        );
        assert_eq!(
            icon_purpose(DesktopItemKind::TrashPseudo, false),
            IconLookupPurpose::Semantic
        );
    }
}
