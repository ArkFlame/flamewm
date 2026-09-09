use flamewm_skin::DEFAULT;
use flamewm_skin::icons::IconRole;
use flamewm_skin::recipes::window_chrome::{
    self as recipe, SceneRect, WINDOW_CHROME, WindowChromeRecipe, WindowChromeScene,
    WindowControlRole,
};
use flamewm_skin::typography::Typography;

/// Skin-owned titlebar height (flamewm-skin RWR 0.0.9 chrome titlebar).
pub const TITLEBAR_HEIGHT: u16 = DEFAULT.chrome.titlebar;
/// Frame border width (no skin token; WM-owned 1px border).
pub const FRAME_BORDER: u16 = 1;
/// Close-hover affordance radius (skin chrome radius).
pub const CLOSE_HOVER_RADIUS: u16 = DEFAULT.chrome.radius;

/// WM-owned view of control roles (mirrors skin WindowControlRole).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlRole {
    Minimize,
    Maximize,
    Restore,
    Close,
}

impl ControlRole {
    #[must_use]
    pub const fn skin(self) -> WindowControlRole {
        match self {
            Self::Minimize => WindowControlRole::Minimize,
            Self::Maximize => WindowControlRole::Maximize,
            Self::Restore => WindowControlRole::Restore,
            Self::Close => WindowControlRole::Close,
        }
    }

    #[must_use]
    pub const fn icon_role(self) -> IconRole {
        self.skin().icon_role()
    }

    /// Breeze asset path for this control (`assets/web/breeze/window-*.svg`).
    #[must_use]
    pub fn asset_path(self) -> &'static str {
        self.icon_role().source().path
    }
}

