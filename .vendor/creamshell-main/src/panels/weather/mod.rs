use crate::icons::pixel_icon;
use crate::panels::chrome::{BORDER, CARD, CARD_RADIUS, MUTED, PANEL, TEXT};
use creamui_core::layout::FlexDirection;
use creamui_core::{BoxedWidget, Size};
use creamui_macros::jsx;
use creamui_widgets::layout::{Align, Justify};

pub const WIDTH: u32 = 280;
pub const HEIGHT: u32 = 240;

#[derive(Clone, Copy)]
struct Hour {
    label: &'static str,
    icon: &'static str,
    temp: &'static str,
}

const HOURS: [Hour; 5] = [
    Hour {
        label: "Now",
        icon: "weather-sun-cloud",
        temp: "24",
    },
    Hour {
        label: "13",
        icon: "weather-sun",
        temp: "25",
    },
    Hour {
        label: "16",
        icon: "weather-cloudly",
        temp: "23",
    },
    Hour {
        label: "19",
        icon: "weather-rain",
        temp: "20",
    },
    Hour {
        label: "22",
        icon: "weather-wind",
        temp: "18",
    },
];

pub fn build(_: Size) -> BoxedWidget {
    let mut strip = creamui_widgets::layout::Flex::row()
        .gap(8.0)
        .justify(Justify::Between);
    for hour in HOURS {
        strip = strip.child(hour_cell(hour));
    }

    Box::new(jsx! {
        <Flex direction={FlexDirection::Column} size={(WIDTH as f32, HEIGHT as f32)} padding={18.0} gap={14.0} background={CARD} border={(BORDER, 1.0)} corner_radius={CARD_RADIUS}>
            <Flex direction={FlexDirection::Row} gap={14.0} align={Align::Center}>
                {pixel_icon("weather-sun-cloud", 52.0)}
                <Flex direction={FlexDirection::Column} gap={6.0} grow={1.0}>
                    <Flex direction={FlexDirection::Row} align={Align::End} gap={8.0}>
                        <RawText color={TEXT} font_size={44.0}>"24°"</RawText>
                        <RawText color={MUTED} font_size={16.0}>"Clear"</RawText>
                    </Flex>
                    <RawText color={MUTED} font_size={13.0}>"Buenos Aires · Now"</RawText>
                </Flex>
            </Flex>
            <Flex direction={FlexDirection::Row} gap={10.0}>
                {stat_chip("H", "27")}
                {stat_chip("L", "17")}
            </Flex>
            <Flex direction={FlexDirection::Row} padding={8.0} background={PANEL} border={(BORDER, 1.0)} corner_radius={12.0}>
                {Box::new(strip) as BoxedWidget}
            </Flex>
        </Flex>
    })
}

fn stat_chip(label: &str, value: &str) -> BoxedWidget {
    Box::new(jsx! {
        <Flex direction={FlexDirection::Row} grow={1.0} padding={10.0} gap={10.0} align={Align::Center} justify={Justify::Center} background={PANEL} border={(BORDER, 1.0)} corner_radius={10.0}>
            <RawText color={MUTED} font_size={12.0}>{label}</RawText>
            <RawText color={TEXT} font_size={16.0}>{value}</RawText>
        </Flex>
    })
}

fn hour_cell(hour: Hour) -> BoxedWidget {
    Box::new(jsx! {
        <Flex direction={FlexDirection::Column} size={(44.0, 64.0)} gap={5.0} align={Align::Center} justify={Justify::Center}>
            <RawText color={MUTED} font_size={10.0}>{hour.label}</RawText>
            {pixel_icon(hour.icon, 18.0)}
            <RawText color={TEXT} font_size={13.0}>{hour.temp}</RawText>
        </Flex>
    })
}
