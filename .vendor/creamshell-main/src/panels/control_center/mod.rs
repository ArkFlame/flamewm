use crate::config::TrayConfig;
use crate::icons::pixel_icon;
use crate::integrations::{
    battery::BatteryIntegration, bluetooth::BluetoothIntegration,
    brightness::BrightnessIntegration, network::NetworkIntegration, volume::VolumeIntegration,
};
use crate::panels::chrome::{
    BORDER, CARD, CARD_RADIUS, FILL, ISLAND_RADIUS, MUTED, PANEL, TEXT, TRACK,
};
use creamui_core::layout::{FlexDirection, Style as LayoutStyle};
use creamui_core::{Border, BoxedWidget, Size, StateStyle, Style, StyleProp, Styled, TextAlign};
use creamui_macros::jsx;
use creamui_reactive::Signal;
use creamui_theme::Color;
use creamui_widgets::{
    layout::{fixed, Align, Flex},
    RawMarquee, RawSlider, RawSwitch,
};
use std::rc::Rc;

pub const WIDTH: u32 = 380;
pub const HEIGHT: u32 = 420;

const TILE_W: f32 = 169.0;
const TILE_H: f32 = 86.0;
const SLIDER_W: f32 = 324.0;

pub fn build_with_integrations(
    _: Size,
    network: Rc<dyn NetworkIntegration>,
    brightness: Rc<dyn BrightnessIntegration>,
    battery: Rc<dyn BatteryIntegration>,
    volume: Rc<dyn VolumeIntegration>,
    bluetooth: Rc<dyn BluetoothIntegration>,
    toggle_awake: Rc<dyn Fn()>,
    awake: bool,
    brightness_level: Signal<f32>,
    volume_level: Signal<f32>,
    wifi_enabled: Signal<bool>,
    bluetooth_powered: Signal<bool>,
    tray: &TrayConfig,
) -> BoxedWidget {
    let wifi_on = wifi_enabled.get();
    let connected = wifi_on && network.connected();
    let network_name = if wifi_on {
        network
            .network_name()
            .unwrap_or_else(|| "No network".into())
    } else {
        "Off".into()
    };
    let wifi_icon = if !wifi_on {
        "wifi-off"
    } else {
        match network.strength().unwrap_or(100) {
            75.. => "wifi-high",
            50..=74 => "wifi-mid",
            25..=49 => "wifi-low",
            _ => "wifi-none",
        }
    };
    let wifi_caption = if connected {
        "Connected"
    } else if wifi_on {
        "Disconnected"
    } else {
        "Radio off"
    };

    let bluetooth_on = bluetooth_powered.get();
    let bluetooth_connected = bluetooth_on && bluetooth.connected();
    let bluetooth_icon = if bluetooth_connected {
        "bluetooth-connected"
    } else if bluetooth_on {
        "bluetooth-on"
    } else {
        "bluetooth-off"
    };
    let bluetooth_name = if bluetooth_connected {
        bluetooth
            .device_name()
            .unwrap_or_else(|| "Connected".into())
    } else if bluetooth_on {
        "On".into()
    } else {
        "Off".into()
    };
    let bluetooth_caption = if bluetooth_connected {
        "Connected"
    } else if bluetooth_on {
        "Available"
    } else {
        "Powered off"
    };

    let battery_percent = battery.percentage();
    let battery_icon = if battery.charging() {
        "battery-bolt"
    } else {
        match battery_percent.unwrap_or(0) {
            80.. => "battery-full",
            50..=79 => "battery-mid",
            20..=49 => "battery-low",
            _ => "battery-empty",
        }
    };
    let battery_value = battery_percent
        .map(|percent| format!("{percent}%"))
        .unwrap_or_else(|| "--".into());
    let battery_caption = if battery_percent.is_none() {
        "(Unavailable)"
    } else if battery.charging() {
        "(Charging)"
    } else {
        ""
    };

    let brightness_value = brightness_level.get();
    let volume_value = volume_level.get();
    let brightness_text = format!("{}%", (brightness_value * 100.0).round());
    let volume_text = format!(
        "{}%{}",
        (volume_value * 100.0).round(),
        if volume.muted() { "  M" } else { "" }
    );
    let volume_icon = if volume.muted() || volume_value <= 0.01 {
        "volume-mute"
    } else if volume_value < 0.34 {
        "volume-low"
    } else if volume_value < 0.67 {
        "volume-mid"
    } else {
        "volume-max"
    };

    let brightness_backend = brightness.clone();
    let volume_backend = volume.clone();
    let set_brightness = brightness_level.clone();
    let set_volume = volume_level.clone();
    let toggle_wifi = {
        let network = network.clone();
        let wifi_enabled = wifi_enabled.clone();
        Rc::new(move || {
            let next = !wifi_enabled.get();
            wifi_enabled.set(next);
            network.set_enabled(next);
        })
    };
    let toggle_bluetooth = {
        let bluetooth = bluetooth.clone();
        let bluetooth_powered = bluetooth_powered.clone();
        Rc::new(move || {
            let next = !bluetooth_powered.get();
            bluetooth_powered.set(next);
            bluetooth.set_powered(next);
        })
    };

    let mut device_row = Flex::row().gap(10.0);
    if tray.wifi.shows_in_panel() {
        device_row = device_row.child(device_tile(
            wifi_icon,
            "Wi-Fi",
            network_name,
            wifi_caption,
            wifi_on,
            toggle_wifi,
        ));
    }
    if tray.bluetooth.shows_in_panel() {
        device_row = device_row.child(device_tile(
            bluetooth_icon,
            "Bluetooth",
            bluetooth_name,
            bluetooth_caption,
            bluetooth_on,
            toggle_bluetooth,
        ));
    }

    let mut status_row = Flex::row().gap(10.0);
    if tray.battery.shows_in_panel() {
        status_row = status_row.child(status_tile(
            battery_icon,
            "Battery",
            battery_value,
            battery_caption,
        ));
    }
    status_row = status_row.child(awake_tile(awake, toggle_awake));

    let mut sliders = Flex::column()
        .grow(1.0)
        .padding(12.0)
        .gap(12.0)
        .justify(creamui_widgets::layout::Justify::Center)
        .property(StyleProp::Background(PANEL.into()))
        .property(StyleProp::Border(Border::new(BORDER, 1.0)))
        .property(StyleProp::CornerRadius(ISLAND_RADIUS));
    if tray.brightness.shows_in_panel() {
        sliders = sliders.child(fat_slider(
            "brightness",
            "Brightness",
            brightness_value,
            brightness_text,
            move |level| {
                set_brightness.set(level);
                brightness_backend.set_level(level);
            },
        ));
    }
    if tray.volume.shows_in_panel() {
        sliders = sliders.child(fat_slider(
            volume_icon,
            "Volume",
            volume_value,
            volume_text,
            move |level| {
                set_volume.set(level);
                volume_backend.set_level(level);
            },
        ));
    }

    Box::new(jsx! {
        <Flex direction={FlexDirection::Column} size={(WIDTH as f32, HEIGHT as f32)} padding={16.0} gap={12.0} background={CARD} border={(BORDER, 1.0)} corner_radius={CARD_RADIUS}>
            <RawText color={MUTED} font_size={14.0}>"CONTROL"</RawText>
            {Box::new(device_row) as BoxedWidget}
            {Box::new(status_row) as BoxedWidget}
            {Box::new(sliders) as BoxedWidget}
        </Flex>
    })
}