impl From<WindowControlRole> for ControlRole {
    fn from(role: WindowControlRole) -> Self {
        match role {
            WindowControlRole::Minimize => Self::Minimize,
            WindowControlRole::Maximize | WindowControlRole::Restore => Self::Maximize,
            WindowControlRole::Close => Self::Close,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlPolicy {
    pub roles: [ControlRole; 3],
    pub button_width: u16,
}

impl Default for ControlPolicy {
    fn default() -> Self {
        Self::for_state(false, false)
    }
}

impl ControlPolicy {
    /// Skin control order; the middle slot becomes Restore when maximized or
    /// fullscreen so the hit target keeps its geometry while the glyph swaps.
    #[must_use]
    pub fn for_state(maximized: bool, fullscreen: bool) -> Self {
        let middle = if maximized || fullscreen {
            ControlRole::Restore
        } else {
            ControlRole::Maximize
        };
        Self {
            roles: [ControlRole::Minimize, middle, ControlRole::Close],
            button_width: WINDOW_CHROME.metrics.button_width,
        }
    }

    #[must_use]
    pub fn recipe() -> WindowChromeRecipe {
        WINDOW_CHROME
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Metrics {
    pub titlebar_height: u16,
}

impl Metrics {
    pub const fn new(titlebar_height: u16) -> Self {
        Self { titlebar_height }
    }
}

/// Resolved app icon source, in priority order: live `_NET_WM_ICON` pixels,
/// catalog `DesktopEntry` `Icon=` raster via the icon resolver, coarse
/// `WM_CLASS` skin role as last resort before the generic fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppIconSource {
    Native(IconImage),
    CatalogRaster(flamewm_integrations_linux::icons::Rgba8Raster),
    ClassRole(flamewm_skin::icons::IconRole),
    Generic,
}

/// Validated `_NET_WM_ICON` image (EWMH ARGB32 cardinals, non-premultiplied).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconImage {
    pub width: u32,
    pub height: u32,
    pub argb: Vec<u32>,
}

/// Parse concatenated EWMH `_NET_WM_ICON` cardinals; select largest valid image.
/// Rejects zero/overflow dimensions and truncated payloads.
pub fn parse_net_wm_icon(cardinals: &[u32]) -> Option<IconImage> {
    let mut cursor = 0_usize;
    let mut best: Option<IconImage> = None;
    while cursor.saturating_add(2) <= cardinals.len() {
        let width = cardinals[cursor];
        let height = cardinals[cursor + 1];
        cursor += 2;
        if width == 0 || height == 0 {
            return None;
        }
        let count = width
            .checked_mul(height)
            .and_then(|pixels| usize::try_from(pixels).ok())?;
        if cursor.saturating_add(count) > cardinals.len() {
            return None;
        }
        let argb = cardinals[cursor..cursor + count].to_vec();
        cursor += count;
        let area = u64::from(width) * u64::from(height);
        let best_area = best.as_ref().map_or(0, |image: &IconImage| {
            u64::from(image.width) * u64::from(image.height)
        });
        if area >= best_area {
            best = Some(IconImage {
                width,
                height,
                argb,
            });
        }
    }
    if cursor != cardinals.len() {
        return None;
    }
    best.filter(|image| !image.argb.is_empty())
}

/// Nearest-neighbor scale into a square titlebar slot, alpha preserved.
pub fn scale_icon_to_slot(icon: &IconImage, slot: u32) -> Option<IconImage> {
    if slot == 0 || icon.width == 0 || icon.height == 0 {
        return None;
    }
    let slot_usize = usize::try_from(slot).ok()?;
    let width_usize = usize::try_from(icon.width).ok()?;
    let height_usize = usize::try_from(icon.height).ok()?;
    let mut argb = Vec::with_capacity(slot_usize * slot_usize);
    for y in 0..slot_usize {
        let source_y = y * height_usize / slot_usize;
        for x in 0..slot_usize {
            let source_x = x * width_usize / slot_usize;
            argb.push(icon.argb[source_y * width_usize + source_x]);
        }
    }
    Some(IconImage {
        width: slot,
        height: slot,
        argb,
    })
}

/// Straight RGBA8 raster for a native `_NET_WM_ICON` selection, scaled into
/// the square titlebar slot (canonical blit input for `blit_rgba`).
#[must_use]
pub fn native_icon_rgba(icon: &IconImage, slot: u32) -> Option<flamewm_image_core::RgbaImage> {
    let scaled = scale_icon_to_slot(icon, slot)?;
    let mut pixels = Vec::with_capacity(scaled.argb.len() * 4);
    for pixel in &scaled.argb {
        pixels.push(((pixel >> 16) & 0xff) as u8);
        pixels.push(((pixel >> 8) & 0xff) as u8);
        pixels.push((pixel & 0xff) as u8);
        pixels.push(((pixel >> 24) & 0xff) as u8);
    }
    flamewm_image_core::RgbaImage::from_rgba8(scaled.width, scaled.height, pixels)
}

/// Load a Breeze `window-*.svg` control glyph at its asset path with the
/// semantic Flame color scheme (canonical `image-core` SVG entry point).
#[must_use]
pub fn control_glyph_svg(role: ControlRole, edge: u32) -> Option<flamewm_image_core::RgbaImage> {
    control_glyph_svg_at(env!("CARGO_MANIFEST_DIR"), role, edge)
}

fn control_glyph_svg_at(
    manifest_dir: &str,
    role: ControlRole,
    edge: u32,
) -> Option<flamewm_image_core::RgbaImage> {
    if edge == 0 || edge > flamewm_image_core::MAX_DIMENSION {
        return None;
    }
    let path = std::path::Path::new(manifest_dir)
        .join("../../")
        .join(role.asset_path());
    let bytes = std::fs::read(path).ok()?;
    flamewm_image_core::svg::render_with_color_scheme(
        &bytes,
        edge,
        edge,
        &flamewm_image_core::SvgColorScheme::flame_default(),
    )
    .ok()
}

/// Skin color as canonical `render-core` RGBA for `ExternalDrawableTarget`.
#[must_use]
pub fn skin_color(color: flamewm_skin::Rgb) -> flamewm_render_core::Color {
    flamewm_render_core::Color {
        r: ((color.0 >> 16) & 0xff) as u8,
        g: ((color.0 >> 8) & 0xff) as u8,
        b: (color.0 & 0xff) as u8,
        a: 255,
    }
}

/// Titlebar icon slot edge for the configured titlebar height.
#[must_use]
pub fn icon_slot_for(titlebar_height: u16) -> u16 {
    recipe::icon_slot_edge()
        .min(titlebar_height.saturating_sub(4))
        .max(recipe::ICON_SLOT_MIN)
}

/// Title baseline (Xft device pixels) for the configured titlebar height.
#[must_use]
pub fn title_baseline(titlebar_height: u16) -> f32 {
    f32::from(titlebar_height.saturating_sub(11).max(11))
}

/// Map a `WM_CLASS` (instance or class, NUL-separated) to a skin icon role.
/// Used only when `_NET_WM_ICON` is absent; pure fallback, no state minted.
#[must_use]
pub fn icon_role_for_class(wm_class: &str) -> Option<IconRole> {
    let lowered = wm_class.to_ascii_lowercase();
    for token in lowered.split(['\0', ' ', '-', '_']) {
        let role = match token {
            t if t.contains("terminal") || t == "xterm" || t == "kitty" || t == "alacritty" => {
                IconRole::Terminal
            }
            t if t.contains("browser") || t.contains("firefox") || t.contains("chrome") => {
                IconRole::Browser
            }
            t if t.contains("file") || t.contains("nautilus") || t.contains("dolphin") => {
                IconRole::Files
            }
            t if t.contains("code") || t.contains("editor") || t.contains("vim") => IconRole::Code,
            t if t.contains("setting") || t.contains("config") => IconRole::Settings,
            t if t.contains("trash") => IconRole::Trash,
            t if t.contains("music") || t.contains("audio") => IconRole::Volume,
            t if t.contains("lock") => IconRole::Lock,
            _ => continue,
        };
        return Some(role);
    }
    None
}

/// Build the renderer-neutral [`WindowChromeScene`] for one frame.
///
/// `app_icon` carries the resolved icon role: `Some` when `_NET_WM_ICON` is
/// present (mapped through the caller) or when the `WM_CLASS` fallback
/// resolves, else `None`. Hit rects derive from skin
/// `control_button_geometries`, never from glyph pixels.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn build_scene(
    frame_width: u32,
    title: &str,
    app_icon: Option<IconRole>,
    hover: Option<ControlRole>,
    active: bool,
    maximized: bool,
    fullscreen: bool,
) -> WindowChromeScene {
    let policy = ControlPolicy::for_state(maximized, fullscreen);
    let bounds = SceneRect::new(
        0,
        0,
        i32::try_from(frame_width).unwrap_or(i32::MAX),
        i32::from(DEFAULT.chrome.titlebar),
    );
    let mut scene = WindowChromeScene::new(bounds, title);
    scene.title_style = Typography::RWR_0_0_9.title;
    scene.app_icon = app_icon;
    scene.control_roles = policy.roles.map(ControlRole::skin);
    scene.hover = hover.map(ControlRole::skin);
    scene.pressed = None;
    scene.active = active;
    scene.maximized = maximized;
    scene.fullscreen = fullscreen;
    scene
}

/// Renderer-owned title measurement for Xft IBM Plex Sans 12px semibold
/// (skin typography title). Proportional advance estimate: ASCII 7px, CJK
/// 12px, other 8px; keeps the skin `center_title_x` contract without the
/// retired 9px core-font advance.
#[must_use]
pub fn title_text_width(title: &str) -> i32 {
    let mut width = 0_i32;
    let mut count = 0_usize;
    for ch in title.chars() {
        if count >= TITLE_MAX_CHARS {
            break;
        }
        count += 1;
        let advance = if ch.is_ascii() {
            7
        } else if is_wide(ch) {
            12
        } else {
            8
        };
        width = width.saturating_add(advance);
    }
    width
}

const TITLE_MAX_CHARS: usize = 96;

fn is_wide(ch: char) -> bool {
    matches!(ch,
        '\u{1100}'..='\u{115F}' | '\u{2E80}'..='\u{A4CF}' | '\u{AC00}'..='\u{D7A3}'
        | '\u{F900}'..='\u{FAFF}' | '\u{FE30}'..='\u{FE4F}' | '\u{FF00}'..='\u{FFEF}')
}

/// Center-title formula, delegated to the skin contract.
/// `paint_x = clamp((W - tw) / 2, left + pad, right - pad - tw)`.
#[must_use]
pub fn center_title_x(
    titlebar_width: i32,
    title_text_width: i32,
    left_occupied: i32,
    right_occupied: i32,
    padding: i32,
) -> i32 {
    recipe::center_title_x(
        titlebar_width,
        title_text_width,
        left_occupied,
        right_occupied,
        padding,
    )
}

/// Frame shaping: rounded radius normally, rectangular when maximized or
/// fullscreen. The bounding shape stays rectangular so the reparented client
/// is never over-clipped; shaping is applied atomically with the geometry
/// configure in `wm.rs`, and hit rects stay glyph-independent.
#[must_use]
pub fn effective_radius(maximized: bool, fullscreen: bool) -> u16 {
    recipe::effective_radius(maximized, fullscreen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_formula_matches_flame_contract() {
        // desired=(400-90)/2=155 inside free region.
        assert_eq!(center_title_x(400, 90, 22, 304, 6), 155);
        // Desired below minimum but free region inverted: sorted clamp keeps desired (20).
        assert_eq!(center_title_x(400, 360, 22, 304, 6), 20);
        // Title wider than the free region: sorted clamp keeps desired (10).
        assert_eq!(center_title_x(200, 180, 22, 104, 6), 10);
    }

    #[test]
    fn title_measurement_uses_proportional_advance() {
        assert_eq!(title_text_width(""), 0);
        assert_eq!(title_text_width("ab"), 14);
        assert_eq!(title_text_width("あ"), 12);
    }

    #[test]
    fn net_wm_icon_parser_accepts_valid_argb() {
        let cardinals = vec![
            2,
            1,
            0xff00_0000,
            0xff00_ff00, // 2x1
            1,
            1,
            0x80ff_0000, // 1x1
        ];
        let icon = parse_net_wm_icon(&cardinals).expect("valid payload");
        assert_eq!((icon.width, icon.height), (2, 1));
        assert_eq!(icon.argb, vec![0xff00_0000, 0xff00_ff00]);
    }

    #[test]
    fn net_wm_icon_parser_rejects_invalid_payload() {
        assert!(parse_net_wm_icon(&[2, 2, 1, 2, 3]).is_none());
        assert!(parse_net_wm_icon(&[0, 1, 0]).is_none());
        assert!(parse_net_wm_icon(&[1, 1]).is_none());
        assert!(parse_net_wm_icon(&[1, 1, 0, 99]).is_none());
    }

    #[test]
    fn icon_scales_into_slot_with_alpha_preserved() {
        let icon = IconImage {
            width: 2,
            height: 2,
            argb: vec![0xff00_0000, 0x0000_0000, 0x80ff_0000, 0xff00_ff00],
        };
        let scaled = scale_icon_to_slot(&icon, 4).expect("scaled");
        assert_eq!((scaled.width, scaled.height), (4, 4));
        assert_eq!(scaled.argb.len(), 16);
        assert!(scaled.argb.contains(&0x0000_0000));
        assert!(scaled.argb.contains(&0x80ff_0000));
    }

    #[test]
    fn scene_uses_skin_controls_and_rectangular_maximized() {
        let scene = build_scene(400, "app", None, None, true, true, false);
        assert_eq!(scene.control_roles[1], WindowControlRole::Restore);
        assert_eq!(scene.effective_radius(), 0);
        let floating = build_scene(400, "app", None, None, true, false, false);
        assert_eq!(floating.control_roles[1], WindowControlRole::Maximize);
        assert_eq!(floating.effective_radius(), WINDOW_CHROME.metrics.radius);
    }

    #[test]
    fn class_fallback_maps_known_apps() {
        assert_eq!(
            icon_role_for_class("org.gnome.Terminal"),
            Some(IconRole::Terminal)
        );
        assert!(icon_role_for_class("").is_none());
    }

    #[test]
    fn native_icon_converts_to_straight_rgba() {
        let icon = IconImage {
            width: 1,
            height: 1,
            argb: vec![0x80ff_0000],
        };
        let rgba = native_icon_rgba(&icon, 2).expect("rgba");
        assert_eq!((rgba.width, rgba.height), (2, 2));
        assert_eq!(rgba.pixels, vec![255, 0, 0, 128].repeat(4));
    }

    #[test]
    fn icon_slot_and_baseline_follow_titlebar_height() {
        assert_eq!(icon_slot_for(31), recipe::icon_slot_edge());
        assert_eq!(title_baseline(31), 20.0);
        assert_eq!(title_baseline(10), 11.0);
    }

    #[test]
    fn control_glyphs_render_from_breeze_assets() {
        for role in [
            ControlRole::Minimize,
            ControlRole::Maximize,
            ControlRole::Restore,
            ControlRole::Close,
        ] {
            let image = control_glyph_svg(role, 16).expect("breeze control svg");
            assert_eq!((image.width, image.height), (16, 16));
            assert!(image.pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        }
        assert!(control_glyph_svg(ControlRole::Close, 0).is_none());
    }

    #[test]
    fn control_glyph_rejects_missing_assets() {
        assert!(control_glyph_svg_at("/nonexistent", ControlRole::Close, 16).is_none());
    }

    #[test]
    fn skin_color_maps_titlebar_background() {
        let color = skin_color(WINDOW_CHROME.titlebar_background);
        assert_eq!(
            (color.r, color.g, color.b, color.a),
            (0x1b, 0x1e, 0x20, 255)
        );
    }
}
