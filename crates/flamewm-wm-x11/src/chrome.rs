use flamewm_skin::icons::IconRole;
use flamewm_skin::recipes::window_chrome::{
    self as recipe, SceneRect, WINDOW_CHROME, WindowChromeRecipe, WindowChromeScene,
    WindowControlRole,
};
use flamewm_skin::typography::Typography;
use flamewm_skin::{DEFAULT, Rgb};
use x11rb::connection::Connection;
use x11rb::errors::ReplyError;
use x11rb::protocol::xproto::*;

/// Skin-owned titlebar height (flamewm-skin RWR 0.0.9 chrome titlebar).
pub const TITLEBAR_HEIGHT: u16 = DEFAULT.chrome.titlebar;
/// Frame border width (no skin token; WM-owned 1px border).
pub const FRAME_BORDER: u16 = 1;
/// Close-hover affordance radius (skin chrome radius).
pub const CLOSE_HOVER_RADIUS: u16 = DEFAULT.chrome.radius;

/// WM-owned view of control roles (mirrors skin WindowControlRole).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub titlebar_height: u16,
}

impl Metrics {
    pub const fn new(titlebar_height: u16) -> Self {
        Self { titlebar_height }
    }
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

/// Narrow drawable renderer. render-x11 owns no `NativeDrawableRenderer` /
/// `external_drawable` API, so wm-x11 defines this local trait; the x11rb
/// fallback below implements it without core-font text.
pub trait NativeDrawableRenderer {
    fn fill_rect(&mut self, x: i16, y: i16, width: u16, height: u16, pixel: u32);
    fn draw_title_run(&mut self, x: i32, baseline: i16, text: &str, pixel: u32);
    fn draw_control_glyph(&mut self, role: ControlRole, bounds: SceneRect, pixel: u32);
    fn paint_icon(&mut self, icon: &IconImage, slot: u16, dst_x: i16, dst_y: i16);
    fn finish(&mut self) -> Result<(), ReplyError>;
}

/// x11rb fallback painter: solid fills, block-advance title runs (Xft glyph
/// rasterization belongs to render-x11, which has no external-drawable API),
/// Breeze-geometry vector control glyphs, composited icon blits.
pub struct X11ChromePainter<'a, C: Connection> {
    conn: &'a C,
    gc: Gcontext,
    frame: Window,
    screen: &'a Screen,
    titlebar_height: u16,
}

impl<'a, C: Connection> X11ChromePainter<'a, C> {
    pub fn new(
        conn: &'a C,
        gc: Gcontext,
        frame: Window,
        screen: &'a Screen,
        titlebar: u16,
    ) -> Self {
        Self {
            conn,
            gc,
            frame,
            screen,
            titlebar_height: titlebar,
        }
    }

    fn set_foreground(&self, pixel: u32) -> Result<(), ReplyError> {
        self.conn
            .change_gc(self.gc, &ChangeGCAux::new().foreground(pixel))?;
        Ok(())
    }
}

