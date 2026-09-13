pub mod widgets;

use crate::config::{BarLayout, ShellConfig, TrayConfig};
use crate::icons::pixel_icon;
use crate::platform::{OpenWindow, Playback};
use creamui_core::layout::{FlexDirection, Style as LayoutStyle};
use creamui_core::{
    BoxedWidget, Painter, Point, Rect, Size, StateStyle, Style, Styled, TextAlign, Widget,
};
use creamui_image::{Image, ImageData, ImageFit};
use creamui_macros::jsx;
use creamui_reactive::Signal;
use creamui_theme::{Color, ColorScheme, Theme};
use creamui_widgets::layout::{fixed, Align, Flex, Justify};
use creamui_widgets::RawButton;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};

pub const DOCK_HEIGHT: u32 = 44;
const BAR_HEIGHT: f32 = 44.0;
const TASK_SIZE: f32 = 32.0;
const TASK_ICON_SIZE: f32 = 24.0;
const MAX_TASKS: usize = 10;
const CLOCK_WIDTH: f32 = 60.0;
const EDGE_GAP: f32 = 16.0;

const BAR: Color = Color::rgba(20, 21, 22, 248);
const ISLAND: Color = Color::rgba(39, 40, 42, 245);
const ISLAND_BORDER: Color = Color::rgba(255, 255, 255, 20);
const CONTROL: Color = Color::rgba(255, 255, 255, 12);
const CONTROL_HOVER: Color = Color::rgba(255, 255, 255, 26);
const SELECTED: Color = Color::rgba(255, 255, 255, 42);
const PRIMARY: Color = Color::rgb(244, 244, 245);
const MUTED: Color = Color::rgb(166, 168, 171);

pub struct CreamTheme;

