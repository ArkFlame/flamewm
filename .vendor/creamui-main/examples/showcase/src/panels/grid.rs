//! Live examples of CreamUI's grid layout primitives.
//!
//! Every cell sizes itself from its parent natively — percentage widths and
//! a `repeat(auto-fit, minmax(..))` grid template — instead of a pixel
//! width computed by hand in Rust. Resize the window: the column count
//! recomputes as part of ordinary layout.

use crate::prelude::*;

const GAP: f32 = 14.0;
const GALLERY_MIN_CARD_WIDTH: f32 = 176.0;
const GALLERY_CARD_HEIGHT: f32 = 96.0;

/// A card that fills whatever grid cell it's placed in, rather than
/// carrying its own pixel dimensions.
#[component]
fn DemoCard(title: String, description: String, accent: Color) -> BoxedWidget {
    let theme = use_theme();
    Box::new(jsx! {
        <Flex direction={FlexDirection::Column} fill={true} gap={theme.spacing_small} padding={theme.spacing_large} background={theme.surface_elevated} corner_radius={theme.card_radius}>
            <BoldText text={title} color={accent} font_size={16.0} align={TextAlign::Center} />
            <RawText color={theme.text_secondary} font_size={13.0} style={Style::default().grow(1.0)}>{description}</RawText>
        </Flex>
    })
}

/// A fixed-height gallery card that stretches to whatever width its
/// `auto_fit_columns` track resolves to.
#[component]
fn GalleryCard(title: String, description: String, accent: Color) -> BoxedWidget {
    let theme = use_theme();
    Box::new(
        jsx! {
            <Flex direction={FlexDirection::Column} full_width={true} gap={6.0} padding={theme.spacing_medium} background={theme.surface_elevated} corner_radius={theme.card_radius}>
                <BoldText text={title} color={accent} font_size={15.0} align={TextAlign::Center} />
                <RawText color={theme.text_secondary} font_size={12.0}>{description}</RawText>
            </Flex>
        }
        .height(GALLERY_CARD_HEIGHT),
    )
}

/// A bento box: three explicit cells on a 3-column, 2-row `fr` grid. Spans
/// are the one thing that genuinely needs an explicit template — there's no
/// way to say "twice as wide as your neighbor" without naming a track — but
/// every cell still just says `fill()` and lets the grid resolve the actual
/// pixels. `full_width`/`height` aren't `jsx!`'s `Grid` props, so they're
/// chained onto the constructed value below.
fn bento_grid() -> BoxedWidget {
    let theme = use_theme();
    let overview = jsx! { <DemoCard title={"Overview".to_owned()} description={"Column 1, row 1 · spans 2 columns and 2 rows.".to_owned()} accent={theme.accent} /> };
    let cell = jsx! { <DemoCard title={"Cell".to_owned()} description={"Targets one explicit cell.".to_owned()} accent={Color::rgb(0x8b, 0x5c, 0xf6)} /> };
    let tracks = jsx! { <DemoCard title={"Tracks".to_owned()} description={"Three 1fr columns share the width evenly.".to_owned()} accent={Color::rgb(0x22, 0xc5, 0x5e)} /> };
    Box::new(
        jsx! {
            <Grid
                template_columns={[Track::fr(1.0), Track::fr(1.0), Track::fr(1.0)]}
                template_rows={[Track::fr(1.0), Track::fr(1.0)]}
                gap={GAP}
            >
                <GridItem column={1} row={1} column_span={2} row_span={2}>{overview}</GridItem>
                <GridItem column={3} row={1}>{cell}</GridItem>
                <GridItem column={3} row={2}>{tracks}</GridItem>
            </Grid>
        }
        .full_width()
        .height(220.0),
    )
}

/// A card gallery on `repeat(auto-fit, minmax(176px, 1fr))`: as many
/// 176px-or-wider columns as fit the row, sharing whatever's left evenly,
/// collapsing unused tracks instead of leaving gaps. Taffy resolves the
/// column count during layout — this function never sees a pixel width.
/// `full_width`/`auto_fit_columns` aren't `jsx!`'s `Grid` props, so they're
/// chained onto the constructed value below.
fn gallery_grid() -> BoxedWidget {
    let cards = [
        (
            "Auto-fit",
            "repeat(auto-fit, minmax(176px, 1fr)) — no column count in Rust.",
            None,
        ),
        (
            "Shrink-safe",
            "min-size defaults to 0, so a card never overflows a tight row.",
            Some(Color::rgb(0x8b, 0x5c, 0xf6)),
        ),
        (
            "Stretch",
            "Each column shares the remaining width evenly.",
            Some(Color::rgb(0x22, 0xc5, 0x5e)),
        ),
        (
            "Resize me",
            "Resize the window — the column count recomputes during layout.",
            Some(Color::rgb(0xf5, 0x9e, 0x0b)),
        ),
        (
            "Collapses",
            "Too few cards for a row, and the extra track is just unused.",
            Some(Color::rgb(0xec, 0x48, 0x99)),
        ),
        (
            "No overlap",
            "Each card is a plain vertical Flex column underneath.",
            Some(Color::rgb(0x06, 0xb6, 0xd4)),
        ),
    ];
    let theme = use_theme();
    let children: Vec<BoxedWidget> = cards
        .into_iter()
        .map(|(title, description, accent)| {
            let card = jsx! {
                <GalleryCard
                    title={title.to_owned()}
                    description={description.to_owned()}
                    accent={accent.unwrap_or(theme.accent)}
                />
            };
            Box::new(jsx! { <GridItem>{card}</GridItem> }) as BoxedWidget
        })
        .collect();
    Box::new(
        jsx! { <Grid gap={GAP} children={children} /> }
            .full_width()
            .auto_fit_columns(GALLERY_MIN_CARD_WIDTH),
    )
}

/// Grid examples: a bento box (explicit spans) and an auto-fit gallery —
/// every cell sized from its parent, none of it carrying a pixel width
/// computed by hand.
#[allow(non_snake_case)]
pub fn GridPanel() -> BoxedWidget {
    let bento = bento_grid();
    let gallery = gallery_grid();
    Box::new(jsx! {
        <RawView style={column(section_gap())}>
            <SectionHeader
                title={"Grid".to_owned()}
                subtitle={"Grid containers that size themselves from whatever space their parent actually has.".to_owned()}
            />
            <FieldCard label={"Grid · GridItem::at · column_span · row_span · fr tracks".to_owned()} control={bento} />
            <FieldCard label={"Grid::auto_fit_columns · repeat(auto-fit, minmax(176px, 1fr))".to_owned()} control={gallery} />
        </RawView>
    })
}
