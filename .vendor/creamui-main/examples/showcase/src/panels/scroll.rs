use crate::prelude::*;

/// A row of numbered list items long enough to overflow a fixed-height
/// scroll view, used by both lists in [`ScrollPanel`].
fn scroll_rows(count: usize, row_style: Style, text_color: Color) -> Vec<BoxedWidget> {
    let theme = use_theme();
    let text_style = Style {
        size: creamui_core::layout::Size {
            width: Dimension::Percent(1.0),
            height: Dimension::Percent(1.0),
        },
        ..Default::default()
    };
    (0..count)
        .map(|i| {
            let background = if i % 2 == 0 {
                theme.surface_elevated
            } else {
                theme.surface
            };
            Box::new(jsx! {
                <RawView style={row_style.clone()} background={background}>
                    <RawText color={text_color} font_size={13.0} align={TextAlign::Start} style={text_style.clone()}>{format!("Row {:02}", i + 1)}</RawText>
                </RawView>
            }) as BoxedWidget
        })
        .collect()
}

/// The "Scroll" panel: a themed `ScrollView` and a hand-colored
/// `RawScrollView` side by side, each holding a long enough list to show
/// off the draggable `RawScrollbar` overlay — drag either thumb, or turn
/// the mouse wheel over either list, and they stay in sync.
#[component]
pub fn ScrollPanel(
    themed_scroll: ScrollController,
    custom_scroll: ScrollController,
) -> BoxedWidget {
    let theme = use_theme();
    const ROWS: usize = 28;
    let list_style = Style {
        size: creamui_core::layout::Size {
            width: Dimension::Length(240.0),
            height: Dimension::Length(280.0),
        },
        flex_shrink: 0.0,
        ..Default::default()
    };
    let row_style = padding(
        Style {
            size: creamui_core::layout::Size {
                width: Dimension::Percent(1.0),
                height: Dimension::Length(34.0),
            },
            align_items: Some(AlignItems::Center),
            ..Default::default()
        },
        theme.spacing_medium,
    );

    const NEON: Color = Color::rgb(0x5c, 0xe1, 0xff);
    let themed_rows = scroll_rows(ROWS, row_style.clone(), theme.text_primary);
    let custom_rows = scroll_rows(ROWS, row_style, Color::rgb(0xbf, 0xef, 0xff));

    let themed_list: BoxedWidget = jsx! {
        <ControlledScrollView style={list_style.clone()} controller={themed_scroll} children={themed_rows} />
    };
    let custom_list: BoxedWidget = jsx! {
        <RawScrollView
            style={list_style}
            controller={custom_scroll}
            content_gap={None}
            scrollbar_gap={None}
            background={Some(Color::rgb(0x0c, 0x14, 0x1a))}
            corner_radius={Some(theme.radius_medium)}
            scrollbar_width={Some(7.0)}
            scrollbar_color={Some(Color::rgba(NEON.r, NEON.g, NEON.b, 150))}
            scrollbar_hover_color={Some(Color::rgba(NEON.r, NEON.g, NEON.b, 220))}
            children={custom_rows}
        />
    };

    Box::new(jsx! {
        <RawView style={column(section_gap())}>
            <SectionHeader title={"Scroll".to_owned()} subtitle={"A draggable scrollbar thumb tracks the mouse wheel automatically, and vice versa.".to_owned()} />
            <CardRow>
                <FieldCard label={"Themed · ScrollView".to_owned()} control={themed_list} />
                <FieldCard label={"Custom · RawScrollView".to_owned()} control={custom_list} />
            </CardRow>
        </RawView>
    })
}
