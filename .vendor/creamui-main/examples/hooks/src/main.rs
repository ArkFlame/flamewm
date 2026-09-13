//! Demonstrates `use_theme()` and `AppBuilder::on_panic`.
//!
//! `use_theme()` reads the theme passed via `WindowOptions::theme` without
//! threading it through `build_ui` by hand. The "Crash" button makes
//! `build_ui` panic once; `on_panic` catches it and logs it, and the window
//! keeps running instead of the app crashing.
//!
//! `build_ui` also reruns on its own between clicks (e.g. caret blink on a
//! focused widget), so the panic condition is gated by `last_seen_trigger`
//! (a plain `Cell`, not a `Signal`) to fire exactly once per click rather
//! than on every such rerun.

use creamui_core::layout::Dimension;
use creamui_core::{BoxedWidget, Size};
use creamui_macros::jsx;
use creamui_reactive::Signal;
use creamui_render::{AppBuilder, PanicDetails, WindowOptions};
use creamui_theme::{use_theme, Theme};
use creamui_widgets::layout::{centered, column, padding};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn main() {
    let count = Signal::new(0_i32);
    let crash_trigger = Signal::new(0_u32);
    let last_seen_trigger: Rc<Cell<u32>> = Rc::new(Cell::new(0));
    let panic_log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

    AppBuilder::new()
        .on_panic({
            let panic_log = panic_log.clone();
            move |details: &PanicDetails| {
                eprintln!("[hooks] recovered: {}", details.message);
                panic_log.borrow_mut().push(details.message.clone());
            }
        })
        .window(
            WindowOptions {
                title: "CreamUI — Hooks".into(),
                width: 480,
                height: 420,
                theme: Theme::dark(),
                ..Default::default()
            },
            Theme::dark().surface,
            |_| {},
            {
                let count = count.clone();
                let crash_trigger = crash_trigger.clone();
                let last_seen_trigger = last_seen_trigger.clone();
                let panic_log = panic_log.clone();
                move |size: Size| -> BoxedWidget {
                    let theme = use_theme();

                    let trigger = crash_trigger.get();
                    if trigger != last_seen_trigger.get() {
                        last_seen_trigger.set(trigger);
                        let items: Vec<i32> = Vec::new();
                        let _ = items[0];
                    }

                    let mut style = centered(padding(column(10.0), 24.0));
                    style.size = creamui_core::layout::Size {
                        width: Dimension::Length(size.width),
                        height: Dimension::Length(size.height),
                    };

                    let recovered = panic_log.borrow().len();
                    let last_panic = panic_log
                        .borrow()
                        .last()
                        .cloned()
                        .unwrap_or_else(|| "none yet".to_string());

                    let click_count = count.clone();
                    let click_crash = crash_trigger.clone();
                    Box::new(jsx! {
                        <RawView style={style} background={theme.surface}>
                            <Heading>"use_theme() / AppBuilder::on_panic"</Heading>
                            <Text>{format!("Clicked {} times", count.get())}</Text>
                            <Button on_click={move || click_count.update(|value| *value += 1)}>
                                "Click me"
                            </Button>
                            <Button on_click={move || click_crash.update(|c| *c += 1)}>
                                "Crash build_ui"
                            </Button>
                            <Text>
                                {format!("Recovered panics: {recovered} — last: {last_panic}")}
                            </Text>
                        </RawView>
                    })
                }
            },
        )
        .run();
}
