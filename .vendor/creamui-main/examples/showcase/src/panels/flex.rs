//! Live examples of CreamUI's flex layout primitives.
//!
//! Every container sizes itself from its parent natively — percentage
//! widths and `flex-wrap` — instead of a pixel width computed by hand in
//! Rust. Resize the window: the wrap points recompute as part of ordinary
//! layout.

use crate::prelude::*;

const GAP: f32 = 14.0;
const CARD_MIN_WIDTH: f32 = 200.0;
const CARD_HEIGHT: f32 = 132.0;

/// One wrapping card: `flex: 1 1 200px` with an explicit `min_width` opting
/// back into "never shrink below this" — the one case on this page where
/// that CSS default is actually what you want, since a narrower card would
/// clip its own title. `min_width`/`height` aren't `jsx!`'s `Flex` props, so
/// they're chained onto the constructed value below.
#[component]
fn WrapCard(title: String, description: String, accent: Color) -> BoxedWidget {
    let theme = use_theme();
    Box::new(
        jsx! {
            <Flex direction={FlexDirection::Column} basis={CARD_MIN_WIDTH} grow={1.0} gap={8.0} padding={16.0} background={theme.surface_elevated} corner_radius={theme.card_radius}>
                <BoldText text={title} color={accent} font_size={17.0} align={TextAlign::Center} />
                <RawText color={theme.text_secondary} font_size={13.0}>{description}</RawText>
            </Flex>
        }
        .min_width(CARD_MIN_WIDTH)
        .height(CARD_HEIGHT),
    )
}

/// A wrapping flex row: each card comes from [`WrapCard`].
fn flex_wrap_row() -> BoxedWidget {
    let theme = use_theme();
    let cards = [
        (
            "Wrap",
            "Cards flow onto another line once the row runs out of width.",
            theme.accent,
        ),
        (
            "Gap",
            "Horizontal and vertical gaps can be set independently.",
            Color::rgb(0x8b, 0x5c, 0xf6),
        ),
        (
            "Alignment",
            "Items can be centered on both axes.",
            Color::rgb(0x22, 0xc5, 0x5e),
        ),
        (
            "Grow",
            "flex: 1 1 200px — grows to fill the row, wraps once it can't.",
            Color::rgb(0xf5, 0x9e, 0x0b),
        ),
    ];
    let children: Vec<BoxedWidget> = cards
        .into_iter()
        .map(|(title, description, accent)| {
            jsx! { <WrapCard title={title.to_owned()} description={description.to_owned()} accent={accent} /> }
        })
        .collect();
    Box::new(jsx! {
        <Flex full_width={true} gap={GAP} wrap={Wrap::Wrap} align_content={Justify::Start} children={children} />
    })
}

/// A toolbar: `justify-content: space-between` with centered items,
/// stretched to the panel's width instead of a pixel width. `height` isn't
/// one of `jsx!`'s `Flex` props, so it's chained onto the built value.
fn toolbar() -> BoxedWidget {
    let theme = use_theme();
    Box::new(
        jsx! {
            <Flex full_width={true} align={Align::Center} justify={Justify::Between} padding={theme.spacing_medium} background={theme.surface_elevated} corner_radius={theme.card_radius}>
                <Text font_size={17.0}>"Flex, without the style boilerplate"</Text>
                <RawText color={theme.accent} font_size={13.0} style={Style::default().padding_all(8.0)}>"display: flex"</RawText>
            </Flex>
        }
        .height(50.0),
    )
}

/// Flex examples: a wrapping card row and a toolbar — every container
/// stretches or shrinks with its parent instead of a pixel width computed
/// by hand.
#[allow(non_snake_case)]
pub fn FlexPanel() -> BoxedWidget {
    let wrap_row = flex_wrap_row();
    let toolbar_widget = toolbar();
    Box::new(jsx! {
        <RawView style={column(section_gap())}>
            <SectionHeader
                title={"Flex".to_owned()}
                subtitle={"Flex containers that size themselves from whatever space their parent actually has.".to_owned()}
            />
            <FieldCard label={"Flex::row · Wrap::Wrap · gap · align_content".to_owned()} control={wrap_row} />
            <FieldCard label={"Toolbar · justify-content · align-items".to_owned()} control={toolbar_widget} />
        </RawView>
    })
}
