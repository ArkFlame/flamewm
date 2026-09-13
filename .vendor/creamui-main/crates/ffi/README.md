# creamui-ffi

C ABI and shared-library build for CreamUI.

The crate exports a `cdylib` named `creamui` alongside its Rust library. It is intended for hosts that need to load CreamUI dynamically or call it from another language. Native Rust applications should normally use the regular CreamUI crates directly.

See the [ABI guide](https://github.com/sammwyy/creamui/blob/main/docs/ffi.md) for the role of this crate in the dynamic-runtime stack.
