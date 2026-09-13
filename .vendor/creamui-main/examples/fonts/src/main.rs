//! Demonstrates the font registry: `register_file` at runtime, `use_font()`,
//! and `.font_family()`'s CSS-style fallback stack on `Text`/`Heading`.

use creamui_core::layout::Dimension;
use creamui_core::{BoxedWidget, Size, TextAlign};
use creamui_fonts::{register_file, use_font, FontWeight, DEFAULT_FAMILY};
use creamui_macros::jsx;
use creamui_render::{run, WindowOptions};
use creamui_theme::{use_theme, Theme};
use creamui_widgets::layout::{centered, column, padding};

const CUSTOM_FAMILY: &str = "Liberation Serif";

/// Tries a handful of common system paths for a serif font, so the example
/// works without shipping a font file of its own. `.font_family`'s CSS-style
/// fallback falls back to `DEFAULT_FAMILY` wherever none of these exist.
fn register_system_font() -> bool {
    const CANDIDATES: &[&str] = &[
        "/usr/share/fonts/liberation-serif-fonts/LiberationSerif-Regular.ttf",
        "/usr/share/fonts/liberation-serif/LiberationSerif-Regular.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
        "/usr/share/fonts/TTF/LiberationSerif-Regular.ttf",
        "/Library/Fonts/Georgia.ttf",
        "C:\\Windows\\Fonts\\georgia.ttf",
    ];
    CANDIDATES
        .iter()
        .any(|path| register_file(CUSTOM_FAMILY, FontWeight::Regular, path).is_ok())
}

fn main() {
    let custom_font_loaded = register_system_font();

    run(
        WindowOptions {
            title: "CreamUI — Fonts".into(),
            width: 560,
            height: 360,
            theme: Theme::dark(),
            ..Default::default()
        },
        Theme::dark().surface,
        |_| {},
        move |size: Size| -> BoxedWidget {
            let theme = use_theme();
            let custom_glyphs = use_font(format!("{CUSTOM_FAMILY}, {DEFAULT_FAMILY}"))
                .regular
                .glyph_count();

            let mut style = centered(padding(column(10.0), 24.0));
            style.size = creamui_core::layout::Size {
                width: Dimension::Length(size.width),
                height: Dimension::Length(size.height),
            };

            let font_status = if custom_font_loaded {
                format!("{CUSTOM_FAMILY} loaded from disk ({custom_glyphs} glyphs)")
            } else {
                format!(
                    "{CUSTOM_FAMILY} not found on this machine — showing {DEFAULT_FAMILY} instead"
                )
            };

            Box::new(jsx! {
                <RawView style={style} background={theme.surface}>
                    <Heading>"register_file() / use_font() / .font_family()"</Heading>
                    <Text align={TextAlign::Start}>
                        {format!("Default theme family: {}", theme.font_family)}
                    </Text>
                    <Text font_family={format!("{CUSTOM_FAMILY}, {DEFAULT_FAMILY}")}>
                        {format!("This line asks for \"{CUSTOM_FAMILY}, {DEFAULT_FAMILY}\"")}
                    </Text>
                    <Text align={TextAlign::Start}>{font_status}</Text>
                </RawView>
            })
        },
    );
}