#[derive(Clone)]
pub struct BarActions {
    pub refresh_windows: Rc<dyn Fn()>,
    pub activate_window: Rc<dyn Fn(String)>,
    pub current_playback: Rc<dyn Fn() -> Option<Playback>>,
    pub toggle_playback: Rc<dyn Fn()>,
    pub previous_playback: Rc<dyn Fn()>,
    pub next_playback: Rc<dyn Fn()>,
    pub open_weather: Rc<dyn Fn(Point)>,
    pub open_clock: Rc<dyn Fn(Point)>,
    pub open_current_playing: Rc<dyn Fn(Point)>,
    pub open_app_drawer: Rc<dyn Fn(Point)>,
    pub open_control_center: Rc<dyn Fn(Point)>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemStatus {
    pub network_connected: bool,
    pub network_strength: Option<u8>,
    pub battery_percentage: Option<u8>,
    pub battery_charging: bool,
    pub bluetooth_powered: bool,
    pub bluetooth_connected: bool,
    pub volume: f32,
    pub volume_muted: bool,
}

impl CreamTheme {
    pub fn theme() -> Theme {
        Theme::dark().with_colors(ColorScheme {
            surface: BAR,
            surface_elevated: CONTROL,
            surface_hover: CONTROL_HOVER,
            accent: PRIMARY,
            accent_hover: Color::rgb(255, 255, 255),
            accent_pressed: Color::rgb(205, 206, 208),
            selection_background: SELECTED,
            selection_text: PRIMARY,
            text_primary: PRIMARY,
            text_secondary: MUTED,
            text_disabled: Color::rgb(104, 106, 109),
            border: ISLAND_BORDER,
            border_strong: Color::rgba(255, 255, 255, 36),
            danger: Color::rgb(220, 220, 220),
            warning: Color::rgb(220, 220, 220),
            success: Color::rgb(220, 220, 220),
        })
    }
}

/// Widget ids known to the bar. Any other id in `shell.toml`'s
/// `[bar.layout]` is ignored with a warning.
pub const KNOWN_WIDGET_IDS: &[&str] = &[
    "logo",
    "weather",
    "current_playing",
    "app_launcher",
    "control_center",
    "clock",
];

/// Warns once (at startup) about any `[bar.layout]` id the bar does not
/// recognize. The per-frame layout code stays silent about unknown ids so
/// rebuilding the bar never spams the log.
pub fn warn_unknown_widgets(layout: &BarLayout) {
    for id in layout
        .left
        .iter()
        .chain(layout.center.iter())
        .chain(layout.right.iter())
    {
        if !KNOWN_WIDGET_IDS.contains(&id.as_str()) {
            eprintln!("bar: unknown widget id '{id}' in shell.toml layout; ignoring");
        }
    }
}

pub fn build_dock(
    viewport: Size,
    windows: Vec<OpenWindow>,
    launcher_open: Signal<bool>,
    active_window: Signal<Option<String>>,
    status: SystemStatus,
    clock_text: Signal<String>,
    actions: BarActions,
    config: &ShellConfig,
) -> BoxedWidget {
    let launcher_background = if launcher_open.get() {
        SELECTED
    } else {
        CONTROL
    };
    let (tasks, task_count) = window_strip(
        windows,
        active_window,
        actions.activate_window.clone(),
        actions.refresh_windows.clone(),
    );
    let dock_width = dock_width(task_count);
    let clock: BoxedWidget = Box::new(LiveClock {
        text: clock_text.get(),
    });
    let playback = (actions.current_playback)();
    let section_width = viewport.width / 3.0;
    let weather_click = actions.open_weather.clone();
    let clock_click = actions.open_clock.clone();
    let music_click = actions.open_current_playing.clone();
    let drawer_click = actions.open_app_drawer.clone();
    let control_click = actions.open_control_center.clone();

    let weather_button: BoxedWidget = Box::new(
        RawButton::new(island_button_style(126.0, 32.0), || {})
            .with_click_position(move |point| weather_click(point))
            .child(Box::new(jsx! {
                <Flex direction={FlexDirection::Row} size={(126.0, 32.0)} padding={6.0} gap={4.0} align={Align::Center}>
                    {pixel_icon("weather-sun-cloud", 14.0)}
                    <RawText color={MUTED} font_size={10.0} align={TextAlign::Start}>{widgets::weather::DEFAULT_SUMMARY}</RawText>
                </Flex>
            })),
    );
    let music_button = playback.as_ref().map(|item| {
        compact_playback_widget(
            item,
            music_click,
            actions.previous_playback.clone(),
            actions.toggle_playback.clone(),
            actions.next_playback.clone(),
        )
    });
    let drawer_button: BoxedWidget = Box::new(
        RawButton::new(square_style(launcher_background), || {})
            .with_click_position(move |point| drawer_click(point))
            .child(Box::new(jsx! {
                <Flex size={(32.0, 32.0)} align={Align::Center} justify={Justify::Center}>{pixel_icon("appgrid", 21.0)}</Flex>
            })),
    );
    let control_button = control_button_widget(&status, &config.tray, control_click);
    let clock_button: BoxedWidget = Box::new(
        RawButton::new(island_button_style(68.0, 32.0), || {})
            .with_click_position(move |point| clock_click(point))
            .child(Box::new(jsx! {
                <Flex direction={FlexDirection::Column} size={(68.0, 32.0)} padding={2.0} justify={Justify::Center} align={Align::Center}>
                    {clock}
                </Flex>
            })),
    );

    let mut catalog: HashMap<String, BoxedWidget> = HashMap::new();
    catalog.insert("logo".to_owned(), logo_widget());
    if config.widgets.weather.enabled {
        catalog.insert("weather".to_owned(), weather_button);
    }
    if config.widgets.current_playing.enabled && music_button.is_some() {
        catalog.insert(
            "current_playing".to_owned(),
            music_button.expect("checked above"),
        );
    }
    if config.widgets.app_launcher.enabled {
        catalog.insert(
            "app_launcher".to_owned(),
            app_launcher_widget(dock_width, drawer_button, tasks),
        );
    }
    if config.widgets.control_center.enabled {
        catalog.insert("control_center".to_owned(), control_button);
    }
    if config.widgets.clock.enabled {
        catalog.insert("clock".to_owned(), clock_button);
    }

    let left = bar_section(
        &config.bar.layout.left,
        &mut catalog,
        section_width,
        Justify::Start,
        true,
        false,
    );
    let center = bar_section(
        &config.bar.layout.center,
        &mut catalog,
        section_width,
        Justify::Center,
        false,
        false,
    );
    let right = bar_section(
        &config.bar.layout.right,
        &mut catalog,
        section_width,
        Justify::End,
        true,
        true,
    );

    Box::new(jsx! {
        <Flex direction={FlexDirection::Column} size={(viewport.width, viewport.height)} justify={Justify::End} align={Align::Center} background={Color::rgba(0, 0, 0, 0)}>
            <Flex direction={FlexDirection::Row} size={(viewport.width, BAR_HEIGHT)} align={Align::Center} background={BAR} border={(ISLAND_BORDER, 1.0)}>
                {left}
                {center}
                {right}
            </Flex>
        </Flex>
    })
}

fn compact_playback_widget(
    playback: &Playback,
    open: Rc<dyn Fn(Point)>,
    previous: Rc<dyn Fn()>,
    toggle: Rc<dyn Fn()>,
    next: Rc<dyn Fn()>,
) -> BoxedWidget {
    let duration = (!playback.position.is_empty()).then_some(playback.position.as_str());
    let width = if duration.is_some() { 144.0 } else { 108.0 };
    let duration_widget: BoxedWidget = duration
        .map(|duration| {
            Box::new(jsx! {
                <RawText color={MUTED} font_size={10.0} align={TextAlign::End}>{duration}</RawText>
            }) as BoxedWidget
        })
        .unwrap_or_else(|| Box::new(Flex::row().size(0.0, 0.0)));
    let play_icon = if playback.status == "Playing" {
        "pause"
    } else {
        "play"
    };
    Box::new(
        RawButton::new(island_button_style(width, 32.0), || {})
            .with_click_position(move |point| open(point))
            .child(Box::new(jsx! {
                <Flex direction={FlexDirection::Row} size={(width, 32.0)} padding={4.0} gap={3.0} align={Align::Center}>
                    {music_cover(Some(playback))}
                    {compact_media_button("back", previous)}
                    {compact_media_button(play_icon, toggle)}
                    {compact_media_button("next", next)}
                    {duration_widget}
                </Flex>
            })),
    )
}

fn compact_media_button(icon: &'static str, on_click: Rc<dyn Fn()>) -> BoxedWidget {
    Box::new(
        RawButton::new(media_control_style(), || {})
            .with_click_position(move |_| on_click())
            .child(Box::new(jsx! {
                <Flex size={(18.0, 24.0)} align={Align::Center} justify={Justify::Center}>
                    {pixel_icon(icon, 13.0)}
                </Flex>
            })),
    )
}

/// Builds one section (left/center/right) of the bar from its configured
/// widget ids, in order. Ids that are unknown, disabled, or already placed
/// in another section are silently skipped.
fn bar_section(
    ids: &[String],
    catalog: &mut HashMap<String, BoxedWidget>,
    width: f32,
    justify: Justify,
    leading_gap: bool,
    trailing_gap: bool,
) -> BoxedWidget {
    let mut row = Flex::row()
        .size(width, BAR_HEIGHT)
        .gap(6.0)
        .justify(justify)
        .align(Align::Center);
    if leading_gap {
        row = row.child(Box::new(Flex::row().size(EDGE_GAP, BAR_HEIGHT)));
    }
    for id in ids {
        if let Some(widget) = catalog.remove(id) {
            row = row.child(widget);
        }
    }
    if trailing_gap {
        row = row.child(Box::new(Flex::row().size(EDGE_GAP, BAR_HEIGHT)));
    }
    Box::new(row)
}

fn logo_widget() -> BoxedWidget {
    Box::new(jsx! {
        <Flex direction={FlexDirection::Row} size={(48.0, 32.0)} padding={5.0} gap={2.0} align={Align::Center} background={ISLAND} border={(ISLAND_BORDER, 1.0)} corner_radius={9.0}>
            <Flex size={(5.0, 5.0)} background={PRIMARY} corner_radius={2.5} />
            <Flex size={(3.0, 3.0)} background={MUTED} corner_radius={1.5} />
            <Flex size={(3.0, 3.0)} background={MUTED} corner_radius={1.5} />
            <Flex size={(3.0, 3.0)} background={MUTED} corner_radius={1.5} />
            <Flex size={(3.0, 3.0)} background={MUTED} corner_radius={1.5} />
        </Flex>
    })
}

fn app_launcher_widget(
    dock_width: f32,
    drawer_button: BoxedWidget,
    tasks: BoxedWidget,
) -> BoxedWidget {
    Box::new(jsx! {
        <Flex direction={FlexDirection::Row} size={(dock_width, 40.0)} padding={4.0} gap={6.0} align={Align::Center} background={ISLAND} border={(ISLAND_BORDER, 1.0)} corner_radius={12.0}>
            {drawer_button}
            {tasks}
        </Flex>
    })
}

fn control_button_width(icon_count: usize) -> f32 {
    const ICON_WIDTH: f32 = 16.0;
    const CHEVRON_WIDTH: f32 = 13.0;
    const GAP: f32 = 7.0;
    const PADDING: f32 = 8.0;
    let icons = icon_count as f32;
    PADDING + icons * ICON_WIDTH + (icons + 1.0) * GAP + CHEVRON_WIDTH
}

fn control_button_widget(
    status: &SystemStatus,
    tray: &TrayConfig,
    on_click: Rc<dyn Fn(Point)>,
) -> BoxedWidget {
    let network_icon = network_icon(status.network_connected, status.network_strength);
    let battery_icon = battery_icon(status.battery_percentage, status.battery_charging);
    let bluetooth_icon = if status.bluetooth_connected {
        "bluetooth-connected"
    } else if status.bluetooth_powered {
        "bluetooth-on"
    } else {
        "bluetooth-off"
    };
    let volume_icon = volume_icon(status.volume, status.volume_muted);

    let mut icon_count = 0;
    let mut row = Flex::row()
        .padding(4.0)
        .gap(7.0)
        .justify(Justify::Center)
        .align(Align::Center);
    if tray.wifi.shows_in_bar() {
        row = row.child(pixel_icon(network_icon, 16.0));
        icon_count += 1;
    }
    if tray.brightness.shows_in_bar() {
        row = row.child(pixel_icon("brightness", 16.0));
        icon_count += 1;
    }
    if tray.volume.shows_in_bar() {
        row = row.child(pixel_icon(volume_icon, 16.0));
        icon_count += 1;
    }
    if tray.bluetooth.shows_in_bar() {
        row = row.child(pixel_icon(bluetooth_icon, 16.0));
        icon_count += 1;
    }
    if tray.battery.shows_in_bar() {
        row = row.child(pixel_icon(battery_icon, 16.0));
        icon_count += 1;
    }
    row = row.child(pixel_icon("chevron-up", 13.0));
    let width = control_button_width(icon_count);
    row = row.size(width, 32.0);

    Box::new(
        RawButton::new(island_button_style(width, 32.0), || {})
            .with_click_position(move |point| on_click(point))
            .child(Box::new(row)),
    )
}

fn network_icon(connected: bool, strength: Option<u8>) -> &'static str {
    if !connected {
        return "wifi-off";
    }
    match strength.unwrap_or(100) {
        75.. => "wifi-high",
        50..=74 => "wifi-mid",
        25..=49 => "wifi-low",
        _ => "wifi-none",
    }
}

