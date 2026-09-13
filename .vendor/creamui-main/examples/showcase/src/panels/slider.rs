use crate::prelude::*;

fn slider_row(label: &str, value: Signal<f32>, format: impl Fn(f32) -> String) -> BoxedWidget {
    let theme = use_theme();
    let current = value.get();
    let set = value.clone();
    Box::new(jsx! {
        <RawView style={column(theme.spacing_small)}>
            <Text align={TextAlign::Start} color={theme.text_secondary} style={label_style()}>{label.to_owned()}</Text>
            <Slider value={current} on_change={move |v| set.set(v)} />
            <Text align={TextAlign::Start} color={theme.text_disabled} style={label_style()}>{format(current)}</Text>
        </RawView>
    })
}

/// The "Slider" panel: three independent sliders, each with its live value
/// printed underneath.
#[component]
pub fn SliderPanel(volume: Signal<f32>, brightness: Signal<f32>, zoom: Signal<f32>) -> BoxedWidget {
    let theme = use_theme();
    let volume_row = slider_row("Volume", volume, |v| format!("{:.0}%", v * 100.0));
    let brightness_row = slider_row("Brightness", brightness, |v| format!("{:.0}%", v * 100.0));
    let zoom_row = slider_row("Zoom", zoom, |v| format!("{:.2}x", 0.5 + v * 1.5));
    Box::new(jsx! {
        <RawView style={column(section_gap())}>
            <SectionHeader title={"Slider".to_owned()} subtitle={"Fine adjustments with immediate feedback.".to_owned()} />
            <Card gap={theme.spacing_large}>
                {volume_row}
                {brightness_row}
                {zoom_row}
            </Card>
        </RawView>
    })
}
