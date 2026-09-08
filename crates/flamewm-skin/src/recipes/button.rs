use crate::palette::Rgb;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonRecipe {
    pub width: u16,
    pub height: u16,
    pub background: Rgb,
    pub text: Rgb,
}

/// CSS `button`: transparent, zero padding, 13px text; title buttons use chrome metrics.
pub const TITLE_BUTTON: ButtonRecipe = ButtonRecipe {
    width: crate::DEFAULT.chrome.button_width,
    height: crate::DEFAULT.chrome.button_height,
    background: Rgb(0x000000),
    text: Rgb(0xf1f2f3),
};
pub const fn title_button_size() -> (u16, u16) {
    (
        crate::DEFAULT.chrome.button_width,
        crate::DEFAULT.chrome.button_height,
    )
}
