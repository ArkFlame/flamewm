//! A complete application that depends only on the `creamui` facade crate.

use creamui::core::layout::{AlignItems, Dimension, FlexDirection, JustifyContent, Style};
use creamui::{jsx, run, use_theme, BoxedWidget, Signal, Size, Theme, WindowOptions};

fn main() {
    let clicks = Signal::new(0_u32);
    run(
        WindowOptions {
            title: "CreamUI — Facade".into(),
            width: 480,
            height: 320,
            theme: Theme::default(),
            ..Default::default()
        },
        Theme::default().surface,
        |_| {},
        move |viewport: Size| -> BoxedWidget {
            let theme = use_theme();
            let increment = clicks.clone();
            let style = Style {
                size: creamui::core::layout::Size {
                    width: Dimension::Length(viewport.width),
                    height: Dimension::Length(viewport.height),
                },
                flex_direction: FlexDirection::Column,
                justify_content: Some(JustifyContent::Center),
                align_items: Some(AlignItems::Center),
                ..Default::default()
            };
            Box::new(jsx! {
                <RawView style={style} background={theme.surface}>
                    <Text font_size={28.0}>"Hello, CreamUI!"</Text>
                    <Button on_click={move || increment.update(|n| *n += 1)}>
                        {format!("Clicked {} times", clicks.get())}
                    </Button>
                </RawView>
            })
        },
    );
}