fn device_tile(
    icon: &str,
    kicker: &str,
    name: String,
    caption: &str,
    enabled: bool,
    on_toggle: Rc<dyn Fn()>,
) -> BoxedWidget {
    let mut name = RawMarquee::new(name, TEXT, 15.0, TILE_W - 20.0);
    name.style.layout.size.height = creamui_core::layout::Dimension::Length(18.0);
    let name = Box::new(name) as BoxedWidget;
    Box::new(jsx! {
        <Flex direction={FlexDirection::Column} size={(TILE_W, TILE_H)} padding={10.0} gap={7.0} background={PANEL} border={(BORDER, 1.0)} corner_radius={ISLAND_RADIUS}>
            <Flex direction={FlexDirection::Row} gap={8.0} align={Align::Center}>
                {pixel_icon(icon, 16.0)}
                <RawText color={MUTED} font_size={11.0} align={TextAlign::Start}>{kicker}</RawText>
                <Flex grow={1.0} />
                {compact_switch(enabled, on_toggle)}
            </Flex>
            {name}
            <RawText color={MUTED} font_size={10.0} align={TextAlign::Start}>{caption}</RawText>
        </Flex>
    })
}

fn compact_switch(checked: bool, on_toggle: Rc<dyn Fn()>) -> BoxedWidget {
    let mut toggle = RawSwitch::new(checked, FILL, TRACK, TEXT, move || on_toggle())
        .radii(10.0, 8.0)
        .thumb_inset(2.0)
        .hover_colors(Color::rgb(230, 235, 255), TRACK)
        .pressed_colors(FILL, TRACK);
    toggle.style = Style::new().layout(LayoutStyle {
        size: fixed(36.0, 20.0),
        ..Default::default()
    });
    Box::new(toggle)
}

