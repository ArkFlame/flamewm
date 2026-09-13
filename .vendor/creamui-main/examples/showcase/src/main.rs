//! Native entry point — the actual app lives in the `showcase` library
//! crate (`src/lib.rs`) so `demo/showcase` (the WASM build) can reuse it.

fn main() {
    showcase::launch();
}
