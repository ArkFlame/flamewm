use flamewm_api::Point;
use flamewm_api::settings::SettingValue;
use flamewm_desktop_core::model::DesktopItemKind;
use flamewm_desktop_core::presentation::desktop_label_lines;
use flamewm_integrations_linux::icons::{IconResolver, IconSize, Rgb8Raster};
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

pub fn project_item_icon(
    document: &mut impl UiDocumentAccess,
    icon_resolver: &mut IconResolver,
    index: usize,
    kind: DesktopItemKind,
    icon_override: Option<&str>,
) -> Result<(), String> {
    let selected = icon_semantic(kind);
    let candidates = icon_candidates(kind, icon_override);
    let icon = resolve_first(icon_resolver, &candidates, IconSize::new(44, 44))?;
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
    size: IconSize,
) -> Result<RuntimeImage, String> {
    let mut last_error = String::new();
    for candidate in candidates {
        match resolve_icon(resolver, candidate, size) {
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
    size: IconSize,
) -> Result<RuntimeImage, String> {
    let raster = resolver.prepare_name(icon_name, size).or_else(|error| {
        resolver.prepare_name("", size).map_err(|fallback| {
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
}