fn battery_icon(percentage: Option<u8>, charging: bool) -> &'static str {
    if charging {
        return "battery-bolt";
    }
    match percentage.unwrap_or(0) {
        80.. => "battery-full",
        50..=79 => "battery-mid",
        20..=49 => "battery-low",
        _ => "battery-empty",
    }
}

fn volume_icon(level: f32, muted: bool) -> &'static str {
    if muted || level <= 0.01 {
        "volume-mute"
    } else if level < 0.34 {
        "volume-low"
    } else if level < 0.67 {
        "volume-mid"
    } else {
        "volume-max"
    }
}

struct LiveClock {
    text: String,
}

impl Widget for LiveClock {
    fn style(&self) -> Style {
        Style::new().layout(LayoutStyle {
            size: fixed(CLOCK_WIDTH, 24.0),
            ..Default::default()
        })
    }

    fn paint(&self, painter: &mut dyn Painter, rect: Rect) {
        painter.fill_text_font(
            rect,
            &self.text,
            PRIMARY,
            13.0,
            TextAlign::Center,
            None,
            false,
            false,
        );
    }
}

fn window_strip(
    windows: Vec<OpenWindow>,
    active_window: Signal<Option<String>>,
    activate_window: Rc<dyn Fn(String)>,
    refresh_windows: Rc<dyn Fn()>,
) -> (BoxedWidget, usize) {
    if windows.is_empty() {
        return (
            Box::new(jsx! {
                <Flex direction={FlexDirection::Row} grow={1.0} align={Align::Center}>
                    <RawText color={MUTED} font_size={13.0}>"No windows"</RawText>
                </Flex>
            }),
            0,
        );
    }

    let count = windows.len().min(MAX_TASKS);
    let mut strip = Flex::row().gap(6.0).align(Align::Center);
    for entry in windows.into_iter().take(MAX_TASKS) {
        let id = entry.id.clone();
        let selected = active_window.get().as_deref() == Some(entry.id.as_str()) || entry.active;
        let select = active_window.clone();
        let activate = activate_window.clone();
        let refresh = refresh_windows.clone();
        let icon = task_icon(&entry, selected);
        let item: BoxedWidget = Box::new(
            RawButton::new(window_style(selected), move || {
                select.set(Some(id.clone()));
                activate(id.clone());
                refresh();
            })
            .child(icon),
        );
        strip = strip.child(item);
    }
    (Box::new(strip), count)
}