impl<C: Connection> NativeDrawableRenderer for X11ChromePainter<'_, C> {
    fn fill_rect(&mut self, x: i16, y: i16, width: u16, height: u16, pixel: u32) {
        let _ = self.set_foreground(pixel);
        let _ = self.conn.poly_fill_rectangle(
            self.frame,
            self.gc,
            &[Rectangle {
                x,
                y,
                width,
                height,
            }],
        );
    }

    fn draw_title_run(&mut self, x: i32, baseline: i16, text: &str, pixel: u32) {
        // Xft-equivalent placeholder over the existing x11rb path: one
        // advance block per character in skin text color. Real glyph shapes
        // come from render-x11 once it exposes an external-drawable API.
        let _ = self.set_foreground(pixel);
        let mut cursor = x;
        for ch in text.chars().take(TITLE_MAX_CHARS) {
            let advance: i32 = if ch == ' ' {
                4
            } else if ch.is_ascii() {
                7
            } else if is_wide(ch) {
                12
            } else {
                8
            };
            if ch != ' ' {
                let _ = self.conn.poly_fill_rectangle(
                    self.frame,
                    self.gc,
                    &[Rectangle {
                        x: cursor.clamp(0, i16::MAX as i32) as i16,
                        y: baseline.saturating_sub(9),
                        width: u16::try_from(advance.saturating_sub(2).max(1)).unwrap_or(1),
                        height: 9.min(self.titlebar_height.saturating_sub(4)).max(1),
                    }],
                );
            }
            cursor = cursor.saturating_add(advance);
        }
    }

    fn draw_control_glyph(&mut self, role: ControlRole, bounds: SceneRect, pixel: u32) {
        // Breeze vector recipe (`assets/web/breeze/window-*.svg`): minimize =
        // baseline bar, maximize = square outline, restore = offset double
        // square, close = X. Painted with skin colors only.
        let _ = self.set_foreground(pixel);
        let button = i16::try_from(bounds.width).unwrap_or(TITLEBAR_HEIGHT as i16);
        let left = i16::try_from(bounds.x).unwrap_or(0);
        let top = i16::try_from(bounds.y).unwrap_or(0);
        let center_x = left + button / 2;
        let center_y = top + button / 2;
        let inset = 8_i16.min(button / 3);
        let right = left + button;
        match role {
            ControlRole::Minimize => {
                let _ = self.conn.poly_line(
                    CoordMode::ORIGIN,
                    self.frame,
                    self.gc,
                    &[
                        Point {
                            x: left + inset,
                            y: center_y + inset / 2,
                        },
                        Point {
                            x: right - inset,
                            y: center_y + inset / 2,
                        },
                    ],
                );
            }
            ControlRole::Maximize => {
                let _ = self.conn.poly_rectangle(
                    self.frame,
                    self.gc,
                    &[Rectangle {
                        x: center_x - inset / 2,
                        y: center_y - inset / 2,
                        width: inset as u16,
                        height: inset as u16,
                    }],
                );
            }
            ControlRole::Restore => {
                let offset = 2_i16;
                let _ = self.conn.poly_rectangle(
                    self.frame,
                    self.gc,
                    &[Rectangle {
                        x: center_x - inset / 2,
                        y: center_y - inset / 2 + offset,
                        width: inset as u16,
                        height: inset as u16,
                    }],
                );
                let _ = self.conn.poly_line(
                    CoordMode::ORIGIN,
                    self.frame,
                    self.gc,
                    &[
                        Point {
                            x: center_x - inset / 2,
                            y: center_y - inset / 2 + offset,
                        },
                        Point {
                            x: center_x - inset / 2,
                            y: center_y - inset / 2,
                        },
                        Point {
                            x: center_x + inset / 2 + offset,
                            y: center_y - inset / 2,
                        },
                        Point {
                            x: center_x + inset / 2 + offset,
                            y: center_y + inset / 2,
                        },
                    ],
                );
            }
            ControlRole::Close => {
                let close_inset = 10_i16.min(button / 3);
                let _ = self.conn.poly_line(
                    CoordMode::ORIGIN,
                    self.frame,
                    self.gc,
                    &[
                        Point {
                            x: left + close_inset,
                            y: top + close_inset,
                        },
                        Point {
                            x: right - close_inset,
                            y: top + button - close_inset,
                        },
                    ],
                );
                let _ = self.conn.poly_line(
                    CoordMode::ORIGIN,
                    self.frame,
                    self.gc,
                    &[
                        Point {
                            x: left + close_inset,
                            y: top + button - close_inset,
                        },
                        Point {
                            x: right - close_inset,
                            y: top + close_inset,
                        },
                    ],
                );
            }
        }
    }

    fn paint_icon(&mut self, icon: &IconImage, slot: u16, dst_x: i16, dst_y: i16) {
        let Some(scaled) = scale_icon_to_slot(icon, u32::from(slot)) else {
            return;
        };
        let depth = self.screen.root_depth;
        let mut data = Vec::with_capacity(scaled.argb.len() * 4);
        for pixel in &scaled.argb {
            let (red, green, blue) =
                composite_over(*pixel, skin_rgb(WINDOW_CHROME.titlebar_background));
            let packed = (u32::from(red) << 16) | (u32::from(green) << 8) | u32::from(blue);
            data.extend_from_slice(&packed.to_ne_bytes());
        }
        let _ = self.conn.put_image(
            ImageFormat::Z_PIXMAP,
            self.frame,
            self.gc,
            scaled.width as u16,
            scaled.height as u16,
            dst_x,
            dst_y,
            0,
            depth,
            &data,
        );
    }

    fn finish(&mut self) -> Result<(), ReplyError> {
        Ok(())
    }
}

fn skin_rgb(color: Rgb) -> (u8, u8, u8) {
    (
        ((color.0 >> 16) & 0xff) as u8,
        ((color.0 >> 8) & 0xff) as u8,
        (color.0 & 0xff) as u8,
    )
}

fn skin_pixel<C: Connection>(conn: &C, screen: &Screen, color: Rgb) -> u32 {
    let (red, green, blue) = skin_rgb(color);
    conn.alloc_color(
        screen.default_colormap,
        u16::from(red) * 257,
        u16::from(green) * 257,
        u16::from(blue) * 257,
    )
    .ok()
    .and_then(|cookie| cookie.reply().ok())
    .map(|reply| reply.pixel)
    .unwrap_or_else(|| (u32::from(red) << 16) | (u32::from(green) << 8) | u32::from(blue))
}

