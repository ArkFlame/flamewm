//! WASM entry point for the CreamUI component showcase.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Starts the showcase in the canvas appended by winit to this document.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn start() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    showcase::launch();
}
