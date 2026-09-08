pub mod codec;
pub mod layout;
pub mod model;
pub mod paint;

pub use codec::{decode, encode};
pub use layout::{LayoutBox, LayoutEngine, LayoutResult};
pub use model::*;
pub use paint::{
    PaintCommand, build_paint_commands, build_paint_commands_with_scroll, emit_scroll_chrome,
    scroll_clip, scrollbar_visible,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionPhase {
    Press,
    Hover,
    Motion,
    Release,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionEvent {
    pub action: String,
    pub phase: ActionPhase,
    pub button: u32,
    pub x: f32,
    pub y: f32,
    pub inside: bool,
    pub time_ms: u64,
    pub text: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ControllerEvent {
    Action(ActionEvent),
}

pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