fn composite_over(argb: u32, bg: (u8, u8, u8)) -> (u8, u8, u8) {
    let alpha = ((argb >> 24) & 0xff) as u32;
    if alpha == 0 {
        return bg;
    }
    if alpha == 255 {
        return (
            ((argb >> 16) & 0xff) as u8,
            ((argb >> 8) & 0xff) as u8,
            (argb & 0xff) as u8,
        );
    }
    let blend = |foreground: u32, background: u8| {
        ((foreground * alpha + u32::from(background) * (255 - alpha) + 127) / 255) as u8
    };
    (
        blend((argb >> 16) & 0xff, bg.0),
        blend((argb >> 8) & 0xff, bg.1),
        blend(argb & 0xff, bg.2),
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

/// Restore the default arrow cursor on a frame (cursor `None` = server arrow).
pub fn define_arrow_cursor<C: Connection>(conn: &C, frame: Window) -> Result<(), ReplyError> {
    conn.change_window_attributes(frame, &ChangeWindowAttributesAux::new().cursor(0))?;
    Ok(())
}

/// Full chrome paint from a [`WindowChromeScene`]: skin titlebar, cached icon
/// slot, measured centered title, Breeze-geometry control glyphs.
/// Hit targets unchanged (skin `control_button_geometries`).
#[allow(clippy::too_many_arguments)]
pub fn paint_scene<C: Connection>(
    conn: &C,
    gc: Gcontext,
    frame: Window,
    width: u16,
    screen: &Screen,
    scene: &WindowChromeScene,
    icon: Option<&IconImage>,
    metrics: Metrics,
    close_hover: bool,
) -> Result<(), ReplyError> {
    let title_height = metrics.titlebar_height;
    let background = skin_pixel(conn, screen, WINDOW_CHROME.titlebar_background);
    let foreground = skin_pixel(conn, screen, scene.title_style.color);
    let border = skin_pixel(conn, screen, WINDOW_CHROME.border);
    let hover = skin_pixel(conn, screen, WINDOW_CHROME.close_hover);
    let mut painter = X11ChromePainter::new(conn, gc, frame, screen, title_height);

    painter.fill_rect(0, 0, width, title_height, background);

    let slot = recipe::icon_slot_edge()
        .min(title_height.saturating_sub(4))
        .max(recipe::ICON_SLOT_MIN);
    let icon_y = (i16::try_from(title_height)
        .unwrap_or(i16::MAX)
        .saturating_sub(i16::try_from(slot).unwrap_or(i16::MAX)))
        / 2;
    if let Some(image) = icon {
        painter.paint_icon(image, slot, recipe::ICON_PAD_LEFT as i16, icon_y);
    }
    let left_occupied = recipe::ICON_PAD_LEFT + i32::from(slot);

    let titlebar = SceneRect::new(0, 0, scene.bounds.width, i32::from(title_height));
    let geometries = recipe::control_button_geometries(titlebar);
    let right_occupied = geometries[0].bounds.x;

    let text_width = title_text_width(&scene.title);
    let paint_x = recipe::center_title_x(
        scene.bounds.width,
        text_width,
        left_occupied,
        right_occupied,
        recipe::TITLE_PAD,
    );
    let baseline = i16::try_from(title_height)
        .unwrap_or(i16::MAX)
        .saturating_sub(11)
        .max(11);
    painter.draw_title_run(paint_x, baseline, &scene.title, foreground);

    conn.change_gc(gc, &ChangeGCAux::new().foreground(border))?;
    for geometry in &geometries {
        conn.poly_line(
            CoordMode::ORIGIN,
            frame,
            gc,
            &[
                Point {
                    x: i16::try_from(geometry.bounds.x).unwrap_or(i16::MAX),
                    y: 0,
                },
                Point {
                    x: i16::try_from(geometry.bounds.x).unwrap_or(i16::MAX),
                    y: i16::try_from(title_height).unwrap_or(i16::MAX),
                },
            ],
        )?;
    }
    for (geometry, role) in geometries.iter().zip(scene.control_roles) {
        let control = ControlRole::from(role);
        if control == ControlRole::Close && close_hover {
            conn.change_gc(gc, &ChangeGCAux::new().foreground(hover))?;
            let radius = i16::try_from(CLOSE_HOVER_RADIUS.min(title_height / 2)).unwrap_or(6);
            let right = i16::try_from(scene.bounds.width).unwrap_or(i16::MAX);
            conn.poly_fill_arc(
                frame,
                gc,
                &[Arc {
                    x: right - radius * 2,
                    y: 0,
                    width: (radius * 2) as u16,
                    height: (radius * 2) as u16,
                    angle1: 0,
                    angle2: 360_i16 * 64,
                }],
            )?;
        }
        painter.draw_control_glyph(control, geometry.bounds, foreground);
    }
    painter.finish()
}

/// Back-compat entry: same paint without an explicit scene (active, floating).
pub fn draw_full<C: Connection>(
    conn: &C,
    gc: Gcontext,
    frame: Window,
    width: u16,
    title: &str,
    icon: Option<&IconImage>,
    metrics: Metrics,
    close_hover: bool,
    screen: &Screen,
) -> Result<(), ReplyError> {
    let scene = build_scene(u32::from(width), title, None, None, true, false, false);
    paint_scene(
        conn,
        gc,
        frame,
        width,
        screen,
        &scene,
        icon,
        metrics,
        close_hover,
    )
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
}
