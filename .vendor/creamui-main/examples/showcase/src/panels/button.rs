use crate::prelude::*;

/// The "Button" panel: the default themed `Button`, a couple of
/// semantically-colored variants built straight from `RawButton`, and a
/// disabled-looking one, plus a click counter to prove the handlers fire.
#[component]
pub fn ButtonPanel(clicks: Signal<i32>) -> BoxedWidget {
    let theme = use_theme();
    let actions: Vec<BoxedWidget> = [
        ("Continue", ButtonVariant::Primary),
        ("Cancel", ButtonVariant::Secondary),
        ("Learn more", ButtonVariant::Tertiary),
        ("Delete", ButtonVariant::Destructive),
    ]
    .into_iter()
    .map(|(label, variant)| {
        let clicks = clicks.clone();
        jsx! {
            <StyledButton
                variant={variant}
                size={ButtonSize::Md}
                label={label.to_owned()}
                state={ButtonState::Normal}
                on_click={Box::new(move || clicks.update(|c| *c += 1)) as Box<dyn Fn()>}
                disabled={false}
            />
        }
    })
    .collect();

    let sizes: Vec<BoxedWidget> = [
        ("Extra small", ButtonSize::Xs),
        ("Small", ButtonSize::Sm),
        ("Medium", ButtonSize::Md),
        ("Large", ButtonSize::Lg),
    ]
    .into_iter()
    .map(|(label, size)| {
        let clicks = clicks.clone();
        jsx! {
            <StyledButton
                variant={ButtonVariant::Secondary}
                size={size}
                label={label.to_owned()}
                state={ButtonState::Normal}
                on_click={Box::new(move || clicks.update(|c| *c += 1)) as Box<dyn Fn()>}
                disabled={false}
            />
        }
    })
    .collect();

    Box::new(jsx! {
        <RawView style={column(section_gap())}>
            <SectionHeader title={"Buttons".to_owned()} subtitle={"A clear hierarchy, from everyday actions to important decisions.".to_owned()} />
            <Card gap={theme.spacing_medium}>
                <FieldLabel text={"Variants".to_owned()} />
                <RawView style={row(10.)} children={actions} />
            </Card>
            <Card gap={theme.spacing_medium}>
                <FieldLabel text={"One family, four sizes".to_owned()} />
                <RawView style={row(10.)} children={sizes} />
            </Card>
            <Card gap={theme.spacing_medium}>
                <FieldLabel text={"States".to_owned()} />
                <RawView style={row(10.)}>
                    <StyledButton variant={ButtonVariant::Primary} size={ButtonSize::Md} label={"Unavailable".to_owned()} state={ButtonState::Normal} on_click={Box::new(|| {}) as Box<dyn Fn()>} disabled={true} />
                    <StyledButton variant={ButtonVariant::Primary} size={ButtonSize::Md} label={"Working".to_owned()} state={ButtonState::Loading} on_click={Box::new(|| {}) as Box<dyn Fn()>} disabled={false} />
                    <StyledButton variant={ButtonVariant::Primary} size={ButtonSize::Md} label={"Saved".to_owned()} state={ButtonState::Success} on_click={Box::new(|| {}) as Box<dyn Fn()>} disabled={false} />
                </RawView>
                <RawText color={theme.text_secondary} font_size={12.0} align={TextAlign::Start}>{format!("{} actions · Try Tab, then Enter or Space", clicks.get())}</RawText>
            </Card>
        </RawView>
    })
}
