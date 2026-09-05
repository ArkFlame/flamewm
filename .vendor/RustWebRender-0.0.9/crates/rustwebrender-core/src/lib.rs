pub mod codec;
pub mod layout;
pub mod model;
pub mod paint;

pub use codec::{decode, encode};
pub use layout::{LayoutBox, LayoutEngine, LayoutResult};
pub use model::*;
pub use paint::{build_paint_commands, PaintCommand};

pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
