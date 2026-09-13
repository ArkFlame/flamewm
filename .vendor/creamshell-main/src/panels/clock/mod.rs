use crate::panels::chrome::{BORDER, CARD, CARD_RADIUS, MUTED, PANEL, TEXT};
use chrono::Local;
use creamui_core::layout::{FlexDirection, Style as LayoutStyle};
use creamui_core::{BoxedWidget, Painter, Rect, Size, Style, TextAlign, Widget};
use creamui_macros::jsx;
use creamui_widgets::layout::{fixed, Align, Justify};

pub const WIDTH: u32 = 280;
pub const HEIGHT: u32 = 188;

pub fn build(_: Size) -> BoxedWidget {
    Box::new(jsx! {
        <Flex direction={FlexDirection::Column} size={(WIDTH as f32, HEIGHT as f32)} padding={16.0} gap={12.0} background={CARD} border={(BORDER, 1.0)} corner_radius={CARD_RADIUS}>
            <Flex direction={FlexDirection::Column} size={(248.0, 122.0)} padding={10.0} gap={4.0} align={Align::Center} justify={Justify::Center} background={PANEL} border={(BORDER, 1.0)} corner_radius={14.0}>
                {Box::new(LiveTime) as BoxedWidget}
                {Box::new(LiveDate) as BoxedWidget}
            </Flex>
            {Box::new(LiveFooter) as BoxedWidget}
        </Flex>
    })
}

struct LiveTime;
struct LiveDate;
struct LiveFooter;

impl Widget for LiveTime {
    fn style(&self) -> Style {
        Style::new().layout(LayoutStyle {
            size: fixed(228.0, 64.0),
            ..Default::default()
        })
    }

    fn paint(&self, painter: &mut dyn Painter, rect: Rect) {
        painter.animation_time();
        let time = Local::now().format("%H:%M").to_string();
        painter.fill_text_font(
            rect,
            &time,
            TEXT,
            56.0,
            TextAlign::Center,
            None,
            false,
            false,
        );
    }
}

impl Widget for LiveDate {
    fn style(&self) -> Style {
        Style::new().layout(LayoutStyle {
            size: fixed(228.0, 22.0),
            ..Default::default()
        })
    }

    fn paint(&self, painter: &mut dyn Painter, rect: Rect) {
        painter.animation_time();
        let date = Local::now().format("%A, %-d de %B").to_string();
        painter.fill_text_font(
            rect,
            &date,
            MUTED,
            14.0,
            TextAlign::Center,
            None,
            false,
            false,
        );
    }
}

impl Widget for LiveFooter {
    fn style(&self) -> Style {
        Style::new().layout(LayoutStyle {
            size: fixed(248.0, 22.0),
            ..Default::default()
        })
    }

    fn paint(&self, painter: &mut dyn Painter, rect: Rect) {
        painter.animation_time();
        let now = Local::now();
        let weekday = now.format("%a").to_string().to_uppercase();
        let year = now.format("%Y").to_string();
        painter.fill_text_font(
            rect,
            &weekday,
            MUTED,
            14.0,
            TextAlign::Start,
            None,
            false,
            false,
        );
        painter.fill_text_font(rect, &year, MUTED, 14.0, TextAlign::End, None, false, false);
    }
}
