use crate::prelude::*;

/// The "Sidebar" panel: a small, self-contained `Sidebar`/`SidebarItem` demo
/// with its own selection state, next to the panel it controls.
#[component]
pub fn SidebarPanel(active: Signal<usize>) -> BoxedWidget {
    let theme = use_theme();
    const ITEMS: [&str; 3] = ["Inbox", "Drafts", "Sent"];
    let colors = TabColors::sidebar();
    let item_style = padding(
        Style {
            size: creamui_core::layout::Size {
                width: Dimension::Percent(1.0),
                height: Dimension::Length(36.0),
            },
            align_items: Some(AlignItems::Center),
            ..Default::default()
        },
        theme.spacing_medium,
    );
    let rail_style = padding(
        Style {
            size: creamui_core::layout::Size {
                width: Dimension::Length(176.0),
                height: Dimension::Percent(1.0),
            },
            flex_shrink: 0.0,
            ..column(theme.spacing_small)
        },
        theme.spacing_medium,
    );
    let items: Vec<BoxedWidget> = ITEMS
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let is_active = active.get() == index;
            let select = active.clone();
            jsx! {
                <SidebarItem
                    colors={colors}
                    style={item_style.clone()}
                    label={(*label).to_owned()}
                    active={is_active}
                    on_click={Box::new(move || select.set(index)) as Box<dyn Fn()>}
                />
            }
        })
        .collect();
    let preview_style = padding(
        Style {
            size: creamui_core::layout::Size {
                width: Dimension::Length(264.0),
                height: Dimension::Percent(1.0),
            },
            ..column(theme.spacing_medium)
        },
        theme.spacing_large,
    );
    let current_label = ITEMS[active.get()].to_owned();
    Box::new(jsx! {
        <RawView style={column(section_gap())}>
            <SectionHeader title={"Sidebar".to_owned()} subtitle={"A compact navigation rail with independent selection.".to_owned()} />
            <Surface
                role={SurfaceRole::Inset}
                style={padding(Style {
                    size: creamui_core::layout::Size { width: Dimension::Length(488.0), height: Dimension::Length(192.0) },
                    align_items: Some(AlignItems::Stretch),
                    ..row(theme.spacing_large)
                }, theme.spacing_medium)}
            >
                <Sidebar colors={colors} style={rail_style} children={items} />
                <PreviewCard style={preview_style}>
                    <Heading>{current_label.clone()}</Heading>
                    <Text secondary={true}>"The selected section is shown here."</Text>
                </PreviewCard>
            </Surface>
        </RawView>
    })
}
