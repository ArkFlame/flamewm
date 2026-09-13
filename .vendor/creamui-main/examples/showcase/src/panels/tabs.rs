use crate::prelude::*;

const TAB_LABELS: [&str; 3] = ["Overview", "Activity", "Settings"];
fn tab_bar(
    labels: &[&str],
    controller: TabController,
    colors: TabColors,
    sizing: TabSizing,
    height: f32,
    tab_padding: f32,
    inset: f32,
) -> BoxedWidget {
    let bar_style = padding(
        Style {
            size: creamui_core::layout::Size {
                width: if sizing == TabSizing::Fill {
                    Dimension::Percent(1.0)
                } else {
                    Dimension::Auto
                },
                height: Dimension::Auto,
            },
            flex_shrink: 0.0,
            align_self: if sizing == TabSizing::Fill {
                None
            } else {
                Some(creamui_core::layout::AlignSelf::Start)
            },
            ..row(colors.gap)
        },
        inset,
    );
    let styles = tab_styles(labels, sizing, height, tab_padding);
    let tabs: Vec<BoxedWidget> = labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let tabs = controller.clone();
            jsx! {
                <Tab
                    colors={colors}
                    style={styles[index].clone()}
                    label={(*label).to_owned()}
                    active={controller.is_selected(index)}
                    on_click={Box::new(move || tabs.select(index)) as Box<dyn Fn()>}
                />
            }
        })
        .collect();
    jsx! { <Tabs colors={colors} style={bar_style} children={tabs} /> }
}

fn tab_example(label: &str, bar: BoxedWidget) -> BoxedWidget {
    let theme = use_theme();
    Box::new(jsx! {
        <RawView style={column(theme.spacing_small)}>
            <BoldText text={label.to_owned()} color={theme.text_secondary} font_size={theme.typography.caption} align={TextAlign::Start} />
            {bar}
        </RawView>
    })
}

/// Filled, pill, and indicator treatments plus a content-linked tab set.
#[component]
pub fn TabsPanel(
    filled: TabController,
    pill: TabController,
    indicator: TabController,
    content: TabController,
) -> BoxedWidget {
    let theme = use_theme();
    let mut filled_colors = TabColors::dark();
    filled_colors.gap = theme.spacing_medium;

    let mut pill_colors = filled_colors;
    pill_colors.inactive_background = Some(theme.surface_hover);
    pill_colors.active_background = theme.surface_elevated;
    pill_colors.active_text = theme.text_primary;
    pill_colors.radius = 16.0;
    pill_colors.container_radius = 20.0;
    pill_colors.gap = theme.spacing_medium;

    let mut indicator_colors = filled_colors;
    indicator_colors.background = theme.surface;
    indicator_colors.inactive_background = None;
    indicator_colors.selection = SelectionStyle::Indicator;
    indicator_colors.radius = 0.0;
    indicator_colors.container_radius = 0.0;
    indicator_colors.gap = theme.spacing_large;

    let content_selected = content.selected().min(TAB_LABELS.len() - 1);
    let preview_style = padding(
        Style {
            size: creamui_core::layout::Size {
                width: Dimension::Percent(1.0),
                height: Dimension::Auto,
            },
            flex_grow: 1.0,
            ..column(theme.spacing_small)
        },
        theme.spacing_large,
    );
    let current_label = TAB_LABELS[content_selected];
    let (title, detail) = match content_selected {
        0 => (
            "Everything in one place",
            "A calm overview of what matters right now.",
        ),
        1 => (
            "You are all caught up",
            "New activity will appear here as it happens.",
        ),
        _ => (
            "Make it yours",
            "Preferences stay close without leaving this view.",
        ),
    };
    let filled_example = tab_example(
        "Filled tabs · content width",
        tab_bar(
            &TAB_LABELS,
            filled,
            filled_colors,
            TabSizing::Content,
            38.0,
            theme.spacing_medium,
            theme.spacing_small,
        ),
    );
    let pill_example = tab_example(
        "Pill tabs · equal width",
        tab_bar(
            &TAB_LABELS,
            pill,
            pill_colors,
            TabSizing::Equal,
            34.0,
            theme.spacing_medium,
            theme.spacing_small,
        ),
    );
    let indicator_example = tab_example(
        "Indicator tabs · content width",
        tab_bar(
            &TAB_LABELS,
            indicator,
            indicator_colors,
            TabSizing::Content,
            34.0,
            theme.spacing_medium,
            theme.spacing_small,
        ),
    );
    let content_bar = tab_bar(
        &TAB_LABELS,
        content,
        filled_colors,
        TabSizing::Equal,
        36.0,
        theme.spacing_medium,
        theme.spacing_small,
    );
    Box::new(jsx! {
        <RawView style={column(section_gap())}>
            <SectionHeader title={"Tabs".to_owned()} subtitle={"Three visual styles, followed by a tab bar connected to its content.".to_owned()} />
            <RawView style={column(theme.spacing_large)}>
                {filled_example}
                {pill_example}
                {indicator_example}
            </RawView>
            <Surface
                role={SurfaceRole::Inset}
                style={padding(Style {
                    size: creamui_core::layout::Size { width: Dimension::Length(488.0), height: Dimension::Length(184.0) },
                    ..column(theme.spacing_large)
                }, theme.spacing_medium)}
            >
                {content_bar}
                <PreviewCard style={preview_style}>
                    <Heading>{title.to_owned()}</Heading>
                    <Text secondary={true}>{detail.to_owned()}</Text>
                    <BoldText text={current_label.to_owned()} color={theme.accent} font_size={12.0} align={TextAlign::Start} />
                </PreviewCard>
            </Surface>
        </RawView>
    })
}