fn dock_width(tasks: usize) -> f32 {
    if tasks == 0 {
        120.0
    } else {
        46.0 + tasks as f32 * TASK_SIZE + (tasks - 1) as f32 * 6.0
    }
}

fn task_icon(window: &OpenWindow, active: bool) -> BoxedWidget {
    let indicator_width = if active { 10.0 } else { 3.0 };
    let indicator_color = if active {
        PRIMARY
    } else {
        Color::rgba(0, 0, 0, 0)
    };
    Box::new(jsx! {
        <Flex direction={FlexDirection::Column} size={(TASK_SIZE, TASK_SIZE)} gap={1.0} align={Align::Center} justify={Justify::Center}>
            <Flex size={(TASK_ICON_SIZE, TASK_ICON_SIZE)} align={Align::Center} justify={Justify::Center}>{app_icon(window)}</Flex>
            <Flex size={(indicator_width, 3.0)} background={indicator_color} corner_radius={1.5} />
        </Flex>
    })
}

fn app_icon(window: &OpenWindow) -> BoxedWidget {
    if let Some(data) = window.icon_path.as_ref().and_then(cached_icon_data) {
        return Box::new(
            Image::new(data)
                .layout(LayoutStyle {
                    size: fixed(22.0, 22.0),
                    ..Default::default()
                })
                .fit(ImageFit::Contain),
        );
    }
    Box::new(jsx! {
        <Flex size={(TASK_ICON_SIZE, TASK_ICON_SIZE)} align={Align::Center} justify={Justify::Center} background={CONTROL} corner_radius={7.0}>
            <RawText color={PRIMARY} font_size={11.0} align={TextAlign::Center}>{fallback_icon(&window.app_name)}</RawText>
        </Flex>
    })
}

