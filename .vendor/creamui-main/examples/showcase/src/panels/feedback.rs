use crate::prelude::*;

/// Determinate/indeterminate feedback plus a regular floating popover and a
/// modal alert. The alert itself is attached at the root below.
#[component]
pub fn FeedbackPanel(
    progress: Signal<f32>,
    show_popover: Signal<bool>,
    show_alert: Signal<bool>,
) -> BoxedWidget {
    let theme = use_theme();
    let value = progress.get();
    let set_progress = progress.clone();
    let open_popover = show_popover.clone();
    let open_alert = show_alert.clone();
    let popover: BoxedWidget = if show_popover.get() {
        jsx! {
            <Popover style={padding(column(theme.spacing_small), theme.spacing_medium)}>
                <BoldText text={"Popover".to_owned()} color={theme.text_primary} font_size={13.0} align={TextAlign::Center} />
                <RawText color={theme.text_secondary} font_size={12.0}>"A floating surface can contain any widget tree."</RawText>
            </Popover>
        }
    } else {
        Box::new(jsx! { <RawView style={Style::default()} /> })
    };
    let progress_detail: BoxedWidget = Box::new(jsx! {
        <RawView style={column(theme.spacing_small)}>
            <ProgressBar value={Some(value)} />
            <Slider value={value} on_change={move |next| set_progress.set(next)} />
        </RawView>
    });
    let progress_ring_detail: BoxedWidget = Box::new(jsx! {
        <RawView style={row(theme.spacing_medium)}>
            <ProgressRing value={Some(value)} size={32.0} />
            <ProgressRing value={None} size={32.0} />
        </RawView>
    });
    Box::new(jsx! {
        <RawView style={column(section_gap())}>
            <SectionHeader title={"Feedback & overlays".to_owned()} subtitle={"Show work in progress, surface contextual detail, and ask for confirmation without losing context.".to_owned()} />
            <FieldCard label={format!("Determinate progress · {:.0}%", value * 100.0)} control={progress_detail} />
            <CardRow>
                <FieldCard label={"Progress ring".to_owned()} control={progress_ring_detail} />
                <FieldCard label={"Indeterminate bar".to_owned()} control={jsx!{<ProgressBar value={None} />}} />
            </CardRow>
            <Card gap={theme.spacing_medium}>
                <FieldLabel text={"Overlays".to_owned()} />
                <RawView style={row(theme.spacing_medium)}>
                    <StyledButton variant={ButtonVariant::Secondary} size={ButtonSize::Md} label={"Toggle popover".to_owned()} state={ButtonState::Normal} on_click={Box::new(move || open_popover.update(|open| *open = !*open)) as Box<dyn Fn()>} disabled={false} />
                    <StyledButton variant={ButtonVariant::Primary} size={ButtonSize::Md} label={"Open alert dialog".to_owned()} state={ButtonState::Normal} on_click={Box::new(move || open_alert.set(true)) as Box<dyn Fn()>} disabled={false} />
                </RawView>
                {popover}
            </Card>
        </RawView>
    })
}
