//! Desktop presentation contracts learned from the current native desktop renderer.

use flamewm_api::Rect;

pub const WATERMARK_WIDTH: i32 = 220;
pub const WATERMARK_HEIGHT: i32 = 73;
pub const WATERMARK_PADDING: i32 = 24;
pub const DESKTOP_SELECTION_INSET: i32 = 4;
pub const DESKTOP_LABEL_RESERVE: i32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatermarkGeometry {
    pub rect: Rect,
}

#[must_use]
pub fn desktop_icon_candidates(preferred: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    if !preferred.trim().is_empty() {
        candidates.push(preferred.trim().to_owned());
    }
    for fallback in ["text-x-generic", "file"] {
        if !candidates.iter().any(|item| item == fallback) {
            candidates.push(fallback.to_owned());
        }
    }
    candidates
}

#[must_use]
pub fn desktop_item_visual_rect(item: Rect) -> Rect {
    let inset = DESKTOP_SELECTION_INSET.max(0);
    Rect::new(
        item.x + inset,
        item.y + inset,
        (item.width - inset * 2).max(0),
        (item.height - DESKTOP_LABEL_RESERVE - inset * 2).max(0),
    )
}

#[must_use]
pub fn watermark_geometry(
    work_area: Rect,
    width: i32,
    height: i32,
    padding: i32,
) -> WatermarkGeometry {
    let x = (work_area.right() - width - padding).max(work_area.x);
    let y = (work_area.bottom() - height - padding).max(work_area.y);
    WatermarkGeometry {
        rect: Rect::new(x, y, width.max(0), height.max(0)),
    }
}

#[must_use]
pub fn default_watermark_geometry(work_area: Rect) -> WatermarkGeometry {
    watermark_geometry(
        work_area,
        WATERMARK_WIDTH,
        WATERMARK_HEIGHT,
        WATERMARK_PADDING,
    )
}

#[must_use]
pub const fn watermark_visible(fullscreen: bool, user_enabled: bool) -> bool {
    user_enabled && !fullscreen
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlankDesktopAction {
    OpenTerminal,
    CreateNewFolder,
    NewStickyNote,
    DesktopAndWallpaper,
}
impl BlankDesktopAction {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::OpenTerminal => "Open Terminal",
            Self::CreateNewFolder => "Create New Folder",
            Self::NewStickyNote => "New Sticky Note",
            Self::DesktopAndWallpaper => "Desktop and Wallpaper",
        }
    }
}

#[must_use]
pub fn blank_context_menu(sticky_enabled: bool) -> Vec<BlankDesktopAction> {
    let mut actions = vec![
        BlankDesktopAction::OpenTerminal,
        BlankDesktopAction::CreateNewFolder,
    ];
    if sticky_enabled {
        actions.push(BlankDesktopAction::NewStickyNote);
    }
    actions.push(BlankDesktopAction::DesktopAndWallpaper);
    actions
}

pub const SELECTION_ACCENT_RGB: u32 = 0xff5533;
pub const SELECTION_FILL_BASE_RGB: u32 = 0x111111;
pub const SELECTION_BORDER_BASE_RGB: u32 = 0xe6e6e6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopSelectionMaterial {
    pub fill_rgb: u32,
    pub border_rgb: u32,
}

#[must_use]
pub fn selection_material(
    fill_opacity_percent: u8,
    border_opacity_percent: u8,
) -> DesktopSelectionMaterial {
    DesktopSelectionMaterial {
        fill_rgb: blend_rgb(
            SELECTION_ACCENT_RGB,
            SELECTION_FILL_BASE_RGB,
            fill_opacity_percent,
        ),
        border_rgb: blend_rgb(
            SELECTION_ACCENT_RGB,
            SELECTION_BORDER_BASE_RGB,
            border_opacity_percent,
        ),
    }
}

#[must_use]
pub fn blend_rgb(foreground: u32, background: u32, opacity_percent: u8) -> u32 {
    let foreground_weight = u32::from(opacity_percent.min(100));
    let background_weight = 100_u32 - foreground_weight;
    let blend_channel = |shift: u32| {
        let foreground_channel = (foreground >> shift) & 0xff_u32;
        let background_channel = (background >> shift) & 0xff_u32;
        ((foreground_channel * foreground_weight + background_channel * background_weight)
            / 100_u32)
            << shift
    };
    blend_channel(16_u32) | blend_channel(8_u32) | blend_channel(0_u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn watermark_is_bottom_right_and_hidden_fullscreen() {
        let g = watermark_geometry(Rect::new(0, 0, 1920, 1040), 220, 73, 24);
        assert_eq!(g.rect, Rect::new(1676, 943, 220, 73));
        assert!(!watermark_visible(true, true));
    }
    #[test]
    fn sticky_menu_item_is_conditional() {
        assert_eq!(blank_context_menu(false).len(), 3);
        assert_eq!(
            blank_context_menu(true)[2],
            BlankDesktopAction::NewStickyNote
        );
    }
    #[test]
    fn material_blend_matches_native_integer_blend() {
        assert_eq!(blend_rgb(0xff5533, 0x111111, 20), 0x401e17);
    }
    #[test]
    fn zero_opacity_returns_background() {
        assert_eq!(blend_rgb(0xff5533, 0x111111, 0), 0x111111);
    }
    #[test]
    fn full_opacity_returns_foreground() {
        assert_eq!(blend_rgb(0xff5533, 0x111111, 100), 0xff5533);
    }
    #[test]
    fn default_selection_material_matches_native_colors_and_opacity() {
        let m = selection_material(20, 60);
        assert_eq!(m.fill_rgb, 0x401e17);
        assert_eq!(m.border_rgb, blend_rgb(0xff5533, 0xe6e6e6, 60));
    }
    #[test]
    fn default_watermark_uses_current_native_size() {
        assert_eq!(
            default_watermark_geometry(Rect::new(0, 0, 1920, 1040)).rect,
            Rect::new(1676, 943, 220, 73)
        );
    }
    #[test]
    fn desktop_icon_fallback_keeps_semantic_then_generic_order() {
        assert_eq!(
            desktop_icon_candidates("folder"),
            vec!["folder", "text-x-generic", "file"]
        );
    }
    #[test]
    fn desktop_selection_material_excludes_label_area() {
        assert_eq!(
            desktop_item_visual_rect(Rect::new(10, 10, 80, 96)),
            Rect::new(14, 14, 72, 72)
        );
    }
}