/// The dock widget tree is rebuilt for clock and playback updates. Retain
/// decoded window icons so those updates never reopen and decode every icon.
/// Failed decodes are cached as well, avoiding a retry on every rebuild.
fn cached_icon_data(path: &std::path::PathBuf) -> Option<ImageData> {
    static CACHE: OnceLock<Mutex<HashMap<std::path::PathBuf, Option<ImageData>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(data) = cache.lock().ok().and_then(|cache| cache.get(path).cloned()) {
        return data;
    }

    let data = ImageData::from_path(path).ok();
    if let Ok(mut cache) = cache.lock() {
        cache.insert(path.clone(), data.clone());
    }
    data
}

fn music_cover(playback: Option<&Playback>) -> BoxedWidget {
    let Some(data) = playback
        .and_then(|item| item.art_url.as_ref())
        .and_then(safe_image_data)
    else {
        let Some(data) = playback
            .and_then(|item| item.app_icon.as_ref())
            .and_then(safe_image_data)
        else {
            return Box::new(jsx! { <Flex size={(24.0, 24.0)} /> });
        };
        return Box::new(
            Image::new(data)
                .layout(LayoutStyle {
                    size: fixed(24.0, 24.0),
                    ..Default::default()
                })
                .fit(ImageFit::Contain),
        );
    };
    Box::new(
        Image::new(data)
            .layout(LayoutStyle {
                size: fixed(24.0, 24.0),
                ..Default::default()
            })
            .fit(ImageFit::Cover),
    )
}