fn status_tile(icon: &str, kicker: &str, value: String, caption: &str) -> BoxedWidget {
    Box::new(jsx! {
        <Flex direction={FlexDirection::Column} size={(TILE_W, TILE_H)} padding={10.0} gap={6.0} background={PANEL} border={(BORDER, 1.0)} corner_radius={ISLAND_RADIUS} justify={creamui_widgets::layout::Justify::Center}>
            <Flex direction={FlexDirection::Row} gap={8.0} align={Align::Center}>
                {pixel_icon(icon, 16.0)}
                <RawText color={MUTED} font_size={11.0} align={TextAlign::Start}>{kicker}</RawText>
            </Flex>
            <Flex direction={FlexDirection::Row} align={Align::Center} gap={8.0}>
                <RawText color={TEXT} font_size={18.0} align={TextAlign::Start}>{value}</RawText>
                <RawText color={MUTED} font_size={11.0} align={TextAlign::Start}>{caption}</RawText>
            </Flex>
        </Flex>
    })
}

fn awake_tile(awake: bool, toggle_awake: Rc<dyn Fn()>) -> BoxedWidget {
    let caption = if awake { "Holding" } else { "Idle" };
    Box::new(jsx! {
        <Flex direction={FlexDirection::Column} size={(TILE_W, TILE_H)} padding={10.0} gap={6.0} background={PANEL} border={(BORDER, 1.0)} corner_radius={ISLAND_RADIUS} justify={creamui_widgets::layout::Justify::Center}>
            <Flex direction={FlexDirection::Row} gap={8.0} align={Align::Center}>
                <RawText color={MUTED} font_size={11.0} align={TextAlign::Start}>"Keep awake"</RawText>
                <Flex grow={1.0} />
                {compact_switch(awake, toggle_awake)}
            </Flex>
            <RawText color={TEXT} font_size={15.0} align={TextAlign::Start}>{caption}</RawText>
        </Flex>
    })
}

fn fat_slider(
    icon: &str,
    label: &str,
    value: f32,
    value_text: String,
    on_change: impl Fn(f32) + 'static,
) -> BoxedWidget {
    let slider: BoxedWidget = Box::new(
        RawSlider::new(
            Style::new()
                .layout(LayoutStyle {
                    size: fixed(SLIDER_W, 32.0),
                    ..Default::default()
                })
                .focus(StateStyle::new().outline(Color::rgba(0, 0, 0, 0), 0.0)),
            value,
            TRACK,
            FILL,
            TEXT,
            on_change,
        )
        .track(22.0, 11.0)
        .handle(22.0, 8.0)
        .hover_handle_color(Color::rgb(255, 255, 255))
        .pressed_handle_color(FILL),
    );
    Box::new(jsx! {
        <Flex direction={FlexDirection::Column} gap={8.0}>
            <Flex direction={FlexDirection::Row} align={Align::Center} gap={8.0}>
                {pixel_icon(icon, 18.0)}
                <RawText color={TEXT} font_size={13.0}>{label}</RawText>
                <Flex grow={1.0} />
                <RawText color={MUTED} font_size={18.0}>{value_text}</RawText>
            </Flex>
            {slider}
        </Flex>
    })
}
