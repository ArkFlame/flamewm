use crate::prelude::*;

/// A pill-shaped selectable button, used for the dark/light and accent
/// pickers on the Appearance page. Selection is fully controlled: it carries
/// no state of its own.
fn pill(label: &str, active: bool, on_click: impl Fn() + 'static) -> BoxedWidget {
    jsx! { <Choice label={label.to_owned()} active={active} on_click={Box::new(on_click) as Box<dyn Fn()>} /> }
}

/// A single accent color swatch: a rounded square filled with the color
/// itself, with a ring drawn around whichever one is active.
fn swatch(color: Color, active: bool, on_click: impl Fn() + 'static) -> BoxedWidget {
    let theme = use_theme();
    let outer = Style {
        size: fixed(40.0, 40.0),
        justify_content: Some(JustifyContent::Center),
        align_items: Some(AlignItems::Center),
        flex_shrink: 0.0,
        ..Default::default()
    };
    let inner_size = if active { 28.0 } else { 32.0 };
    let inner = Style {
        size: fixed(inner_size, inner_size),
        ..Default::default()
    };
    let ring = if active {
        theme.text_primary
    } else {
        theme.surface
    };
    Box::new(jsx! {
        <RawButton style={outer} background={ring} corner_radius={20.0} on_click={on_click}>
            <RawView style={inner} background={color} corner_radius={16.0} />
        </RawButton>
    })
}

/// The "Appearance" panel: toggles dark/light mode and picks an accent
/// color, both stored in `Signal`s owned by `main`, so changes are visible
/// immediately across every other section.
#[component]
pub fn AppearancePanel(dark_mode: Signal<bool>, accent_index: Signal<usize>) -> BoxedWidget {
    let theme = use_theme();
    let is_dark = dark_mode.get();
    let selected_accent = accent_index.get();

    let dark_flag = dark_mode.clone();
    let light_flag = dark_mode.clone();
    let mode_pills: Vec<BoxedWidget> = vec![
        pill("Dark", is_dark, move || dark_flag.set(true)),
        pill("Light", !is_dark, move || light_flag.set(false)),
    ];

    let mut swatches: Vec<BoxedWidget> = Vec::new();
    for (index, (_, color)) in ACCENTS.iter().enumerate() {
        let set_accent = accent_index.clone();
        swatches.push(swatch(*color, index == selected_accent, move || {
            set_accent.set(index)
        }));
    }

    let accent_name = ACCENTS[selected_accent].0;
    let summary = format!(
        "{} mode · {} accent",
        if is_dark { "Dark" } else { "Light" },
        accent_name
    );

    let primary_mode = dark_mode.clone();
    let secondary_mode = dark_mode.clone();

    Box::new(jsx! {
        <RawView style={column(section_gap())}>
            <SectionHeader title={"Appearance".to_owned()} subtitle={"Explore the same components in a different light.".to_owned()} />
            <Card gap={theme.spacing_large}>
                <Card gap={theme.spacing_small}>
                    <FieldLabel text={"Mode".to_owned()} />
                    <RawView style={row(theme.spacing_medium)} children={mode_pills} />
                </Card>
                <Card gap={theme.spacing_small}>
                    <FieldLabel text={"Accent color".to_owned()} />
                    <RawView style={row(theme.spacing_medium)} children={swatches} />
                </Card>
                <Text align={TextAlign::Start} color={theme.text_disabled} style={label_style()}>{summary}</Text>
            </Card>
            <Card gap={theme.spacing_medium}>
                <Heading>"Component preview"</Heading>
                <Text secondary={true} align={TextAlign::Start}>"Open a category to explore sizes, states, and interactions."</Text>
                <CardRow>
                    <StyledButton variant={ButtonVariant::Primary} size={ButtonSize::Md} label={"Primary".to_owned()} state={ButtonState::Normal} on_click={Box::new(move || primary_mode.update(|v| *v = !*v)) as Box<dyn Fn()>} disabled={false} />
                    <StyledButton variant={ButtonVariant::Secondary} size={ButtonSize::Md} label={"Secondary".to_owned()} state={ButtonState::Normal} on_click={Box::new(move || secondary_mode.update(|v| *v = !*v)) as Box<dyn Fn()>} disabled={false} />
                    <StyledButton variant={ButtonVariant::Primary} size={ButtonSize::Md} label={"Disabled".to_owned()} state={ButtonState::Normal} on_click={Box::new(|| {}) as Box<dyn Fn()>} disabled={true} />
                </CardRow>
                <Text secondary={true} align={TextAlign::Start}>"These preview buttons switch the color scheme."</Text>
            </Card>
        </RawView>
    })
}