fn safe_image_data(path: &std::path::PathBuf) -> Option<ImageData> {
    const MAX_ART_BYTES: u64 = 8 * 1024 * 1024;
    static CACHE: OnceLock<Mutex<HashMap<std::path::PathBuf, ImageData>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(data) = cache.lock().ok().and_then(|cache| cache.get(path).cloned()) {
        return Some(data);
    }
    let data = std::fs::metadata(path)
        .ok()
        .filter(|meta| meta.len() <= MAX_ART_BYTES)
        .and_then(|_| ImageData::from_path(path).ok())?;
    if let Ok(mut cache) = cache.lock() {
        cache.insert(path.clone(), data.clone());
    }
    Some(data)
}

fn fallback_icon(app_name: &str) -> String {
    app_name
        .chars()
        .next()
        .unwrap_or('•')
        .to_uppercase()
        .to_string()
}

fn square_style(background: Color) -> Style {
    Style::new()
        .layout(LayoutStyle {
            size: fixed(TASK_SIZE, TASK_SIZE),
            ..Default::default()
        })
        .background(background)
        .corner_radius(8.0)
        .hover(StateStyle::new().background(CONTROL_HOVER))
        .pressed(StateStyle::new().background(PRIMARY))
}

fn island_button_style(width: f32, height: f32) -> Style {
    Style::new()
        .layout(LayoutStyle {
            size: fixed(width, height),
            ..Default::default()
        })
        .background(ISLAND)
        .corner_radius(9.0)
        .hover(StateStyle::new().background(CONTROL_HOVER))
        .pressed(StateStyle::new().background(SELECTED))
}

fn window_style(active: bool) -> Style {
    let background = if active {
        Color::rgba(255, 255, 255, 18)
    } else {
        Color::rgba(0, 0, 0, 0)
    };
    Style::new()
        .layout(LayoutStyle {
            size: fixed(TASK_SIZE, TASK_SIZE),
            ..Default::default()
        })
        .background(background)
        .corner_radius(10.0)
        .hover(StateStyle::new().background(CONTROL_HOVER))
        .pressed(StateStyle::new().background(SELECTED))
}

fn media_control_style() -> Style {
    Style::new()
        .layout(LayoutStyle {
            size: fixed(18.0, 24.0),
            ..Default::default()
        })
        .corner_radius(6.0)
        .hover(StateStyle::new().background(CONTROL_HOVER))
        .pressed(StateStyle::new().background(PRIMARY))
}

#[cfg(test)]
mod tests {
    use super::{bar_section, control_button_width, dock_width, fallback_icon};
    use creamui_core::BoxedWidget;
    use creamui_widgets::layout::{Flex, Justify};
    use std::collections::HashMap;

    #[test]
    fn dock_fits_visible_tasks() {
        assert_eq!(dock_width(0), 120.0);
        assert_eq!(dock_width(4), 192.0);
    }

    #[test]
    fn fallback_uses_the_app_initial() {
        assert_eq!(fallback_icon("firefox"), "F");
    }

    #[test]
    fn control_button_grows_with_visible_icons() {
        assert!(control_button_width(0) < control_button_width(3));
        assert!(control_button_width(3) < control_button_width(5));
    }

    #[test]
    fn bar_section_skips_unknown_and_already_placed_ids() {
        let mut catalog: HashMap<String, BoxedWidget> = HashMap::new();
        catalog.insert("a".to_owned(), Box::new(Flex::row()));
        catalog.insert("b".to_owned(), Box::new(Flex::row()));
        let ids = vec!["a".to_owned(), "unknown".to_owned(), "b".to_owned()];

        let mut widget = bar_section(&ids, &mut catalog, 100.0, Justify::Start, false, false);

        assert_eq!(widget.children().len(), 2);
        assert!(catalog.is_empty());
    }
}
