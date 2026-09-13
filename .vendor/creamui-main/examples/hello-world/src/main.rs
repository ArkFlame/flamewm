use creamui_core::layout::FlexDirection;
use creamui_core::{BoxedWidget, Size};
use creamui_macros::jsx;
use creamui_reactive::Signal;
use creamui_render::{run, WindowOptions};
use creamui_theme::{use_theme, Theme};
use creamui_widgets::layout::{Align, Justify};

fn main() {
    let count = Signal::new(0_i32);
    run(
        WindowOptions {
            title: "CreamUI — Hello World".into(),
            width: 480,
            height: 320,
            theme: Theme::dark(),
            ..Default::default()
        },
        Theme::dark().surface,
        |_| {},
        move |size: Size| -> BoxedWidget {
            let theme = use_theme();
            let click_count = count.clone();
            Box::new(jsx! {
                <Flex direction={FlexDirection::Column} size={(size.width, size.height)} gap={12.0} justify={Justify::Center} align={Align::Center} background={theme.surface}>
                    <Text font_size={28.0}>"Hello, CreamUI!"</Text>
                    <Text>{format!("Clicked {} times", count.get())}</Text>
                    <Button on_click={move || click_count.update(|value| *value += 1)}>"Click me"</Button>
                </Flex>
            })
        },
    );
}
